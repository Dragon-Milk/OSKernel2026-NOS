use alloc::vec::Vec;
use core::mem::{self, MaybeUninit};

use axerrno::{AxError, AxResult};
use axio::prelude::*;
use bytemuck::AnyBitPattern;
use starry_vm::{vm_read_slice, vm_write_slice};

use super::{UserConstPtr, UserPtr};

#[repr(C)]
#[derive(Debug, Copy, Clone, AnyBitPattern)]
pub struct IoVec {
    pub iov_base: *mut u8,
    pub iov_len: isize,
}

#[derive(Default)]
pub struct IoVectorBuf {
    iovs: Vec<IoVec>,
    iovcnt: usize,
    len: usize,
}

impl IoVectorBuf {
    pub fn new(iovs: *const IoVec, iovcnt: usize) -> AxResult<Self> {
        if iovcnt > 1024 {
            return Err(AxError::InvalidInput);
        }

        let iovs = if iovcnt == 0 {
            &[]
        } else {
            UserConstPtr::<IoVec>::from(iovs).get_as_slice(iovcnt)?
        };

        let mut len: usize = 0;
        let mut copied = Vec::with_capacity(iovcnt);
        for &iov in iovs {
            if iov.iov_len < 0 {
                return Err(AxError::InvalidInput);
            }
            len = len
                .checked_add(iov.iov_len as usize)
                .filter(|len| *len <= isize::MAX as usize)
                .ok_or(AxError::InvalidInput)?;
            copied.push(iov);
        }
        Ok(Self {
            iovs: copied,
            iovcnt,
            len,
        })
    }

    pub fn validate_readable(&self) -> AxResult<()> {
        for iov in &self.iovs {
            if iov.iov_len == 0 {
                continue;
            }
            UserConstPtr::<u8>::from(iov.iov_base as *const u8)
                .get_as_slice(iov.iov_len as usize)?;
        }
        Ok(())
    }

    pub fn validate_writable(&self) -> AxResult<()> {
        for iov in &self.iovs {
            if iov.iov_len == 0 {
                continue;
            }
            UserPtr::<u8>::from(iov.iov_base).get_as_mut_slice(iov.iov_len as usize)?;
        }
        Ok(())
    }

    pub fn read_with(
        self,
        mut f: impl FnMut(*const u8, usize) -> AxResult<usize>,
    ) -> AxResult<usize> {
        let mut count = 0;
        for iov in self.iovs {
            if iov.iov_len == 0 {
                continue;
            }
            let read = f(iov.iov_base, iov.iov_len as usize)?;
            if read == 0 {
                break;
            }
            count += read;
        }
        Ok(count)
    }

    pub fn fill_with(
        self,
        mut f: impl FnMut(*mut u8, usize) -> AxResult<usize>,
    ) -> AxResult<usize> {
        let mut count = 0;
        for iov in self.iovs {
            if iov.iov_len == 0 {
                continue;
            }
            let written = f(iov.iov_base, iov.iov_len as usize)?;
            if written == 0 {
                break;
            }
            count += written;
        }
        Ok(count)
    }

    pub fn into_io(self) -> IoVectorBufIo {
        IoVectorBufIo {
            inner: self,
            start: 0,
            offset: 0,
        }
    }
}

pub struct IoVectorBufIo {
    inner: IoVectorBuf,
    start: usize,
    offset: usize,
}

impl IoVectorBufIo {
    fn skip_empty(&mut self) -> AxResult<()> {
        while self.start < self.inner.iovcnt {
            let iov = self.inner.iovs[self.start];
            if iov.iov_len as usize > self.offset {
                break;
            }
            self.offset = 0;
            self.start += 1;
        }
        Ok(())
    }
}

impl Read for IoVectorBufIo {
    fn read(&mut self, buf: &mut [u8]) -> AxResult<usize> {
        let mut count = 0;
        loop {
            self.skip_empty()?;
            if self.start >= self.inner.iovcnt {
                break;
            }
            let iov = self.inner.iovs[self.start];
            let len = (iov.iov_len as usize - self.offset).min(buf.len() - count);
            if len == 0 {
                break;
            }
            vm_read_slice(iov.iov_base.wrapping_add(self.offset), unsafe {
                mem::transmute::<&mut [u8], &mut [MaybeUninit<u8>]>(&mut buf[count..count + len])
            })?;
            self.offset += len;
            self.inner.len -= len;
            count += len;
        }
        Ok(count)
    }
}

impl Write for IoVectorBufIo {
    fn write(&mut self, buf: &[u8]) -> AxResult<usize> {
        let mut count = 0;
        loop {
            self.skip_empty()?;
            if self.start >= self.inner.iovcnt {
                break;
            }
            let iov = self.inner.iovs[self.start];
            let len = (iov.iov_len as usize - self.offset).min(buf.len() - count);
            if len == 0 {
                break;
            }
            vm_write_slice(
                iov.iov_base.wrapping_add(self.offset),
                &buf[count..count + len],
            )?;
            self.offset += len;
            self.inner.len -= len;
            count += len;
        }
        Ok(count)
    }

    fn flush(&mut self) -> AxResult {
        Ok(())
    }
}

impl IoBuf for IoVectorBufIo {
    fn remaining(&self) -> usize {
        self.inner.len
    }
}

impl IoBufMut for IoVectorBufIo {
    fn remaining_mut(&self) -> usize {
        self.inner.len
    }
}
