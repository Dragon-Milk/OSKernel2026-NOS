pub mod epoll;
pub mod event;
mod fs;
mod net;
mod pidfd;
mod pipe;
pub mod record_lock;
pub mod signalfd;
pub mod xattr;

use alloc::{borrow::Cow, sync::Arc};
use core::{
    ffi::c_int,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use axerrno::{AxError, AxResult};
use axfs::{FS_CONTEXT, OpenOptions};
use axfs_ng_vfs::DeviceId;
use axio::prelude::*;
use axpoll::Pollable;
use axtask::current;
use downcast_rs::{DowncastSync, impl_downcast};
use flatten_objects::FlattenObjects;
use linux_raw_sys::general::{O_NONBLOCK, RLIMIT_NOFILE, STATX_BASIC_STATS, stat, statx, statx_timestamp};
use spin::RwLock;

pub use self::{
    fs::{
        AccessMode, Directory, File, VfsCredentials,
        check_parent_permission, check_path_len, check_path_search, check_path_search_stat,
        check_permission, check_writable_filesystem, clear_setgid_if_not_in_group,
        creation_metadata, resolve_at, mark_directory_deleted, is_directory_deleted, with_fs, with_fs_at,
    },
    net::Socket,
    pidfd::PidFd,
    pipe::{NamedPipe, Pipe, PIPE_MAX_SIZE},
    record_lock::FileOwnerEx,
    xattr::{
        XATTR_CREATE, XATTR_REPLACE,
        do_setxattr, do_getxattr, do_listxattr, do_removexattr,
        remove_xattr_map,
    },
};
// Inode-flags helpers are defined as `pub fn` / `pub const` below — no
// separate `pub use self::` needed.
use crate::task::{AX_FILE_LIMIT, AsThread};

// ---------------------------------------------------------------------------
// Inode flags (immutable / append-only)
// ---------------------------------------------------------------------------

use alloc::collections::BTreeMap;
use axfs_ng_vfs::Location;
use spin::Mutex as SpinMutex;

/// FS_IOC_GETFLAGS — read inode flags.
pub const FS_IOC_GETFLAGS: u32 = 0x80086601;
/// FS_IOC_SETFLAGS — write inode flags.
pub const FS_IOC_SETFLAGS: u32 = 0x40086602;

/// Inode flag: immutable (cannot be modified, deleted, or renamed over).
pub const FS_IMMUTABLE_FL: u32 = 0x0000_0010;
/// Inode flag: append-only (writes only allowed at end-of-file).
pub const FS_APPEND_FL: u32 = 0x0000_0020;
/// Inode flag: nodump (exclude from backups).
pub const FS_NODUMP_FL: u32 = 0x0000_0040;

/// Allowed settable flags.
const SETTABLE_FLAGS: u32 = FS_IMMUTABLE_FL | FS_APPEND_FL | FS_NODUMP_FL;

type InodeFlagsKey = (u64, u64);

static INODE_FLAGS: SpinMutex<BTreeMap<InodeFlagsKey, u32>> = SpinMutex::new(BTreeMap::new());

fn inode_flags_key(loc: &Location) -> InodeFlagsKey {
    (loc.mountpoint().device(), loc.inode())
}

/// Read current inode flags (atomic for the same inode).
pub fn get_inode_flags(loc: &Location) -> u32 {
    INODE_FLAGS
        .lock()
        .get(&inode_flags_key(loc))
        .copied()
        .unwrap_or(0)
}

/// Set inode flags. `mask` controls which bits may be changed; only
/// `SETTABLE_FLAGS` bits are applied. Returns the new flags value.
pub fn set_inode_flags(loc: &Location, flags: u32) -> u32 {
    let key = inode_flags_key(loc);
    let mut table = INODE_FLAGS.lock();
    let applied = flags & SETTABLE_FLAGS;
    if applied == 0 {
        table.remove(&key);
        0
    } else {
        table.insert(key, applied);
        applied
    }
}

/// Remove inode flags tracking when the inode is truly reclaimed.
pub fn remove_inode_flags(loc: &Location) {
    INODE_FLAGS.lock().remove(&inode_flags_key(loc));
}

/// Check that an inode is NOT immutable. Returns `EPERM` if it is.
pub fn check_not_immutable(loc: &Location) -> AxResult<()> {
    if get_inode_flags(loc) & FS_IMMUTABLE_FL != 0 {
        Err(AxError::OperationNotPermitted)
    } else {
        Ok(())
    }
}

/// Check that an inode is NOT append-only. Returns `EPERM` if it is.
/// Append-only inodes refuse setxattr, removexattr, unlink, rename,
/// truncate, chmod, chown, and non-append writes.
pub fn check_not_append_only(loc: &Location) -> AxResult<()> {
    if get_inode_flags(loc) & FS_APPEND_FL != 0 {
        Err(AxError::OperationNotPermitted)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Kstat {
    pub dev: u64,
    pub ino: u64,
    pub nlink: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
    pub blksize: u32,
    pub blocks: u64,
    pub rdev: DeviceId,
    pub attributes: u32,
    pub atime: Duration,
    pub mtime: Duration,
    pub ctime: Duration,
}

impl Default for Kstat {
    fn default() -> Self {
        Self {
            dev: 0,
            ino: 1,
            nlink: 1,
            mode: 0,
            uid: 1,
            gid: 1,
            size: 0,
            blksize: 4096,
            blocks: 0,
            rdev: DeviceId::default(),
            attributes: 0,
            atime: Duration::default(),
            mtime: Duration::default(),
            ctime: Duration::default(),
        }
    }
}

impl From<Kstat> for stat {
    fn from(value: Kstat) -> Self {
        // SAFETY: valid for stat
        let mut stat: stat = unsafe { core::mem::zeroed() };
        stat.st_dev = value.dev as _;
        stat.st_ino = value.ino as _;
        stat.st_nlink = value.nlink as _;
        stat.st_mode = value.mode as _;
        stat.st_uid = value.uid as _;
        stat.st_gid = value.gid as _;
        stat.st_size = value.size as _;
        stat.st_blksize = value.blksize as _;
        stat.st_blocks = value.blocks as _;
        stat.st_rdev = value.rdev.0 as _;

        stat.st_atime = value.atime.as_secs() as _;
        stat.st_atime_nsec = value.atime.subsec_nanos() as _;
        stat.st_mtime = value.mtime.as_secs() as _;
        stat.st_mtime_nsec = value.mtime.subsec_nanos() as _;
        stat.st_ctime = value.ctime.as_secs() as _;
        stat.st_ctime_nsec = value.ctime.subsec_nanos() as _;

        stat
    }
}

impl From<Kstat> for statx {
    fn from(value: Kstat) -> Self {
        // SAFETY: valid for statx
        let mut statx: statx = unsafe { core::mem::zeroed() };
        statx.stx_mask = STATX_BASIC_STATS;
        statx.stx_blksize = value.blksize as _;
        statx.stx_attributes = value.attributes as u64;
        statx.stx_attributes_mask =
            (FS_IMMUTABLE_FL | FS_APPEND_FL | FS_NODUMP_FL) as u64;
        statx.stx_nlink = value.nlink as _;
        statx.stx_uid = value.uid as _;
        statx.stx_gid = value.gid as _;
        statx.stx_mode = value.mode as _;
        statx.stx_ino = value.ino as _;
        statx.stx_size = value.size as _;
        statx.stx_blocks = value.blocks as _;
        statx.stx_rdev_major = value.rdev.major();
        statx.stx_rdev_minor = value.rdev.minor();

        fn time_to_statx(time: &Duration) -> statx_timestamp {
            statx_timestamp {
                tv_sec: time.as_secs() as _,
                tv_nsec: time.subsec_nanos() as _,
                __reserved: 0,
            }
        }
        statx.stx_atime = time_to_statx(&value.atime);
        statx.stx_btime = statx_timestamp { tv_sec: 0, tv_nsec: 0, __reserved: 0 };
        statx.stx_ctime = time_to_statx(&value.ctime);
        statx.stx_mtime = time_to_statx(&value.mtime);

        statx.stx_dev_major = (value.dev >> 32) as _;
        statx.stx_dev_minor = value.dev as _;

        statx
    }
}

pub trait WriteBuf: Write + IoBufMut {}
impl<T: Write + IoBufMut> WriteBuf for T {}
pub type IoDst<'a> = dyn WriteBuf + 'a;

pub trait ReadBuf: Read + IoBuf {}
impl<T: Read + IoBuf> ReadBuf for T {}
pub type IoSrc<'a> = dyn ReadBuf + 'a;

#[allow(dead_code)]
pub trait FileLike: Pollable + DowncastSync {
    fn read(&self, _dst: &mut IoDst) -> AxResult<usize> {
        Err(AxError::InvalidInput)
    }

    fn write(&self, _src: &mut IoSrc) -> AxResult<usize> {
        Err(AxError::InvalidInput)
    }

    fn stat(&self) -> AxResult<Kstat> {
        Ok(Kstat::default())
    }

    fn path(&self) -> Cow<'_, str>;

    fn ioctl(&self, _cmd: u32, _arg: usize) -> AxResult<usize> {
        Err(AxError::NotATty)
    }

    fn nonblocking(&self) -> bool {
        false
    }

    fn set_nonblocking(&self, _nonblocking: bool) -> AxResult {
        Ok(())
    }

    fn access_mode(&self) -> u32 {
        0
    }

    fn status_flags(&self) -> u32 {
        if self.nonblocking() { O_NONBLOCK } else { 0 }
    }

    fn set_status_flags(&self, flags: u32) -> AxResult {
        self.set_nonblocking(flags & O_NONBLOCK != 0)
    }

    /// Returns `false` for fd types that cannot be used with socket operations
    /// (e.g., O_PATH files, dummy fds from open_tree). In Linux these return
    /// `EBADF` rather than `ENOTSOCK` because the fd is not valid for the operation.
    fn is_socket_operable(&self) -> bool {
        true
    }

    fn from_fd(fd: c_int) -> AxResult<Arc<Self>>
    where
        Self: Sized + 'static,
    {
        get_file_like(fd)?
            .downcast_arc()
            .map_err(|_| AxError::InvalidInput)
    }

    fn add_to_fd_table(self, cloexec: bool) -> AxResult<c_int>
    where
        Self: Sized + 'static,
    {
        add_file_like(Arc::new(self), cloexec)
    }

    /// Returns the globally-unique inode identity for record locking,
    /// or `None` if this fd type does not support record locks (e.g. pipe/socket).
    fn inode_key(&self) -> Option<record_lock::InodeKey> {
        None
    }

    /// Returns the current file position (0 for non-seekable types).
    fn file_position(&self) -> u64 {
        0
    }

    /// OFD owner identity — a stable numeric id that is unique per
    /// open-file-description.  `dup`/`fork` share the same id; independent
    /// `open` calls get different ids.
    fn ofd_owner(&self) -> u64 {
        0
    }
}
impl_downcast!(sync FileLike);

#[derive(Clone)]
pub struct FileDescriptor {
    pub inner: Arc<dyn FileLike>,
    pub cloexec: bool,
}

scope_local::scope_local! {
    /// The current file descriptor table.
    pub static FD_TABLE: Arc<RwLock<FlattenObjects<FileDescriptor, AX_FILE_LIMIT>>> = Arc::default();
}

/// Get a file-like object by `fd`.
pub fn get_file_like(fd: c_int) -> AxResult<Arc<dyn FileLike>> {
    FD_TABLE
        .read()
        .get(fd as usize)
        .map(|fd| fd.inner.clone())
        .ok_or(AxError::BadFileDescriptor)
}

/// Add a file to the file descriptor table.
pub fn add_file_like(f: Arc<dyn FileLike>, cloexec: bool) -> AxResult<c_int> {
    let max_nofile = current().as_thread().proc_data.rlim.read()[RLIMIT_NOFILE].current;
    let mut table = FD_TABLE.write();
    if table.count() as u64 >= max_nofile {
        return Err(AxError::TooManyOpenFiles);
    }
    let fd = FileDescriptor { inner: f, cloexec };
    Ok(table.add(fd).map_err(|_| AxError::TooManyOpenFiles)? as c_int)
}

/// Add a file to the descriptor table at the first free fd >= `min_fd`.
pub fn add_file_like_from(
    f: Arc<dyn FileLike>,
    cloexec: bool,
    min_fd: usize,
) -> AxResult<c_int> {
    let max_nofile = current().as_thread().proc_data.rlim.read()[RLIMIT_NOFILE].current;
    let mut table = FD_TABLE.write();
    let limit = (max_nofile as usize).min(table.capacity());

    if min_fd >= limit {
        return Err(AxError::InvalidInput);
    }
    if table.count() as u64 >= max_nofile {
        return Err(AxError::TooManyOpenFiles);
    }

    let fd = FileDescriptor { inner: f, cloexec };
    for id in min_fd..limit {
        if !table.is_assigned(id) {
            return Ok(table
                .add_at(id, fd)
                .map_err(|_| AxError::TooManyOpenFiles)? as c_int);
        }
    }

    Err(AxError::TooManyOpenFiles)
}

/// Global counter for OFD owner identity.
/// Each independent `open()` gets a new id; `dup`/`fork` share the same id.
static NEXT_OFD_ID: AtomicU64 = AtomicU64::new(1);

pub fn alloc_ofd_id() -> u64 {
    NEXT_OFD_ID.fetch_add(1, Ordering::Relaxed)
}

/// Close a file by `fd`.
pub fn close_file_like(fd: c_int) -> AxResult {
    let f = FD_TABLE
        .write()
        .remove(fd as usize)
        .ok_or(AxError::BadFileDescriptor)?;
    debug!("close_file_like <= count: {}", Arc::strong_count(&f.inner));

    // Release all POSIX record locks held by this process on the inode
    // referenced by the closed fd.  Linux semantics: closing *any* fd
    // referencing an inode releases all POSIX locks the process holds
    // on that inode.
    if let Some(key) = f.inner.inode_key() {
        let owner = Arc::downgrade(&current().as_thread().proc_data);
        record_lock::release_posix_locks_on_inode(key, &owner);
    }

    Ok(())
}

pub fn add_stdio(fd_table: &mut FlattenObjects<FileDescriptor, AX_FILE_LIMIT>) -> AxResult<()> {
    assert_eq!(fd_table.count(), 0);
    let cx = FS_CONTEXT.lock();
    let open = |options: &mut OpenOptions| {
        AxResult::Ok(Arc::new(File::new(
            options.open(&cx, "/dev/console")?.into_file()?,
        )))
    };

    let tty_in = open(OpenOptions::new().read(true).write(false))?;
    let tty_out = open(OpenOptions::new().read(false).write(true))?;
    fd_table
        .add(FileDescriptor {
            inner: tty_in,
            cloexec: false,
        })
        .map_err(|_| AxError::TooManyOpenFiles)?;
    fd_table
        .add(FileDescriptor {
            inner: tty_out.clone(),
            cloexec: false,
        })
        .map_err(|_| AxError::TooManyOpenFiles)?;
    fd_table
        .add(FileDescriptor {
            inner: tty_out,
            cloexec: false,
        })
        .map_err(|_| AxError::TooManyOpenFiles)?;

    Ok(())
}
