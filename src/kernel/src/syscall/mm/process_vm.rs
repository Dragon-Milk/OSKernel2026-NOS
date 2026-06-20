use alloc::vec;

use axerrno::{AxError, AxResult};
use axio::{IoBuf, IoBufMut, Read, Write};
use axtask::current;

use crate::{
    mm::{IoVec, IoVectorBuf},
    task::{AsThread, get_process_data},
};

fn check_process_vm_target(pid: i32) -> AxResult<()> {
    let pid: u32 = pid.try_into().map_err(|_| AxError::NoSuchProcess)?;
    let curr = current();
    let curr_proc = &curr.as_thread().proc_data;
    if pid == 0 || pid == curr_proc.proc.pid() {
        return Ok(());
    }

    let target = get_process_data(pid)?;
    let (_, curr_euid, _, _, _, _) = curr_proc.ids();
    let (target_uid, target_euid, target_suid, _, _, _) = target.ids();
    if curr_euid == 0
        || curr_euid == target_uid
        || curr_euid == target_euid
        || curr_euid == target_suid
    {
        return Err(AxError::OperationNotPermitted);
    }
    Err(AxError::OperationNotPermitted)
}

fn process_vm_transfer(
    pid: i32,
    local_iov: *const IoVec,
    liovcnt: usize,
    remote_iov: *const IoVec,
    riovcnt: usize,
    flags: usize,
    write_remote: bool,
) -> AxResult<isize> {
    if flags != 0 {
        return Err(AxError::InvalidInput);
    }

    let local = IoVectorBuf::new(local_iov, liovcnt)?;
    let remote = IoVectorBuf::new(remote_iov, riovcnt)?;
    check_process_vm_target(pid)?;

    let mut local_io = local.into_io();
    let mut remote_io = remote.into_io();
    let len = local_io.remaining_mut().min(remote_io.remaining());
    if len == 0 {
        return Ok(0);
    }

    let mut buf = vec![0; len];
    let copied = if write_remote {
        let read = local_io.read(&mut buf)?;
        remote_io.write(&buf[..read])?
    } else {
        let read = remote_io.read(&mut buf)?;
        local_io.write(&buf[..read])?
    };
    Ok(copied as _)
}

pub fn sys_process_vm_readv(
    pid: i32,
    local_iov: *const IoVec,
    liovcnt: usize,
    remote_iov: *const IoVec,
    riovcnt: usize,
    flags: usize,
) -> AxResult<isize> {
    process_vm_transfer(pid, local_iov, liovcnt, remote_iov, riovcnt, flags, false)
}

pub fn sys_process_vm_writev(
    pid: i32,
    local_iov: *const IoVec,
    liovcnt: usize,
    remote_iov: *const IoVec,
    riovcnt: usize,
    flags: usize,
) -> AxResult<isize> {
    process_vm_transfer(pid, local_iov, liovcnt, remote_iov, riovcnt, flags, true)
}
