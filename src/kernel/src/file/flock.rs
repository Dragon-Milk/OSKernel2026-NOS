//! BSD flock(2) — whole-file advisory locks.
//!
//! flock locks are associated with the **open file description** (ofd_id),
//! not the process.  Lock identity uses the same `InodeKey` as POSIX record
//! locks so that locks on the same underlying file (including via hard links)
//! are visible to each other.
//!
//! Unlike POSIX byte-range locks, flock operates on the entire file — there
//! are no byte ranges to track.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use axerrno::{AxError, AxResult};
use axsync::Mutex;

use super::record_lock::{InodeKey, OfdOwner};

// ---------------------------------------------------------------------------
// Constants (from <sys/file.h> on Linux)
// ---------------------------------------------------------------------------

const LOCK_SH: i32 = 1;
const LOCK_EX: i32 = 2;
const LOCK_NB: i32 = 4;
const LOCK_UN: i32 = 8;

// ---------------------------------------------------------------------------
// Per-inode flock state
// ---------------------------------------------------------------------------

/// Current flock state for one inode.
///
/// - `None`           → no flock held on the file.
/// - `Shared(set)`    → one or more OFD owners hold a shared lock.
/// - `Exclusive(one)` → exactly one OFD owner holds an exclusive lock.
enum FlockState {
    Shared(Vec<OfdOwner>),
    Exclusive(OfdOwner),
}

// ---------------------------------------------------------------------------
// Global flock table
// ---------------------------------------------------------------------------

static FLOCK_TABLE: spin::Lazy<Mutex<BTreeMap<InodeKey, FlockState>>> =
    spin::Lazy::new(|| Mutex::new(BTreeMap::new()));

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Handle the `flock(2)` system call.
///
/// `fd`        – file descriptor.
/// `operation` – `LOCK_SH | LOCK_EX | LOCK_UN`, optionally ORed with `LOCK_NB`.
pub fn sys_flock(fd: i32, operation: i32) -> AxResult<isize> {
    let nonblock = (operation & LOCK_NB) != 0;
    let lock_cmd = operation & !LOCK_NB;

    if lock_cmd != LOCK_SH && lock_cmd != LOCK_EX && lock_cmd != LOCK_UN {
        return Err(AxError::InvalidInput);
    }

    // Resolve the fd → inode key + OFD owner.
    let file = super::get_file_like(fd)?;
    let key = file.inode_key().ok_or(AxError::BadFileDescriptor)?;
    let owner = file.ofd_owner();

    // Unlock and non-blocking operations: single attempt.
    if nonblock || lock_cmd == LOCK_UN {
        let mut table = FLOCK_TABLE.lock();
        return match lock_cmd {
            LOCK_SH => try_shared(&mut table, key, owner),
            LOCK_EX => try_exclusive(&mut table, key, owner),
            LOCK_UN => flock_unlock(&mut table, key, owner),
            _ => unreachable!(),
        };
    }

    // Blocking mode: retry loop. Drop the table lock between attempts so that
    // the holder has a chance to release.
    const MAX_RETRIES: u32 = 200;
    for _ in 0..MAX_RETRIES {
        let result = {
            let mut table = FLOCK_TABLE.lock();
            match lock_cmd {
                LOCK_SH => try_shared(&mut table, key, owner),
                LOCK_EX => try_exclusive(&mut table, key, owner),
                _ => unreachable!(),
            }
        };
        match result {
            Ok(v) => return Ok(v),
            Err(AxError::WouldBlock) => {
                // Yield to let the holder release.
                axtask::yield_now();
            }
            Err(e) => return Err(e),
        }
    }
    // Timed out after retries.
    Err(AxError::WouldBlock)
}

/// Release all flock locks held by `owner` on inode `key`.
///
/// Called from `File::drop` when the last reference to an open file
/// description is released.
pub fn release_flock(key: InodeKey, owner: OfdOwner) {
    let mut table = FLOCK_TABLE.lock();
    remove_owner(&mut table, key, owner);
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Try to acquire a shared lock.  Returns `Ok(0)` on success, or
/// `Err(WouldBlock)` if another OFD holds an exclusive lock.
fn try_shared(
    table: &mut BTreeMap<InodeKey, FlockState>,
    key: InodeKey,
    owner: OfdOwner,
) -> AxResult<isize> {
    match table.get(&key) {
        None => {
            table.insert(key, FlockState::Shared(alloc::vec![owner]));
            Ok(0)
        }
        Some(FlockState::Shared(holders)) => {
            if holders.contains(&owner) {
                return Ok(0); // same OFD already holds shared — no-op
            }
            let mut holders = holders.clone();
            holders.push(owner);
            table.insert(key, FlockState::Shared(holders));
            Ok(0)
        }
        Some(FlockState::Exclusive(ex_owner)) => {
            if *ex_owner == owner {
                // Same OFD downgrading from exclusive to shared.
                table.insert(key, FlockState::Shared(alloc::vec![owner]));
                return Ok(0);
            }
            Err(AxError::WouldBlock)
        }
    }
}

/// Try to acquire an exclusive lock.  Returns `Ok(0)` on success, or
/// `Err(WouldBlock)` if any conflicting lock is held by another OFD.
fn try_exclusive(
    table: &mut BTreeMap<InodeKey, FlockState>,
    key: InodeKey,
    owner: OfdOwner,
) -> AxResult<isize> {
    match table.get(&key) {
        None => {
            table.insert(key, FlockState::Exclusive(owner));
            Ok(0)
        }
        Some(FlockState::Exclusive(ex_owner)) => {
            if *ex_owner == owner {
                return Ok(0); // same OFD re-locking exclusive — no-op
            }
            Err(AxError::WouldBlock)
        }
        Some(FlockState::Shared(holders)) => {
            if holders.len() == 1 && holders[0] == owner {
                // Same OFD upgrading from shared to exclusive.
                table.insert(key, FlockState::Exclusive(owner));
                return Ok(0);
            }
            let only_self = holders.iter().all(|h| *h == owner);
            if only_self {
                table.insert(key, FlockState::Exclusive(owner));
                Ok(0)
            } else {
                Err(AxError::WouldBlock)
            }
        }
    }
}

/// Release the caller's lock on `key`.
fn flock_unlock(
    table: &mut BTreeMap<InodeKey, FlockState>,
    key: InodeKey,
    owner: OfdOwner,
) -> AxResult<isize> {
    remove_owner(table, key, owner);
    Ok(0)
}

/// Remove `owner` from the flock state on `key`, cleaning up empty entries.
fn remove_owner(table: &mut BTreeMap<InodeKey, FlockState>, key: InodeKey, owner: OfdOwner) {
    let entry = match table.get(&key) {
        Some(s) => s,
        None => return,
    };

    let new_state: Option<FlockState> = match entry {
        FlockState::Shared(holders) => {
            let mut h = holders.clone();
            h.retain(|o| *o != owner);
            if h.is_empty() {
                None
            } else {
                Some(FlockState::Shared(h))
            }
        }
        FlockState::Exclusive(ex_owner) => {
            if *ex_owner == owner {
                None
            } else {
                // Not our lock — nothing to do.
                return;
            }
        }
    };

    match new_state {
        Some(s) => {
            table.insert(key, s);
        }
        None => {
            table.remove(&key);
        }
    }
}
