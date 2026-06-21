//! POSIX and OFD record lock manager.
//!
//! POSIX record locks are **process-owned**, operate on **inode byte ranges**,
//! and are released when the owning process closes *any* fd referencing the
//! inode.  OFD locks are owned by the open file description and released when
//! the last reference to that description is dropped.
//!
//! Lock identity (inode key) = (filesystem device id, inode number), which
//! correctly handles hard links (different paths → same inode).

use alloc::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Weak},
    vec::Vec,
};
use axerrno::{AxError, AxResult, LinuxError};
use axsync::Mutex;
use core::{
    cmp::Ordering,
    task::Waker,
};
use linux_raw_sys::general::{F_RDLCK, F_UNLCK, F_WRLCK, flock64};
use spin::RwLock;

use crate::task::ProcessData;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Globally-unique inode identity: (filesystem device id, inode number).
pub type InodeKey = (u64, u64);

/// Process-level POSIX lock owner (weak to avoid cycles / keep-alive).
pub type PosixOwner = Weak<ProcessData>;

/// OFD lock owner – a stable numeric id unique per open-file-description.
/// `dup`/`fork` share the same id; independent `open` calls get different ids.
pub type OfdOwner = u64;

/// Normalised byte-range lock interval: `[start, end)`, where `end == None`
/// means "through EOF" (infinite range).
#[derive(Debug, Clone)]
pub struct LockInterval {
    pub start: i64,
    pub end: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    Read,
    Write,
}

// ---------------------------------------------------------------------------
// Interval helpers
// ---------------------------------------------------------------------------

impl LockInterval {
    fn overlaps(&self, other: &Self) -> bool {
        let self_end = self.end.unwrap_or(i64::MAX);
        let other_end = other.end.unwrap_or(i64::MAX);
        self.start < other_end && other.start < self_end
    }

}

/// Convert `lock_type` to `LockType`.  `F_UNLCK` returns `None` — callers
/// must handle unlocking separately.
pub fn lock_type_from_i16(t: i16) -> Option<LockType> {
    match t as u32 {
        F_RDLCK => Some(LockType::Read),
        F_WRLCK => Some(LockType::Write),
        _ => None,
    }
}

fn lock_type_to_i16(lt: LockType) -> i16 {
    match lt {
        LockType::Read => F_RDLCK as i16,
        LockType::Write => F_WRLCK as i16,
    }
}

/// Locks of the same type are compatible; read+write conflict; write+write
/// conflict.
fn types_conflict(a: LockType, b: LockType) -> bool {
    !matches!((a, b), (LockType::Read, LockType::Read))
}

// ---------------------------------------------------------------------------
// Lock records
// ---------------------------------------------------------------------------

struct PosixLock {
    owner: PosixOwner,
    pid: u32,
    lock_type: LockType,
    interval: LockInterval,
}

struct OfdLock {
    owner: OfdOwner,
    lock_type: LockType,
    interval: LockInterval,
}

// ---------------------------------------------------------------------------
// Wait queue (per-inode, simple waker storage)
// ---------------------------------------------------------------------------

struct LockWaitQueue {
    queue: spin::mutex::SpinMutex<alloc::collections::VecDeque<Waker>>,
}

impl LockWaitQueue {
    fn new() -> Self {
        Self {
            queue: spin::mutex::SpinMutex::new(alloc::collections::VecDeque::new()),
        }
    }

    fn push(&self, w: Waker) {
        self.queue.lock().push_back(w);
    }

    fn wake_all(&self) {
        let mut q = self.queue.lock();
        while let Some(w) = q.pop_front() {
            w.wake();
        }
    }

    fn wake_one(&self) {
        if let Some(w) = self.queue.lock().pop_front() {
            w.wake();
        }
    }
}

// ---------------------------------------------------------------------------
// Per-inode lock state
// ---------------------------------------------------------------------------

struct InodeLockState {
    posix: Vec<PosixLock>,
    ofd: Vec<OfdLock>,
    posix_wq: LockWaitQueue,
    ofd_wq: LockWaitQueue,
    /// Monotonic generation counter incremented on every state creation.
    /// Useful for diagnosing whether two Arc's reference the same state
    /// (same generation → same state).
    generation: u64,
}

impl InodeLockState {
    fn new() -> Self {
        static NEXT_GEN: core::sync::atomic::AtomicU64 =
            core::sync::atomic::AtomicU64::new(1);
        Self {
            posix: Vec::new(),
            ofd: Vec::new(),
            posix_wq: LockWaitQueue::new(),
            ofd_wq: LockWaitQueue::new(),
            generation: NEXT_GEN.fetch_add(1, core::sync::atomic::Ordering::Relaxed),
        }
    }

    /// Wake all waiters in both queues.  Must be called after every lock
    /// state change because POSIX and OFD locks can block each other.
    fn wake_all_waiters(&self) {
        self.posix_wq.wake_all();
        self.ofd_wq.wake_all();
    }
}

// ---------------------------------------------------------------------------
// Global lock table
// ---------------------------------------------------------------------------

static LOCK_TABLE: spin::Lazy<RwLock<BTreeMap<InodeKey, Arc<Mutex<InodeLockState>>>>> =
    spin::Lazy::new(|| RwLock::new(BTreeMap::new()));

fn get_or_create_state(key: InodeKey) -> Arc<Mutex<InodeLockState>> {
    // Fast path: read lock
    {
        let table = LOCK_TABLE.read();
        if let Some(state) = table.get(&key) {
            debug!("LOCK_STATE_HIT inode=({},{}) gen={}", key.0, key.1, state.lock().generation);
            return state.clone();
        }
    }
    // Slow path: write lock + insert
    let mut table = LOCK_TABLE.write();
    let state = table
        .entry(key)
        .or_insert_with(|| {
            let s = Arc::new(Mutex::new(InodeLockState::new()));
            debug!("LOCK_STATE_CREATE inode=({},{}) gen={}", key.0, key.1, s.lock().generation);
            s
        })
        .clone();
    debug!("LOCK_STATE_HIT inode=({},{}) gen={}", key.0, key.1, state.lock().generation);
    state
}

/// Remove an inode's lock state if it is empty (no locks, no waiters).
/// Called after releasing locks to avoid leaking entries.
///
/// NOTE: currently unused — the function is kept for future use when
/// reference counting tracks active operations and pending waiters.
#[allow(dead_code)]
fn cleanup_if_empty(key: InodeKey, state: &Mutex<InodeLockState>) {
    let s = state.lock();
    if s.posix.is_empty() && s.ofd.is_empty() {
        // Wait queues can have stale wakers, but those will be cleaned up
        // when they wake and re-check.  Drop the guard so we can acquire the
        // global write lock.
        drop(s);
        let mut table = LOCK_TABLE.write();
        if let Some(entry) = table.get(&key) {
            let inner = entry.lock();
            if inner.posix.is_empty() && inner.ofd.is_empty() {
                drop(inner);
                table.remove(&key);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Owner helpers
// ---------------------------------------------------------------------------

/// Compare two POSIX owners for equality.
fn posix_owner_eq(a: &PosixOwner, b: &PosixOwner) -> bool {
    // Compare the underlying pointer of the ProcessData arcs.
    Weak::as_ptr(a) == Weak::as_ptr(b)
}

// ---------------------------------------------------------------------------
// Conflict detection
// ---------------------------------------------------------------------------

/// Find the first POSIX lock that conflicts with a would-be lock from `owner`
/// on `interval` with `lock_type`.  Returns `None` if no conflict.
///
/// A lock from the same owner never conflicts (POSIX: a process can always
/// override its own locks).
fn find_posix_conflict<'a>(
    locks: &'a [PosixLock],
    owner: &PosixOwner,
    lock_type: LockType,
    interval: &LockInterval,
) -> Option<&'a PosixLock> {
    for lk in locks {
        if posix_owner_eq(&lk.owner, owner) {
            continue; // same process → never a conflict
        }
        if interval.overlaps(&lk.interval) && types_conflict(lock_type, lk.lock_type) {
            return Some(lk);
        }
    }
    None
}

/// Find the first OFD lock that conflicts.  Same-owner locks never conflict.
fn find_ofd_conflict<'a>(
    locks: &'a [OfdLock],
    owner: OfdOwner,
    lock_type: LockType,
    interval: &LockInterval,
) -> Option<&'a OfdLock> {
    for lk in locks {
        if lk.owner == owner {
            continue;
        }
        if interval.overlaps(&lk.interval) && types_conflict(lock_type, lk.lock_type) {
            return Some(lk);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Install / update POSIX locks (merge + split)
// ---------------------------------------------------------------------------

/// Install (or remove) a POSIX lock for `owner` on `interval`.
///
/// For `F_UNLCK`, the interval is removed from the owner's existing locks,
/// potentially splitting one lock into two.
///
/// For `F_RDLCK` / `F_WRLCK`, the new lock replaces any overlapping locks
/// of the same owner, then adjacent/same-type locks are merged.
fn apply_posix_lock(
    locks: &mut Vec<PosixLock>,
    owner: &PosixOwner,
    pid: u32,
    lock_type: Option<LockType>, // None = unlock
    interval: &LockInterval,
) {
    // 1. Remove all locks from the same owner that overlap the interval.
    let mut new_locks: Vec<PosixLock> = Vec::new();

    for lk in locks.drain(..) {
        if !posix_owner_eq(&lk.owner, owner) {
            // Other owner's lock – keep unconditionally.
            new_locks.push(lk);
            continue;
        }

        // Same owner.
        if !interval.overlaps(&lk.interval) {
            // No overlap – keep.
            new_locks.push(lk);
            continue;
        }

        // The existing lock overlaps the target interval.
        // Remove overlapping portion, keep non-overlapping tails.
        let old_start = lk.interval.start;
        let old_end = lk.interval.end;

        // Left remainder (before interval)
        if old_start < interval.start {
            new_locks.push(PosixLock {
                owner: owner.clone(),
                pid,
                lock_type: lk.lock_type,
                interval: LockInterval {
                    start: old_start,
                    end: Some(interval.start),
                },
            });
        }

        // Right remainder (after interval)
        match (old_end, interval.end) {
            (Some(oe), Some(ie)) if oe > ie => {
                new_locks.push(PosixLock {
                    owner: owner.clone(),
                    pid,
                    lock_type: lk.lock_type,
                    interval: LockInterval {
                        start: ie,
                        end: Some(oe),
                    },
                });
            }
            (None, Some(ie)) => {
                // old is infinite, interval is finite – keep [ie, ∞)
                new_locks.push(PosixLock {
                    owner: owner.clone(),
                    pid,
                    lock_type: lk.lock_type,
                    interval: LockInterval {
                        start: ie,
                        end: None,
                    },
                });
            }
            _ => {
                // old_end <= interval_end or both infinite → no right remainder
            }
        }
    }

    // 2. If this is a set-lock (not unlock), insert the new lock.
    if let Some(lt) = lock_type {
        new_locks.push(PosixLock {
            owner: owner.clone(),
            pid,
            lock_type: lt,
            interval: interval.clone(),
        });
    }

    // 3. Merge adjacent locks of the same type and owner.
    // First sort by start, then merge.
    new_locks.sort_by(|a, b| {
        a.interval
            .start
            .cmp(&b.interval.start)
            .then_with(|| match (a.interval.end, b.interval.end) {
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(ae), Some(be)) => ae.cmp(&be),
                (None, None) => Ordering::Equal,
            })
    });

    let mut merged: Vec<PosixLock> = Vec::new();
    for lk in new_locks {
        if let Some(last) = merged.last_mut() {
            if posix_owner_eq(&last.owner, &lk.owner) && last.lock_type == lk.lock_type {
                // Check if contiguous or overlapping
                let last_end = last.interval.end.unwrap_or(i64::MAX);
                if lk.interval.start <= last_end {
                    // Merge
                    last.interval.end = match (last.interval.end, lk.interval.end) {
                        (None, _) | (_, None) => None,
                        (Some(a), Some(b)) => Some(a.max(b)),
                    };
                    continue;
                }
            }
        }
        merged.push(lk);
    }

    *locks = merged;
}

/// Apply an OFD lock (same merge/split logic as POSIX, but using OfdOwner).
fn apply_ofd_lock(
    locks: &mut Vec<OfdLock>,
    owner: OfdOwner,
    lock_type: Option<LockType>,
    interval: &LockInterval,
) {
    let mut new_locks: Vec<OfdLock> = Vec::new();

    for lk in locks.drain(..) {
        if lk.owner != owner {
            new_locks.push(lk);
            continue;
        }
        if !interval.overlaps(&lk.interval) {
            new_locks.push(lk);
            continue;
        }

        let old_start = lk.interval.start;
        let old_end = lk.interval.end;

        if old_start < interval.start {
            new_locks.push(OfdLock {
                owner,
                lock_type: lk.lock_type,
                interval: LockInterval {
                    start: old_start,
                    end: Some(interval.start),
                },
            });
        }

        match (old_end, interval.end) {
            (Some(oe), Some(ie)) if oe > ie => {
                new_locks.push(OfdLock {
                    owner,
                    lock_type: lk.lock_type,
                    interval: LockInterval {
                        start: ie,
                        end: Some(oe),
                    },
                });
            }
            (None, Some(ie)) => {
                new_locks.push(OfdLock {
                    owner,
                    lock_type: lk.lock_type,
                    interval: LockInterval {
                        start: ie,
                        end: None,
                    },
                });
            }
            _ => {}
        }
    }

    if let Some(lt) = lock_type {
        new_locks.push(OfdLock {
            owner,
            lock_type: lt,
            interval: interval.clone(),
        });
    }

    new_locks.sort_by(|a, b| {
        a.interval
            .start
            .cmp(&b.interval.start)
            .then_with(|| match (a.interval.end, b.interval.end) {
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(ae), Some(be)) => ae.cmp(&be),
                (None, None) => Ordering::Equal,
            })
    });

    let mut merged: Vec<OfdLock> = Vec::new();
    for lk in new_locks {
        if let Some(last) = merged.last_mut() {
            if last.owner == lk.owner && last.lock_type == lk.lock_type {
                let last_end = last.interval.end.unwrap_or(i64::MAX);
                if lk.interval.start <= last_end {
                    last.interval.end = match (last.interval.end, lk.interval.end) {
                        (None, _) | (_, None) => None,
                        (Some(a), Some(b)) => Some(a.max(b)),
                    };
                    continue;
                }
            }
        }
        merged.push(lk);
    }

    *locks = merged;
}

// ---------------------------------------------------------------------------
// Interval normalisation
// ---------------------------------------------------------------------------

/// Convert a `flock64` + position info into an absolute `[start, end)` interval.
///
/// `file_pos` is the current file offset (used for `SEEK_CUR`).
/// `file_size` is the current file size (used for `SEEK_END`).
pub fn normalize_flock(
    flock: &flock64,
    file_pos: i64,
    file_size: i64,
) -> AxResult<(i16, LockInterval)> {
    let base: i64 = match flock.l_whence as u32 {
        linux_raw_sys::general::SEEK_SET => 0,
        linux_raw_sys::general::SEEK_CUR => file_pos,
        linux_raw_sys::general::SEEK_END => file_size,
        _ => return Err(AxError::InvalidInput),
    };

    let l_type = flock.l_type as i16;
    if l_type != F_RDLCK as i16 && l_type != F_WRLCK as i16 && l_type != F_UNLCK as i16 {
        return Err(AxError::InvalidInput);
    }

    let abs_start = base
        .checked_add(flock.l_start)
        .ok_or(AxError::InvalidInput)?;

    if abs_start < 0 {
        return Err(AxError::InvalidInput);
    }

    let interval = if flock.l_len > 0 {
        let end = abs_start
            .checked_add(flock.l_len)
            .ok_or(AxError::InvalidInput)?;
        LockInterval {
            start: abs_start,
            end: Some(end),
        }
    } else if flock.l_len == 0 {
        // Lock to EOF (infinite)
        LockInterval {
            start: abs_start,
            end: None,
        }
    } else {
        // l_len < 0: lock [start + l_len, start)
        let new_start = abs_start
            .checked_add(flock.l_len)
            .ok_or(AxError::InvalidInput)?;
        if new_start < 0 {
            return Err(AxError::InvalidInput);
        }
        LockInterval {
            start: new_start,
            end: Some(abs_start),
        }
    };

    Ok((l_type, interval))
}

// ---------------------------------------------------------------------------
// Deadlock graph (global, cross-inode)
// ---------------------------------------------------------------------------

/// Global wait-for graph: maps a POSIX owner (by Weak::as_ptr identity) to
/// the set of owners it is currently blocked by.  Used for cycle detection
/// across multiple inodes.
///
/// Key = `Weak::as_ptr(owner) as usize`, Value = `Vec<usize>` of blocker ids.
static DEADLOCK_GRAPH: spin::Lazy<Mutex<BTreeMap<usize, Vec<usize>>>> =
    spin::Lazy::new(|| Mutex::new(BTreeMap::new()));

fn owner_id(owner: &PosixOwner) -> usize {
    Weak::as_ptr(owner) as usize
}

/// Record that `waiter` is blocked by every owner in `blockers`.
fn add_wait_edges(waiter: &PosixOwner, blockers: &[usize]) {
    let mut graph = DEADLOCK_GRAPH.lock();
    graph.insert(owner_id(waiter), blockers.to_vec());
}

/// Remove all outgoing wait edges from `waiter`.
fn remove_wait_edges(waiter: &PosixOwner) {
    DEADLOCK_GRAPH.lock().remove(&owner_id(waiter));
}

/// Check whether adding a wait from `waiter` → `blockers` would create a
/// cycle.  Uses DFS from each blocker; if any path reaches `waiter`, a
/// deadlock exists.
fn would_deadlock(waiter: &PosixOwner, blockers: &[usize]) -> bool {
    let graph = DEADLOCK_GRAPH.lock();
    let target = owner_id(waiter);
    let mut visited = BTreeSet::new();

    fn dfs(
        graph: &BTreeMap<usize, Vec<usize>>,
        visited: &mut BTreeSet<usize>,
        current: usize,
        target: usize,
    ) -> bool {
        if current == target {
            return true;
        }
        if !visited.insert(current) {
            return false;
        }
        if let Some(blockers) = graph.get(&current) {
            for &b in blockers {
                if dfs(graph, visited, b, target) {
                    return true;
                }
            }
        }
        false
    }

    for &blocker in blockers {
        if dfs(&graph, &mut visited, blocker, target) {
            return true;
        }
        visited.clear();
    }

    false
}

// ---------------------------------------------------------------------------
// Public API: F_GETLK
// ---------------------------------------------------------------------------

/// Handle `F_GETLK`: search for a conflicting lock and write it back to
/// userspace.  The input `flock` describes the lock the caller *would like*
/// to acquire.  On success the `flock` structure is updated in place.
///
/// Returns `Ok(())`; the user buffer has been updated.
pub fn posix_getlk(
    key: InodeKey,
    owner: &PosixOwner,
    flock: &mut flock64,
    file_pos: i64,
    file_size: i64,
) -> AxResult {
    // Normalise the hypothetical lock
    let (want_type, interval) = normalize_flock(flock, file_pos, file_size)?;
    let want_lt = lock_type_from_i16(want_type).ok_or(AxError::InvalidInput)?;

    let state = get_or_create_state(key);
    let guard = state.lock();

    // Check POSIX locks for conflict (ignore same-owner)
    if let Some(conflict) = find_posix_conflict(&guard.posix, owner, want_lt, &interval) {
        flock.l_type = lock_type_to_i16(conflict.lock_type) as _;
        flock.l_whence = linux_raw_sys::general::SEEK_SET as _;
        flock.l_start = conflict.interval.start;
        flock.l_len = match conflict.interval.end {
            Some(end) => end - conflict.interval.start,
            None => 0,
        };
        flock.l_pid = conflict.pid as _;
        return Ok(());
    }

    // No POSIX conflict → return F_UNLCK (only l_type is set; POSIX does not
    // require clearing other fields to zero, but LTP checks may expect them).
    // Linux leaves l_pid/l_start/l_len unchanged when returning F_UNLCK.
    flock.l_type = F_UNLCK as _;
    Ok(())
}

/// Handle `F_OFD_GETLK`.
pub fn ofd_getlk(
    key: InodeKey,
    ofd_owner: OfdOwner,
    flock: &mut flock64,
    file_pos: i64,
    file_size: i64,
) -> AxResult {
    let (want_type, interval) = normalize_flock(flock, file_pos, file_size)?;
    let want_lt = lock_type_from_i16(want_type).ok_or(AxError::InvalidInput)?;

    let state = get_or_create_state(key);
    let guard = state.lock();

    // Check POSIX locks (OFD vs POSIX: they ALWAYS conflict because they
    // have different owner types)
    for lk in &guard.posix {
        if interval.overlaps(&lk.interval) && types_conflict(want_lt, lk.lock_type) {
            // POSIX lock conflicts with OFD request
            flock.l_type = lock_type_to_i16(lk.lock_type) as _;
            flock.l_whence = linux_raw_sys::general::SEEK_SET as _;
            flock.l_start = lk.interval.start;
            flock.l_len = match lk.interval.end {
                Some(end) => end - lk.interval.start,
                None => 0,
            };
            flock.l_pid = lk.pid as _;
            return Ok(());
        }
    }

    // Check OFD locks
    if let Some(conflict) = find_ofd_conflict(&guard.ofd, ofd_owner, want_lt, &interval) {
        flock.l_type = lock_type_to_i16(conflict.lock_type) as _;
        flock.l_whence = linux_raw_sys::general::SEEK_SET as _;
        flock.l_start = conflict.interval.start;
        flock.l_len = match conflict.interval.end {
            Some(end) => end - conflict.interval.start,
            None => 0,
        };
        // Linux convention: OFD conflict l_pid is -1
        flock.l_pid = -1i32 as _;
        return Ok(());
    }

    flock.l_type = F_UNLCK as _;
    Ok(())
}

// ---------------------------------------------------------------------------
// Public API: F_SETLK (non-blocking)
// ---------------------------------------------------------------------------

/// Handle `F_SETLK` (POSIX, non-blocking).  Returns `EAGAIN` on conflict.
pub fn posix_setlk(
    key: InodeKey,
    owner: &PosixOwner,
    pid: u32,
    flock: &flock64,
    file_pos: i64,
    file_size: i64,
) -> AxResult {
    let (l_type, interval) = normalize_flock(flock, file_pos, file_size)?;

    if l_type == F_UNLCK as i16 {
        // Unlock
        let state = get_or_create_state(key);
        let mut guard = state.lock();
        apply_posix_lock(&mut guard.posix, owner, pid, None, &interval);
        guard.wake_all_waiters();
        return Ok(());
    }

    let lock_type = lock_type_from_i16(l_type).ok_or(AxError::InvalidInput)?;
    let state = get_or_create_state(key);
    let mut guard = state.lock();

    if find_posix_conflict(&guard.posix, owner, lock_type, &interval).is_some() {
        return Err(AxError::WouldBlock);
    }

    apply_posix_lock(
        &mut guard.posix,
        owner,
        pid,
        Some(lock_type),
        &interval,
    );
    // Wake any blocked waiters that might now be able to acquire their lock.
    guard.wake_all_waiters();
    Ok(())
}

/// Handle `F_OFD_SETLK` (non-blocking).
pub fn ofd_setlk(
    key: InodeKey,
    ofd_owner: OfdOwner,
    flock: &flock64,
    file_pos: i64,
    file_size: i64,
) -> AxResult {
    let (l_type, interval) = normalize_flock(flock, file_pos, file_size)?;

    if l_type == F_UNLCK as i16 {
        let state = get_or_create_state(key);
        let mut guard = state.lock();
        apply_ofd_lock(&mut guard.ofd, ofd_owner, None, &interval);
        guard.wake_all_waiters();
        return Ok(());
    }

    let lock_type = lock_type_from_i16(l_type).ok_or(AxError::InvalidInput)?;
    let state = get_or_create_state(key);
    let mut guard = state.lock();

    // OFD vs POSIX: different owner domains always conflict.
    for lk in &guard.posix {
        if interval.overlaps(&lk.interval) && types_conflict(lock_type, lk.lock_type) {
            return Err(AxError::WouldBlock);
        }
    }
    if find_ofd_conflict(&guard.ofd, ofd_owner, lock_type, &interval).is_some() {
        return Err(AxError::WouldBlock);
    }

    apply_ofd_lock(&mut guard.ofd, ofd_owner, Some(lock_type), &interval);
    guard.wake_all_waiters();
    Ok(())
}

// ---------------------------------------------------------------------------
// Public API: F_SETLKW (blocking, with deadlock detection)
// ---------------------------------------------------------------------------

/// Handle `F_SETLKW` (POSIX, blocking).  Waits for conflicting locks to be
/// released, with deadlock detection.
pub fn posix_setlkw(
    key: InodeKey,
    owner: &PosixOwner,
    pid: u32,
    flock: &flock64,
    file_pos: i64,
    file_size: i64,
) -> AxResult {
    let (l_type, interval) = normalize_flock(flock, file_pos, file_size)?;

    if l_type == F_UNLCK as i16 {
        // Unlock is always non-blocking.
        return posix_setlk(key, owner, pid, flock, file_pos, file_size);
    }

    let lock_type = lock_type_from_i16(l_type).ok_or(AxError::InvalidInput)?;
    let state = get_or_create_state(key);

    loop {
        // Compute blockers and check deadlock under the inode lock.
        let blockers: Vec<usize> = {
            let mut guard = state.lock();
            if find_posix_conflict(&guard.posix, owner, lock_type, &interval)
                .is_some()
            {
                // Collect all blocking owners (ID form).
                let mut ids = Vec::new();
                for lk in &guard.posix {
                    if posix_owner_eq(&lk.owner, owner) {
                        continue;
                    }
                    if interval.overlaps(&lk.interval)
                        && types_conflict(lock_type, lk.lock_type)
                    {
                        ids.push(owner_id(&lk.owner));
                    }
                }
                ids.sort();
                ids.dedup();
                ids
            } else {
                // No conflict → install and return.
                apply_posix_lock(
                    &mut guard.posix,
                    owner,
                    pid,
                    Some(lock_type),
                    &interval,
                );
                guard.wake_all_waiters();
                remove_wait_edges(owner);
                return Ok(());
            }
        };

        // Deadlock detection: check all blockers for cycles.
        if would_deadlock(owner, &blockers) {
            remove_wait_edges(owner);
            return Err(AxError::from(LinuxError::EDEADLK));
        }

        // Record the wait dependency in the global graph.
        add_wait_edges(owner, &blockers);

        // Block until woken.  The poll_fn atomically re-checks the condition
        // under the lock before pushing the waker, preventing lost wakeups.
        let result = axtask::future::block_on(
            axtask::future::interruptible(core::future::poll_fn(|cx| {
                let guard = state.lock();
                if find_posix_conflict(&guard.posix, owner, lock_type, &interval)
                    .is_none()
                {
                    core::task::Poll::Ready(Ok(()))
                } else {
                    guard.posix_wq.push(cx.waker().clone());
                    core::task::Poll::Pending
                }
            })),
        );

        match result {
            Ok(Ok(())) => {
                // Woken — refresh edges and re-check in next loop iteration.
                continue;
            }
            Ok(Err(e)) => {
                remove_wait_edges(owner);
                return Err(e);
            }
            Err(_interrupted) => {
                remove_wait_edges(owner);
                return Err(AxError::Interrupted);
            }
        }
    }
}

/// Handle `F_OFD_SETLKW` (blocking).
pub fn ofd_setlkw(
    key: InodeKey,
    ofd_owner: OfdOwner,
    flock: &flock64,
    file_pos: i64,
    file_size: i64,
) -> AxResult {
    let (l_type, interval) = normalize_flock(flock, file_pos, file_size)?;

    if l_type == F_UNLCK as i16 {
        return ofd_setlk(key, ofd_owner, flock, file_pos, file_size);
    }

    let lock_type = lock_type_from_i16(l_type).ok_or(AxError::InvalidInput)?;
    let state = get_or_create_state(key);

    loop {
        {
            let mut guard = state.lock();

            // Check POSIX conflicts (always different owner domain)
            let mut has_conflict = false;
            for lk in &guard.posix {
                if interval.overlaps(&lk.interval)
                    && types_conflict(lock_type, lk.lock_type)
                {
                    has_conflict = true;
                    break;
                }
            }
            if !has_conflict {
                has_conflict =
                    find_ofd_conflict(&guard.ofd, ofd_owner, lock_type, &interval)
                        .is_some();
            }

            if !has_conflict {
                apply_ofd_lock(
                    &mut guard.ofd,
                    ofd_owner,
                    Some(lock_type),
                    &interval,
                );
                guard.wake_all_waiters();
                return Ok(());
            }
        }

        let result = axtask::future::block_on(
            axtask::future::interruptible(core::future::poll_fn(|cx| {
                let guard = state.lock();
                let mut has_conflict = false;
                for lk in &guard.posix {
                    if interval.overlaps(&lk.interval)
                        && types_conflict(lock_type, lk.lock_type)
                    {
                        has_conflict = true;
                        break;
                    }
                }
                if !has_conflict {
                    has_conflict = find_ofd_conflict(
                        &guard.ofd,
                        ofd_owner,
                        lock_type,
                        &interval,
                    )
                    .is_some();
                }
                if !has_conflict {
                    core::task::Poll::Ready(Ok(()))
                } else {
                    guard.ofd_wq.push(cx.waker().clone());
                    core::task::Poll::Pending
                }
            })),
        );

        match result {
            Ok(Ok(())) => continue,
            Ok(Err(e)) => return Err(e),
            Err(_interrupted) => return Err(AxError::Interrupted),
        }
    }
}

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

/// Release all POSIX locks held by `owner` on the inode `key`.
/// Called when the process closes *any* fd referencing the inode.
pub fn release_posix_locks_on_inode(key: InodeKey, owner: &PosixOwner) {
    let state = {
        let table = LOCK_TABLE.read();
        table.get(&key).cloned()
    };
    let Some(state) = state else { return };
    let mut guard = state.lock();
    guard.posix.retain(|lk| !posix_owner_eq(&lk.owner, owner));
    guard.wake_all_waiters();
}

/// Release all POSIX locks held by `owner` across **all** inodes.
/// Called on process exit.
pub fn release_all_posix_locks(owner: &PosixOwner) {
    // Collect all states we need to modify (without holding the table read lock
    // across lock state modification + potential cleanup).
    let states: Vec<(InodeKey, Arc<Mutex<InodeLockState>>)> = {
        let table = LOCK_TABLE.read();
        table.iter().map(|(k, v)| (*k, v.clone())).collect()
    };
    for (key, state) in states {
        let mut guard = state.lock();
        let len_before = guard.posix.len();
        guard.posix.retain(|lk| !posix_owner_eq(&lk.owner, owner));
        if guard.posix.len() < len_before {
            guard.wake_all_waiters();
        }
    }
}

/// Release all OFD locks owned by `ofd_owner` on the inode `key`.
/// Called when the last fd referencing an open file description is closed.
pub fn release_ofd_locks_on_inode(key: InodeKey, ofd_owner: OfdOwner) {
    let state = {
        let table = LOCK_TABLE.read();
        table.get(&key).cloned()
    };
    let Some(state) = state else { return };
    let mut guard = state.lock();
    guard.ofd.retain(|lk| lk.owner != ofd_owner);
    guard.wake_all_waiters();
}

// ---------------------------------------------------------------------------
// Access mode check for lock operations
// ---------------------------------------------------------------------------

/// POSIX: a read lock requires the fd to be opened for reading;
/// a write lock requires the fd to be opened for writing.
pub fn check_lock_access_mode(
    fd_access_mode: u32,
    lock_type: i16,
) -> AxResult {
    use linux_raw_sys::general::{O_ACCMODE, O_RDONLY, O_RDWR, O_WRONLY};
    match lock_type as u32 {
        F_RDLCK => {
            let acc = fd_access_mode & O_ACCMODE;
            if acc != O_RDONLY && acc != O_RDWR {
                return Err(AxError::BadFileDescriptor);
            }
        }
        F_WRLCK => {
            let acc = fd_access_mode & O_ACCMODE;
            if acc != O_WRONLY && acc != O_RDWR {
                return Err(AxError::BadFileDescriptor);
            }
        }
        F_UNLCK => {} // unlock always allowed
        _ => return Err(AxError::InvalidInput),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Lease access mode check
// ---------------------------------------------------------------------------

/// `F_SETLEASE` with `F_RDLCK` requires the fd to be opened read-only
/// (`O_RDONLY`).  `O_RDWR` or `O_WRONLY` should return `EAGAIN`.
pub fn check_lease_read_access(fd_access_mode: u32, lease_type: u32) -> AxResult {
    use linux_raw_sys::general::{O_ACCMODE, O_RDONLY};
    if lease_type == F_RDLCK {
        let acc = fd_access_mode & O_ACCMODE;
        if acc != O_RDONLY {
            return Err(AxError::WouldBlock);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// F_SETOWN_EX / F_GETOWN_EX
// ---------------------------------------------------------------------------

/// Owner-ex information stored per open-file-description (NOT per inode).
/// Shared by dup'd fds; independent open calls get separate copies.
#[derive(Debug, Clone, Default)]
pub struct FileOwnerEx {
    pub owner_type: i32, // F_OWNER_PID / F_OWNER_PGRP / F_OWNER_TID
    pub pid: i32,
}
