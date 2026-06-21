use alloc::{borrow::Cow, sync::Arc, vec};
use core::{
    ffi::{c_char, c_int},
    sync::atomic::{AtomicBool, Ordering},
    task::Context,
};

use axerrno::{AxError, AxResult, LinuxError};
use axfs::{FS_CONTEXT, FileFlags, OpenOptions};
use axio::{Seek, SeekFrom};
use axpoll::{IoEvents, Pollable};
use axtask::current;
use linux_raw_sys::general::{
    __kernel_off_t, FALLOC_FL_ALLOCATE_RANGE, FALLOC_FL_KEEP_SIZE, IN_CLOEXEC, IN_NONBLOCK,
    RLIM64_INFINITY, RLIMIT_FSIZE, RWF_APPEND, RWF_DSYNC, RWF_HIPRI, RWF_NOWAIT, RWF_SYNC,
};
use starry_vm::{VmMutPtr, VmPtr};
use syscalls::Sysno;

use crate::{
    file::{
        AccessMode, Directory, File, FileLike, NamedPipe, Pipe, Socket, VfsCredentials,
        FS_APPEND_FL, FS_IMMUTABLE_FL,
        check_not_append_only, check_not_immutable,
        check_permission, check_writable_filesystem, get_file_like, get_inode_flags,
    },
    mm::{IoVec, IoVectorBuf, UserConstPtr, VmBytes, VmBytesMut},
    task::AsThread,
};

struct DummyFd {
    nonblocking: AtomicBool,
}
impl FileLike for DummyFd {
    fn path(&self) -> Cow<'_, str> {
        "anon_inode:[dummy]".into()
    }

    fn nonblocking(&self) -> bool {
        self.nonblocking.load(Ordering::Relaxed)
    }

    fn set_nonblocking(&self, nonblocking: bool) -> AxResult {
        self.nonblocking.store(nonblocking, Ordering::Relaxed);
        Ok(())
    }
}
impl Pollable for DummyFd {
    fn poll(&self) -> IoEvents {
        IoEvents::empty()
    }

    fn register(&self, _context: &mut Context<'_>, _events: IoEvents) {}
}

pub fn sys_dummy_fd(sysno: Sysno) -> AxResult<isize> {
    if current().name().starts_with("qemu-") {
        // We need to be honest to qemu, since it can automatically fallback to
        // other strategies.
        return Err(AxError::Unsupported);
    }
    warn!("Dummy fd created: {sysno}");
    DummyFd {
        nonblocking: AtomicBool::new(false),
    }
    .add_to_fd_table(false)
    .map(|fd| fd as isize)
}

/// inotify_init1(flags) — create a dummy inotify fd with proper flag handling.
///
/// Supported flags: `IN_CLOEXEC`, `IN_NONBLOCK`.
pub fn sys_inotify_init1(flags: u32) -> AxResult<isize> {
    debug!("sys_inotify_init1 <= flags: {flags}");
    const VALID_FLAGS: u32 = IN_CLOEXEC | IN_NONBLOCK;
    if flags & !VALID_FLAGS != 0 {
        return Err(AxError::InvalidInput);
    }
    if current().name().starts_with("qemu-") {
        // We need to be honest to qemu, since it can automatically fallback to
        // other strategies.
        return Err(AxError::Unsupported);
    }
    let f = DummyFd {
        nonblocking: AtomicBool::new(flags & IN_NONBLOCK != 0),
    };
    f.add_to_fd_table(flags & IN_CLOEXEC != 0)
        .map(|fd| fd as isize)
}

/// Read data from the file indicated by `fd`.
///
/// Return the read size if success.
pub fn sys_read(fd: i32, buf: *mut u8, len: usize) -> AxResult<isize> {
    debug!("sys_read <= fd: {fd}, buf: {buf:p}, len: {len}");
    Ok(get_file_like(fd)?.read(&mut VmBytesMut::new(buf, len))? as _)
}

pub fn sys_readv(fd: i32, iov: *const IoVec, iovcnt: usize) -> AxResult<isize> {
    debug!("sys_readv <= fd: {fd}, iovcnt: {iovcnt}");
    let f = get_file_like(fd)?;
    let iov = IoVectorBuf::new(iov, iovcnt)?;
    iov.validate_writable()?;
    f.read(&mut iov.into_io()).map(|n| n as _)
}

/// Write data to the file indicated by `fd`.
///
/// Return the written size if success.
pub fn sys_write(fd: i32, buf: *mut u8, len: usize) -> AxResult<isize> {
    debug!("sys_write <= fd: {fd}, buf: {buf:p}, len: {len}");
    let f = get_file_like(fd)?;
    // Combined immutable/append-only check — single INODE_FLAGS lookup.
    if let Some(file) = f.downcast_ref::<File>() {
        let flags = get_inode_flags(file.inner().location());
        if flags & (FS_IMMUTABLE_FL | FS_APPEND_FL) != 0 {
            return Err(AxError::OperationNotPermitted);
        }
    }
    let written = f.write(&mut VmBytes::new(buf, len))?;
    crate::perf::perf_observe_user_write(fd, buf, written);
    Ok(written as _)
}

pub fn sys_writev(fd: i32, iov: *const IoVec, iovcnt: usize) -> AxResult<isize> {
    debug!("sys_writev <= fd: {fd}, iovcnt: {iovcnt}");
    let f = get_file_like(fd)?;
    // Combined immutable/append-only check — single INODE_FLAGS lookup.
    if let Some(file) = f.downcast_ref::<File>() {
        let flags = get_inode_flags(file.inner().location());
        if flags & (FS_IMMUTABLE_FL | FS_APPEND_FL) != 0 {
            return Err(AxError::OperationNotPermitted);
        }
    }
    let iov_buf = IoVectorBuf::new(iov, iovcnt)?;
    iov_buf.validate_readable()?;
    let written = f.write(&mut iov_buf.into_io())?;
    crate::perf::perf_observe_user_writev(fd, iov, iovcnt, written);
    Ok(written as _)
}

fn check_pipe_offset_io(fd: c_int) -> AxResult<()> {
    if Pipe::from_fd(fd).is_ok() || NamedPipe::from_fd(fd).is_ok() {
        return Err(AxError::from(LinuxError::ESPIPE));
    }
    Ok(())
}

pub fn sys_lseek(fd: c_int, offset: __kernel_off_t, whence: c_int) -> AxResult<isize> {
    debug!("sys_lseek <= {fd} {offset} {whence}");
    // Negative offset with SEEK_SET is invalid (Linux returns EINVAL).
    if whence == 0 && offset < 0 {
        return Err(AxError::InvalidInput);
    }
    let pos = match whence {
        0 => SeekFrom::Start(offset as _),
        1 => SeekFrom::Current(offset as _),
        2 => SeekFrom::End(offset as _),
        _ => return Err(AxError::InvalidInput),
    };
    check_pipe_offset_io(fd)?;
    let off = File::from_fd(fd)?.inner().seek(pos)?;
    Ok(off as _)
}

pub fn sys_truncate(path: UserConstPtr<c_char>, length: __kernel_off_t) -> AxResult<isize> {
    let path = path.get_as_str()?;
    debug!("sys_truncate <= {path:?} {length}");
    if path.is_empty() {
        return Err(AxError::NotFound);
    }
    if length < 0 {
        return Err(AxError::InvalidInput);
    }
    let length = length as u64;
    let file = OpenOptions::new()
        .write(true)
        .open(&FS_CONTEXT.lock(), path)?
        .into_file()?;
    let loc = file.location();
    check_permission(loc, VfsCredentials::effective(), AccessMode::WRITE)?;
    check_writable_filesystem(loc)?;
    check_not_immutable(loc)?;
    check_not_append_only(loc)?;
    check_file_size_limit(length)?;
    file.access(FileFlags::WRITE)?.set_len(length)?;
    Ok(0)
}

pub fn sys_ftruncate(fd: c_int, length: __kernel_off_t) -> AxResult<isize> {
    debug!("sys_ftruncate <= {fd} {length}");
    let file_like = get_file_like(fd)?;
    let Some(f) = file_like.downcast_ref::<File>() else {
        return Err(AxError::InvalidInput);
    };
    if length < 0 {
        return Err(AxError::InvalidInput);
    }
    if !f.inner().flags().contains(FileFlags::WRITE) {
        return Err(AxError::InvalidInput);
    }
    let loc = f.inner().location();
    check_not_immutable(loc)?;
    check_not_append_only(loc)?;
    let length = length as u64;
    f.inner().access(FileFlags::WRITE)?.set_len(length)?;
    Ok(0)
}

fn check_file_size_limit(length: u64) -> AxResult<()> {
    let limit = current().as_thread().proc_data.rlim.read()[RLIMIT_FSIZE].current;
    if limit != RLIM64_INFINITY as u64 && length > limit {
        return Err(AxError::from(LinuxError::EFBIG));
    }
    Ok(())
}

pub fn sys_fallocate(
    fd: c_int,
    mode: u32,
    offset: __kernel_off_t,
    len: __kernel_off_t,
) -> AxResult<isize> {
    debug!("sys_fallocate <= fd: {fd}, mode: {mode}, offset: {offset}, len: {len}");

    // Only FALLOC_FL_ALLOCATE_RANGE (mode=0) and FALLOC_FL_KEEP_SIZE (mode=1)
    // are supported. Reject all other flags.
    const SUPPORTED_FLAGS: u32 = FALLOC_FL_ALLOCATE_RANGE | FALLOC_FL_KEEP_SIZE;
    if mode & !SUPPORTED_FLAGS != 0 {
        return Err(AxError::OperationNotSupported);
    }

    // Validate offset is non-negative and len is non-negative
    if offset < 0 {
        return Err(AxError::InvalidInput);
    }
    if len < 0 {
        return Err(AxError::InvalidInput);
    }

    // len == 0 is a valid no-op
    if len == 0 {
        return Ok(0);
    }

    // Check for overflow: offset + len must not exceed max file size
    let end: u64 = match (offset as u64).checked_add(len as u64) {
        Some(v) => v,
        None => return Err(AxError::from(LinuxError::EFBIG)),
    };

    // File::from_fd returns EBADF for invalid fd, EISDIR for directory fd
    let f = File::from_fd(fd)?;
    let inner = f.inner();
    let file = inner.access(FileFlags::WRITE)?;
    let loc = file.location();

    // Check filesystem writability and file integrity flags
    check_writable_filesystem(loc)?;
    check_not_immutable(loc)?;
    check_not_append_only(loc)?;

    if mode & FALLOC_FL_KEEP_SIZE != 0 {
        // FALLOC_FL_KEEP_SIZE: preallocate space but don't change file size.
        // For basic support, if the requested range fits within the current file
        // size, the space is already allocated. If it extends beyond, we cannot
        // preallocate without a block allocator — just succeed silently.
        // In either case, do not change the file size.
    } else {
        // Default: extend the file to accommodate the requested range
        let current_len = loc.len()?;
        file.set_len(end.max(current_len))?;
    }

    Ok(0)
}

fn sync_fd(fd: c_int, data_only: bool) -> AxResult<isize> {
    let file_like = get_file_like(fd)?;
    if file_like.is::<Pipe>() || file_like.is::<NamedPipe>() || file_like.is::<Socket>() {
        return Err(AxError::InvalidInput);
    }

    let Some(f) = file_like.downcast_ref::<File>() else {
        return Err(if file_like.is::<Directory>() {
            AxError::IsADirectory
        } else {
            AxError::BrokenPipe
        });
    };
    f.inner().sync(data_only)?;
    Ok(0)
}

pub fn sys_fsync(fd: c_int) -> AxResult<isize> {
    debug!("sys_fsync <= {fd}");
    sync_fd(fd, false)
}

pub fn sys_fdatasync(fd: c_int) -> AxResult<isize> {
    debug!("sys_fdatasync <= {fd}");
    sync_fd(fd, true)
}

pub fn sys_fadvise64(
    fd: c_int,
    offset: __kernel_off_t,
    len: __kernel_off_t,
    advice: u32,
) -> AxResult<isize> {
    debug!("sys_fadvise64 <= fd: {fd}, offset: {offset}, len: {len}, advice: {advice}");
    // Validate fd first (LTP posix_fadvise02: invalid fd → EBADF).
    let _ = get_file_like(fd)?;
    if Pipe::from_fd(fd).is_ok() {
        return Err(AxError::from(LinuxError::ESPIPE));
    }
    if advice > 5 {
        return Err(AxError::InvalidInput);
    }
    Ok(0)
}

pub fn sys_pread64(fd: c_int, buf: *mut u8, len: usize, offset: __kernel_off_t) -> AxResult<isize> {
    if offset < 0 {
        return Err(AxError::InvalidInput);
    }
    check_pipe_offset_io(fd)?;
    let f = File::from_fd(fd)?;
    let read = f.inner().read_at(VmBytesMut::new(buf, len), offset as _)?;
    Ok(read as _)
}

pub fn sys_pwrite64(
    fd: c_int,
    buf: *const u8,
    len: usize,
    offset: __kernel_off_t,
) -> AxResult<isize> {
    check_pipe_offset_io(fd)?;
    if offset < 0 {
        return Err(AxError::InvalidInput);
    }
    if len == 0 {
        return Ok(0);
    }
    let f = File::from_fd(fd)?;
    // Combined immutable/append-only check — single INODE_FLAGS lookup.
    {
        let flags = get_inode_flags(f.inner().location());
        if flags & (FS_IMMUTABLE_FL | FS_APPEND_FL) != 0 {
            return Err(AxError::OperationNotPermitted);
        }
    }
    let write = f.inner().write_at(VmBytes::new(buf, len), offset as _)?;
    Ok(write as _)
}

pub fn sys_preadv(
    fd: c_int,
    iov: *const IoVec,
    iovcnt: usize,
    offset: __kernel_off_t,
) -> AxResult<isize> {
    if offset < 0 {
        return Err(AxError::InvalidInput);
    }
    sys_preadv2(fd, iov, iovcnt, offset, 0)
}

pub fn sys_pwritev(
    fd: c_int,
    iov: *const IoVec,
    iovcnt: usize,
    offset: __kernel_off_t,
) -> AxResult<isize> {
    if offset < 0 {
        return Err(AxError::InvalidInput);
    }
    sys_pwritev2(fd, iov, iovcnt, offset, 0)
}

fn check_rwf_flags(flags: u32) -> AxResult<()> {
    const KNOWN_RWF_FLAGS: u32 = RWF_HIPRI | RWF_DSYNC | RWF_SYNC | RWF_NOWAIT | RWF_APPEND;

    if flags == 0 {
        return Ok(());
    }

    let unsupported = flags & KNOWN_RWF_FLAGS;
    let unknown = flags & !KNOWN_RWF_FLAGS;
    if unsupported != 0 || unknown != 0 {
        return Err(AxError::OperationNotSupported);
    }

    Ok(())
}

pub fn sys_preadv2(
    fd: c_int,
    iov: *const IoVec,
    iovcnt: usize,
    offset: __kernel_off_t,
    flags: u32,
) -> AxResult<isize> {
    debug!("sys_preadv2 <= fd: {fd}, iovcnt: {iovcnt}, offset: {offset}, flags: {flags}");
    check_rwf_flags(flags)?;
    if offset == -1 {
        return sys_readv(fd, iov, iovcnt);
    }
    if offset < 0 {
        return Err(AxError::InvalidInput);
    }
    check_pipe_offset_io(fd)?;
    let f = File::from_fd(fd)?;
    let iov = IoVectorBuf::new(iov, iovcnt)?;
    iov.validate_writable()?;
    f.inner()
        .read_at(iov.into_io(), offset as _)
        .map(|n| n as _)
}

pub fn sys_pwritev2(
    fd: c_int,
    iov: *const IoVec,
    iovcnt: usize,
    offset: __kernel_off_t,
    flags: u32,
) -> AxResult<isize> {
    debug!("sys_pwritev2 <= fd: {fd}, iovcnt: {iovcnt}, offset: {offset}, flags: {flags}");
    check_rwf_flags(flags)?;
    if offset == -1 {
        return sys_writev(fd, iov, iovcnt);
    }
    if offset < 0 {
        return Err(AxError::InvalidInput);
    }
    check_pipe_offset_io(fd)?;
    let f = File::from_fd(fd)?;
    // Combined immutable/append-only check — single INODE_FLAGS lookup.
    {
        let flags = get_inode_flags(f.inner().location());
        if flags & (FS_IMMUTABLE_FL | FS_APPEND_FL) != 0 {
            return Err(AxError::OperationNotPermitted);
        }
    }
    let iov = IoVectorBuf::new(iov, iovcnt)?;
    iov.validate_readable()?;
    f.inner()
        .write_at(iov.into_io(), offset as _)
        .map(|n| n as _)
}

enum SendFile {
    Direct(Arc<dyn FileLike>),
    Offset(Arc<File>, *mut u64),
}

impl SendFile {
    fn has_data(&self) -> bool {
        match self {
            SendFile::Direct(file) => file.poll(),
            SendFile::Offset(file, ..) => file.poll(),
        }
        .contains(IoEvents::IN)
    }

    fn read(&mut self, mut buf: &mut [u8]) -> AxResult<usize> {
        match self {
            SendFile::Direct(file) => file.read(&mut buf),
            SendFile::Offset(file, offset) => {
                let off = offset.vm_read()?;
                let bytes_read = file.inner().read_at(&mut buf, off)?;
                offset.vm_write(off + bytes_read as u64)?;
                Ok(bytes_read)
            }
        }
    }

    fn write(&mut self, mut buf: &[u8]) -> AxResult<usize> {
        match self {
            SendFile::Direct(file) => file.write(&mut buf),
            SendFile::Offset(file, offset) => {
                let off = offset.vm_read()?;
                let bytes_written = file.inner().write_at(buf, off)?;
                offset.vm_write(off + bytes_written as u64)?;
                Ok(bytes_written)
            }
        }
    }
}

fn do_send(mut src: SendFile, mut dst: SendFile, len: usize) -> AxResult<usize> {
    let mut buf = vec![0; 0x1000];
    let mut total_written = 0;
    let mut remaining = len;

    while remaining > 0 {
        if total_written > 0 && !src.has_data() {
            break;
        }
        let to_read = buf.len().min(remaining);
        let bytes_read = match src.read(&mut buf[..to_read]) {
            Ok(n) => n,
            Err(AxError::WouldBlock) if total_written > 0 => break,
            Err(e) => return Err(e),
        };
        if bytes_read == 0 {
            break;
        }

        let bytes_written = dst.write(&buf[..bytes_read])?;
        if bytes_written < bytes_read {
            break;
        }

        total_written += bytes_written;
        remaining -= bytes_written;
    }

    Ok(total_written)
}

pub fn sys_sendfile(out_fd: c_int, in_fd: c_int, offset: *mut u64, len: usize) -> AxResult<isize> {
    debug!(
        "sys_sendfile <= out_fd: {}, in_fd: {}, offset: {}, len: {}",
        out_fd,
        in_fd,
        !offset.is_null(),
        len
    );

    let src = if !offset.is_null() {
        if offset.vm_read()? > u32::MAX as u64 {
            return Err(AxError::InvalidInput);
        }
        SendFile::Offset(File::from_fd(in_fd)?, offset)
    } else {
        SendFile::Direct(get_file_like(in_fd)?)
    };

    // out_fd must be writable (LTP sendfile03: read-only out_fd → EBADF).
    let out_f = get_file_like(out_fd)?;
    if let Some(file) = out_f.downcast_ref::<File>() {
        if !file.inner().flags().contains(FileFlags::WRITE) {
            return Err(AxError::BadFileDescriptor);
        }
    }
    let dst = SendFile::Direct(out_f);

    do_send(src, dst, len).map(|n| n as _)
}

pub fn sys_copy_file_range(
    fd_in: c_int,
    off_in: *mut u64,
    fd_out: c_int,
    off_out: *mut u64,
    len: usize,
    _flags: u32,
) -> AxResult<isize> {
    debug!(
        "sys_copy_file_range <= fd_in: {}, off_in: {}, fd_out: {}, off_out: {}, len: {}, flags: {}",
        fd_in,
        !off_in.is_null(),
        fd_out,
        !off_out.is_null(),
        len,
        _flags
    );

    // TODO: check flags
    // TODO: check both regular files
    // TODO: check same file and overlap

    let src = if !off_in.is_null() {
        SendFile::Offset(File::from_fd(fd_in)?, off_in)
    } else {
        SendFile::Direct(get_file_like(fd_in)?)
    };

    let dst = if !off_out.is_null() {
        SendFile::Offset(File::from_fd(fd_out)?, off_out)
    } else {
        SendFile::Direct(get_file_like(fd_out)?)
    };

    do_send(src, dst, len).map(|n| n as _)
}

pub fn sys_splice(
    fd_in: c_int,
    off_in: *mut i64,
    fd_out: c_int,
    off_out: *mut i64,
    len: usize,
    _flags: u32,
) -> AxResult<isize> {
    debug!(
        "sys_splice <= fd_in: {}, off_in: {}, fd_out: {}, off_out: {}, len: {}, flags: {}",
        fd_in,
        !off_in.is_null(),
        fd_out,
        !off_out.is_null(),
        len,
        _flags
    );

    let mut has_pipe = false;

    if DummyFd::from_fd(fd_in).is_ok() || DummyFd::from_fd(fd_out).is_ok() {
        return Err(AxError::BadFileDescriptor);
    }

    let src = if !off_in.is_null() {
        // Pipes do not support offset-based I/O; must return ESPIPE
        if Pipe::from_fd(fd_in).is_ok() || NamedPipe::from_fd(fd_in).is_ok() {
            return Err(AxError::from(LinuxError::ESPIPE));
        }
        if off_in.vm_read()? < 0 {
            return Err(AxError::InvalidInput);
        }
        SendFile::Offset(File::from_fd(fd_in)?, off_in.cast())
    } else {
        if let Ok(src) = Pipe::from_fd(fd_in) {
            if !src.is_read() {
                return Err(AxError::BadFileDescriptor);
            }
            has_pipe = true;
        }
        if let Ok(file) = File::from_fd(fd_in)
            && file.inner().is_path()
        {
            return Err(AxError::InvalidInput);
        }
        SendFile::Direct(get_file_like(fd_in)?)
    };

    let dst = if !off_out.is_null() {
        // Pipes do not support offset-based I/O; must return ESPIPE
        if Pipe::from_fd(fd_out).is_ok() || NamedPipe::from_fd(fd_out).is_ok() {
            return Err(AxError::from(LinuxError::ESPIPE));
        }
        if off_out.vm_read()? < 0 {
            return Err(AxError::InvalidInput);
        }
        SendFile::Offset(File::from_fd(fd_out)?, off_out.cast())
    } else {
        if let Ok(dst) = Pipe::from_fd(fd_out) {
            if !dst.is_write() {
                return Err(AxError::BadFileDescriptor);
            }
            has_pipe = true;
        }
        if let Ok(file) = File::from_fd(fd_out)
            && file.inner().access(FileFlags::APPEND).is_ok()
        {
            return Err(AxError::InvalidInput);
        }
        let f = get_file_like(fd_out)?;
        f.write(&mut b"".as_slice())?;
        SendFile::Direct(f)
    };

    if !has_pipe {
        return Err(AxError::InvalidInput);
    }

    do_send(src, dst, len).map(|n| n as _)
}
