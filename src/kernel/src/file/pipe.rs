use alloc::{
    borrow::Cow,
    collections::BTreeMap,
    format,
    string::{String, ToString},
    sync::{Arc, Weak},
};
use core::{
    mem,
    sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering},
    task::Context,
};

use axerrno::{AxError, AxResult, LinuxError};
use axfs_ng_vfs::Location;
use axpoll::{IoEvents, PollSet, Pollable};
use axsync::Mutex;
use axtask::{
    current,
    future::{block_on, poll_io},
};
use linux_raw_sys::{
    general::{O_RDWR, O_WRONLY, S_IFIFO},
    ioctl::FIONREAD,
};
use memory_addr::PAGE_SIZE_4K;
use ringbuf::{
    HeapRb,
    traits::{Consumer, Observer, Producer},
};
use starry_signal::{SignalInfo, Signo};
use starry_vm::VmMutPtr;
use spin::{Lazy, Mutex as SpinMutex};

use super::{FileLike, Kstat, fs::metadata_to_kstat};
use crate::{
    file::{FileOwnerEx, IoDst, IoSrc},
    task::{AsThread, send_signal_to_process, send_signal_to_thread},
};

const RING_BUFFER_INIT_SIZE: usize = 65536; // 64 KiB

/// Maximum pipe capacity for unprivileged users (1 MiB).
/// Exposed at /proc/sys/fs/pipe-max-size — writable via procfs.
pub static PIPE_MAX_SIZE: AtomicUsize = AtomicUsize::new(1048576);
type FifoKey = (u64, u64);

static NAMED_PIPES: Lazy<SpinMutex<BTreeMap<FifoKey, Weak<Shared>>>> =
    Lazy::new(|| SpinMutex::new(BTreeMap::new()));

struct Shared {
    buffer: Mutex<HeapRb<u8>>,
    poll_rx: PollSet,
    poll_tx: PollSet,
    poll_close: PollSet,
    readers: AtomicUsize,
    writers: AtomicUsize,
    /// F_SETOWN_EX / F_SETSIG state for the read-end fd (async notification).
    async_owner: Mutex<Option<FileOwnerEx>>,
    async_signal: AtomicI32,
    /// Whether the read-end fd has FASYNC set (gates signal delivery).
    read_async_enabled: AtomicBool,
}

impl Shared {
    fn new() -> Arc<Self> {
        // Default pipe capacity is 64 KiB.  If the admin has lowered
        // /proc/sys/fs/pipe-max-size below that, unprivileged callers get
        // the smaller limit; privileged (root) callers still get 64 KiB.
        // Use the current process's effective uid directly rather than going
        // through VfsCredentials, to stay in sync with setuid/seteuid/setresuid.
        let init_size = {
            let max = PIPE_MAX_SIZE.load(Ordering::Acquire);
            if max < RING_BUFFER_INIT_SIZE
                && current().as_thread().proc_data.ids().1 != 0
            {
                max
            } else {
                RING_BUFFER_INIT_SIZE
            }
        };
        Arc::new(Self {
            buffer: Mutex::new(HeapRb::new(init_size)),
            poll_rx: PollSet::new(),
            poll_tx: PollSet::new(),
            poll_close: PollSet::new(),
            readers: AtomicUsize::new(0),
            writers: AtomicUsize::new(0),
            async_owner: Mutex::new(None),
            async_signal: AtomicI32::new(0),
            read_async_enabled: AtomicBool::new(false),
        })
    }

    fn has_readers(&self) -> bool {
        self.readers.load(Ordering::Acquire) > 0
    }

    fn has_writers(&self) -> bool {
        self.writers.load(Ordering::Acquire) > 0
    }

    /// Send async signal if the read-end has FASYNC set and data became
    /// available.  Called from the write path after `poll_rx.wake()`.
    fn send_async_signal_if_needed(&self) {
        // Gate: only send if the read-end set FASYNC.
        if !self.read_async_enabled.load(Ordering::Acquire) {
            return;
        }
        // Copy owner/signal state, then release the lock before sending the
        // signal to avoid lock ordering issues (async_owner mutex vs process
        // table locks in send_signal_to_process).
        let (oe_type, pid, signo) = {
            let owner = self.async_owner.lock();
            let oe = match *owner {
                Some(ref oe) => oe.clone(),
                None => return,
            };
            let sig = self.async_signal.load(Ordering::Acquire);
            let signo = if sig == 0 {
                Signo::SIGIO
            } else {
                Signo::from_repr(sig as u8).unwrap_or(Signo::SIGIO)
            };
            (oe.owner_type, oe.pid, signo)
        };
        // Lock released — safe to call signal delivery.
        match oe_type as u32 {
            linux_raw_sys::general::F_OWNER_PID => {
                let _ = send_signal_to_process(
                    pid as _,
                    Some(SignalInfo::new_kernel(signo)),
                );
            }
            linux_raw_sys::general::F_OWNER_PGRP => {
                let _ = crate::task::send_signal_to_process_group(
                    pid as _,
                    Some(SignalInfo::new_kernel(signo)),
                );
            }
            linux_raw_sys::general::F_OWNER_TID => {
                let _ = send_signal_to_thread(
                    None,
                    pid as _,
                    Some(SignalInfo::new_kernel(signo)),
                );
            }
            _ => {}
        }
    }
}

pub struct Pipe {
    read_side: bool,
    shared: Arc<Shared>,
    non_blocking: AtomicBool,
    async_flag: AtomicBool, // FASYNC
}
impl Drop for Pipe {
    fn drop(&mut self) {
        if self.read_side {
            self.shared.readers.fetch_sub(1, Ordering::AcqRel);
            self.shared.poll_tx.wake();
        } else {
            self.shared.writers.fetch_sub(1, Ordering::AcqRel);
            self.shared.poll_rx.wake();
        }
        self.shared.poll_close.wake();
    }
}

impl Pipe {
    pub fn new() -> (Pipe, Pipe) {
        let shared = Shared::new();
        shared.readers.store(1, Ordering::Release);
        shared.writers.store(1, Ordering::Release);
        let read_end = Pipe {
            read_side: true,
            shared: shared.clone(),
            non_blocking: AtomicBool::new(false),
            async_flag: AtomicBool::new(false),
        };
        let write_end = Pipe {
            read_side: false,
            shared,
            non_blocking: AtomicBool::new(false),
            async_flag: AtomicBool::new(false),
        };
        (read_end, write_end)
    }

    pub const fn is_read(&self) -> bool {
        self.read_side
    }

    pub const fn is_write(&self) -> bool {
        !self.read_side
    }

    pub fn closed(&self) -> bool {
        if self.read_side {
            !self.shared.has_writers()
        } else {
            !self.shared.has_readers()
        }
    }

    pub fn capacity(&self) -> usize {
        self.shared.buffer.lock().capacity().get()
    }

    pub fn resize(&self, new_size: usize) -> AxResult<()> {
        // Refuse zero-sized pipes.
        if new_size == 0 {
            return Err(AxError::InvalidInput);
        }
        // Refuse sizes that would overflow isize (Vec::with_capacity limit).
        if new_size > isize::MAX as usize {
            return Err(AxError::InvalidInput);
        }

        let new_size = new_size.div_ceil(PAGE_SIZE_4K).max(1) * PAGE_SIZE_4K;

        // Double-check after rounding: must still be allocatable.
        if new_size > isize::MAX as usize {
            return Err(AxError::InvalidInput);
        }

        let mut buffer = self.shared.buffer.lock();
        if new_size == buffer.capacity().get() {
            return Ok(());
        }
        if new_size < buffer.occupied_len() {
            return Err(AxError::ResourceBusy);
        }
        let old_buffer = mem::replace(&mut *buffer, HeapRb::new(new_size));
        let (left, right) = old_buffer.as_slices();
        buffer.push_slice(left);
        buffer.push_slice(right);
        Ok(())
    }

    /// Shared pointer for async owner access (F_GETOWN_EX / F_SETOWN_EX).
    pub fn shared(&self) -> &Arc<Shared> {
        &self.shared
    }

    /// Accessor for F_GETOWN_EX / F_SETOWN_EX state.
    pub fn async_owner(&self) -> &Mutex<Option<FileOwnerEx>> {
        &self.shared.async_owner
    }

    /// Accessor for F_GETSIG / F_SETSIG state.
    pub fn async_signal(&self) -> &AtomicI32 {
        &self.shared.async_signal
    }
}

pub struct NamedPipe {
    readable: bool,
    writable: bool,
    shared: Arc<Shared>,
    non_blocking: AtomicBool,
    path: String,
    stat: Kstat,
    key: FifoKey,
    /// Original filesystem location, preserved so that fd-based xattr
    /// operations can reach the underlying inode.
    location: Location,
}

impl Drop for NamedPipe {
    fn drop(&mut self) {
        if self.readable {
            self.shared.readers.fetch_sub(1, Ordering::AcqRel);
            self.shared.poll_tx.wake();
        }
        if self.writable {
            self.shared.writers.fetch_sub(1, Ordering::AcqRel);
            self.shared.poll_rx.wake();
        }
        if !self.shared.has_readers() && !self.shared.has_writers() {
            let mut named_pipes = NAMED_PIPES.lock();
            let remove = named_pipes
                .get(&self.key)
                .and_then(Weak::upgrade)
                .map_or(false, |shared| Arc::ptr_eq(&shared, &self.shared));
            if remove {
                named_pipes.remove(&self.key);
            }
        }
        self.shared.poll_close.wake();
    }
}

impl NamedPipe {
    pub fn open(
        location: &Location,
        readable: bool,
        writable: bool,
        nonblocking: bool,
    ) -> AxResult<Self> {
        if !readable && !writable {
            return Err(AxError::InvalidInput);
        }

        let metadata = location.metadata()?;
        let key = (metadata.device, metadata.inode);
        let shared = {
            let mut named_pipes = NAMED_PIPES.lock();
            if let Some(shared) = named_pipes.get(&key).and_then(Weak::upgrade) {
                shared
            } else {
                let shared = Shared::new();
                named_pipes.insert(key, Arc::downgrade(&shared));
                shared
            }
        };

        if writable && !readable && nonblocking && !shared.has_readers() {
            return Err(AxError::from(LinuxError::ENXIO));
        }

        if readable {
            shared.readers.fetch_add(1, Ordering::AcqRel);
        }
        if writable {
            shared.writers.fetch_add(1, Ordering::AcqRel);
        }

        Ok(Self {
            readable,
            writable,
            shared,
            non_blocking: AtomicBool::new(nonblocking),
            path: location
                .absolute_path()
                .map(|path| path.to_string())
                .unwrap_or_else(|_| format!("fifo:[{}]", key.1)),
            stat: metadata_to_kstat(&metadata),
            key,
            location: location.clone(),
        })
    }

    fn read_closed(&self) -> bool {
        !self.shared.has_writers()
    }

    fn write_closed(&self) -> bool {
        !self.shared.has_readers()
    }

    /// Returns the filesystem location of this named pipe so that fd-based
    /// xattr operations can resolve the underlying inode.
    pub fn location(&self) -> &Location {
        &self.location
    }
}

fn raise_pipe() {
    let curr = current();
    send_signal_to_process(
        curr.as_thread().proc_data.proc.pid(),
        Some(SignalInfo::new_kernel(Signo::SIGPIPE)),
    )
    .expect("Failed to send SIGPIPE");
}

impl FileLike for Pipe {
    fn read(&self, dst: &mut IoDst) -> AxResult<usize> {
        if !self.is_read() {
            return Err(AxError::BadFileDescriptor);
        }
        if dst.is_full() {
            return Ok(0);
        }

        let mut try_read = || {
            let read = {
                let cons = self.shared.buffer.lock();
                let (left, right) = cons.as_slices();
                let mut count = dst.write(left)?;
                if count >= left.len() {
                    count += dst.write(right)?;
                }
                unsafe { cons.advance_read_index(count) };
                count
            };
            if read > 0 {
                self.shared.poll_tx.wake();
                Ok(read)
            } else if self.closed() {
                Ok(0)
            } else {
                Err(AxError::WouldBlock)
            }
        };

        match try_read() {
            Ok(read) => Ok(read),
            Err(AxError::WouldBlock) => {
                block_on(poll_io(self, IoEvents::IN, self.nonblocking(), try_read))
            }
            Err(err) => Err(err),
        }
    }

    fn write(&self, src: &mut IoSrc) -> AxResult<usize> {
        if !self.is_write() {
            return Err(AxError::BadFileDescriptor);
        }
        let size = src.remaining();
        if size == 0 {
            return Ok(0);
        }

        let mut total_written = 0;

        let mut try_write = || {
            if self.closed() {
                raise_pipe();
                return Err(AxError::BrokenPipe);
            }

            let written = {
                let mut prod = self.shared.buffer.lock();
                let (left, right) = prod.vacant_slices_mut();
                let mut count = src.read(unsafe { left.assume_init_mut() })?;
                if count >= left.len() {
                    count += src.read(unsafe { right.assume_init_mut() })?;
                }
                unsafe { prod.advance_write_index(count) };
                count
            };
            if written > 0 {
                self.shared.poll_rx.wake();
                self.shared.send_async_signal_if_needed();
                total_written += written;
                if total_written == size || self.nonblocking() {
                    return Ok(total_written);
                }
            }
            Err(AxError::WouldBlock)
        };

        match try_write() {
            Ok(written) => Ok(written),
            Err(AxError::WouldBlock) => {
                block_on(poll_io(self, IoEvents::OUT, self.nonblocking(), try_write))
            }
            Err(err) => Err(err),
        }
    }

    fn stat(&self) -> AxResult<Kstat> {
        Ok(Kstat {
            mode: S_IFIFO | if self.is_read() { 0o444 } else { 0o222 },
            ..Default::default()
        })
    }

    fn path(&self) -> Cow<'_, str> {
        format!("pipe:[{}]", self as *const _ as usize).into()
    }

    fn set_nonblocking(&self, nonblocking: bool) -> AxResult {
        self.non_blocking.store(nonblocking, Ordering::Release);
        Ok(())
    }

    fn nonblocking(&self) -> bool {
        self.non_blocking.load(Ordering::Acquire)
    }

    fn access_mode(&self) -> u32 {
        if self.is_write() { O_WRONLY } else { 0 }
    }

    fn status_flags(&self) -> u32 {
        let mut flags = 0;
        if self.nonblocking() {
            flags |= linux_raw_sys::general::O_NONBLOCK;
        }
        if self.async_flag.load(Ordering::Acquire) {
            flags |= linux_raw_sys::general::FASYNC;
        }
        flags
    }

    fn set_status_flags(&self, flags: u32) -> AxResult {
        use linux_raw_sys::general::{FASYNC, O_NONBLOCK};
        // The syscall layer already masked the incoming flags to the mutable
        // subset, so we only extract the bits we support.
        self.set_nonblocking(flags & O_NONBLOCK != 0);
        let async_enabled = flags & FASYNC != 0;
        self.async_flag.store(async_enabled, Ordering::Release);
        // Update the shared read-end FASYNC state so that the write path can
        // see whether the read-end wants async notification.
        if self.read_side {
            self.shared
                .read_async_enabled
                .store(async_enabled, Ordering::Release);
        }
        Ok(())
    }

    fn ioctl(&self, cmd: u32, arg: usize) -> AxResult<usize> {
        match cmd {
            FIONREAD => {
                (arg as *mut u32).vm_write(self.shared.buffer.lock().occupied_len() as u32)?;
                Ok(0)
            }
            _ => Err(AxError::NotATty),
        }
    }
}

impl FileLike for NamedPipe {
    fn read(&self, dst: &mut IoDst) -> AxResult<usize> {
        if !self.readable {
            return Err(AxError::BadFileDescriptor);
        }
        if dst.is_full() {
            return Ok(0);
        }

        let mut try_read = || {
            let read = {
                let cons = self.shared.buffer.lock();
                let (left, right) = cons.as_slices();
                let mut count = dst.write(left)?;
                if count >= left.len() {
                    count += dst.write(right)?;
                }
                unsafe { cons.advance_read_index(count) };
                count
            };
            if read > 0 {
                self.shared.poll_tx.wake();
                Ok(read)
            } else if self.read_closed() {
                Ok(0)
            } else {
                Err(AxError::WouldBlock)
            }
        };

        match try_read() {
            Ok(read) => Ok(read),
            Err(AxError::WouldBlock) => {
                block_on(poll_io(self, IoEvents::IN, self.nonblocking(), try_read))
            }
            Err(err) => Err(err),
        }
    }

    fn write(&self, src: &mut IoSrc) -> AxResult<usize> {
        if !self.writable {
            return Err(AxError::BadFileDescriptor);
        }
        let size = src.remaining();
        if size == 0 {
            return Ok(0);
        }

        let mut total_written = 0;

        let mut try_write = || {
            if self.write_closed() {
                raise_pipe();
                return Err(AxError::BrokenPipe);
            }

            let written = {
                let mut prod = self.shared.buffer.lock();
                let (left, right) = prod.vacant_slices_mut();
                let mut count = src.read(unsafe { left.assume_init_mut() })?;
                if count >= left.len() {
                    count += src.read(unsafe { right.assume_init_mut() })?;
                }
                unsafe { prod.advance_write_index(count) };
                count
            };
            if written > 0 {
                self.shared.poll_rx.wake();
                self.shared.send_async_signal_if_needed();
                total_written += written;
                if total_written == size || self.nonblocking() {
                    return Ok(total_written);
                }
            }
            Err(AxError::WouldBlock)
        };

        match try_write() {
            Ok(written) => Ok(written),
            Err(AxError::WouldBlock) => {
                block_on(poll_io(self, IoEvents::OUT, self.nonblocking(), try_write))
            }
            Err(err) => Err(err),
        }
    }

    fn stat(&self) -> AxResult<Kstat> {
        Ok(self.stat)
    }

    fn path(&self) -> Cow<'_, str> {
        self.path.as_str().into()
    }

    fn set_nonblocking(&self, nonblocking: bool) -> AxResult {
        self.non_blocking.store(nonblocking, Ordering::Release);
        Ok(())
    }

    fn nonblocking(&self) -> bool {
        self.non_blocking.load(Ordering::Acquire)
    }

    fn access_mode(&self) -> u32 {
        match (self.readable, self.writable) {
            (true, true) => O_RDWR,
            (false, true) => O_WRONLY,
            _ => 0,
        }
    }

    fn ioctl(&self, cmd: u32, arg: usize) -> AxResult<usize> {
        match cmd {
            FIONREAD => {
                (arg as *mut u32).vm_write(self.shared.buffer.lock().occupied_len() as u32)?;
                Ok(0)
            }
            _ => Err(AxError::NotATty),
        }
    }
}

impl Pollable for Pipe {
    fn poll(&self) -> IoEvents {
        let mut events = IoEvents::empty();
        let buf = self.shared.buffer.lock();
        if self.read_side {
            events.set(IoEvents::IN, buf.occupied_len() > 0);
            events.set(IoEvents::HUP, self.closed());
        } else {
            events.set(IoEvents::OUT, buf.vacant_len() > 0);
        }
        events
    }

    fn register(&self, context: &mut Context<'_>, events: IoEvents) {
        if events.contains(IoEvents::IN) {
            self.shared.poll_rx.register(context.waker());
        }
        if events.contains(IoEvents::OUT) {
            self.shared.poll_tx.register(context.waker());
        }
        self.shared.poll_close.register(context.waker());
    }
}

impl Pollable for NamedPipe {
    fn poll(&self) -> IoEvents {
        let mut events = IoEvents::empty();
        let buf = self.shared.buffer.lock();
        if self.readable {
            events.set(IoEvents::IN, buf.occupied_len() > 0);
            events.set(IoEvents::HUP, self.read_closed());
        }
        if self.writable {
            events.set(IoEvents::OUT, buf.vacant_len() > 0);
        }
        events
    }

    fn register(&self, context: &mut Context<'_>, events: IoEvents) {
        if events.contains(IoEvents::IN) {
            self.shared.poll_rx.register(context.waker());
        }
        if events.contains(IoEvents::OUT) {
            self.shared.poll_tx.register(context.waker());
        }
        self.shared.poll_close.register(context.waker());
    }
}
