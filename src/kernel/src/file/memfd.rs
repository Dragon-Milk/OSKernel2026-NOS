use alloc::{borrow::Cow, string::String, vec::Vec};
use core::{cmp::min, task::Context};

use axerrno::{AxError, AxResult};
use axpoll::{IoEvents, Pollable};
use linux_raw_sys::general::{
    F_SEAL_GROW, F_SEAL_SEAL, F_SEAL_SHRINK, F_SEAL_WRITE, O_RDWR,
};
use spin::Mutex;

use super::{FileLike, IoDst, IoSrc, Kstat};

const SUPPORTED_SEALS: u32 = F_SEAL_SEAL | F_SEAL_SHRINK | F_SEAL_GROW | F_SEAL_WRITE;

pub struct MemFd {
    name: String,
    inner: Mutex<MemFdInner>,
}

struct MemFdInner {
    data: Vec<u8>,
    offset: usize,
    seals: u32,
}

impl MemFd {
    pub fn new(name: String, allow_sealing: bool) -> Self {
        Self {
            name,
            inner: Mutex::new(MemFdInner {
                data: Vec::new(),
                offset: 0,
                seals: if allow_sealing { 0 } else { F_SEAL_SEAL },
            }),
        }
    }

    pub fn seals(&self) -> u32 {
        self.inner.lock().seals
    }

    pub fn add_seals(&self, seals: u32) -> AxResult<()> {
        if seals & !SUPPORTED_SEALS != 0 {
            return Err(AxError::InvalidInput);
        }
        let mut inner = self.inner.lock();
        if inner.seals & F_SEAL_SEAL != 0 {
            return Err(AxError::OperationNotPermitted);
        }
        inner.seals |= seals;
        Ok(())
    }

    pub fn set_len(&self, len: u64) -> AxResult<()> {
        let len: usize = len.try_into().map_err(|_| AxError::NoMemory)?;
        let mut inner = self.inner.lock();
        if len < inner.data.len() && inner.seals & F_SEAL_SHRINK != 0 {
            return Err(AxError::OperationNotPermitted);
        }
        if len > inner.data.len() && inner.seals & F_SEAL_GROW != 0 {
            return Err(AxError::OperationNotPermitted);
        }
        inner.data.resize(len, 0);
        if inner.offset > len {
            inner.offset = len;
        }
        Ok(())
    }
}

impl FileLike for MemFd {
    fn read(&self, dst: &mut IoDst) -> AxResult<usize> {
        let mut inner = self.inner.lock();
        let available = inner.data.len().saturating_sub(inner.offset);
        let count = min(dst.remaining_mut(), available);
        if count == 0 {
            return Ok(0);
        }
        let start = inner.offset;
        let end = start + count;
        let written = dst.write(&inner.data[start..end])?;
        inner.offset += written;
        Ok(written)
    }

    fn write(&self, src: &mut IoSrc) -> AxResult<usize> {
        let mut inner = self.inner.lock();
        if inner.seals & F_SEAL_WRITE != 0 {
            return Err(AxError::OperationNotPermitted);
        }
        let count = src.remaining();
        if count == 0 {
            return Ok(0);
        }
        let start = inner.offset;
        let end = start.checked_add(count).ok_or(AxError::NoMemory)?;
        if end > inner.data.len() {
            if inner.seals & F_SEAL_GROW != 0 {
                return Err(AxError::OperationNotPermitted);
            }
            inner.data.resize(end, 0);
        }
        let read = src.read(&mut inner.data[start..end])?;
        inner.offset += read;
        Ok(read)
    }

    fn stat(&self) -> AxResult<Kstat> {
        let inner = self.inner.lock();
        Ok(Kstat {
            mode: linux_raw_sys::general::S_IFREG | 0o777,
            size: inner.data.len() as u64,
            ..Default::default()
        })
    }

    fn path(&self) -> Cow<'_, str> {
        Cow::Owned(alloc::format!("memfd:{}", self.name))
    }

    fn access_mode(&self) -> u32 {
        O_RDWR
    }

    fn file_position(&self) -> u64 {
        self.inner.lock().offset as u64
    }
}

impl Pollable for MemFd {
    fn poll(&self) -> IoEvents {
        IoEvents::IN | IoEvents::OUT
    }

    fn register(&self, _context: &mut Context<'_>, _events: IoEvents) {}
}
