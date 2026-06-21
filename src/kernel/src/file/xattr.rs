//! Extended attributes (xattr) storage at inode granularity.
//!
//! Keys are `(filesystem device id, inode number)`, so hard links share
//! the same xattr set, while symlinks and their targets are independent.
//! This is an in-memory compatibility store; it is NOT persisted to disk.

use alloc::{
    collections::BTreeMap,
    sync::Arc,
    vec::Vec,
};
use axerrno::{AxError, AxResult, LinuxError};
use axfs_ng_vfs::{Location, NodeType};
use axtask::current;
use spin::Mutex;

use crate::task::AsThread;

/// Maximum xattr name length (excluding the terminating NUL).
pub const XATTR_NAME_MAX: usize = 255;

/// Maximum xattr value size we accept in a single set/list operation.
pub const XATTR_SIZE_MAX: usize = 65536;

/// XATTR_CREATE — fail if the attribute already exists.
pub const XATTR_CREATE: u32 = 1;

/// XATTR_REPLACE — fail if the attribute does not already exist.
pub const XATTR_REPLACE: u32 = 2;

type InodeKey = (u64, u64);

/// Global in-memory xattr table: `(device_id, inode) -> (name -> value)`.
static XATTR_TABLE: Mutex<BTreeMap<InodeKey, Arc<Mutex<XattrMap>>>> =
    Mutex::new(BTreeMap::new());

type XattrMap = BTreeMap<Vec<u8>, Vec<u8>>;

fn inode_key(loc: &Location) -> InodeKey {
    (loc.mountpoint().device(), loc.inode())
}

fn get_map(loc: &Location) -> Arc<Mutex<XattrMap>> {
    let key = inode_key(loc);
    let mut table = XATTR_TABLE.lock();
    table
        .entry(key)
        .or_insert_with(|| Arc::new(Mutex::new(BTreeMap::new())))
        .clone()
}

/// Remove the xattr map for a given inode when it is truly reclaimed.
/// Called when nlink drops to zero and the last reference is released.
pub fn remove_xattr_map(loc: &Location) {
    let key = inode_key(loc);
    XATTR_TABLE.lock().remove(&key);
}

// ---------------------------------------------------------------------------
// Namespace / name validation
// ---------------------------------------------------------------------------

fn validate_xattr_name(name: &[u8]) -> AxResult<()> {
    // Length checks come first: empty and too-long both map to ERANGE
    // per LTP setxattr01/fsetxattr01 cases 02 and 07.
    if name.is_empty() || name.len() > XATTR_NAME_MAX {
        return Err(AxError::OutOfRange);
    }
    // Must have a namespace prefix (contain at least one '.')
    if !name.contains(&b'.') {
        return Err(AxError::InvalidInput);
    }
    Ok(())
}

fn check_xattr_namespace_permission(name: &[u8], _loc: &Location) -> AxResult<()> {
    let ns_end = name.iter().position(|&b| b == b'.').unwrap_or(name.len());
    let ns = &name[..ns_end];

    match ns {
        b"user" => {
            // user.* is always allowed for any inode type.
            Ok(())
        }
        b"trusted" | b"security" | b"system" => {
            // Require effective root (uid == 0).
            if current().as_thread().proc_data.ids().1 == 0 {
                Ok(())
            } else {
                // For system.*, Linux returns EOPNOTSUPP for regular users.
                // For trusted.* and security.*, EPERM or EOPNOTSUPP.
                // LTP tests expect these to fail for non-root.
                if ns == b"system" {
                    Err(AxError::OperationNotSupported)
                } else {
                    Err(AxError::OperationNotPermitted)
                }
            }
        }
        _ => {
            // Unknown namespace — not supported.
            Err(AxError::OperationNotSupported)
        }
    }
}

// ---------------------------------------------------------------------------
// Node-type gate for setxattr
// ---------------------------------------------------------------------------

/// Only regular files, directories, and symlinks may carry user.* extended
/// attributes.  All other node types (FIFO, character device, block device,
/// socket) return `EPERM`, matching the Linux kernel behaviour tested by
/// LTP `setxattr02`.
fn check_xattr_set_allowed(loc: &Location) -> AxResult<()> {
    let ty = loc.node_type();
    match ty {
        NodeType::RegularFile | NodeType::Directory | NodeType::Symlink => Ok(()),
        _ => Err(AxError::OperationNotPermitted),
    }
}

// ---------------------------------------------------------------------------
// Set
// ---------------------------------------------------------------------------

pub fn do_setxattr(
    loc: &Location,
    name: &[u8],
    value: &[u8],
    flags: u32,
) -> AxResult<()> {
    validate_xattr_name(name)?;
    if value.len() > XATTR_SIZE_MAX {
        return Err(AxError::from(LinuxError::E2BIG));
    }
    if flags & !(XATTR_CREATE | XATTR_REPLACE) != 0 {
        return Err(AxError::InvalidInput);
    }
    if flags == (XATTR_CREATE | XATTR_REPLACE) {
        return Err(AxError::InvalidInput);
    }

    // user.* xattrs are only allowed on regular files, directories, and symlinks.
    // FIFO, character device, block device, and socket special inodes all fail EPERM.
    check_xattr_set_allowed(loc)?;

    check_xattr_namespace_permission(name, loc)?;

    let map = get_map(loc);
    let mut guard = map.lock();
    let exists = guard.contains_key(name);

    if flags == XATTR_CREATE && exists {
        return Err(AxError::AlreadyExists);
    }
    if flags == XATTR_REPLACE && !exists {
        return Err(AxError::from(LinuxError::ENODATA));
    }

    guard.insert(name.to_vec(), value.to_vec());
    Ok(())
}

// ---------------------------------------------------------------------------
// Get
// ---------------------------------------------------------------------------

/// Return the raw value bytes for the named xattr.
/// The caller is responsible for size checks and user-space copying.
pub fn do_getxattr(loc: &Location, name: &[u8]) -> AxResult<Vec<u8>> {
    validate_xattr_name(name)?;
    check_xattr_namespace_permission(name, loc)?;

    let map = get_map(loc);
    let guard = map.lock();
    guard.get(name).cloned().ok_or(AxError::from(LinuxError::ENODATA))
}

// ---------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------

/// Return the NUL-separated list of xattr names as raw bytes.
/// The caller is responsible for size checks and user-space copying.
pub fn do_listxattr(loc: &Location) -> AxResult<Vec<u8>> {
    let map = get_map(loc);
    let guard = map.lock();

    let mut buf: Vec<u8> = Vec::new();
    for name in guard.keys() {
        // Only include names the caller has permission to observe.
        let ns_end = name.iter().position(|&b| b == b'.').unwrap_or(name.len());
        let ns = &name[..ns_end];
        let allowed = match ns {
            b"user" => true,
            _ => {
                current().as_thread().proc_data.ids().1 == 0
            }
        };
        if allowed {
            buf.extend_from_slice(name);
            buf.push(0u8);
        }
    }
    Ok(buf)
}

// ---------------------------------------------------------------------------
// Remove
// ---------------------------------------------------------------------------

pub fn do_removexattr(
    loc: &Location,
    name: &[u8],
) -> AxResult<()> {
    validate_xattr_name(name)?;
    check_xattr_namespace_permission(name, loc)?;

    let map = get_map(loc);
    let mut guard = map.lock();
    if guard.remove(name).is_none() {
        return Err(AxError::from(LinuxError::ENODATA));
    }
    Ok(())
}
