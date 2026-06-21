use core::ffi::{c_char, c_void};

use axerrno::{AxError, AxResult};
use axfs::FS_CONTEXT;
use linux_raw_sys::general::{MS_RDONLY, MS_REMOUNT};

use crate::{mm::vm_load_string, pseudofs::MemoryFs};

pub fn sys_mount(
    source: *const c_char,
    target: *const c_char,
    fs_type: *const c_char,
    flags: i32,
    data: *const c_void,
) -> AxResult<isize> {
    let target = vm_load_string(target)?;
    let fs_type = vm_load_string(fs_type)?;
    let source = if source.is_null() {
        None
    } else {
        Some(vm_load_string(source)?)
    };
    let data = if data.is_null() {
        None
    } else {
        Some(vm_load_string(data.cast())?)
    };
    debug!("sys_mount <= source: {source:?}, target: {target:?}, fs_type: {fs_type:?}");

    let flags = flags as u32;
    if flags & MS_REMOUNT != 0 {
        let target = FS_CONTEXT.lock().resolve(target)?;
        target.filesystem().set_mount_flags(flags & MS_RDONLY)?;
        return Ok(0);
    }

    match fs_type.as_str() {
        "tmpfs" => {}
        "vfat" => {
            if source.is_none() {
                return Err(AxError::InvalidInput);
            }
        }
        _ => return Err(AxError::NoSuchDevice),
    }
    let _ = data;

    let fs = MemoryFs::new();
    fs.set_mount_flags(flags & MS_RDONLY)?;

    let target = FS_CONTEXT.lock().resolve(target)?;
    target.mount(&fs)?;

    Ok(0)
}

pub fn sys_umount2(target: *const c_char, _flags: i32) -> AxResult<isize> {
    let target = vm_load_string(target)?;
    debug!("sys_umount2 <= target: {target:?}");
    let target = FS_CONTEXT.lock().resolve(target)?;
    target.unmount()?;
    Ok(0)
}
