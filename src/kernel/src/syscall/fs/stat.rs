use alloc::string::String;
use core::ffi::{c_char, c_int};

use axerrno::{AxError, AxResult};
use axfs::FS_CONTEXT;
use axfs_ng_vfs::Location;
use linux_raw_sys::general::{
    __kernel_fsid_t, AT_EACCESS, AT_EMPTY_PATH, AT_NO_AUTOMOUNT, AT_STATX_DONT_SYNC,
    AT_STATX_FORCE_SYNC, AT_STATX_SYNC_TYPE, AT_SYMLINK_NOFOLLOW, R_OK, STATX__RESERVED, W_OK,
    X_OK, stat, statfs, statx,
};
use starry_vm::{VmMutPtr, VmPtr};

use crate::{
    file::{
        AccessMode, File, FileLike, VfsCredentials, check_path_len, check_path_search,
        check_path_search_stat, check_permission, check_writable_filesystem, resolve_at,
        with_fs_at,
    },
    mm::vm_load_string,
};

fn load_stat_path(path: *const c_char, _flags: u32) -> AxResult<Option<String>> {
    if path.is_null() {
        Err(AxError::BadAddress)
    } else {
        vm_load_string(path).map(Some)
    }
}

fn check_stat_path_search(dirfd: c_int, path: Option<&str>, flags: u32) -> AxResult<()> {
    let Some(path) = path else {
        return Ok(());
    };
    if path.is_empty() {
        return Ok(());
    }

    check_path_len(path)?;
    with_fs_at(dirfd, path, |fs| {
        check_path_search_stat(
            fs,
            path,
            VfsCredentials::effective(),
            flags & AT_SYMLINK_NOFOLLOW == 0,
        )
    })
}

fn validate_statx_flags(flags: u32) -> AxResult<()> {
    const VALID_FLAGS: u32 =
        AT_SYMLINK_NOFOLLOW | AT_NO_AUTOMOUNT | AT_EMPTY_PATH | AT_STATX_SYNC_TYPE;

    if flags & !VALID_FLAGS != 0 {
        return Err(AxError::InvalidInput);
    }

    match flags & AT_STATX_SYNC_TYPE {
        0 | AT_STATX_FORCE_SYNC | AT_STATX_DONT_SYNC => Ok(()),
        _ => Err(AxError::InvalidInput),
    }
}

fn validate_statx_mask(mask: u32) -> AxResult<()> {
    if mask & STATX__RESERVED != 0 {
        Err(AxError::InvalidInput)
    } else {
        Ok(())
    }
}

/// Get the file metadata by `path` and write into `statbuf`.
///
/// Return 0 if success.
#[cfg(target_arch = "x86_64")]
pub fn sys_stat(path: *const c_char, statbuf: *mut stat) -> AxResult<isize> {
    use linux_raw_sys::general::AT_FDCWD;

    sys_fstatat(AT_FDCWD, path, statbuf, 0)
}

/// Get file metadata by `fd` and write into `statbuf`.
///
/// Return 0 if success.
pub fn sys_fstat(fd: i32, statbuf: *mut stat) -> AxResult<isize> {
    let loc = resolve_at(fd, None, AT_EMPTY_PATH)?;
    statbuf.vm_write(loc.stat()?.into())?;
    Ok(0)
}

/// Get the metadata of the symbolic link and write into `buf`.
///
/// Return 0 if success.
#[cfg(target_arch = "x86_64")]
pub fn sys_lstat(path: *const c_char, statbuf: *mut stat) -> AxResult<isize> {
    use linux_raw_sys::general::{AT_FDCWD, AT_SYMLINK_NOFOLLOW};

    sys_fstatat(AT_FDCWD, path, statbuf, AT_SYMLINK_NOFOLLOW)
}

pub fn sys_fstatat(
    dirfd: i32,
    path: *const c_char,
    statbuf: *mut stat,
    flags: u32,
) -> AxResult<isize> {
    let path = load_stat_path(path, flags)?;

    debug!("sys_fstatat <= dirfd: {dirfd}, path: {path:?}, flags: {flags}");

    check_stat_path_search(dirfd, path.as_deref(), flags)?;
    let loc = resolve_at(dirfd, path.as_deref(), flags)?;
    statbuf.vm_write(loc.stat()?.into())?;

    Ok(0)
}

pub fn sys_statx(
    dirfd: c_int,
    path: *const c_char,
    flags: u32,
    mask: u32,
    statxbuf: *mut statx,
) -> AxResult<isize> {
    // `statx()` uses pathname, dirfd, and flags to identify the target
    // file in one of the following ways:

    // An absolute pathname(situation 1)
    //        If pathname begins with a slash, then it is an absolute
    //        pathname that identifies the target file.  In this case,
    //        dirfd is ignored.

    // A relative pathname(situation 2)
    //        If pathname is a string that begins with a character other
    //        than a slash and dirfd is AT_FDCWD, then pathname is a
    //        relative pathname that is interpreted relative to the
    //        process's current working directory.

    // A directory-relative pathname(situation 3)
    //        If pathname is a string that begins with a character other
    //        than a slash and dirfd is a file descriptor that refers to
    //        a directory, then pathname is a relative pathname that is
    //        interpreted relative to the directory referred to by dirfd.
    //        (See openat(2) for an explanation of why this is useful.)

    // By file descriptor(situation 4)
    //        If pathname is an empty string and the AT_EMPTY_PATH flag
    //        is specified in flags (see
    //        below), then the target file is the one referred to by the
    //        file descriptor dirfd.

    if path.is_null() {
        return Err(AxError::BadAddress);
    }
    let path = vm_load_string(path)?;
    debug!("sys_statx <= dirfd: {dirfd}, path: {path:?}, flags: {flags}");

    validate_statx_flags(flags)?;
    validate_statx_mask(mask)?;
    check_stat_path_search(dirfd, Some(path.as_str()), flags)?;
    statxbuf.vm_write(resolve_at(dirfd, Some(path.as_str()), flags)?.stat()?.into())?;

    Ok(0)
}

#[cfg(target_arch = "x86_64")]
pub fn sys_access(path: *const c_char, mode: u32) -> AxResult<isize> {
    use linux_raw_sys::general::AT_FDCWD;

    sys_faccessat2(AT_FDCWD, path, mode, 0)
}

pub fn sys_faccessat2(dirfd: c_int, path: *const c_char, mode: u32, flags: u32) -> AxResult<isize> {
    const VALID_FLAGS: u32 = AT_EACCESS | AT_EMPTY_PATH | AT_SYMLINK_NOFOLLOW;
    const VALID_MODES: u32 = R_OK | W_OK | X_OK;

    if flags & !VALID_FLAGS != 0 || mode & !VALID_MODES != 0 {
        return Err(AxError::InvalidInput);
    }

    let path = path.nullable().map(vm_load_string).transpose()?;
    debug!("sys_faccessat2 <= dirfd: {dirfd}, path: {path:?}, mode: {mode}, flags: {flags}");

    let credentials = if flags & AT_EACCESS != 0 {
        VfsCredentials::effective()
    } else {
        VfsCredentials::real()
    };
    if let Some(path) = path.as_deref() {
        if !path.is_empty() {
            check_path_len(path)?;
            with_fs_at(dirfd, path, |fs| check_path_search(fs, path, credentials))?;
        }
    }
    let file = resolve_at(dirfd, path.as_deref(), flags)?;

    if mode == 0 {
        return Ok(0);
    }
    let mut required_mode = AccessMode::empty();
    if mode & R_OK != 0 {
        required_mode |= AccessMode::READ;
    }
    if mode & W_OK != 0 {
        required_mode |= AccessMode::WRITE;
    }
    if mode & X_OK != 0 {
        required_mode |= AccessMode::EXEC;
    }
    let loc = file.into_file().ok_or(AxError::BadFileDescriptor)?;
    if mode & W_OK != 0 {
        check_writable_filesystem(&loc)?;
    }
    check_permission(&loc, credentials, required_mode)?;

    Ok(0)
}

fn statfs(loc: &Location) -> AxResult<statfs> {
    let stat = loc.filesystem().stat()?;
    // FIXME: Zeroable
    let mut result: statfs = unsafe { core::mem::zeroed() };
    result.f_type = stat.fs_type as _;
    result.f_bsize = stat.block_size as _;
    result.f_blocks = stat.blocks as _;
    result.f_bfree = stat.blocks_free as _;
    result.f_bavail = stat.blocks_available as _;
    result.f_files = stat.file_count as _;
    result.f_ffree = stat.free_file_count as _;
    // TODO: fsid
    result.f_fsid = __kernel_fsid_t {
        val: [0, loc.mountpoint().device() as _],
    };
    result.f_namelen = stat.name_length as _;
    result.f_frsize = stat.fragment_size as _;
    result.f_flags = stat.mount_flags as _;
    Ok(result)
}

pub fn sys_statfs(path: *const c_char, buf: *mut statfs) -> AxResult<isize> {
    let path = vm_load_string(path)?;
    debug!("sys_statfs <= path: {path:?}");

    if path.is_empty() {
        return Err(AxError::NotFound);
    }
    check_path_len(&path)?;
    let fs = FS_CONTEXT.lock();
    check_path_search_stat(&fs, &path, VfsCredentials::effective(), true)?;
    let loc = fs.resolve(path)?.mountpoint().root_location();
    drop(fs);
    buf.vm_write(statfs(&loc)?)?;
    Ok(0)
}

pub fn sys_fstatfs(fd: i32, buf: *mut statfs) -> AxResult<isize> {
    debug!("sys_fstatfs <= fd: {fd}");

    buf.vm_write(statfs(File::from_fd(fd)?.inner().location())?)?;
    Ok(0)
}
