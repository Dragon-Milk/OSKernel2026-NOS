use axerrno::{AxError, AxResult};
use syscalls::Sysno;

pub fn sys_unsupported_feature(sysno: Sysno) -> AxResult<isize> {
    warn!("Unsupported optional syscall: {sysno}");
    Err(AxError::Unsupported)
}
