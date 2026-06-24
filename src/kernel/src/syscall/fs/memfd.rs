use alloc::string::String;
use core::ffi::c_char;

use axerrno::{AxError, AxResult};
use linux_raw_sys::general::{MFD_ALLOW_SEALING, MFD_CLOEXEC};

use crate::{
    file::{FileLike, MemFd},
    mm::UserConstPtr,
};

const MEMFD_NAME_MAX: usize = 249;
const SUPPORTED_MEMFD_FLAGS: u32 = MFD_CLOEXEC | MFD_ALLOW_SEALING;

pub fn sys_memfd_create(name: UserConstPtr<c_char>, flags: u32) -> AxResult<isize> {
    if flags & !SUPPORTED_MEMFD_FLAGS != 0 {
        return Err(AxError::InvalidInput);
    }
    let name = name.get_as_null_terminated()?;
    if name.len() > MEMFD_NAME_MAX {
        return Err(AxError::InvalidInput);
    }
    let allow_sealing = flags & MFD_ALLOW_SEALING != 0;
    let cloexec = flags & MFD_CLOEXEC != 0;
    let name_bytes =
        unsafe { core::slice::from_raw_parts(name.as_ptr().cast::<u8>(), name.len()) };
    let name = String::from_utf8_lossy(name_bytes).into_owned();
    MemFd::new(name, allow_sealing)
        .add_to_fd_table(cloexec)
        .map(|fd| fd as _)
}
