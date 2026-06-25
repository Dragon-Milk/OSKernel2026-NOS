use alloc::{boxed::Box, vec, vec::Vec};
use core::net::Ipv4Addr;

use axerrno::{AxError, AxResult, LinuxError};
use axio::prelude::*;
use axnet::{
    CMsgData, RecvFlags, RecvOptions, SendFlags, SendOptions, Socket as SocketInner, SocketAddrEx,
    SocketOps,
};
use linux_raw_sys::general::timespec;
use linux_raw_sys::net::{
    MSG_DONTWAIT, MSG_ERRQUEUE, MSG_MORE, MSG_OOB, MSG_PEEK, MSG_TRUNC, SCM_RIGHTS, SOL_SOCKET,
    cmsghdr, mmsghdr, msghdr, sockaddr, socklen_t,
};

use super::addr::SocketAddrExt;
use crate::{
    file::{FileLike, Socket, add_file_like},
    mm::{check_access, IoVec, IoVectorBuf, UserConstPtr, UserPtr, VmBytes, VmBytesMut},
    syscall::net::{CMsg, CMsgBuilder},
};

fn read_send_data(mut src: impl Read + IoBuf) -> AxResult<Vec<u8>> {
    let mut data = vec![0; src.remaining()];
    let read = src.read(&mut data)?;
    data.truncate(read);
    Ok(data)
}

fn send_impl(
    fd: i32,
    mut src: impl Read + IoBuf,
    flags: u32,
    addr: UserConstPtr<sockaddr>,
    addrlen: socklen_t,
    cmsg: Vec<CMsgData>,
) -> AxResult<isize> {
    let addr = if addr.is_null() || addrlen == 0 {
        None
    } else {
        Some(SocketAddrEx::read_from_user(addr, addrlen)?)
    };

    debug!("sys_send <= fd: {fd}, flags: {flags}, addr: {addr:?}");

    let socket = Socket::from_fd(fd)?;
    if flags & MSG_OOB != 0 && matches!(&socket.0, SocketInner::Udp(_)) {
        return Err(AxError::OperationNotSupported);
    }
    let is_tcp = matches!(&socket.0, SocketInner::Tcp(_));
    let is_udp = matches!(&socket.0, SocketInner::Udp(_));

    if flags & MSG_MORE != 0 && is_udp {
        let data = read_send_data(src)?;
        let sent = data.len();
        socket.append_pending_send(data, addr, cmsg);
        return Ok(sent as isize);
    }

    if let Some(mut pending) = socket.take_pending_send() {
        let data = read_send_data(src)?;
        let sent = data.len();
        pending.data.extend(data);
        if pending.to.is_none() {
            pending.to = addr;
        }
        pending.cmsg.extend(cmsg);
        if pending.data.len() > 65_507 && is_udp {
            return Err(LinuxError::EMSGSIZE.into());
        }

        let mut combined = pending.data.as_slice();
        socket
            .send(
                &mut combined,
                SendOptions {
                    to: pending.to,
                    flags: SendFlags::default(),
                    cmsg: pending.cmsg,
                },
            )
            .map_err(|err| {
                if is_tcp && err == AxError::NotConnected {
                    AxError::BrokenPipe
                } else {
                    err
                }
            })?;
        return Ok(sent as isize);
    }

    let sent = socket
        .send(
            &mut src,
            SendOptions {
                to: addr,
                flags: SendFlags::default(),
                cmsg,
            },
        )
        .map_err(|err| {
            if is_tcp && err == AxError::NotConnected {
                AxError::BrokenPipe
            } else {
                err
            }
        })?;

    Ok(sent as isize)
}

pub fn sys_sendto(
    fd: i32,
    buf: *const u8,
    len: usize,
    flags: u32,
    addr: UserConstPtr<sockaddr>,
    addrlen: socklen_t,
) -> AxResult<isize> {
    let socket = Socket::from_fd(fd)?;
    if len != 0 {
        check_access(buf as usize, len).map_err(|_| AxError::BadAddress)?;
    }
    if len > 65_507 && matches!(&socket.0, SocketInner::Udp(_)) {
        return Err(LinuxError::EMSGSIZE.into());
    }
    let (addr, addrlen) = if matches!(&socket.0, SocketInner::Tcp(_)) {
        (UserConstPtr::from(0), 0)
    } else {
        (addr, addrlen)
    };
    send_impl(fd, VmBytes::new(buf, len), flags, addr, addrlen, Vec::new())
}

fn send_msg(fd: i32, msg: &msghdr, flags: u32) -> AxResult<isize> {
    if msg.msg_iovlen > 1024 {
        return Err(LinuxError::EMSGSIZE.into());
    }
    let mut cmsg = Vec::new();
    if !msg.msg_control.is_null() {
        let mut ptr = msg.msg_control as usize;
        let ptr_end = ptr + msg.msg_controllen;
        while ptr + size_of::<cmsghdr>() <= ptr_end {
            let hdr = UserConstPtr::<cmsghdr>::from(ptr).get_as_ref()?;
            if ptr_end - ptr < hdr.cmsg_len {
                return Err(AxError::InvalidInput);
            }
            cmsg.push(Box::new(CMsg::parse(hdr)?) as CMsgData);
            ptr += hdr.cmsg_len;
        }
    }
    send_impl(
        fd,
        IoVectorBuf::new(msg.msg_iov as *const IoVec, msg.msg_iovlen)?.into_io(),
        flags,
        UserConstPtr::from(msg.msg_name as usize),
        msg.msg_namelen as socklen_t,
        cmsg,
    )
}

pub fn sys_sendmsg(fd: i32, msg: UserConstPtr<msghdr>, flags: u32) -> AxResult<isize> {
    send_msg(fd, msg.get_as_ref()?, flags)
}

pub fn sys_sendmmsg(
    fd: i32,
    msgvec: UserPtr<mmsghdr>,
    vlen: usize,
    flags: u32,
) -> AxResult<isize> {
    Socket::from_fd(fd)?;
    if vlen == 0 || vlen > 1024 {
        return Err(AxError::InvalidInput);
    }

    let messages = msgvec.get_as_mut_slice(vlen)?;
    let mut sent_count = 0;
    for message in messages {
        match send_msg(fd, &message.msg_hdr, flags) {
            Ok(sent) => {
                message.msg_len = sent as _;
                sent_count += 1;
            }
            Err(err) if sent_count == 0 => return Err(err),
            Err(_) => break,
        }
    }
    Ok(sent_count)
}

fn recv_impl(
    fd: i32,
    mut dst: impl Write + IoBufMut,
    flags: u32,
    addr: UserPtr<sockaddr>,
    addrlen: UserPtr<socklen_t>,
    cmsg_builder: Option<CMsgBuilder>,
) -> AxResult<isize> {
    debug!("sys_recv <= fd: {fd}, flags: {flags}");

    let socket = Socket::from_fd(fd)?;
    if flags & MSG_OOB != 0 {
        return Err(AxError::InvalidInput);
    }
    if flags & MSG_ERRQUEUE != 0 {
        return Err(AxError::WouldBlock);
    }
    let mut recv_flags = RecvFlags::empty();
    if flags & MSG_PEEK != 0 {
        recv_flags |= RecvFlags::PEEK;
    }
    if flags & MSG_TRUNC != 0 {
        recv_flags |= RecvFlags::TRUNCATE;
    }

    let mut cmsg = Vec::new();

    let write_remote_addr = !addr.is_null();
    let mut remote_addr = (write_remote_addr || matches!(&socket.0, SocketInner::Udp(_)))
        .then(|| SocketAddrEx::Ip((Ipv4Addr::UNSPECIFIED, 0).into()));
    let restore_blocking = flags & MSG_DONTWAIT != 0 && !socket.nonblocking();
    if restore_blocking {
        socket.set_nonblocking(true)?;
    }
    let recv_result = socket.recv(
        &mut dst,
        RecvOptions {
            from: remote_addr.as_mut(),
            flags: recv_flags,
            cmsg: Some(&mut cmsg),
        },
    );
    if restore_blocking {
        let restore_result = socket.set_nonblocking(false);
        if recv_result.is_ok() {
            restore_result?;
        }
    }
    let recv = recv_result?;

    if recv != 0 && write_remote_addr {
        if let Some(remote_addr) = remote_addr {
            remote_addr.write_to_user(addr, addrlen.get_as_mut()?)?;
        }
    }

    if let Some(mut builder) = cmsg_builder {
        for cmsg in cmsg {
            let Ok(cmsg) = cmsg.downcast::<CMsg>() else {
                warn!("received unexpected cmsg");
                continue;
            };

            let pushed = match *cmsg {
                CMsg::Rights { fds } => builder.push(SOL_SOCKET, SCM_RIGHTS, |data| {
                    let mut written = 0;
                    for (f, chunk) in fds.into_iter().zip(data.chunks_exact_mut(size_of::<i32>())) {
                        let fd = add_file_like(f, false)?;
                        chunk.copy_from_slice(&fd.to_ne_bytes());
                        written += size_of::<i32>();
                    }
                    Ok(written)
                })?,
            };
            if !pushed {
                break;
            }
        }
    }

    debug!("sys_recv => fd: {fd}, recv: {recv}");
    Ok(recv as isize)
}

pub fn sys_recvfrom(
    fd: i32,
    buf: *mut u8,
    len: usize,
    flags: u32,
    addr: UserPtr<sockaddr>,
    addrlen: UserPtr<socklen_t>,
) -> AxResult<isize> {
    let socket = Socket::from_fd(fd)?;
    let (addr, addrlen) = if matches!(&socket.0, SocketInner::Tcp(_)) {
        if !addr.is_null() && *addrlen.get_as_mut()? > 4096 {
            return Err(AxError::InvalidInput);
        }
        (UserPtr::from(0), UserPtr::from(0))
    } else {
        (addr, addrlen)
    };
    recv_impl(fd, VmBytesMut::new(buf, len), flags, addr, addrlen, None)
}

fn recv_msg(fd: i32, msg: &mut msghdr, flags: u32) -> AxResult<isize> {
    if msg.msg_iovlen > 1024 {
        return Err(LinuxError::EMSGSIZE.into());
    }
    recv_impl(
        fd,
        IoVectorBuf::new(msg.msg_iov as *mut IoVec, msg.msg_iovlen)?.into_io(),
        flags,
        UserPtr::from(msg.msg_name as usize),
        UserPtr::from(&mut msg.msg_namelen as *mut _ as *mut socklen_t),
        (!msg.msg_control.is_null()).then(|| {
            CMsgBuilder::new(
                UserPtr::from(msg.msg_control as *mut cmsghdr),
                &mut msg.msg_controllen,
            )
        }),
    )
}

pub fn sys_recvmsg(fd: i32, msg: UserPtr<msghdr>, flags: u32) -> AxResult<isize> {
    recv_msg(fd, msg.get_as_mut()?, flags)
}

pub fn sys_recvmmsg(
    fd: i32,
    msgvec: UserPtr<mmsghdr>,
    vlen: usize,
    flags: u32,
    timeout: UserConstPtr<timespec>,
) -> AxResult<isize> {
    Socket::from_fd(fd)?;
    if vlen == 0 || vlen > 1024 {
        return Err(AxError::InvalidInput);
    }

    let messages = msgvec.get_as_mut_slice(vlen)?;
    if !timeout.is_null() {
        let timeout = timeout.get_as_ref()?;
        if timeout.tv_sec < 0 || timeout.tv_nsec < 0 || timeout.tv_nsec >= 1_000_000_000 {
            return Err(AxError::InvalidInput);
        }
    }

    let mut received_count = 0;
    for message in messages {
        match recv_msg(fd, &mut message.msg_hdr, flags) {
            Ok(received) => {
                message.msg_len = received as _;
                received_count += 1;
            }
            Err(err) if received_count == 0 => return Err(err),
            Err(_) => break,
        }
    }
    Ok(received_count)
}
