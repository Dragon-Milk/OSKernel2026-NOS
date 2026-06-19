use alloc::{borrow::Cow, string::ToString, sync::Arc};
use core::{
    ffi::c_int,
    hint::likely,
    sync::atomic::{AtomicBool, AtomicI32, Ordering},
    task::Context,
};

use axerrno::{AxError, AxResult};
use axfs::{FS_CONTEXT, FileFlags, FsContext, SYMLINKS_MAX};
use axfs_ng_vfs::{
    DeviceId, Location, Metadata, NodeFlags, NodePermission, NodeType,
    path::{Component, Path, PathBuf},
};
use axpoll::{IoEvents, Pollable};
use axsync::Mutex;
use axtask::{
    current,
    future::{block_on, poll_io},
};
use linux_raw_sys::general::{
    AT_EMPTY_PATH, AT_FDCWD, AT_SYMLINK_NOFOLLOW, MS_RDONLY, O_APPEND, O_NONBLOCK, O_PATH, O_RDWR,
    O_NOATIME, O_WRONLY,
};

use super::{FileLike, Kstat, get_file_like, get_inode_flags};
use crate::{
    file::{FileOwnerEx, IoDst, IoSrc, record_lock},
    mm::busybox_applet,
    task::AsThread,
};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct AccessMode: u8 {
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const EXEC = 1 << 2;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VfsCredentials {
    pub uid: u32,
    pub gid: u32,
}

impl VfsCredentials {
    pub fn real() -> Self {
        let credentials = current().as_thread().proc_data.credentials();
        Self {
            uid: credentials.real_uid,
            gid: credentials.real_gid,
        }
    }

    pub fn effective() -> Self {
        let credentials = current().as_thread().proc_data.credentials();
        Self {
            uid: credentials.effective_uid,
            gid: credentials.effective_gid,
        }
    }

    pub fn is_privileged(self) -> bool {
        self.uid == 0
    }

    pub fn in_group(self, gid: u32) -> bool {
        self.gid == gid
            || current()
                .as_thread()
                .proc_data
                .has_supplementary_group(gid)
    }
}

const PATH_MAX: usize = 4096;

pub fn check_path_len(path: &str) -> AxResult<()> {
    if path.len() >= PATH_MAX {
        Err(AxError::NameTooLong)
    } else {
        Ok(())
    }
}

pub fn with_fs<R>(dirfd: c_int, f: impl FnOnce(&mut FsContext) -> AxResult<R>) -> AxResult<R> {
    let mut fs = FS_CONTEXT.lock();
    if dirfd == AT_FDCWD {
        f(&mut fs)
    } else {
        let dir = Directory::from_fd(dirfd)?.inner.clone();
        f(&mut fs.with_current_dir(dir)?)
    }
}

pub fn with_fs_at<R>(
    dirfd: c_int,
    path: &str,
    f: impl FnOnce(&mut FsContext) -> AxResult<R>,
) -> AxResult<R> {
    let mut fs = FS_CONTEXT.lock();
    if dirfd == AT_FDCWD || Path::new(path).is_absolute() {
        f(&mut fs)
    } else {
        let dir = Directory::from_fd(dirfd)?.inner.clone();
        f(&mut fs.with_current_dir(dir)?)
    }
}

fn class_bits(metadata: &Metadata, credentials: VfsCredentials) -> u16 {
    let mode = metadata.mode.bits();
    if credentials.uid == metadata.uid {
        (mode >> 6) & 0o7
    } else if credentials.in_group(metadata.gid) {
        (mode >> 3) & 0o7
    } else {
        mode & 0o7
    }
}

pub fn check_permission(
    loc: &Location,
    credentials: VfsCredentials,
    requested: AccessMode,
) -> AxResult<()> {
    if requested.is_empty() {
        return Ok(());
    }

    let metadata = loc.metadata()?;
    let mode = metadata.mode.bits();
    if credentials.uid == 0 {
        if requested.contains(AccessMode::EXEC)
            && metadata.node_type != NodeType::Directory
            && mode & 0o111 == 0
        {
            return Err(AxError::PermissionDenied);
        }
        return Ok(());
    }

    let bits = class_bits(&metadata, credentials);
    if requested.contains(AccessMode::READ) && bits & 0o4 == 0 {
        return Err(AxError::PermissionDenied);
    }
    if requested.contains(AccessMode::WRITE) && bits & 0o2 == 0 {
        return Err(AxError::PermissionDenied);
    }
    if requested.contains(AccessMode::EXEC) && bits & 0o1 == 0 {
        return Err(AxError::PermissionDenied);
    }
    Ok(())
}

pub fn clear_setgid_if_not_in_group(
    mut mode: NodePermission,
    gid: u32,
    credentials: VfsCredentials,
) -> NodePermission {
    if mode.contains(NodePermission::SET_GID)
        && !credentials.is_privileged()
        && !credentials.in_group(gid)
    {
        mode.remove(NodePermission::SET_GID);
    }
    mode
}

pub fn creation_metadata(
    parent: &Location,
    node_type: NodeType,
    requested_mode: NodePermission,
    credentials: VfsCredentials,
) -> AxResult<((u32, u32), NodePermission)> {
    let parent_meta = parent.metadata()?;
    let parent_setgid = parent_meta.mode.contains(NodePermission::SET_GID);
    let gid = if parent_setgid {
        parent_meta.gid
    } else {
        credentials.gid
    };
    let mut mode = requested_mode;

    match node_type {
        NodeType::Directory if parent_setgid => mode.insert(NodePermission::SET_GID),
        NodeType::RegularFile => {
            mode = clear_setgid_if_not_in_group(mode, gid, credentials);
        }
        _ => {}
    }

    Ok(((credentials.uid, gid), mode))
}

pub fn check_writable_filesystem(loc: &Location) -> AxResult<()> {
    if loc.filesystem().stat()?.mount_flags & MS_RDONLY != 0 {
        return Err(AxError::ReadOnlyFilesystem);
    }
    Ok(())
}

pub fn check_path_search(fs: &FsContext, path: &str, credentials: VfsCredentials) -> AxResult<()> {
    let path = Path::new(path);
    let mut dir = if path.is_absolute() {
        fs.root_dir().clone()
    } else {
        fs.current_dir().clone()
    };
    let mut follow_count = 0;
    let mut components = path.components().peekable();

    while let Some(component) = components.next() {
        let is_last = components.peek().is_none();
        match component {
            Component::RootDir => {
                dir = fs.root_dir().clone();
            }
            Component::CurDir => {
                if is_last {
                    check_permission(&dir, credentials, AccessMode::EXEC)?;
                }
            }
            Component::ParentDir => {
                check_permission(&dir, credentials, AccessMode::EXEC)?;
                dir = dir.parent().unwrap_or_else(|| fs.root_dir().clone());
            }
            Component::Normal(name) => {
                check_permission(&dir, credentials, AccessMode::EXEC)?;
                if is_last {
                    break;
                }

                let loc = dir.lookup_no_follow(name)?;
                let loc = fs
                    .with_current_dir(dir.clone())?
                    .try_resolve_symlink(loc, &mut follow_count)?;
                loc.check_is_dir()?;
                dir = loc;
            }
        }
    }

    Ok(())
}

fn resolve_search_symlink(
    fs: &FsContext,
    base_dir: Location,
    loc: Location,
    credentials: VfsCredentials,
    follow_count: &mut usize,
) -> AxResult<Location> {
    if loc.node_type() != NodeType::Symlink {
        return Ok(loc);
    }
    if *follow_count >= SYMLINKS_MAX {
        return Err(AxError::FilesystemLoop);
    }
    *follow_count += 1;

    let target = loc.read_link()?;
    if target.is_empty() {
        return Err(AxError::NotFound);
    }

    let path = PathBuf::from(target);
    let dir = if path.is_absolute() {
        fs.root_dir().clone()
    } else {
        base_dir
    };
    check_path_search_components(fs, dir, &path, credentials, true, follow_count)
}

fn check_path_search_components(
    fs: &FsContext,
    mut dir: Location,
    path: &Path,
    credentials: VfsCredentials,
    follow_final_symlink: bool,
    follow_count: &mut usize,
) -> AxResult<Location> {
    let mut components = path.components().peekable();
    while let Some(component) = components.next() {
        let is_last = components.peek().is_none();
        match component {
            Component::RootDir => {
                dir = fs.root_dir().clone();
            }
            Component::CurDir => {
                if is_last {
                    check_permission(&dir, credentials, AccessMode::EXEC)?;
                }
            }
            Component::ParentDir => {
                check_permission(&dir, credentials, AccessMode::EXEC)?;
                dir = dir.parent().unwrap_or_else(|| fs.root_dir().clone());
            }
            Component::Normal(name) => {
                check_permission(&dir, credentials, AccessMode::EXEC)?;
                if is_last && !follow_final_symlink {
                    return Ok(dir);
                }

                let loc = dir.lookup_no_follow(name)?;
                let loc =
                    resolve_search_symlink(fs, dir.clone(), loc, credentials, follow_count)?;
                if !is_last {
                    loc.check_is_dir()?;
                }
                dir = loc;
            }
        }
    }

    Ok(dir)
}

pub fn check_path_search_stat(
    fs: &FsContext,
    path: &str,
    credentials: VfsCredentials,
    follow_final_symlink: bool,
) -> AxResult<()> {
    let path = Path::new(path);
    let dir = if path.is_absolute() {
        fs.root_dir().clone()
    } else {
        fs.current_dir().clone()
    };
    let mut follow_count = 0;
    check_path_search_components(
        fs,
        dir,
        path,
        credentials,
        follow_final_symlink,
        &mut follow_count,
    )?;
    Ok(())
}

pub fn check_parent_permission(
    fs: &FsContext,
    path: &str,
    credentials: VfsCredentials,
    requested: AccessMode,
) -> AxResult<()> {
    check_path_search(fs, path, credentials)?;
    let (parent, _) = fs.resolve_nonexistent(Path::new(path))?;
    check_permission(&parent, credentials, requested)
}

pub enum ResolveAtResult {
    File(Location),
    Other(Arc<dyn FileLike>),
}

impl ResolveAtResult {
    pub fn into_file(self) -> Option<Location> {
        match self {
            Self::File(file) => Some(file),
            Self::Other(_) => None,
        }
    }

    pub fn stat(&self) -> AxResult<Kstat> {
        match self {
            Self::File(loc) => {
                let metadata = loc.metadata()?;
                let rdev = loc.user_data().get::<DeviceId>().map(|d| *d).unwrap_or(metadata.rdev);
                let mut kstat = metadata_to_kstat_with_rdev(&metadata, rdev, Some(loc));
                kstat.attributes = get_inode_flags(loc);
                Ok(kstat)
            }
            Self::Other(file_like) => file_like.stat(),
        }
    }
}

pub fn resolve_at(dirfd: c_int, path: Option<&str>, flags: u32) -> AxResult<ResolveAtResult> {
    match path {
        Some("") | None => {
            if flags & AT_EMPTY_PATH == 0 {
                return Err(AxError::NotFound);
            }
            let file_like = get_file_like(dirfd)?;
            let f = file_like.clone();
            Ok(if let Some(file) = f.downcast_ref::<File>() {
                ResolveAtResult::File(file.inner().backend()?.location().clone())
            } else if let Some(dir) = f.downcast_ref::<Directory>() {
                ResolveAtResult::File(dir.inner().clone())
            } else {
                ResolveAtResult::Other(file_like)
            })
        }
        Some(path) => {
            check_path_len(path)?;

            let busybox = if dirfd == AT_FDCWD || path.starts_with('/') {
                busybox_applet(path).map(|(busybox, _)| busybox)
            } else {
                None
            };

            with_fs_at(dirfd, path, |fs| {
                let resolved = if flags & AT_SYMLINK_NOFOLLOW != 0 {
                    fs.resolve_no_follow(path)
                } else {
                    fs.resolve(path)
                };

                match resolved {
                    Ok(loc) => Ok(ResolveAtResult::File(loc)),
                    Err(err) => {
                        if err == AxError::NotFound
                            && let Some(busybox) = busybox
                        {
                            fs.resolve(busybox).map(ResolveAtResult::File)
                        } else {
                            Err(err)
                        }
                    }
                }
            })
        }
    }
}

/// Count the number of direct child subdirectories (excluding `.` and `..`).
/// Used for computing directory st_nlink dynamically when the backend nlink
/// may be unreliable (e.g. tmpfs not decrementing parent nlink on rmdir).
fn count_dir_subdirs(loc: &Location) -> u32 {
    let mut subdir_count: u32 = 0;
    if let Ok(dir_node) = loc.entry().as_dir() {
        let _ = dir_node.read_dir(0, &mut |name: &str, _, node_type, _| {
            if name != "." && name != ".." && node_type == NodeType::Directory {
                subdir_count += 1;
            }
            true
        });
    }
    subdir_count
}

pub fn metadata_to_kstat(metadata: &Metadata) -> Kstat {
    metadata_to_kstat_with_rdev(metadata, metadata.rdev, None)
}

pub fn metadata_to_kstat_with_rdev(metadata: &Metadata, rdev: DeviceId, loc: Option<&Location>) -> Kstat {
    let ty = metadata.node_type as u8;
    let perm = metadata.mode.bits() as u32;
    let mode = ((ty as u32) << 12) | perm;
    let nlink = if metadata.node_type == NodeType::Directory {
        if let Some(loc) = loc {
            2 + count_dir_subdirs(loc)
        } else {
            metadata.nlink as u32
        }
    } else {
        metadata.nlink as u32
    };
    Kstat {
        dev: metadata.device,
        ino: metadata.inode,
        mode,
        nlink,
        uid: metadata.uid,
        gid: metadata.gid,
        size: metadata.size,
        blksize: metadata.block_size as _,
        blocks: metadata.blocks,
        rdev,
        attributes: 0,
        atime: metadata.atime,
        mtime: metadata.mtime,
        ctime: metadata.ctime,
    }
}

/// File wrapper for `axfs::fops::File`.
pub struct File {
    inner: axfs::File,
    nonblock: AtomicBool,
    /// Stable OFD owner id — shared by dup/fork, unique per independent open.
    ofd_id: u64,
    /// F_GETOWN_EX / F_SETOWN_EX owner state (per open file description).
    pub owner_ex: Mutex<Option<FileOwnerEx>>,
    /// F_GETSIG / F_SETSIG signal number (0 = default SIGIO).
    pub async_signal: AtomicI32,
}

impl File {
    pub fn new(inner: axfs::File) -> Self {
        Self {
            inner,
            nonblock: AtomicBool::new(false),
            ofd_id: super::alloc_ofd_id(),
            owner_ex: Mutex::new(None),
            async_signal: AtomicI32::new(0),
        }
    }

    pub fn inner(&self) -> &axfs::File {
        &self.inner
    }

    fn is_blocking(&self) -> bool {
        self.inner.location().flags().contains(NodeFlags::BLOCKING)
    }
}

impl Drop for File {
    fn drop(&mut self) {
        // Release all OFD locks owned by this open-file-description.
        // POSIX locks are released in `close_file_like()` (per-fd semantic);
        // OFD locks are released here (per-description, last reference).
        if let Some(key) = self.inode_key() {
            record_lock::release_ofd_locks_on_inode(key, self.ofd_id);
        }
    }
}

fn path_for(loc: &Location) -> Cow<'static, str> {
    loc.absolute_path()
        .map_or_else(|_| "<error>".into(), |f| Cow::Owned(f.to_string()))
}

impl FileLike for File {
    fn read(&self, dst: &mut IoDst) -> AxResult<usize> {
        let inner = self.inner();
        if likely(self.is_blocking()) {
            inner.read(dst)
        } else {
            block_on(poll_io(self, IoEvents::IN, self.nonblocking(), || {
                inner.read(&mut *dst)
            }))
        }
    }

    fn write(&self, src: &mut IoSrc) -> AxResult<usize> {
        let inner = self.inner();
        if likely(self.is_blocking()) {
            inner.write(src)
        } else {
            block_on(poll_io(self, IoEvents::OUT, self.nonblocking(), || {
                inner.write(&mut *src)
            }))
        }
    }

    fn stat(&self) -> AxResult<Kstat> {
        let loc = self.inner().location();
        let metadata = loc.metadata()?;
        // Check for device node rdev stored in user_data (from mknod).
        let rdev = loc.user_data().get::<DeviceId>().map(|d| *d).unwrap_or(metadata.rdev);
        let mut kstat = metadata_to_kstat_with_rdev(&metadata, rdev, Some(loc));
        kstat.attributes = get_inode_flags(loc);
        Ok(kstat)
    }

    fn ioctl(&self, cmd: u32, arg: usize) -> AxResult<usize> {
        self.inner().backend()?.location().ioctl(cmd, arg)
    }

    fn set_nonblocking(&self, flag: bool) -> AxResult {
        self.nonblock.store(flag, Ordering::Release);
        Ok(())
    }

    fn nonblocking(&self) -> bool {
        self.nonblock.load(Ordering::Acquire)
    }

    fn access_mode(&self) -> u32 {
        let flags = self.inner().flags();
        let mut mode = match (
            flags.contains(FileFlags::READ),
            flags.contains(FileFlags::WRITE),
        ) {
            (true, true) => O_RDWR,
            (false, true) => O_WRONLY,
            _ => 0,
        };
        if flags.contains(FileFlags::PATH) {
            mode |= O_PATH;
        }
        mode
    }

    fn status_flags(&self) -> u32 {
        let mut flags = 0;
        if self.inner().flags().contains(FileFlags::APPEND) {
            flags |= O_APPEND;
        }
        if self.inner().flags().contains(FileFlags::NOATIME) {
            flags |= O_NOATIME;
        }
        if self.nonblocking() {
            flags |= O_NONBLOCK;
        }
        flags
    }

    fn set_status_flags(&self, flags: u32) -> AxResult {
        self.inner().set_append(flags & O_APPEND != 0);
        self.set_nonblocking(flags & O_NONBLOCK != 0)
    }

    fn inode_key(&self) -> Option<record_lock::InodeKey> {
        let loc = self.inner().location();
        Some((loc.mountpoint().device(), loc.inode()))
    }

    fn file_position(&self) -> u64 {
        self.inner().position()
    }

    fn ofd_owner(&self) -> u64 {
        self.ofd_id
    }

    fn path(&self) -> Cow<'_, str> {
        path_for(self.inner.location())
    }

    fn from_fd(fd: c_int) -> AxResult<Arc<Self>>
    where
        Self: Sized + 'static,
    {
        get_file_like(fd)?.downcast_arc().map_err(|any| {
            if any.is::<Directory>() {
                AxError::IsADirectory
            } else {
                AxError::BrokenPipe
            }
        })
    }
}
impl Pollable for File {
    fn poll(&self) -> IoEvents {
        self.inner().location().poll()
    }

    fn register(&self, context: &mut Context<'_>, events: IoEvents) {
        self.inner().location().register(context, events);
    }
}

/// Directory wrapper for `axfs::fops::Directory`.
struct DeletedDirectory;

pub fn mark_directory_deleted(loc: &Location) {
    loc.user_data().insert(DeletedDirectory);
}

pub fn is_directory_deleted(loc: &Location) -> bool {
    loc.user_data().get::<DeletedDirectory>().is_some()
}

pub struct Directory {
    inner: Location,
    pub offset: Mutex<u64>,
}

impl Directory {
    pub fn new(inner: Location) -> Self {
        Self {
            inner,
            offset: Mutex::new(0),
        }
    }

    /// Get the inner node of the directory.
    pub fn inner(&self) -> &Location {
        &self.inner
    }

    pub fn is_deleted(&self) -> bool {
        self.inner.user_data().get::<DeletedDirectory>().is_some()
    }
}

impl FileLike for Directory {
    fn read(&self, _dst: &mut IoDst) -> AxResult<usize> {
        Err(AxError::IsADirectory)
    }

    fn write(&self, _src: &mut IoSrc) -> AxResult<usize> {
        Err(AxError::BadFileDescriptor)
    }

    fn stat(&self) -> AxResult<Kstat> {
        let metadata = self.inner.metadata()?;
        let rdev = self.inner.user_data().get::<DeviceId>().map(|d| *d).unwrap_or(metadata.rdev);
        let mut kstat = metadata_to_kstat_with_rdev(&metadata, rdev, Some(&self.inner));
        kstat.attributes = get_inode_flags(&self.inner);
        Ok(kstat)
    }

    fn path(&self) -> Cow<'_, str> {
        path_for(&self.inner)
    }

    fn from_fd(fd: c_int) -> AxResult<Arc<Self>> {
        get_file_like(fd)?
            .downcast_arc()
            .map_err(|_| AxError::NotADirectory)
    }
}
impl Pollable for Directory {
    fn poll(&self) -> IoEvents {
        IoEvents::IN | IoEvents::OUT
    }

    fn register(&self, _context: &mut Context<'_>, _events: IoEvents) {}
}
