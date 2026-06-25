use alloc::{borrow::Cow, format, sync::Arc, vec::Vec};
use core::{ffi::c_int, ops::Deref, task::Context};

use axerrno::{AxError, AxResult};
use axnet::{
    CMsgData, RecvOptions, SendFlags, SendOptions, Socket as SocketInner, SocketAddrEx, SocketOps,
    options::{Configurable, GetSocketOption, SetSocketOption},
};
use axpoll::{IoEvents, Pollable};
use axsync::Mutex;
use linux_raw_sys::general::{O_RDWR, S_IFSOCK};

use super::{FileLike, Kstat};
use crate::file::{IoDst, IoSrc, get_file_like};

pub struct PendingSend {
    pub data: Vec<u8>,
    pub to: Option<SocketAddrEx>,
    pub cmsg: Vec<CMsgData>,
}

pub struct Socket(pub SocketInner, Mutex<Option<PendingSend>>);

impl Socket {
    pub fn new(inner: SocketInner) -> Self {
        Self(inner, Mutex::new(None))
    }

    pub fn append_pending_send(
        &self,
        data: Vec<u8>,
        to: Option<SocketAddrEx>,
        cmsg: Vec<CMsgData>,
    ) {
        let mut pending = self.1.lock();
        if let Some(pending) = pending.as_mut() {
            pending.data.extend(data);
            if pending.to.is_none() {
                pending.to = to;
            }
            pending.cmsg.extend(cmsg);
        } else {
            *pending = Some(PendingSend { data, to, cmsg });
        }
    }

    pub fn take_pending_send(&self) -> Option<PendingSend> {
        self.1.lock().take()
    }

    pub fn flush_pending_send(&self) -> AxResult<usize> {
        let Some(pending) = self.take_pending_send() else {
            return Ok(0);
        };

        let mut data = pending.data.as_slice();
        self.send(
            &mut data,
            SendOptions {
                to: pending.to,
                flags: SendFlags::default(),
                cmsg: pending.cmsg,
            },
        )
    }

    fn flush_pending_send_best_effort(&self) {
        let was_nonblocking = self.nonblocking();
        if !was_nonblocking {
            let _ = self.set_nonblocking(true);
        }
        let _ = self.flush_pending_send();
        if !was_nonblocking {
            let _ = self.set_nonblocking(false);
        }
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        self.flush_pending_send_best_effort();
    }
}

impl Deref for Socket {
    type Target = SocketInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FileLike for Socket {
    fn read(&self, dst: &mut IoDst) -> AxResult<usize> {
        self.recv(dst, RecvOptions::default())
    }

    fn write(&self, src: &mut IoSrc) -> AxResult<usize> {
        self.send(src, SendOptions::default())
    }

    fn stat(&self) -> AxResult<Kstat> {
        // TODO(mivik): implement stat for sockets
        Ok(Kstat {
            mode: S_IFSOCK | 0o777u32, // rwxrwxrwx
            blksize: 4096,
            ..Default::default()
        })
    }

    fn nonblocking(&self) -> bool {
        let mut result = false;
        self.get_option(GetSocketOption::NonBlocking(&mut result))
            .unwrap();
        result
    }

    fn set_nonblocking(&self, nonblocking: bool) -> AxResult<()> {
        self.0
            .set_option(SetSocketOption::NonBlocking(&nonblocking))
    }

    fn access_mode(&self) -> u32 {
        O_RDWR
    }

    fn path(&self) -> Cow<'_, str> {
        format!("socket:[{}]", self as *const _ as usize).into()
    }

    fn from_fd(fd: c_int) -> AxResult<Arc<Self>>
    where
        Self: Sized + 'static,
    {
        get_file_like(fd)?
            .downcast_arc()
            .map_err(|_| AxError::NotASocket)
    }
}
impl Pollable for Socket {
    fn poll(&self) -> IoEvents {
        self.0.poll()
    }

    fn register(&self, context: &mut Context<'_>, events: IoEvents) {
        self.0.register(context, events);
    }
}
