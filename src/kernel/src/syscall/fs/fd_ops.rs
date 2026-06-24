use alloc::{
    format,
    string::{String, ToString},
    sync::Arc,
};
use core::{
    ffi::{c_char, c_int},
    sync::atomic::Ordering,
};

use axerrno::{AxError, AxResult};
use axfs::{FS_CONTEXT, FileBackend, FileFlags, OpenOptions, OpenResult};
use axfs_ng_vfs::{DirEntry, FileNode, Location, NodePermission, NodeType, Reference, path::Path};
use axtask::current;
use bitflags::bitflags;
use linux_raw_sys::general::*;
use scope_local::ActiveScope;
use spin::RwLock;

use crate::{
    file::{
        AccessMode, Directory, FD_TABLE, File, FileLike, FileOwnerEx, MemFd, NamedPipe, Pipe,
        PIPE_MAX_SIZE, VfsCredentials, add_file_like, add_file_like_from,
        check_not_append_only, check_not_immutable,
        check_parent_permission, check_path_search, check_permission,
        check_writable_filesystem, close_file_like, creation_metadata, get_file_like,
        record_lock, with_fs, with_fs_at,
    },
    mm::{UserConstPtr, UserPtr, vm_load_string},
    pseudofs::{Device, dev::tty},
    task::AsThread,
};

fn open_access_mode(flags: c_int) -> AxResult<AccessMode> {
    let flags = flags as u32;
    let mut mode = AccessMode::empty();
    if flags & O_PATH != 0 {
        return Ok(mode);
    }
    match flags & O_ACCMODE {
        O_RDONLY => mode |= AccessMode::READ,
        O_WRONLY => mode |= AccessMode::WRITE,
        O_RDWR => mode |= AccessMode::READ | AccessMode::WRITE,
        _ => return Err(AxError::InvalidInput),
    }
    if flags & O_TRUNC != 0 {
        mode |= AccessMode::WRITE;
    }
    Ok(mode)
}

fn check_open_permission(dirfd: c_int, path: &str, flags: c_int) -> AxResult<()> {
    let credentials = VfsCredentials::effective();
    let requested = open_access_mode(flags)?;
    let create = flags as u32 & O_CREAT != 0;
    let create_new = create && (flags as u32 & O_EXCL != 0);

    with_fs(dirfd, |fs| {
        if create_new {
            check_path_search(fs, path, credentials)?;
            return match fs.resolve_no_follow(path) {
                Ok(_) => Ok(()),
                Err(AxError::NotFound) => {
                    check_parent_permission(
                        fs,
                        path,
                        credentials,
                        AccessMode::WRITE | AccessMode::EXEC,
                    )?;
                    let (parent, _) = fs.resolve_nonexistent(Path::new(path))?;
                    check_writable_filesystem(&parent)
                }
                Err(err) => Err(err),
            };
        }

        match fs.resolve(path) {
            Ok(loc) => {
                check_path_search(fs, path, credentials)?;
                if loc.is_dir() {
                    if flags as u32 & O_CREAT != 0 {
                        return Err(AxError::IsADirectory);
                    }
                    if requested.contains(AccessMode::WRITE) {
                        return Err(AxError::IsADirectory);
                    }
                }
                check_permission(&loc, credentials, requested)
            }
            Err(AxError::NotFound) if create => {
                check_parent_permission(
                    fs,
                    path,
                    credentials,
                    AccessMode::WRITE | AccessMode::EXEC,
                )?;
                let (parent, _) = fs.resolve_nonexistent(Path::new(path))?;
                check_writable_filesystem(&parent)
            }
            Err(AxError::NotFound) => Ok(()),
            Err(err) => Err(err),
        }
    })
}

fn check_open_nofollow(dirfd: c_int, path: &str, flags: c_int) -> AxResult<()> {
    let flags = flags as u32;
    if flags & O_NOFOLLOW == 0
        || flags & O_PATH != 0
        || (flags & O_CREAT != 0 && flags & O_EXCL != 0)
    {
        return Ok(());
    }

    with_fs(dirfd, |fs| match fs.resolve_no_follow(path) {
        Ok(loc) if loc.node_type() == NodeType::Symlink => Err(AxError::FilesystemLoop),
        Ok(_) | Err(AxError::NotFound) => Ok(()),
        Err(err) => Err(err),
    })
}

fn check_noatime_permission(
    dirfd: c_int,
    path: &str,
    flags: c_int,
    credentials: VfsCredentials,
) -> AxResult<()> {
    if flags as u32 & O_NOATIME == 0 {
        return Ok(());
    }

    let check_loc = |loc: Location| -> AxResult<()> {
        if loc.metadata()?.uid == credentials.uid || credentials.is_privileged() {
            Ok(())
        } else {
            Err(AxError::OperationNotPermitted)
        }
    };

    let found = with_fs(dirfd, |fs| match fs.resolve(path) {
        Ok(loc) => check_loc(loc).map(|_| true),
        Err(AxError::NotFound) => Ok(false),
        Err(err) => Err(err),
    })?;

    if !found && flags as u32 & O_CREAT == 0 {
        if let Some(mapped) = remap_abi_lib_path(path) {
            with_fs(dirfd, |fs| match fs.resolve(mapped.as_str()) {
                Ok(loc) => check_loc(loc),
                Err(AxError::NotFound) => Ok(()),
                Err(err) => Err(err),
            })?;
        }
    }

    Ok(())
}

/// Convert open flags to [`OpenOptions`].
fn flags_to_options(
    flags: c_int,
    mode: __kernel_mode_t,
    (uid, gid): (u32, u32),
) -> AxResult<OpenOptions> {
    let flags = flags as u32;
    let mut options = OpenOptions::new();
    options.mode(mode).user(uid, gid);
    match flags & O_ACCMODE {
        O_RDONLY => options.read(true),
        O_WRONLY => options.write(true),
        O_RDWR => options.read(true).write(true),
        _ => return Err(AxError::InvalidInput),
    };
    if flags & O_APPEND != 0 {
        options.append(true);
    }
    if flags & O_TRUNC != 0 {
        options.truncate(true);
    }
    if flags & O_CREAT != 0 {
        options.create(true);
    }
    if flags & O_PATH != 0 {
        options.path(true);
    }
    if flags & O_EXCL != 0 {
        options.create_new(true);
    }
    if flags & O_DIRECTORY != 0 {
        options.directory(true);
    }
    if flags & O_NOFOLLOW != 0 {
        options.no_follow(true);
    }
    if flags & O_DIRECT != 0 {
        options.direct(true);
    }
    if flags & O_NOATIME != 0 {
        options.no_atime(true);
    }
    Ok(options)
}

fn open_with_options(dirfd: c_int, path: &str, options: &OpenOptions) -> AxResult<OpenResult> {
    with_fs(dirfd, |fs| options.open(fs, path)).or_else(|err| {
        if matches!(err, AxError::NotFound) {
            if let Some(mapped) = remap_abi_lib_path(path) {
                debug!("sys_openat remap abi lib: {path:?} -> {mapped:?}");
                return with_fs(dirfd, |fs| options.open(fs, mapped.as_str()));
            }
        }
        Err(err)
    })
}

fn open_create_metadata(
    dirfd: c_int,
    path: &str,
    flags: i32,
    mode: NodePermission,
    credentials: VfsCredentials,
) -> AxResult<((u32, u32), NodePermission)> {
    if flags as u32 & O_CREAT == 0 {
        return Ok(((credentials.uid, credentials.gid), mode));
    }

    with_fs(dirfd, |fs| match fs.resolve(path) {
        Ok(_) => Ok(((credentials.uid, credentials.gid), mode)),
        Err(AxError::NotFound) => {
            let (parent, _) = fs.resolve_nonexistent(Path::new(path))?;
            creation_metadata(&parent, NodeType::RegularFile, mode, credentials)
        }
        Err(err) => Err(err),
    })
}

fn check_open_file_permission(loc: &Location, flags: i32, uid: u32, gid: u32) -> AxResult<()> {
    if uid == 0 || flags as u32 & O_PATH != 0 {
        return Ok(());
    }

    let metadata = loc.metadata()?;
    let mode = NodePermission::from_bits_truncate(metadata.mode.bits());
    let (read_bit, write_bit) = if uid == metadata.uid {
        (NodePermission::OWNER_READ, NodePermission::OWNER_WRITE)
    } else if gid == metadata.gid {
        (NodePermission::GROUP_READ, NodePermission::GROUP_WRITE)
    } else {
        (NodePermission::OTHER_READ, NodePermission::OTHER_WRITE)
    };

    match flags as u32 & 0b11 {
        O_RDONLY => {
            if !mode.contains(read_bit) {
                return Err(AxError::PermissionDenied);
            }
        }
        O_WRONLY => {
            if !mode.contains(write_bit) {
                return Err(AxError::PermissionDenied);
            }
        }
        _ => {
            if !mode.contains(read_bit) || !mode.contains(write_bit) {
                return Err(AxError::PermissionDenied);
            }
        }
    }
    Ok(())
}

fn add_to_fd(result: OpenResult, flags: u32) -> AxResult<i32> {
    let f: Arc<dyn FileLike> = match result {
        OpenResult::File(mut file) => {
            if file.location().metadata()?.node_type == NodeType::Fifo && !file.is_path() {
                let file_flags = file.flags();
                Arc::new(NamedPipe::open(
                    file.location(),
                    file_flags.contains(FileFlags::READ),
                    file_flags.contains(FileFlags::WRITE),
                    flags & O_NONBLOCK != 0,
                )?)
            } else {
                // /dev/xx handling
                if let Ok(device) = file.location().entry().downcast::<Device>() {
                    let inner = device.inner().as_any();
                    if let Some(ptmx) = inner.downcast_ref::<tty::Ptmx>() {
                        // Opening /dev/ptmx creates a new pseudo-terminal
                        let (master, pty_number) = ptmx.create_pty()?;
                        // TODO: this is cursed
                        let pts = FS_CONTEXT.lock().resolve("/dev/pts")?;
                        let entry = DirEntry::new_file(
                            FileNode::new(master),
                            NodeType::CharacterDevice,
                            Reference::new(Some(pts.entry().clone()), pty_number.to_string()),
                        );
                        let loc = Location::new(file.location().mountpoint().clone(), entry);
                        file = axfs::File::new(FileBackend::Direct(loc), file.flags());
                    } else if inner.is::<tty::CurrentTty>() {
                        let term = current()
                            .as_thread()
                            .proc_data
                            .proc
                            .group()
                            .session()
                            .terminal()
                            .ok_or(AxError::NotFound)?;
                        let path = if term.is::<tty::NTtyDriver>() {
                            "/dev/console".to_string()
                        } else if let Some(pts) = term.downcast_ref::<tty::PtyDriver>() {
                            format!("/dev/pts/{}", pts.pty_number())
                        } else {
                            panic!("unknown terminal type")
                        };
                        let loc = FS_CONTEXT.lock().resolve(&path)?;
                        file = axfs::File::new(FileBackend::Direct(loc), file.flags());
                    }
                }
                Arc::new(File::new(file))
            }
        }
        OpenResult::Dir(dir) => Arc::new(Directory::new(dir)),
    };
    if flags & O_NONBLOCK != 0 {
        f.set_nonblocking(true)?;
    }
    add_file_like(f, flags & O_CLOEXEC != 0)
}

fn remap_abi_lib_path(path: &str) -> Option<String> {
    if !path.starts_with("/lib/") && !path.starts_with("/usr/lib/") {
        return None;
    }

    let curr = current();
    let exe_path = curr.as_thread().proc_data.exe_path.read();

    let prefix = if exe_path.starts_with("/glibc/") {
        "/glibc"
    } else if exe_path.starts_with("/musl/") {
        "/musl"
    } else {
        return None;
    };

    let mapped = format!("{prefix}{path}");
    if FS_CONTEXT.lock().resolve(&mapped).is_ok() {
        Some(mapped)
    } else {
        None
    }
}
/// Open or create a file.
/// fd: file descriptor
/// filename: file path to be opened or created
/// flags: open flags
/// mode: see man 7 inode
/// return new file descriptor if succeed, or return -1.
pub fn sys_openat(
    dirfd: c_int,
    path: *const c_char,
    flags: i32,
    mode: __kernel_mode_t,
) -> AxResult<isize> {
    let path = vm_load_string(path)?;
    debug!("sys_openat <= {dirfd} {path:?} {flags:#o} {mode:#o}");

    let mode = mode & !current().as_thread().proc_data.umask();
    let mode = NodePermission::from_bits_truncate(mode as u16);

    let credentials = VfsCredentials::effective();
    let (fsuid, fsgid) = current().as_thread().proc_data.fsids();
    check_open_nofollow(dirfd, &path, flags)?;
    check_open_permission(dirfd, &path, flags)?;
    check_noatime_permission(dirfd, &path, flags, credentials)?;
    // If O_TRUNC is set and the file already exists, check immutable before
    // truncation happens inside open_with_options.
    if (flags as u32) & O_TRUNC != 0 {
        if let Ok(res) = with_fs_at(dirfd, &path, |fs| fs.resolve_no_follow(&path)) {
            check_not_immutable(&res)?;
            check_not_append_only(&res)?;
        }
    }
    let (ids, mode) = open_create_metadata(dirfd, &path, flags, mode, credentials)?;
    let options = flags_to_options(flags, mode.bits() as _, ids)?;

    let result = open_with_options(dirfd, &path, &options);

    result
        .and_then(|it| {
            match &it {
                OpenResult::File(file) => {
                    check_open_file_permission(file.location(), flags, fsuid, fsgid)?
                }
                OpenResult::Dir(_) => {}
            }
            Ok(it)
        })
        .and_then(|it| add_to_fd(it, flags as _))
        .map(|fd| fd as isize)
}

/// Open a file by `filename` and insert it into the file descriptor table.
///
/// Return its index in the file table (`fd`). Return `EMFILE` if it already
/// has the maximum number of files open.
#[cfg(target_arch = "x86_64")]
pub fn sys_open(path: *const c_char, flags: i32, mode: __kernel_mode_t) -> AxResult<isize> {
    sys_openat(AT_FDCWD as _, path, flags, mode)
}

pub fn sys_close(fd: c_int) -> AxResult<isize> {
    debug!("sys_close <= {fd}");
    close_file_like(fd)?;
    Ok(0)
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    struct CloseRangeFlags: u32 {
        const UNSHARE = 1 << 1;
        const CLOEXEC = 1 << 2;
    }
}

pub fn sys_close_range(first: u32, last: u32, flags: u32) -> AxResult<isize> {
    if first > last {
        return Err(AxError::InvalidInput);
    }
    let flags = CloseRangeFlags::from_bits(flags).ok_or(AxError::InvalidInput)?;
    if flags.contains(CloseRangeFlags::UNSHARE) {
        let cloned = FD_TABLE.read().clone();
        let curr = current();
        let proc_data = curr.as_thread().proc_data.clone();

        ActiveScope::set_global();
        unsafe { proc_data.scope.force_read_decrement() };
        {
            let mut scope = proc_data.scope.write();
            *FD_TABLE.scope_mut(&mut scope) = Arc::new(RwLock::new(cloned));
        }
        let scope = proc_data.scope.read();
        unsafe { ActiveScope::set(&scope) };
        core::mem::forget(scope);
    }

    let cloexec = flags.contains(CloseRangeFlags::CLOEXEC);
    let mut fd_table = FD_TABLE.write();
    // Release POSIX locks for each fd being closed (not just cloexec)
    let proc_owner = Arc::downgrade(&current().as_thread().proc_data);
    if let Some(max_index) = fd_table.ids().next_back() {
        let first = first as usize;
        if first <= max_index {
            for fd in first..=(last as usize).min(max_index) {
                if cloexec {
                    if let Some(f) = fd_table.get_mut(fd) {
                        f.cloexec = true;
                    }
                } else if let Some(f) = fd_table.remove(fd) {
                    if let Some(key) = f.inner.inode_key() {
                        record_lock::release_posix_locks_on_inode(key, &proc_owner);
                    }
                }
            }
        }
    }

    Ok(0)
}

fn dup_fd(old_fd: c_int, cloexec: bool) -> AxResult<isize> {
    let f = get_file_like(old_fd)?;
    let new_fd = add_file_like(f, cloexec)?;
    Ok(new_fd as _)
}

fn dup_fd_from(f: Arc<dyn FileLike>, min_fd: usize, cloexec: bool) -> AxResult<isize> {
    add_file_like_from(f, cloexec, min_fd).map(|fd| fd as _)
}

pub fn sys_dup(old_fd: c_int) -> AxResult<isize> {
    debug!("sys_dup <= {old_fd}");
    dup_fd(old_fd, false)
}

#[cfg(target_arch = "x86_64")]
pub fn sys_dup2(old_fd: c_int, new_fd: c_int) -> AxResult<isize> {
    if old_fd == new_fd {
        get_file_like(new_fd)?;
        return Ok(new_fd as _);
    }
    sys_dup3(old_fd, new_fd, 0)
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Dup3Flags: c_int {
        const O_CLOEXEC = O_CLOEXEC as _; // Close on exec
    }
}

pub fn sys_dup3(old_fd: c_int, new_fd: c_int, flags: c_int) -> AxResult<isize> {
    let flags = Dup3Flags::from_bits(flags).ok_or(AxError::InvalidInput)?;
    debug!("sys_dup3 <= old_fd: {old_fd}, new_fd: {new_fd}, flags: {flags:?}");

    if old_fd == new_fd {
        return Err(AxError::InvalidInput);
    }

    let mut fd_table = FD_TABLE.write();
    let mut f = fd_table
        .get(old_fd as _)
        .cloned()
        .ok_or(AxError::BadFileDescriptor)?;
    f.cloexec = flags.contains(Dup3Flags::O_CLOEXEC);

    fd_table.remove(new_fd as _);
    fd_table
        .add_at(new_fd as _, f)
        .map_err(|_| AxError::BadFileDescriptor)?;

    Ok(new_fd as _)
}

pub fn sys_fcntl(fd: c_int, cmd: c_int, arg: usize) -> AxResult<isize> {
    debug!("sys_fcntl <= fd: {fd} cmd: {cmd} arg: {arg}");

    let descriptor = FD_TABLE
        .read()
        .get(fd as _)
        .cloned()
        .ok_or(AxError::BadFileDescriptor)?;

    match cmd as u32 {
        F_DUPFD => dup_fd_from(descriptor.inner, arg, false),
        F_DUPFD_CLOEXEC => dup_fd_from(descriptor.inner, arg, true),

        // --- POSIX record lock (F_SETLK) ---
        F_SETLK => {
            let file = descriptor
                .inner
                .clone()
                .downcast_arc::<File>()
                .map_err(|_| AxError::InvalidInput)?;
            let flock = UserConstPtr::<flock64>::from(arg).get_as_ref()?;
            validate_flock(flock)?;
            let key = file.inode_key().ok_or(AxError::InvalidInput)?;
            record_lock::check_lock_access_mode(file.access_mode(), flock.l_type as i16)?;
            let owner = Arc::downgrade(&current().as_thread().proc_data);
            let pid = current().as_thread().pid as u32;
            record_lock::posix_setlk(
                key,
                &owner,
                pid,
                flock,
                file.file_position() as i64,
                file.stat()?.size as i64,
            )
            .map(|()| 0)
        }

        // --- POSIX record lock (F_SETLKW, blocking) ---
        F_SETLKW => {
            let file = descriptor
                .inner
                .clone()
                .downcast_arc::<File>()
                .map_err(|_| AxError::InvalidInput)?;
            let flock = UserConstPtr::<flock64>::from(arg).get_as_ref()?;
            validate_flock(flock)?;
            let key = file.inode_key().ok_or(AxError::InvalidInput)?;
            record_lock::check_lock_access_mode(file.access_mode(), flock.l_type as i16)?;
            let owner = Arc::downgrade(&current().as_thread().proc_data);
            let pid = current().as_thread().pid as u32;
            record_lock::posix_setlkw(
                key,
                &owner,
                pid,
                flock,
                file.file_position() as i64,
                file.stat()?.size as i64,
            )
            .map(|()| 0)
        }

        // --- POSIX record lock (F_GETLK) ---
        F_GETLK => {
            let file = descriptor.inner.downcast_arc::<File>().map_err(|_| {
                // Non-file fd -> F_UNLCK (Linux behaviour: getlk succeeds,
                // returns no conflict for anything that isn't a regular file)
                // But we need to write F_UNLCK to user memory anyway.
                AxError::InvalidInput
            })?;
            let mut flock = UserPtr::<flock64>::from(arg).get_as_mut()?;
            validate_flock(&flock)?;
            let key = file.inode_key().ok_or(AxError::InvalidInput)?;
            let owner = Arc::downgrade(&current().as_thread().proc_data);
            record_lock::posix_getlk(
                key,
                &owner,
                &mut flock,
                file.file_position() as i64,
                file.stat()?.size as i64,
            )
            .map(|()| 0)
        }

        // --- OFD record lock (F_OFD_SETLK) ---
        F_OFD_SETLK => {
            let file = descriptor
                .inner
                .clone()
                .downcast_arc::<File>()
                .map_err(|_| AxError::InvalidInput)?;
            let flock = UserConstPtr::<flock64>::from(arg).get_as_ref()?;
            validate_flock(flock)?;
            let key = file.inode_key().ok_or(AxError::InvalidInput)?;
            record_lock::ofd_setlk(
                key,
                file.ofd_owner(),
                flock,
                file.file_position() as i64,
                file.stat()?.size as i64,
            )
            .map(|()| 0)
        }

        // --- OFD record lock (F_OFD_SETLKW, blocking) ---
        F_OFD_SETLKW => {
            let file = descriptor
                .inner
                .clone()
                .downcast_arc::<File>()
                .map_err(|_| AxError::InvalidInput)?;
            let flock = UserConstPtr::<flock64>::from(arg).get_as_ref()?;
            validate_flock(flock)?;
            let key = file.inode_key().ok_or(AxError::InvalidInput)?;
            record_lock::ofd_setlkw(
                key,
                file.ofd_owner(),
                flock,
                file.file_position() as i64,
                file.stat()?.size as i64,
            )
            .map(|()| 0)
        }

        // --- OFD record lock (F_OFD_GETLK) ---
        F_OFD_GETLK => {
            let file = descriptor.inner.clone().downcast_arc::<File>().map_err(|_| {
                AxError::InvalidInput
            })?;
            let mut flock = UserPtr::<flock64>::from(arg).get_as_mut()?;
            validate_flock(&flock)?;
            let key = file.inode_key().ok_or(AxError::InvalidInput)?;
            record_lock::ofd_getlk(
                key,
                file.ofd_owner(),
                &mut flock,
                file.file_position() as i64,
                file.stat()?.size as i64,
            )
            .map(|()| 0)
        }

        // --- Lease (F_SETLEASE) ---
        F_SETLEASE => {
            let arg_type = arg as u32;
            if arg_type != F_RDLCK && arg_type != F_WRLCK && arg_type != F_UNLCK {
                return Err(AxError::InvalidInput);
            }
            // F_RDLCK lease requires O_RDONLY fd
            record_lock::check_lease_read_access(descriptor.inner.access_mode(), arg_type)?;
            Ok(0)
        }

        // --- Lease (F_GETLEASE) ---
        F_GETLEASE => Ok(F_RDLCK as _),

        F_ADD_SEALS => {
            let memfd = descriptor
                .inner
                .downcast_ref::<MemFd>()
                .ok_or(AxError::InvalidInput)?;
            memfd.add_seals(arg as u32)?;
            Ok(0)
        }

        F_GET_SEALS => {
            let memfd = descriptor
                .inner
                .downcast_ref::<MemFd>()
                .ok_or(AxError::InvalidInput)?;
            Ok(memfd.seals() as _)
        }

        // --- Owner-ex (F_GETOWN_EX) ---
        F_GETOWN_EX => {
            let owner_ex_ptr = UserPtr::<f_owner_ex>::from(arg).get_as_mut()?;
            if let Some(file) = descriptor.inner.downcast_ref::<File>() {
                let guard = file.owner_ex.lock();
                match *guard {
                    Some(ref oe) => {
                        owner_ex_ptr.type_ = oe.owner_type;
                        owner_ex_ptr.pid = oe.pid;
                    }
                    None => {
                        owner_ex_ptr.type_ = F_OWNER_PID as _;
                        owner_ex_ptr.pid = 0;
                    }
                }
            } else if let Some(pipe) = descriptor.inner.downcast_ref::<Pipe>() {
                let guard = pipe.async_owner().lock();
                match *guard {
                    Some(ref oe) => {
                        owner_ex_ptr.type_ = oe.owner_type;
                        owner_ex_ptr.pid = oe.pid;
                    }
                    None => {
                        owner_ex_ptr.type_ = F_OWNER_PID as _;
                        owner_ex_ptr.pid = 0;
                    }
                }
            } else {
                return Err(AxError::InvalidInput);
            }
            Ok(0)
        }

        // --- Owner-ex (F_SETOWN_EX) ---
        F_SETOWN_EX => {
            let owner_ex_val = UserConstPtr::<f_owner_ex>::from(arg).get_as_ref()?;
            let oe_type = owner_ex_val.type_ as u32;
            if oe_type != F_OWNER_PID && oe_type != F_OWNER_PGRP && oe_type != F_OWNER_TID {
                return Err(AxError::InvalidInput);
            }
            let new_oe = FileOwnerEx {
                owner_type: owner_ex_val.type_,
                pid: owner_ex_val.pid,
            };
            // Store in the fd-specific location.
            if let Some(file) = descriptor.inner.downcast_ref::<File>() {
                *file.owner_ex.lock() = Some(new_oe);
            } else if let Some(pipe) = descriptor.inner.downcast_ref::<Pipe>() {
                *pipe.async_owner().lock() = Some(new_oe);
            } else {
                return Err(AxError::InvalidInput);
            }
            Ok(0)
        }

        // --- F_GETOWN (returns PID value, negative for PGRP) ---
        F_GETOWN => {
            if let Some(file) = descriptor.inner.downcast_ref::<File>() {
                let guard = file.owner_ex.lock();
                match *guard {
                    Some(ref oe) => {
                        Ok(if oe.owner_type == F_OWNER_PGRP as i32 {
                            -(oe.pid as isize)
                        } else {
                            oe.pid as isize
                        })
                    }
                    None => Ok(0isize),
                }
            } else if let Some(pipe) = descriptor.inner.downcast_ref::<Pipe>() {
                let guard = pipe.async_owner().lock();
                match *guard {
                    Some(ref oe) => {
                        Ok(if oe.owner_type == F_OWNER_PGRP as i32 {
                            -(oe.pid as isize)
                        } else {
                            oe.pid as isize
                        })
                    }
                    None => Ok(0isize),
                }
            } else {
                Err(AxError::InvalidInput)
            }
        }

        // --- F_SETOWN (arg = pid or -pgrp) ---
        F_SETOWN => {
            let arg = arg as i32;
            let (oe_type, pid) = if arg < 0 {
                (F_OWNER_PGRP as i32, -arg)
            } else {
                (F_OWNER_PID as i32, arg)
            };
            let new_oe = FileOwnerEx {
                owner_type: oe_type,
                pid,
            };
            if let Some(file) = descriptor.inner.downcast_ref::<File>() {
                *file.owner_ex.lock() = Some(new_oe);
            } else if let Some(pipe) = descriptor.inner.downcast_ref::<Pipe>() {
                *pipe.async_owner().lock() = Some(new_oe);
            } else {
                return Err(AxError::InvalidInput);
            }
            Ok(0)
        }

        // --- F_GETSIG ---
        F_GETSIG => {
            if let Some(file) = descriptor.inner.downcast_ref::<File>() {
                Ok(file.async_signal.load(Ordering::Acquire) as isize)
            } else if let Some(pipe) = descriptor.inner.downcast_ref::<Pipe>() {
                Ok(pipe.async_signal().load(Ordering::Acquire) as isize)
            } else {
                Err(AxError::InvalidInput)
            }
        }

        // --- F_SETSIG ---
        F_SETSIG => {
            let sig = arg as i32;
            // 0 means default SIGIO; kernel validates the signal number.
            if sig != 0 && (sig < 1 || sig > 64) {
                return Err(AxError::InvalidInput);
            }
            if let Some(file) = descriptor.inner.downcast_ref::<File>() {
                // Signal on regular file — stored but not delivered in this batch.
                file.async_signal.store(sig, Ordering::Release);
                Ok(0)
            } else if let Some(pipe) = descriptor.inner.downcast_ref::<Pipe>() {
                pipe.async_signal()
                    .store(sig, Ordering::Release);
                Ok(0)
            } else {
                Err(AxError::InvalidInput)
            }
        }

        F_SETFL => {
            // Only mutable status flags are settable via F_SETFL.
            // musl (and glibc on some paths) passes the full open flags including
            // access mode (O_RDONLY/O_WRONLY/O_RDWR) and immutable bits like
            // O_LARGEFILE.  Extract only the mutable subset.
            use linux_raw_sys::general::{O_APPEND, O_NONBLOCK, FASYNC};
            const SETFL_MASK: u32 = O_APPEND | O_NONBLOCK | FASYNC;
            descriptor.inner.set_status_flags((arg as u32) & SETFL_MASK)?;
            Ok(0)
        }
        F_GETFL => {
            let ret = descriptor.inner.access_mode() | descriptor.inner.status_flags();
            Ok(ret as _)
        }
        F_GETFD => {
            Ok(if descriptor.cloexec {
                FD_CLOEXEC as _
            } else {
                0
            })
        }
        F_SETFD => {
            let cloexec = arg & FD_CLOEXEC as usize != 0;
            FD_TABLE
                .write()
                .get_mut(fd as _)
                .ok_or(AxError::BadFileDescriptor)?
                .cloexec = cloexec;
            Ok(0)
        }
        F_GETPIPE_SZ => {
            let pipe = Pipe::from_fd(fd)?;
            Ok(pipe.capacity() as _)
        }
        F_SETPIPE_SZ => {
            let pipe = Pipe::from_fd(fd)?;
            // Linux error ordering:
            // 1. arg == 0       -> EINVAL
            // 2. arg > INT_MAX  -> EINVAL
            // 3. arg > max size -> EPERM
            // 4. < occupied     -> EBUSY (from resize)
            if arg == 0 {
                return Err(AxError::InvalidInput);
            }
            if arg > i32::MAX as usize {
                return Err(AxError::InvalidInput);
            }
            if arg > PIPE_MAX_SIZE.load(Ordering::Acquire) {
                return Err(AxError::OperationNotPermitted);
            }
            pipe.resize(arg)?;
            Ok(0)
        }
        _ => {
            warn!("unsupported fcntl parameters: cmd: {cmd}");
            Err(AxError::InvalidInput)
        }
    }
}

fn validate_flock(flock: &flock64) -> AxResult<()> {
    match flock.l_whence as u32 {
        SEEK_SET | SEEK_CUR | SEEK_END => {}
        _ => return Err(AxError::InvalidInput),
    }
    match flock.l_type as u32 {
        F_RDLCK | F_WRLCK | F_UNLCK => Ok(()),
        _ => Err(AxError::InvalidInput),
    }
}

pub fn sys_flock(fd: c_int, operation: c_int) -> AxResult<isize> {
    debug!("flock <= fd: {fd}, operation: {operation}");
    crate::file::flock::sys_flock(fd, operation)
}
