use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use core::ffi::c_int;
use core::mem::MaybeUninit;

use axerrno::{AxError, AxResult, LinuxError};
use axfs::OpenOptions;
use axfs::FS_CONTEXT;
use axfs_ng_vfs::{Location, NodeType};
use axsync::Mutex;
use axtask::current;
use linux_raw_sys::general::{
    AT_EMPTY_PATH, AT_FDCWD, AT_SYMLINK_FOLLOW, AT_SYMLINK_NOFOLLOW, O_ACCMODE, O_APPEND,
    O_CLOEXEC, O_RDWR, O_TRUNC, O_WRONLY,
};
use starry_vm::{VmMutPtr, VmPtr};

use crate::file::{Directory, File, Kstat, add_file_like, get_file_like, resolve_at, with_fs};
use crate::mm::vm_load_string;
use crate::task::AsThread;

// --- file_handle ABI ---

/// User-space `struct file_handle` header (handle_bytes + handle_type).
#[repr(C)]
#[derive(Copy, Clone)]
struct UserFileHandleHeader {
    handle_bytes: u32,
    handle_type: i32,
}

/// Size of the `file_handle` header in bytes.
const FH_HEADER_SIZE: u32 = 8;

/// Size of our opaque f_handle payload: opaque_id (u64 LE) + generation (u64 LE).
const FH_PAYLOAD_SIZE: u32 = 16;

/// Required `handle_bytes` — size of f_handle[] payload (NOT including header).
const REQUIRED_HANDLE_BYTES: u32 = FH_PAYLOAD_SIZE;

/// Maximum f_handle[] payload size we accept from userspace (Linux MAX_HANDLE_SZ = 128).
const MAX_HANDLE_SZ: u32 = 128;

/// Handle type 1: generic file handle.
const HANDLE_TYPE: i32 = 1;

/// Fixed mount ID — we don't have real mount namespaces.
const MOUNT_ID: i32 = 0;

/// Valid flags for name_to_handle_at: 0, AT_EMPTY_PATH, AT_SYMLINK_FOLLOW.
const NAME_TO_HANDLE_AT_VALID_FLAGS: u32 = AT_EMPTY_PATH | AT_SYMLINK_FOLLOW;

// --- Opaque handle table ---

static NEXT_HANDLE_ID: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(1);

struct HandleEntry {
    path: String,
    dev: u64,
    ino: u64,
    #[allow(dead_code)]
    mode: u32,
    generation: u64,
    is_symlink: bool,
    /// Device of the filesystem at handle-creation time (for stale detection).
    mount_dev: u64,
}

/// Global runtime opaque handle table (id → entry).
static HANDLE_TABLE: Mutex<BTreeMap<u64, HandleEntry>> = Mutex::new(BTreeMap::new());

fn alloc_handle_id() -> u64 {
    NEXT_HANDLE_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed)
}

// --- Helpers ---

fn read_handle_header(handle_ptr: *const u8) -> AxResult<(u32, i32)> {
    let ptr: *const UserFileHandleHeader = handle_ptr as *const UserFileHandleHeader;
    let header = ptr
        .nullable()
        .ok_or(AxError::BadAddress)?
        .vm_read_uninit()
        .map_err(|_| AxError::BadAddress)?;
    let header: UserFileHandleHeader = unsafe { header.assume_init() };
    Ok((header.handle_bytes, header.handle_type))
}

fn write_handle_header(handle: *mut u8) -> AxResult<()> {
    let hdr = UserFileHandleHeader {
        handle_bytes: REQUIRED_HANDLE_BYTES,
        handle_type: HANDLE_TYPE,
    };
    let ptr: *mut UserFileHandleHeader = handle as *mut UserFileHandleHeader;
    ptr.nullable()
        .ok_or(AxError::BadAddress)?
        .vm_write(hdr)
        .map_err(|_| AxError::BadAddress)?;
    Ok(())
}

fn write_handle_payload(handle_ptr: *mut u8, opaque_id: u64, generation: u64) -> AxResult<()> {
    let payload_ptr = handle_ptr.wrapping_add(FH_HEADER_SIZE as usize);
    let mut buf = [0u8; FH_PAYLOAD_SIZE as usize];
    buf[..8].copy_from_slice(&opaque_id.to_le_bytes());
    buf[8..].copy_from_slice(&generation.to_le_bytes());
    starry_vm::vm_write_slice(payload_ptr, &buf).map_err(|_| AxError::BadAddress)?;
    Ok(())
}

fn write_handle_output(handle: *mut u8, opaque_id: u64, generation: u64) -> AxResult<()> {
    write_handle_header(handle)?;
    write_handle_payload(handle, opaque_id, generation)
}

fn decode_handle_payload(handle_ptr: *const u8) -> AxResult<(u64, u64)> {
    let payload_ptr = handle_ptr.wrapping_add(FH_HEADER_SIZE as usize);
    let mut buf: [MaybeUninit<u8>; FH_PAYLOAD_SIZE as usize] =
        [MaybeUninit::uninit(); FH_PAYLOAD_SIZE as usize];
    starry_vm::vm_read_slice(payload_ptr, &mut buf).map_err(|_| AxError::BadAddress)?;
    let buf: [u8; FH_PAYLOAD_SIZE as usize] = unsafe { core::mem::transmute(buf) };
    let opaque_id = u64::from_le_bytes(buf[..8].try_into().unwrap());
    let generation = u64::from_le_bytes(buf[8..].try_into().unwrap());
    Ok((opaque_id, generation))
}

/// Get the Location of a fd without the access check done by backend().
/// Returns None for unsupported fd types (pipe, socket, etc.).
fn get_mount_location(fd: c_int) -> Option<Location> {
    let file_like = get_file_like(fd).ok()?;
    if let Some(file) = file_like.downcast_ref::<File>() {
        Some(file.inner().location().clone())
    } else if let Some(dir) = file_like.downcast_ref::<Directory>() {
        Some(dir.inner().clone())
    } else {
        None
    }
}

fn handle_flags_to_options(flags: c_int) -> OpenOptions {
    let flags = flags as u32;
    let mut options = OpenOptions::new();
    match flags & O_ACCMODE {
        O_WRONLY => {
            options.write(true);
        }
        O_RDWR => {
            options.read(true).write(true);
        }
        _ => {
            options.read(true);
        }
    };
    if flags & O_APPEND != 0 {
        options.append(true);
    }
    if flags & O_TRUNC != 0 {
        options.truncate(true);
    }
    options
}

// --- Syscalls ---

/// `name_to_handle_at(dirfd, pathname, handle, mount_id, flags)`
///
/// Error order:
///   1. invalid flags → EINVAL
///   2. handle pointer null → EFAULT
///   3. mount_id pointer null → EFAULT
///   4. pathname pointer null → EFAULT
///   5. read handle_bytes → EFAULT (bad handle pointer)
///   6. handle_bytes > MAX → EINVAL
///   7. write mount_id (catches invalid mount_id ptr → EFAULT)
///   8. empty path without AT_EMPTY_PATH → ENOENT
///   9. resolve path → EBADF / ENOTDIR
///   10. handle_bytes < REQUIRED → EOVERFLOW
pub fn sys_name_to_handle_at(
    dirfd: c_int,
    pathname: *const core::ffi::c_char,
    handle: *mut u8,
    mount_id: *mut i32,
    flags: u32,
) -> AxResult<isize> {
    // 1. Validate flags
    if flags & !NAME_TO_HANDLE_AT_VALID_FLAGS != 0 {
        return Err(AxError::InvalidInput);
    }

    // 2. Validate handle pointer
    if handle.is_null() {
        return Err(AxError::BadAddress);
    }

    // 3. Validate mount_id pointer (null check)
    if mount_id.is_null() {
        return Err(AxError::BadAddress);
    }

    // 4. Validate pathname pointer
    if pathname.is_null() {
        return Err(AxError::BadAddress);
    }

    // 5. Read handle_bytes from userspace
    let (handle_bytes, _handle_type) = read_handle_header(handle)?;

    // 6. Reject oversized handle
    if handle_bytes > MAX_HANDLE_SZ {
        return Err(AxError::InvalidInput);
    }

    // 7. Write mount_id early — catches invalid mount_id pointer (EFAULT)
    //    before the EOVERFLOW check below.
    mount_id
        .nullable()
        .ok_or(AxError::BadAddress)?
        .vm_write(MOUNT_ID)
        .map_err(|_| AxError::BadAddress)?;

    // 8. Load path string; check empty path
    let path = vm_load_string(pathname)?;
    if path.is_empty() && flags & AT_EMPTY_PATH == 0 {
        return Err(AxError::NotFound); // ENOENT
    }

    // 9. Resolve the target.
    //    For AT_EMPTY_PATH we bypass resolve_at() because its internal
    //    backend() call rejects O_PATH files.  We resolve the fd directly
    //    using location() which has no access check.
    let (_resolved, kstat, is_symlink, absolute_path, mount_dev) =
        if path.is_empty() && flags & AT_EMPTY_PATH != 0 {
            // --- AT_EMPTY_PATH: resolve the fd itself ---
            let (loc, abs_path) = if dirfd == AT_FDCWD as c_int {
                let cwd = {
                    let fs = FS_CONTEXT.lock();
                    fs.current_dir().clone()
                };
                let abs = cwd
                    .absolute_path()
                    .map(|p| p.to_string())
                    .map_err(|_| AxError::Io)?;
                (cwd, abs)
            } else {
                let file_like = get_file_like(dirfd)?;
                if let Some(file) = file_like.downcast_ref::<File>() {
                    let loc = file.inner().location().clone();
                    let abs = loc
                        .absolute_path()
                        .map(|p| p.to_string())
                        .unwrap_or_default();
                    (loc, abs)
                } else if let Some(dir) = file_like.downcast_ref::<Directory>() {
                    let loc = dir.inner().clone();
                    let abs = loc
                        .absolute_path()
                        .map(|p| p.to_string())
                        .unwrap_or_default();
                    (loc, abs)
                } else {
                    return Err(AxError::BadFileDescriptor);
                }
            };
            let md = loc.metadata()?;
            let ks = Kstat {
                dev: md.device,
                ino: md.inode,
                nlink: md.nlink as u32,
                mode: ((md.node_type as u8 as u32) << 12) | md.mode.bits() as u32,
                uid: md.uid,
                gid: md.gid,
                size: md.size,
                blksize: md.block_size as u32,
                blocks: md.blocks,
                rdev: md.rdev,
                attributes: 0,
                atime: md.atime,
                mtime: md.mtime,
                ctime: md.ctime,
            };
            let sym = md.node_type == NodeType::Symlink;
            let mnt_dev = md.device;
            (
                crate::file::ResolveAtResult::File(loc),
                ks,
                sym,
                abs_path,
                mnt_dev,
            )
        } else {
            // --- Normal path resolution ---
            let mut resolve_flags = 0u32;
            if flags & AT_SYMLINK_FOLLOW == 0 {
                resolve_flags |= AT_SYMLINK_NOFOLLOW;
            }
            let resolved = resolve_at(dirfd, Some(&path), resolve_flags)?;
            let ks = resolved.stat()?;
            let sym = match &resolved {
                crate::file::ResolveAtResult::File(loc) => {
                    matches!(loc.metadata().map(|m| m.node_type), Ok(NodeType::Symlink))
                }
                _ => false,
            };
            let abs_path = match &resolved {
                crate::file::ResolveAtResult::File(loc) => loc
                    .absolute_path()
                    .map(|p| p.to_string())
                    .unwrap_or_else(|_| path.clone()),
                _ => path.clone(),
            };
            let mnt_dev = ks.dev;
            (resolved, ks, sym, abs_path, mnt_dev)
        };

    // 10. Handle too-small only after path resolution succeeds
    if handle_bytes == 0 || handle_bytes < REQUIRED_HANDLE_BYTES {
        let _ = write_handle_header(handle);
        return Err(AxError::from(LinuxError::EOVERFLOW));
    }

    // 11. Allocate and store handle entry
    let opaque_id = alloc_handle_id();
    let generation = opaque_id;

    HANDLE_TABLE.lock().insert(
        opaque_id,
        HandleEntry {
            path: absolute_path,
            dev: kstat.dev,
            ino: kstat.ino,
            mode: kstat.mode,
            generation,
            is_symlink,
            mount_dev,
        },
    );

    // 15. Write handle output to userspace
    write_handle_output(handle, opaque_id, generation)?;

    debug!(
        "name_to_handle_at: dirfd={dirfd} path={path} id={opaque_id} symlink={is_symlink} dev={} ino={}",
        kstat.dev, kstat.ino
    );
    Ok(0)
}

/// `open_by_handle_at(mount_fd, handle, flags)`
///
/// Error order:
///   1. handle pointer null → EFAULT
///   2. read header + handle_bytes == 0 → EINVAL
///   3. handle_bytes > MAX → EINVAL
///   4. handle_type mismatch → EINVAL
///   5. capability → EPERM
///   6. decode payload → EFAULT
///   7. mount_fd → EBADF (skip for AT_FDCWD)
///   8. handle lookup → ESTALE
///   9. symlink → ELOOP
///   10. reopen
pub fn sys_open_by_handle_at(
    mount_fd: c_int,
    handle: *const u8,
    flags: c_int,
) -> AxResult<isize> {
    const CAP_DAC_READ_SEARCH: u32 = 2;

    // 1. Validate handle pointer
    if handle.is_null() {
        return Err(AxError::BadAddress);
    }

    // 2. Read handle header
    let header_ptr: *const UserFileHandleHeader = handle as *const UserFileHandleHeader;
    let header = header_ptr
        .vm_read_uninit()
        .map_err(|_| AxError::BadAddress)?;
    let header: UserFileHandleHeader = unsafe { header.assume_init() };
    let handle_bytes = header.handle_bytes;
    let handle_type = header.handle_type;

    // 3. Validate handle_bytes
    if handle_bytes == 0 {
        return Err(AxError::InvalidInput);
    }
    if handle_bytes > MAX_HANDLE_SZ {
        return Err(AxError::InvalidInput);
    }

    // 4. Validate handle_type
    if handle_type != HANDLE_TYPE {
        return Err(AxError::InvalidInput);
    }

    // 5. Check CAP_DAC_READ_SEARCH
    let cur = current();
    let proc_data = &cur.as_thread().proc_data;
    if !proc_data.has_capability(CAP_DAC_READ_SEARCH) {
        return Err(AxError::OperationNotPermitted); // EPERM
    }

    // 6. Decode opaque payload
    let (opaque_id, generation) = decode_handle_payload(handle)?;

    // 7. Validate mount_fd (skip fd lookup for AT_FDCWD)
    if mount_fd != AT_FDCWD as c_int {
        let _mount_file = get_file_like(mount_fd)?; // EBADF if invalid
    }

    // 8. Look up handle
    let (handle_path, is_symlink, handle_mount_dev) = {
        let table = HANDLE_TABLE.lock();
        match table.get(&opaque_id) {
            Some(entry) if entry.generation == generation => {
                (entry.path.clone(), entry.is_symlink, entry.mount_dev)
            }
            _ => return Err(AxError::from(LinuxError::ESTALE)),
        }
    };

    // 9. Staleness check: verify mount_fd belongs to the same filesystem
    //    as when the handle was created.
    //    Uses direct location access (not resolve_at) to avoid the
    //    backend() access check that rejects O_PATH fds.
    if mount_fd != AT_FDCWD as c_int {
        if let Some(mount_loc) = get_mount_location(mount_fd) {
            if let Ok(mount_md) = mount_loc.metadata() {
                if mount_md.device != handle_mount_dev {
                    return Err(AxError::from(LinuxError::ESTALE));
                }
            }
        }
    }

    // 10. Symlink handle → ELOOP
    if is_symlink {
        return Err(AxError::FilesystemLoop); // ELOOP
    }

    // 11. Reopen file by path
    let options = handle_flags_to_options(flags);
    let result = with_fs(AT_FDCWD as c_int, |fs| options.open(fs, &handle_path));

    match result {
        Ok(open_result) => match open_result {
            axfs::OpenResult::File(f) => {
                let kernel_file = File::new(f);
                let cloexec = flags as u32 & O_CLOEXEC != 0;
                let fd = add_file_like(Arc::new(kernel_file), cloexec)?;
                debug!("open_by_handle_at: reopened file path={handle_path} as fd={fd}");
                Ok(fd as isize)
            }
            axfs::OpenResult::Dir(dir_loc) => {
                let dir = Directory::new(dir_loc);
                let cloexec = flags as u32 & O_CLOEXEC != 0;
                let fd = add_file_like(Arc::new(dir), cloexec)?;
                debug!("open_by_handle_at: reopened dir path={handle_path} as fd={fd}");
                Ok(fd as isize)
            }
        },
        Err(_e) => {
            debug!("open_by_handle_at: failed to reopen path={handle_path}: {_e:?}");
            Err(AxError::from(LinuxError::ESTALE))
        }
    }
}
