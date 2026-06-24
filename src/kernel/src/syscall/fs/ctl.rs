use alloc::{ffi::CString, string::String, vec, vec::Vec};
use core::{
    ffi::{c_char, c_int},
    mem::{offset_of, size_of},
    time::Duration,
};

use axerrno::{AxError, AxResult, LinuxError};
use axfs::{FS_CONTEXT, FsContext};
use axfs_ng_vfs::{DeviceId, Location, MetadataUpdate, NodePermission, NodeType, path::Path};
use axhal::time::wall_time;
use axtask::current;
use linux_raw_sys::{
    general::*,
    ioctl::{FIONBIO, SIOCATMARK, SIOCGIFCONF, SIOCGIFFLAGS, SIOCSIFFLAGS, TIOCGWINSZ},
    net::{ifconf, ifreq, net_device_flags},
};
use starry_vm::{VmMutPtr, VmPtr, vm_read_slice, vm_write_slice};

use crate::{
    file::{
        AccessMode, Directory, File, FileLike, NamedPipe, VfsCredentials,
        check_not_append_only, check_not_immutable,
        check_parent_permission, check_path_len, check_path_search,
        check_permission, check_writable_filesystem, clear_setgid_if_not_in_group,
        creation_metadata, do_getxattr, do_listxattr, do_removexattr, do_setxattr,
        get_file_like, get_inode_flags, is_directory_deleted, mark_directory_deleted, remove_inode_flags,
        remove_xattr_map, resolve_at, set_inode_flags,
        with_fs, with_fs_at,
        FS_IOC_GETFLAGS, FS_IOC_SETFLAGS, Socket,
    },
    mm::{check_access, vm_load_string},
    task::AsThread,
    time::TimeValueLike,
};

/// The ioctl() system call manipulates the underlying device parameters
/// of special files.
pub fn sys_ioctl(fd: i32, cmd: u32, arg: usize) -> AxResult<isize> {
    debug!("sys_ioctl <= fd: {fd}, cmd: {cmd}, arg: {arg}");
    let f = get_file_like(fd)?;
    if cmd == FIONBIO {
        let val = (arg as *const u8).vm_read()?;
        if val != 0 && val != 1 {
            return Err(AxError::InvalidInput);
        }
        f.set_nonblocking(val != 0)?;
        return Ok(0);
    }
    if cmd == FS_IOC_GETFLAGS {
        // Only regular files, directories, symlinks, and FIFOs support inode flags.
        let loc = if let Some(file) = f.downcast_ref::<File>() {
            file.inner().location().clone()
        } else if let Some(dir) = f.downcast_ref::<Directory>() {
            dir.inner().clone()
        } else {
            return Err(AxError::NotATty);
        };
        let flags = get_inode_flags(&loc);
        (arg as *mut u32).vm_write(flags)?;
        return Ok(0);
    }
    if cmd == FS_IOC_SETFLAGS {
        let loc = if let Some(file) = f.downcast_ref::<File>() {
            file.inner().location().clone()
        } else if let Some(dir) = f.downcast_ref::<Directory>() {
            dir.inner().clone()
        } else {
            return Err(AxError::NotATty);
        };
        check_writable_filesystem(&loc)?;
        let credentials = VfsCredentials::effective();
        // Only effective-root can set immutable/append flags.
        if !credentials.is_privileged() {
            return Err(AxError::OperationNotPermitted);
        }
        let user_flags = (arg as *const u32).vm_read()?;
        set_inode_flags(&loc, user_flags);
        return Ok(0);
    }
    if let Some(socket) = f.downcast_ref::<Socket>() {
        match cmd {
            SIOCATMARK => {
                if matches!(&socket.0, axnet::Socket::Udp(_)) {
                    return Err(AxError::NotATty);
                }
                (arg as *mut i32).vm_write(0)?;
                return Ok(0);
            }
            SIOCGIFCONF => {
                let mut config = unsafe { (arg as *const ifconf).vm_read_uninit()?.assume_init() };
                let mut request: ifreq = unsafe { core::mem::zeroed() };
                unsafe {
                    request.ifr_ifrn.ifrn_name[0] = b'l' as _;
                    request.ifr_ifrn.ifrn_name[1] = b'o' as _;
                }
                if config.ifc_len >= size_of::<ifreq>() as i32 {
                    let request_ptr = unsafe { config.ifc_ifcu.ifcu_req };
                    request_ptr.vm_write(request)?;
                    config.ifc_len = size_of::<ifreq>() as i32;
                } else {
                    config.ifc_len = 0;
                }
                (arg as *mut ifconf).vm_write(config)?;
                return Ok(0);
            }
            SIOCGIFFLAGS => {
                let mut request = unsafe { (arg as *const ifreq).vm_read_uninit()?.assume_init() };
                request.ifr_ifru.ifru_flags =
                    net_device_flags::IFF_UP as i16
                    | net_device_flags::IFF_LOOPBACK as i16
                    | net_device_flags::IFF_RUNNING as i16;
                (arg as *mut ifreq).vm_write(request)?;
                return Ok(0);
            }
            SIOCSIFFLAGS => {
                let _ = unsafe { (arg as *const ifreq).vm_read_uninit()?.assume_init() };
                return Ok(0);
            }
            _ => {}
        }
    }
    // TCGETA / TCSETA read/write termio structs from/to user memory.
    // Validate the user pointer BEFORE dispatching to the file-specific
    // ioctl handler: EFAULT takes priority over ENOTTY/NotATty.
    // TCGETA=21509, but older LTP tests may use variant 21569 for the old
    // termio interface.
    if cmd == 21509 || cmd == 21569 || cmd == 21510 {
        (arg as *const u8).vm_read()?;
    }
    f.ioctl(cmd, arg)
        .map(|result| result as isize)
        .inspect_err(|err| {
            if *err == AxError::NotATty {
                // glibc likes to call TIOCGWINSZ on non-terminal files, just
                // ignore it
                if cmd == TIOCGWINSZ {
                    return;
                }
                warn!("Unsupported ioctl command: {cmd} for fd: {fd}");
            }
        })
}

pub fn sys_chdir(path: *const c_char) -> AxResult<isize> {
    let path = vm_load_string(path)?;
    debug!("sys_chdir <= path: {path}");

    let mut fs = FS_CONTEXT.lock();
    let credentials = VfsCredentials::effective();
    check_path_search(&fs, &path, credentials)?;
    let entry = fs.resolve(path)?;
    entry.check_is_dir()?;
    check_permission(&entry, credentials, AccessMode::EXEC)?;
    fs.set_current_dir(entry)?;
    Ok(0)
}

pub fn sys_fchdir(dirfd: i32) -> AxResult<isize> {
    debug!("sys_fchdir <= dirfd: {dirfd}");

    let entry = with_fs(dirfd, |fs| Ok(fs.current_dir().clone()))?;
    entry.check_is_dir()?;
    check_permission(&entry, VfsCredentials::effective(), AccessMode::EXEC)?;
    FS_CONTEXT.lock().set_current_dir(entry)?;
    Ok(0)
}

#[cfg(target_arch = "x86_64")]
pub fn sys_mkdir(path: *const c_char, mode: u32) -> AxResult<isize> {
    sys_mkdirat(AT_FDCWD, path, mode)
}

pub fn sys_chroot(path: *const c_char) -> AxResult<isize> {
    let path = vm_load_string(path)?;
    debug!("sys_chroot <= path: {path}");

    let credentials = VfsCredentials::effective();

    let mut fs = FS_CONTEXT.lock();
    check_path_search(&fs, &path, credentials)?;
    let loc = fs.resolve(path)?;
    if loc.node_type() != NodeType::Directory {
        return Err(AxError::NotADirectory);
    }
    check_permission(&loc, credentials, AccessMode::EXEC)?;

    if !credentials.is_privileged()
        || !current()
            .as_thread()
            .proc_data
            .has_capability(CAP_SYS_CHROOT)
    {
        return Err(AxError::OperationNotPermitted);
    }

    *fs = FsContext::new(loc);
    Ok(0)
}

pub fn sys_mkdirat(dirfd: i32, path: *const c_char, mode: u32) -> AxResult<isize> {
    let path = vm_load_string(path)?;
    debug!("sys_mkdirat <= dirfd: {dirfd}, path: {path}, mode: {mode}");

    let mode = mode & !current().as_thread().proc_data.umask();
    let mode = NodePermission::from_bits_truncate(mode as u16);

    let credentials = VfsCredentials::effective();
    with_fs_at(dirfd, &path, |fs| {
        if fs.resolve(&path).is_ok() {
            return Err(AxError::AlreadyExists);
        }
        check_parent_permission(fs, &path, credentials, AccessMode::WRITE | AccessMode::EXEC)?;
        let (parent, name) = fs.resolve_nonexistent(Path::new(&path))?;
        check_writable_filesystem(&parent)?;
        let (owner, mode) = creation_metadata(&parent, NodeType::Directory, mode, credentials)?;
        let loc = parent.create(name, NodeType::Directory, mode)?;
        loc.update_metadata(MetadataUpdate {
            owner: Some(owner),
            mode: Some(mode),
            ..Default::default()
        })?;
        Ok(0)
    })
}

#[cfg(target_arch = "x86_64")]
pub fn sys_mknod(path: *const c_char, mode: u32, dev: u32) -> AxResult<isize> {
    sys_mknodat(AT_FDCWD, path, mode, dev)
}

pub fn sys_mknodat(dirfd: i32, path: *const c_char, mode: u32, dev: u32) -> AxResult<isize> {
    let path = vm_load_string(path)?;
    debug!("sys_mknodat <= dirfd: {dirfd}, path: {path}, mode: {mode:#o}, dev: {dev}");

    if path.is_empty() {
        return Err(AxError::NotFound);
    }

    let node_type = match mode & S_IFMT {
        0 | S_IFREG => NodeType::RegularFile,
        S_IFIFO => NodeType::Fifo,
        S_IFDIR => return Err(AxError::OperationNotPermitted),
        S_IFCHR => NodeType::CharacterDevice,
        S_IFBLK => NodeType::BlockDevice,
        S_IFSOCK => NodeType::Socket,
        _ => return Err(AxError::InvalidInput),
    };

    let rdev = match node_type {
        NodeType::CharacterDevice | NodeType::BlockDevice => {
            // Linux 16-bit dev_t encoding: major 12 bits, minor 20 bits.
            let major = (dev >> 8) & 0xfff;
            let minor = (dev & 0xff) | ((dev >> 12) & 0xfff00);
            DeviceId::new(major, minor)
        }
        _ => DeviceId::default(),
    };

    let mode = mode & !current().as_thread().proc_data.umask();
    let mode = NodePermission::from_bits_truncate(mode as u16);
    let credentials = VfsCredentials::effective();

    with_fs(dirfd, |fs| {
        match fs.resolve_no_follow(&path) {
            Ok(_) => return Err(AxError::AlreadyExists),
            Err(AxError::NotFound) => {}
            Err(err) => return Err(err),
        }
        check_parent_permission(fs, &path, credentials, AccessMode::WRITE | AccessMode::EXEC)?;
        let (parent, name) = fs.resolve_nonexistent(Path::new(&path))?;
        let (owner, mode) = creation_metadata(&parent, node_type, mode, credentials)?;
        let loc = parent.create(name, node_type, mode)?;
        loc.update_metadata(MetadataUpdate {
            owner: Some(owner),
            ..Default::default()
        })?;
        // Store rdev for device nodes via user_data.
        if node_type == NodeType::CharacterDevice || node_type == NodeType::BlockDevice {
            loc.user_data().insert(rdev);
        }
        Ok(0)
    })
}

// Directory buffer for getdents64 syscall
struct DirBuffer {
    buf: Vec<u8>,
    offset: usize,
}

impl DirBuffer {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0; len],
            offset: 0,
        }
    }

    fn remaining_space(&self) -> usize {
        self.buf.len().saturating_sub(self.offset)
    }

    fn write_entry(&mut self, d_ino: u64, d_off: i64, d_type: NodeType, name: &[u8]) -> bool {
        const NAME_OFFSET: usize = offset_of!(linux_dirent64, d_name);

        let len = NAME_OFFSET + name.len() + 1;
        // alignment
        let len = len.next_multiple_of(align_of::<linux_dirent64>());
        if self.remaining_space() < len {
            return false;
        }

        // FIXME: safety
        unsafe {
            let entry_ptr = self.buf.as_mut_ptr().add(self.offset);
            entry_ptr.cast::<linux_dirent64>().write(linux_dirent64 {
                d_ino,
                d_off,
                d_reclen: len as _,
                d_type: d_type as _,
                d_name: Default::default(),
            });

            let name_ptr = entry_ptr.add(NAME_OFFSET);
            name_ptr.copy_from_nonoverlapping(name.as_ptr(), name.len());
            name_ptr.add(name.len()).write(0);
        }

        self.offset += len;
        true
    }
}

pub fn sys_getdents64(fd: i32, buf: *mut u8, len: usize) -> AxResult<isize> {
    debug!("sys_getdents64 <= fd: {fd}, buf: {buf:?}, len: {len}");

    let dir = Directory::from_fd(fd)?;
    if dir.is_deleted() {
        return Err(AxError::NotFound);
    }
    let mut dir_offset = dir.offset.lock();
    let mut buffer = DirBuffer::new(len);

    let mut has_remaining = false;

    dir.inner()
        .read_dir(*dir_offset, &mut |name: &str, ino, node_type, offset| {
            has_remaining = true;
            if !buffer.write_entry(ino, offset as _, node_type, name.as_bytes()) {
                return false;
            }
            *dir_offset = offset;
            true
        })?;

    if has_remaining && buffer.offset == 0 {
        return Err(AxError::InvalidInput);
    }

    vm_write_slice(buf, &buffer.buf)?;

    Ok(buffer.offset as _)
}

/// create a link from new_path to old_path
/// old_path: old file path
/// new_path: new file path
/// flags: link flags
/// return value: return 0 when success, else return -1.
pub fn sys_linkat(
    old_dirfd: c_int,
    old_path: *const c_char,
    new_dirfd: c_int,
    new_path: *const c_char,
    flags: u32,
) -> AxResult<isize> {
    const VALID_FLAGS: u32 = AT_EMPTY_PATH | AT_SYMLINK_FOLLOW;

    if flags & !VALID_FLAGS != 0 {
        return Err(AxError::InvalidInput);
    }

    let old_path = old_path.nullable().map(vm_load_string).transpose()?;
    let new_path = vm_load_string(new_path)?;
    debug!(
        "sys_linkat <= old_dirfd: {old_dirfd}, old_path: {old_path:?}, new_dirfd: {new_dirfd}, \
         new_path: {new_path}, flags: {flags}"
    );

    if new_path.is_empty() {
        return Err(AxError::NotFound);
    }
    check_path_len(&new_path)?;

    let credentials = VfsCredentials::effective();
    if let Some(path) = old_path.as_deref()
        && !path.is_empty()
    {
        check_path_len(path)?;
        with_fs_at(old_dirfd, path, |fs| check_path_search(fs, path, credentials))?;
    }
    let old = resolve_at(old_dirfd, old_path.as_deref(), flags)?
        .into_file()
        .ok_or(AxError::BadFileDescriptor)?;
    if old.is_dir() {
        return Err(AxError::OperationNotPermitted);
    }
    with_fs_at(new_dirfd, &new_path, |fs| {
        check_parent_permission(
            fs,
            &new_path,
            credentials,
            AccessMode::WRITE | AccessMode::EXEC,
        )
    })?;
    let (new_dir, new_name) =
        with_fs_at(new_dirfd, &new_path, |fs| fs.resolve_nonexistent(Path::new(&new_path)))?;

    if !new_dir.same_mountpoint(&old) {
        return Err(AxError::CrossesDevices);
    }
    check_writable_filesystem(&new_dir)?;
    new_dir.link(new_name, &old)?;
    Ok(0)
}

#[cfg(target_arch = "x86_64")]
pub fn sys_link(old_path: *const c_char, new_path: *const c_char) -> AxResult<isize> {
    sys_linkat(AT_FDCWD, old_path, AT_FDCWD, new_path, 0)
}

/// remove link of specific file (can be used to delete file)
/// dir_fd: the directory of link to be removed
/// path: the name of link to be removed
/// flags: can be 0 or AT_REMOVEDIR
/// return 0 when success, else return -1
pub fn sys_unlinkat(dirfd: i32, path: *const c_char, flags: usize) -> AxResult<isize> {
    if flags & !(AT_REMOVEDIR as usize) != 0 {
        return Err(AxError::InvalidInput);
    }

    let path = vm_load_string(path)?;

    debug!("sys_unlinkat <= dirfd: {dirfd}, path: {path:?}, flags: {flags}");

    if path.is_empty() {
        return Err(AxError::NotFound);
    }
    check_path_len(&path)?;

    let credentials = VfsCredentials::effective();
    with_fs_at(dirfd, &path, |fs| {
        if flags == AT_REMOVEDIR as _ {
            let entry = fs.resolve_no_follow(path.as_str())?;
            let parent = entry.parent().ok_or(AxError::ResourceBusy)?;
            if parent.lookup_no_mount(entry.name())?.is_mountpoint() {
                return Err(AxError::ResourceBusy);
            }
            check_parent_permission(
                fs,
                &path,
                credentials,
                AccessMode::WRITE | AccessMode::EXEC,
            )?;
            check_writable_filesystem(&parent)?;
            check_not_immutable(&entry)?;
            check_not_append_only(&entry)?;
            check_sticky_removal(&parent, &entry, credentials)?;
            match fs.remove_dir(path.as_str()) {
                Ok(()) => {}
                Err(e) => return Err(e),
            }
            mark_directory_deleted(&entry);
            // Directories cannot be hard-linked; always clean up inode-level
            // tracking (xattr, inode flags) so that reused inode numbers do
            // not inherit stale data.
            remove_xattr_map(&entry);
            remove_inode_flags(&entry);
        } else {
            // Resolve parent first so we can check read-only filesystem
            // before attempting to find the entry itself.
            // This is needed for LTP unlink09: rofs → EROFS even if the
            // target file is not visible through the mountpoint.
            check_path_search(fs, &path, credentials)?;
            let (parent, _) = fs.resolve_nonexistent(Path::new(&path))?;
            check_permission(&parent, credentials, AccessMode::WRITE | AccessMode::EXEC)?;
            check_writable_filesystem(&parent)?;

            let entry = fs.resolve_no_follow(path.as_str())?;
            if let Some(_parent) = entry.parent() {
                // parent obtained from resolve_nonexistent already covers the
                // filesystem check; use entry.parent() for sticky removal.
                check_not_immutable(&entry)?;
                check_not_append_only(&entry)?;
                check_sticky_removal(&_parent, &entry, credentials)?;
            } else {
                check_not_immutable(&entry)?;
                check_not_append_only(&entry)?;
            }
            // Capture whether this is the last link *before* the backend
            // decrements nlink, so we know when to discard xattr / inode-flags.
            let last_link = entry.metadata().map(|m| m.nlink).unwrap_or(0) <= 1;
            match fs.remove_file(path.as_str()) {
                Ok(()) => {}
                Err(e) => return Err(e),
            }
            if last_link {
                remove_xattr_map(&entry);
                remove_inode_flags(&entry);
            }
        }
        Ok(0)
    })
}

#[cfg(target_arch = "x86_64")]
pub fn sys_rmdir(path: *const c_char) -> AxResult<isize> {
    sys_unlinkat(AT_FDCWD, path, AT_REMOVEDIR as _)
}

#[cfg(target_arch = "x86_64")]
pub fn sys_unlink(path: *const c_char) -> AxResult<isize> {
    sys_unlinkat(AT_FDCWD, path, 0)
}

pub fn sys_getcwd(buf: *mut u8, size: usize) -> AxResult<isize> {
    // Phase 1: obtain the constructed absolute path and the cwd identity
    // under the FS lock, then release it before the (possibly slow) resolve.
    let (cwd_path, cwd_device, cwd_inode, cwd_node_type, cwd_deleted) = {
        let fs = FS_CONTEXT.lock();
        let cur = fs.current_dir();
        let path = cur.absolute_path()?;
        let meta = cur.metadata().map_err(|_| AxError::NotFound)?;
        let deleted = is_directory_deleted(cur);
        (path, meta.device, meta.inode, cur.node_type(), deleted)
    };

    let cwd = CString::new(cwd_path.as_str()).map_err(|_| AxError::InvalidInput)?;
    let cwd = cwd.as_bytes_with_nul();
    // copied_len includes the NUL terminator (Linux ABI).
    let copied_len = cwd.len();

    if size < copied_len {
        return Err(AxError::OutOfRange);
    }

    vm_write_slice(buf, cwd)?;

    // Linux getcwd(2) returns the number of bytes copied to the user
    // buffer, including the NUL terminator. glibc's getcwd() checks that
    // retval > 0 && path[0] == '/' before returning buf to the caller;
    // otherwise it falls back to a manual /proc/self/cwd traversal.
    Ok(copied_len as isize)
}

#[cfg(target_arch = "x86_64")]
pub fn sys_symlink(target: *const c_char, linkpath: *const c_char) -> AxResult<isize> {
    sys_symlinkat(target, AT_FDCWD, linkpath)
}

pub fn sys_symlinkat(
    target: *const c_char,
    new_dirfd: i32,
    linkpath: *const c_char,
) -> AxResult<isize> {
    let target = vm_load_string(target)?;
    let linkpath = vm_load_string(linkpath)?;
    debug!("sys_symlinkat <= target: {target:?}, new_dirfd: {new_dirfd}, linkpath: {linkpath:?}");

    if linkpath.is_empty() {
        return Err(AxError::NotFound);
    }

    let credentials = VfsCredentials::effective();
    with_fs_at(new_dirfd, &linkpath.clone(), |fs| {
        check_parent_permission(
            fs,
            &linkpath,
            credentials,
            AccessMode::WRITE | AccessMode::EXEC,
        )?;
        fs.symlink(target, linkpath)?;
        Ok(0)
    })
}

#[cfg(target_arch = "x86_64")]
pub fn sys_readlink(path: *const c_char, buf: *mut u8, size: usize) -> AxResult<isize> {
    sys_readlinkat(AT_FDCWD, path, buf, size)
}

pub fn sys_readlinkat(
    dirfd: i32,
    path: *const c_char,
    buf: *mut u8,
    size: usize,
) -> AxResult<isize> {
    let path = vm_load_string(path)?;

    debug!("sys_readlinkat <= dirfd: {dirfd}, path: {path:?}");

    if size == 0 {
        return Err(AxError::InvalidInput);
    }

    let link = if path.is_empty() {
        if dirfd == AT_FDCWD {
            return Err(AxError::NotFound);
        }
        let file_like = get_file_like(dirfd)?;
        if let Some(file) = file_like.downcast_ref::<File>() {
            file.inner().location().read_link()?
        } else if let Some(dir) = file_like.downcast_ref::<Directory>() {
            dir.inner().read_link()?
        } else {
            return Err(AxError::BadFileDescriptor);
        }
    } else {
        check_path_len(&path)?;
        let credentials = VfsCredentials::effective();
        with_fs_at(dirfd, &path, |fs| {
            check_path_search(fs, path.as_str(), credentials)?;
            fs.resolve_no_follow(path.as_str())?.read_link()
        })?
    };
    let read = size.min(link.len());
    vm_write_slice(buf, &link.as_bytes()[..read])?;
    Ok(read as isize)
}

#[cfg(target_arch = "x86_64")]
pub fn sys_chown(path: *const c_char, uid: i32, gid: i32) -> AxResult<isize> {
    sys_fchownat(AT_FDCWD, path, uid, gid, 0)
}

#[cfg(target_arch = "x86_64")]
pub fn sys_lchown(path: *const c_char, uid: i32, gid: i32) -> AxResult<isize> {
    use linux_raw_sys::general::AT_SYMLINK_NOFOLLOW;
    sys_fchownat(AT_FDCWD, path, uid, gid, AT_SYMLINK_NOFOLLOW)
}

pub fn sys_fchown(fd: i32, uid: i32, gid: i32) -> AxResult<isize> {
    check_not_o_path_fd(fd)?;
    sys_fchownat(fd, core::ptr::null(), uid, gid, AT_EMPTY_PATH)
}

pub fn sys_fchownat(
    dirfd: i32,
    path: *const c_char,
    uid: i32,
    gid: i32,
    flags: u32,
) -> AxResult<isize> {
    const VALID_FLAGS: u32 = AT_EMPTY_PATH | AT_SYMLINK_NOFOLLOW;

    if flags & !VALID_FLAGS != 0 {
        return Err(AxError::InvalidInput);
    }

    let path = path.nullable().map(vm_load_string).transpose()?;
    let credentials = VfsCredentials::effective();
    if let Some(path) = path.as_deref() {
        if !path.is_empty() {
            if path_refers_to_o_path_fd(path) {
                return Err(AxError::BadFileDescriptor);
            }
            check_path_len(path)?;
            with_fs_at(dirfd, path, |fs| check_path_search(fs, path, credentials))?;
        }
    }
    if flags & AT_EMPTY_PATH != 0 && path.as_deref().map_or(true, |p| p.is_empty()) {
        check_not_o_path_fd(dirfd)?;
    }
    let loc = resolve_at(dirfd, path.as_deref(), flags)?
        .into_file()
        .ok_or(AxError::BadFileDescriptor)?;
    check_writable_filesystem(&loc)?;
    check_not_immutable(&loc)?;
    check_not_append_only(&loc)?;
    let meta = loc.metadata()?;
    let uid = if uid == -1 { meta.uid } else { uid as _ };
    let gid = if gid == -1 { meta.gid } else { gid as _ };
    if credentials.uid != 0 {
        if credentials.uid != meta.uid
            || uid != meta.uid
            || (gid != meta.gid && gid != credentials.gid)
        {
            return Err(AxError::OperationNotPermitted);
        }
    }

    let mut mode = meta.mode;
    // chown always clears the setuid bits
    mode.remove(NodePermission::SET_UID);
    // chown also removes the setgid bit from group-executable non-directories.
    if meta.node_type != NodeType::Directory && mode.contains(NodePermission::GROUP_EXEC) {
        mode.remove(NodePermission::SET_GID);
    }

    loc.update_metadata(MetadataUpdate {
        owner: Some((uid, gid)),
        mode: Some(mode),
        ..Default::default()
    })?;
    Ok(0)
}

#[cfg(target_arch = "x86_64")]
pub fn sys_chmod(path: *const c_char, mode: u32) -> AxResult<isize> {
    sys_fchmodat(AT_FDCWD, path, mode, 0)
}

pub fn sys_fchmod(fd: i32, mode: u32) -> AxResult<isize> {
    check_not_o_path_fd(fd)?;
    sys_fchmodat(fd, core::ptr::null(), mode, AT_EMPTY_PATH)
}

pub fn sys_fchmodat(dirfd: i32, path: *const c_char, mode: u32, flags: u32) -> AxResult<isize> {
    const VALID_FLAGS: u32 = AT_EMPTY_PATH | AT_SYMLINK_NOFOLLOW;

    if flags & !VALID_FLAGS != 0 {
        return Err(AxError::InvalidInput);
    }

    let path = path.nullable().map(vm_load_string).transpose()?;
    let credentials = VfsCredentials::effective();
    if let Some(path) = path.as_deref() {
        if !path.is_empty() {
            if path_refers_to_o_path_fd(path) {
                return Err(AxError::BadFileDescriptor);
            }
            check_path_len(path)?;
            with_fs_at(dirfd, path, |fs| check_path_search(fs, path, credentials))?;
        }
    }
    if flags & AT_EMPTY_PATH != 0 && path.as_deref().map_or(true, |p| p.is_empty()) {
        check_not_o_path_fd(dirfd)?;
    }
    let loc = resolve_at(dirfd, path.as_deref(), flags)?
        .into_file()
        .ok_or(AxError::BadFileDescriptor)?;
    check_writable_filesystem(&loc)?;
    check_not_immutable(&loc)?;
    check_not_append_only(&loc)?;
    let meta = loc.metadata()?;
    if credentials.uid != 0 && credentials.uid != meta.uid {
        return Err(AxError::OperationNotPermitted);
    }
    let mode = clear_setgid_if_not_in_group(
        NodePermission::from_bits_truncate(mode as u16),
        meta.gid,
        credentials,
    );
    loc.update_metadata(MetadataUpdate {
        mode: Some(mode),
        ..Default::default()
    })?;
    Ok(0)
}

fn update_times(
    dirfd: i32,
    path: *const c_char,
    atime: Option<Duration>,
    mtime: Option<Duration>,
    flags: u32,
    set_to_now: bool,
) -> AxResult<()> {
    let path = path.nullable().map(vm_load_string).transpose()?;
    let loc = resolve_at(dirfd, path.as_deref(), flags)?
        .into_file()
        .ok_or(AxError::BadFileDescriptor)?;

    // Reject time modification on read-only filesystems
    check_writable_filesystem(&loc)?;
    // Reject time modification on immutable/append-only files
    check_not_immutable(&loc)?;
    check_not_append_only(&loc)?;

    // Permission check: who can modify file timestamps
    let credentials = VfsCredentials::effective();
    let meta = loc.metadata()?;
    if !credentials.is_privileged() {
        if credentials.uid != meta.uid {
            if set_to_now {
                // Setting to current time: need write permission on the file
                check_permission(&loc, credentials, AccessMode::WRITE)?;
            } else {
                // Setting to arbitrary time: must be the file owner (or privileged)
                return Err(AxError::OperationNotPermitted);
            }
        }
    }

    loc.update_metadata(MetadataUpdate {
        atime,
        mtime,
        ..Default::default()
    })?;
    Ok(())
}

fn check_rename_type(old: &Location, new: Option<&Location>) -> AxResult<()> {
    let Some(new) = new else {
        return Ok(());
    };

    match (
        old.node_type() == NodeType::Directory,
        new.node_type() == NodeType::Directory,
    ) {
        (true, false) => Err(AxError::NotADirectory),
        (false, true) => Err(AxError::IsADirectory),
        _ => Ok(()),
    }
}

fn check_sticky_removal(
    parent: &Location,
    entry: &Location,
    credentials: VfsCredentials,
) -> AxResult<()> {
    let parent_meta = parent.metadata()?;
    if !parent_meta.mode.contains(NodePermission::STICKY) {
        return Ok(());
    }

    let entry_meta = entry.metadata()?;
    if credentials.is_privileged()
        || credentials.uid == parent_meta.uid
        || credentials.uid == entry_meta.uid
    {
        Ok(())
    } else {
        Err(AxError::OperationNotPermitted)
    }
}

#[cfg(target_arch = "x86_64")]
#[allow(non_camel_case_types)]
#[repr(C)]
pub struct utimbuf {
    actime: linux_raw_sys::general::__kernel_old_time_t,
    modtime: linux_raw_sys::general::__kernel_old_time_t,
}

#[cfg(target_arch = "x86_64")]
pub fn sys_utime(path: *const c_char, times: *const utimbuf) -> AxResult<isize> {
    let (atime, mtime) = if let Some(times) = times.nullable() {
        // FIXME: AnyBitPattern
        let times = unsafe { times.vm_read_uninit()?.assume_init() };
        (
            Duration::from_secs(times.actime as _),
            Duration::from_secs(times.modtime as _),
        )
    } else {
        let time = wall_time();
        (time, time)
    };
    let set_to_now = times.is_null();
    update_times(AT_FDCWD, path, Some(atime), Some(mtime), 0, set_to_now)?;
    Ok(0)
}

#[cfg(target_arch = "x86_64")]
pub fn sys_utimes(
    path: *const c_char,
    times: *const [linux_raw_sys::general::timeval; 2],
) -> AxResult<isize> {
    let (atime, mtime) = if let Some(times) = times.nullable() {
        // FIXME: AnyBitPattern
        let [atime, mtime] = unsafe { times.vm_read_uninit()?.assume_init() };
        (atime.try_into_time_value()?, mtime.try_into_time_value()?)
    } else {
        let time = wall_time();
        (time, time)
    };
    let set_to_now = times.is_null();
    update_times(AT_FDCWD, path, Some(atime), Some(mtime), 0, set_to_now)?;
    Ok(0)
}

pub fn sys_utimensat(
    dirfd: i32,
    path: *const c_char,
    times: *const [timespec; 2],
    mut flags: u32,
) -> AxResult<isize> {
    if path.is_null() {
        flags |= AT_EMPTY_PATH;
    }
    fn utime_to_duration(time: &timespec) -> Option<AxResult<Duration>> {
        match time.tv_nsec {
            val if val == UTIME_OMIT as _ => None,
            val if val == UTIME_NOW as _ => Some(Ok(wall_time())),
            _ => Some(time.try_into_time_value()),
        }
    }

    let (atime, mtime, set_to_now) = if let Some(times) = times.nullable() {
        // Validate times pointer is in user address space: EFAULT before EBADF.
        // Reject NULL-adjacent pointers (< 4096) which Linux would fault.
        let ptr_addr = times.as_ptr() as usize;
        if ptr_addr < 4096 {
            return Err(AxError::BadAddress);
        }
        check_access(ptr_addr, core::mem::size_of::<[timespec; 2]>())
            .map_err(|_| AxError::BadAddress)?;
        // FIXME: AnyBitPattern
        let [atime_spec, mtime_spec] = unsafe { times.vm_read_uninit()?.assume_init() };
        // Determine if any timestamp is set to an arbitrary (non-NOW, non-OMIT) value.
        // If any timestamp is arbitrary, only the file owner (or root) may set it.
        let atime_is_arbitrary =
            atime_spec.tv_nsec != UTIME_OMIT as _ && atime_spec.tv_nsec != UTIME_NOW as _;
        let mtime_is_arbitrary =
            mtime_spec.tv_nsec != UTIME_OMIT as _ && mtime_spec.tv_nsec != UTIME_NOW as _;
        let any_arbitrary = atime_is_arbitrary || mtime_is_arbitrary;
        (
            utime_to_duration(&atime_spec).transpose()?,
            utime_to_duration(&mtime_spec).transpose()?,
            !any_arbitrary,
        )
    } else {
        let time = wall_time();
        // times == NULL: both timestamps set to current time (weak permission check)
        (Some(time), Some(time), true)
    };
    if atime.is_none() && mtime.is_none() {
        return Ok(0);
    }

    update_times(dirfd, path, atime, mtime, flags, set_to_now)?;
    Ok(0)
}

#[cfg(target_arch = "x86_64")]
pub fn sys_rename(old_path: *const c_char, new_path: *const c_char) -> AxResult<isize> {
    sys_renameat(AT_FDCWD, old_path, AT_FDCWD, new_path)
}

#[cfg(not(target_arch = "riscv64"))]
pub fn sys_renameat(
    old_dirfd: i32,
    old_path: *const c_char,
    new_dirfd: i32,
    new_path: *const c_char,
) -> AxResult<isize> {
    sys_renameat2(old_dirfd, old_path, new_dirfd, new_path, 0)
}

pub fn sys_renameat2(
    old_dirfd: i32,
    old_path: *const c_char,
    new_dirfd: i32,
    new_path: *const c_char,
    flags: u32,
) -> AxResult<isize> {
    let old_path = vm_load_string(old_path)?;
    let new_path = vm_load_string(new_path)?;
    debug!(
        "sys_renameat2 <= old_dirfd: {old_dirfd}, old_path: {old_path:?}, new_dirfd: {new_dirfd}, \
         new_path: {new_path}, flags: {flags}"
    );

    check_path_len(&old_path)?;
    check_path_len(&new_path)?;

    // Validate flags.
    // RENAME_WHITEOUT is unsupported.
    // RENAME_NOREPLACE and RENAME_EXCHANGE are mutually exclusive.
    const SUPPORTED_FLAGS: u32 = RENAME_NOREPLACE | RENAME_EXCHANGE;
    if flags & RENAME_WHITEOUT != 0 {
        return Err(AxError::InvalidInput);
    }
    if flags & !(SUPPORTED_FLAGS | RENAME_WHITEOUT) != 0 {
        return Err(AxError::InvalidInput);
    }
    if flags & SUPPORTED_FLAGS == SUPPORTED_FLAGS {
        // NOREPLACE | EXCHANGE together is EINVAL (LTP case 4)
        return Err(AxError::InvalidInput);
    }

    let is_exchange = flags & RENAME_EXCHANGE != 0;

    let (old_dir, old_name, old) = with_fs_at(old_dirfd, &old_path, |fs| {
        let (old_dir, old_name) = fs.resolve_parent(Path::new(&old_path))?;
        let old = old_dir.lookup_no_follow(&old_name)?;
        Ok((old_dir, old_name.into_owned(), old))
    })?;
    let (new_dir, new_name, new) = with_fs_at(new_dirfd, &new_path, |fs| {
        let (new_dir, new_name) = fs.resolve_nonexistent(Path::new(&new_path))?;
        let new = match new_dir.lookup_no_follow(new_name) {
            Ok(loc) => Some(loc),
            Err(AxError::NotFound) => None,
            Err(err) => return Err(err),
        };
        Ok((new_dir, String::from(new_name), new))
    })?;

    if !old_dir.same_mountpoint(&new_dir) {
        return Err(AxError::CrossesDevices);
    }

    // RENAME_EXCHANGE requires both paths to exist.
    if is_exchange && new.is_none() {
        return Err(AxError::NotFound);
    }

    // Handle same-inode no-op and RENAME_NOREPLACE target-exists check
    if let Some(new) = new.as_ref() {
        if old.same_mountpoint(new) && old.inode() == new.inode() {
            return Ok(0);
        }
        if !is_exchange && flags & RENAME_NOREPLACE != 0 {
            return Err(AxError::AlreadyExists);
        }
    }

    // Type checks: for exchange Linux allows different types, so skip.
    if !is_exchange {
        check_rename_type(&old, new.as_ref())?;
    }

    check_writable_filesystem(&old_dir)?;
    check_writable_filesystem(&new_dir)?;

    let credentials = VfsCredentials::effective();
    check_permission(&old_dir, credentials, AccessMode::WRITE | AccessMode::EXEC)?;
    check_permission(&new_dir, credentials, AccessMode::WRITE | AccessMode::EXEC)?;
    check_not_immutable(&old)?;
    check_not_append_only(&old)?;
    if let Some(new) = new.as_ref() {
        check_not_immutable(new)?;
        check_not_append_only(new)?;
    }
    check_sticky_removal(&old_dir, &old, credentials)?;
    if let Some(new) = new.as_ref() {
        check_sticky_removal(&new_dir, new, credentials)?;
    }

    if is_exchange {
        old_dir.exchange(&old_name, &new_dir, &new_name)?;
    } else {
        old_dir.rename(&old_name, &new_dir, &new_name)?;
    }
    Ok(0)
}

pub fn sys_sync() -> AxResult<isize> {
    FS_CONTEXT.lock().root_dir().filesystem().flush()?;
    Ok(0)
}

pub fn sys_syncfs(fd: i32) -> AxResult<isize> {
    let file_like = get_file_like(fd)?;
    let loc = if let Some(file) = file_like.downcast_ref::<File>() {
        file.inner().location()
    } else if let Some(dir) = file_like.downcast_ref::<Directory>() {
        dir.inner()
    } else {
        return Err(AxError::BadFileDescriptor);
    };

    loc.filesystem().flush()?;
    Ok(0)
}

// ---------------------------------------------------------------------------
// Extended attributes (xattr)
// ---------------------------------------------------------------------------

fn resolve_xattr_target(
    dirfd: i32,
    path: *const c_char,
    follow: bool,
) -> AxResult<Location> {
    let path = vm_load_string(path)?;
    debug!("resolve_xattr_target: dirfd={dirfd}, path={path:?}, follow={follow}");

    let credentials = VfsCredentials::effective();
    if !path.is_empty() {
        check_path_len(&path)?;
        with_fs_at(dirfd, &path, |fs| check_path_search(fs, &path, credentials))?;
    }
    let res = resolve_at(dirfd, (!path.is_empty()).then(|| path.as_str()), if follow {
        0
    } else {
        AT_SYMLINK_NOFOLLOW
    })?;
    res.into_file().ok_or(AxError::BadFileDescriptor)
}

fn resolve_xattr_fd(fd: i32) -> AxResult<Option<Location>> {
    debug!("resolve_xattr_fd: fd={fd}");
    let file_like = get_file_like(fd)?;
    if is_o_path_fd(file_like.as_ref()) {
        return Err(AxError::BadFileDescriptor);
    }
    if let Some(file) = file_like.downcast_ref::<File>() {
        Ok(Some(file.inner().location().clone()))
    } else if let Some(dir) = file_like.downcast_ref::<Directory>() {
        Ok(Some(dir.inner().clone()))
    } else if let Some(pipe) = file_like.downcast_ref::<NamedPipe>() {
        Ok(Some(pipe.location().clone()))
    } else if file_like.downcast_ref::<Socket>().is_some() {
        // Sockets are valid fds but do not support xattr.
        Ok(None)
    } else {
        Err(AxError::BadFileDescriptor)
    }
}

fn is_o_path_fd(file_like: &dyn FileLike) -> bool {
    file_like.access_mode() & O_PATH != 0
}

fn check_not_o_path_fd(fd: i32) -> AxResult<()> {
    let file_like = get_file_like(fd)?;
    if is_o_path_fd(file_like.as_ref()) {
        Err(AxError::BadFileDescriptor)
    } else {
        Ok(())
    }
}

fn path_refers_to_o_path_fd(path: &str) -> bool {
    let Some(fd) = proc_fd_path_fd(path) else {
        return false;
    };
    get_file_like(fd as _)
        .map(|file_like| is_o_path_fd(file_like.as_ref()))
        .unwrap_or(false)
}

fn proc_fd_path_fd(path: &str) -> Option<u32> {
    if let Some(fd) = path.strip_prefix("/proc/self/fd/") {
        return parse_proc_fd_tail(fd);
    }

    let rest = path.strip_prefix("/proc/")?;
    let (pid, fd) = rest.split_once("/fd/")?;
    let pid = pid.parse::<u64>().ok()?;
    let curr = current();
    let thread = curr.as_thread();
    let current_pid = u64::from(thread.proc_data.proc.pid());
    let current_tid = curr.id().as_u64();
    if pid == current_pid || pid == current_tid {
        parse_proc_fd_tail(fd)
    } else {
        None
    }
}

fn parse_proc_fd_tail(fd: &str) -> Option<u32> {
    if fd.is_empty() || fd.contains('/') {
        None
    } else {
        fd.parse().ok()
    }
}

// setxattr — follow symlinks
pub fn sys_setxattr(
    path: *const c_char,
    name: *const c_char,
    value: *const u8,
    size: usize,
    flags: u32,
) -> AxResult<isize> {
    let name = vm_load_string(name)?;
    debug!("sys_setxattr <= path: {path:?}, name: {name}, size: {size}, flags: {flags}");

    let loc = resolve_xattr_target(AT_FDCWD, path, true)?;
    check_writable_filesystem(&loc)?;
    check_not_immutable(&loc)?;
    check_not_append_only(&loc)?;
    let value_buf = read_xattr_value(value, size)?;
    do_setxattr(&loc, name.as_bytes(), &value_buf, flags)?;
    Ok(0)
}

// lsetxattr — do NOT follow symlinks
pub fn sys_lsetxattr(
    path: *const c_char,
    name: *const c_char,
    value: *const u8,
    size: usize,
    flags: u32,
) -> AxResult<isize> {
    let name = vm_load_string(name)?;
    debug!("sys_lsetxattr <= path: {path:?}, name: {name}, size: {size}, flags: {flags}");

    let loc = resolve_xattr_target(AT_FDCWD, path, false)?;
    check_writable_filesystem(&loc)?;
    check_not_immutable(&loc)?;
    check_not_append_only(&loc)?;
    let value_buf = read_xattr_value(value, size)?;
    do_setxattr(&loc, name.as_bytes(), &value_buf, flags)?;
    Ok(0)
}

// fsetxattr — operate on fd
pub fn sys_fsetxattr(
    fd: i32,
    name: *const c_char,
    value: *const u8,
    size: usize,
    flags: u32,
) -> AxResult<isize> {
    let name = vm_load_string(name)?;
    debug!("sys_fsetxattr <= fd: {fd}, name: {name}, size: {size}, flags: {flags}");

    let loc = resolve_xattr_fd(fd)?;
    let loc = loc.ok_or(AxError::OperationNotSupported)?;
    check_writable_filesystem(&loc)?;
    check_not_immutable(&loc)?;
    check_not_append_only(&loc)?;
    let value_buf = read_xattr_value(value, size)?;
    do_setxattr(&loc, name.as_bytes(), &value_buf, flags)?;
    Ok(0)
}

/// Read xattr value from user space into a kernel buffer (validates user pointer).
/// Size limit is enforced by `do_setxattr`; this function only validates the
/// user pointer and copies the data.
fn read_xattr_value(value: *const u8, size: usize) -> AxResult<Vec<u8>> {
    if size == 0 {
        return Ok(vec![]);
    }
    let mut buf: Vec<core::mem::MaybeUninit<u8>> = vec![core::mem::MaybeUninit::uninit(); size];
    vm_read_slice(value, &mut buf)?;
    // SAFETY: vm_read_slice succeeded, all elements are initialized.
    Ok(unsafe { core::mem::transmute::<Vec<core::mem::MaybeUninit<u8>>, Vec<u8>>(buf) })
}

// getxattr — follow symlinks
pub fn sys_getxattr(
    path: *const c_char,
    name: *const c_char,
    value: *mut u8,
    size: usize,
) -> AxResult<isize> {
    let name = vm_load_string(name)?;
    debug!("sys_getxattr <= path: {path:?}, name: {name}, size: {size}");

    let loc = resolve_xattr_target(AT_FDCWD, path, true)?;
    let data = do_getxattr(&loc, name.as_bytes())?;
    xattr_copy_to_user(value, size, &data)
}

// lgetxattr — do NOT follow symlinks
pub fn sys_lgetxattr(
    path: *const c_char,
    name: *const c_char,
    value: *mut u8,
    size: usize,
) -> AxResult<isize> {
    let name = vm_load_string(name)?;
    debug!("sys_lgetxattr <= path: {path:?}, name: {name}, size: {size}");

    let loc = resolve_xattr_target(AT_FDCWD, path, false)?;
    let data = do_getxattr(&loc, name.as_bytes())?;
    xattr_copy_to_user(value, size, &data)
}

// fgetxattr — operate on fd
pub fn sys_fgetxattr(
    fd: i32,
    name: *const c_char,
    value: *mut u8,
    size: usize,
) -> AxResult<isize> {
    let name = vm_load_string(name)?;
    debug!("sys_fgetxattr <= fd: {fd}, name: {name}, size: {size}");

    let loc = resolve_xattr_fd(fd)?;
    let loc = loc.ok_or(AxError::from(LinuxError::ENODATA))?;
    let data = do_getxattr(&loc, name.as_bytes())?;
    xattr_copy_to_user(value, size, &data)
}

// listxattr — follow symlinks
pub fn sys_listxattr(
    path: *const c_char,
    list: *mut u8,
    size: usize,
) -> AxResult<isize> {
    debug!("sys_listxattr <= path: {path:?}, size: {size}");

    let loc = resolve_xattr_target(AT_FDCWD, path, true)?;
    let data = do_listxattr(&loc)?;
    xattr_copy_to_user(list, size, &data)
}

// llistxattr — do NOT follow symlinks
pub fn sys_llistxattr(
    path: *const c_char,
    list: *mut u8,
    size: usize,
) -> AxResult<isize> {
    debug!("sys_llistxattr <= path: {path:?}, size: {size}");

    let loc = resolve_xattr_target(AT_FDCWD, path, false)?;
    let data = do_listxattr(&loc)?;
    xattr_copy_to_user(list, size, &data)
}

// flistxattr — operate on fd
pub fn sys_flistxattr(
    fd: i32,
    list: *mut u8,
    size: usize,
) -> AxResult<isize> {
    debug!("sys_flistxattr <= fd: {fd}, size: {size}");

    let loc = resolve_xattr_fd(fd)?;
    let loc = match loc {
        Some(loc) => loc,
        None => return Ok(0),
    };
    let data = do_listxattr(&loc)?;
    xattr_copy_to_user(list, size, &data)
}

/// Copy xattr value/list to user space with size checks.
fn xattr_copy_to_user(buf: *mut u8, size: usize, data: &[u8]) -> AxResult<isize> {
    if size == 0 {
        return Ok(data.len() as isize);
    }
    if size < data.len() {
        return Err(AxError::OutOfRange);
    }
    vm_write_slice(buf, data)?;
    Ok(data.len() as isize)
}

// removexattr — follow symlinks
pub fn sys_removexattr(
    path: *const c_char,
    name: *const c_char,
) -> AxResult<isize> {
    let name = vm_load_string(name)?;
    debug!("sys_removexattr <= path: {path:?}, name: {name}");

    let loc = resolve_xattr_target(AT_FDCWD, path, true)?;
    check_writable_filesystem(&loc)?;
    check_not_immutable(&loc)?;
    check_not_append_only(&loc)?;
    do_removexattr(&loc, name.as_bytes())?;
    Ok(0)
}

// lremovexattr — do NOT follow symlinks
pub fn sys_lremovexattr(
    path: *const c_char,
    name: *const c_char,
) -> AxResult<isize> {
    let name = vm_load_string(name)?;
    debug!("sys_lremovexattr <= path: {path:?}, name: {name}");

    let loc = resolve_xattr_target(AT_FDCWD, path, false)?;
    check_writable_filesystem(&loc)?;
    check_not_immutable(&loc)?;
    check_not_append_only(&loc)?;
    do_removexattr(&loc, name.as_bytes())?;
    Ok(0)
}

// fremovexattr — operate on fd
pub fn sys_fremovexattr(
    fd: i32,
    name: *const c_char,
) -> AxResult<isize> {
    let name = vm_load_string(name)?;
    debug!("sys_fremovexattr <= fd: {fd}, name: {name}");

    let loc = resolve_xattr_fd(fd)?;
    let loc = loc.ok_or(AxError::from(LinuxError::ENODATA))?;
    check_writable_filesystem(&loc)?;
    check_not_immutable(&loc)?;
    check_not_append_only(&loc)?;
    do_removexattr(&loc, name.as_bytes())?;
    Ok(0)
}
