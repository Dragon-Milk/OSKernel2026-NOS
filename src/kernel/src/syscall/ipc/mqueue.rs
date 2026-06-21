use alloc::{
    borrow::Cow,
    format,
    collections::{BTreeMap, VecDeque},
    string::String,
    sync::Arc,
    vec::Vec,
};
use core::{
    ffi::{c_char, c_int},
    sync::atomic::{AtomicUsize, Ordering},
    task::Context,
    time::Duration,
};

use axerrno::{AxError, AxResult};
use axpoll::{IoEvents, Pollable};
use axsync::Mutex;
use axtask::{future::{block_on, sleep}, yield_now};
use linux_raw_sys::{ctypes::c_long, general::*};

use crate::{
    file::{add_file_like, get_file_like, FileLike},
    mm::{nullable, vm_load_string, UserConstPtr, UserPtr},
    syscall::{sys_getegid, sys_geteuid, sys_kill},
    task::AsThread,
};

use super::has_ipc_permission;

const MQ_PRIO_MAX: u32 = 32768;
const DEFAULT_MAXMSG: c_long = 10;
const DEFAULT_MSGSIZE: c_long = 8192;
const NAME_MAX: usize = 255;
const SIGEV_NONE: i32 = 1;
const SIGUSR1_FALLBACK: i32 = 10;
const O_ACCMODE_MASK: u32 = 0b11;

pub static MQ_QUEUES_MAX: AtomicUsize = AtomicUsize::new(256);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MqAttr {
    mq_flags: c_long,
    mq_maxmsg: c_long,
    mq_msgsize: c_long,
    mq_curmsgs: c_long,
    reserved: [c_long; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Timespec {
    tv_sec: c_long,
    tv_nsec: c_long,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SigeventHeader {
    value: usize,
    signo: i32,
    notify: i32,
}

#[derive(Clone)]
struct Message {
    prio: u32,
    data: Vec<u8>,
}

#[derive(Clone, Copy)]
struct Notification {
    pid: i32,
    signo: i32,
}

struct QueueInner {
    messages: VecDeque<Message>,
    notification: Option<Notification>,
}

struct PosixQueue {
    name: String,
    mode: u32,
    uid: u32,
    gid: u32,
    maxmsg: usize,
    msgsize: usize,
    inner: Mutex<QueueInner>,
}

pub struct MqDescriptor {
    queue: Arc<PosixQueue>,
    flags: Mutex<u32>,
}

static MQ_MANAGER: Mutex<BTreeMap<String, Arc<PosixQueue>>> = Mutex::new(BTreeMap::new());

fn normalize_name(name: &str) -> AxResult<String> {
    if name.is_empty() || name == "/" {
        return Err(AxError::InvalidInput);
    }
    let trimmed = name.strip_prefix('/').unwrap_or(name);
    if trimmed.is_empty() {
        return Err(AxError::InvalidInput);
    }
    if trimmed.len() > NAME_MAX {
        return Err(AxError::NameTooLong);
    }
    if trimmed.contains('/') {
        return Err(AxError::PermissionDenied);
    }
    Ok(trimmed.into())
}

fn open_for_read(flags: u32) -> bool {
    !matches!(flags & O_ACCMODE_MASK, O_WRONLY)
}

fn open_for_write(flags: u32) -> bool {
    matches!(flags & O_ACCMODE_MASK, O_WRONLY) || matches!(flags & O_ACCMODE_MASK, O_RDWR)
}

fn queue_attr(queue: &PosixQueue, flags: u32) -> MqAttr {
    let inner = queue.inner.lock();
    MqAttr {
        mq_flags: if flags & O_NONBLOCK != 0 { O_NONBLOCK as c_long } else { 0 },
        mq_maxmsg: queue.maxmsg as c_long,
        mq_msgsize: queue.msgsize as c_long,
        mq_curmsgs: inner.messages.len() as c_long,
        reserved: [0; 4],
    }
}

fn validate_timeout(timeout: UserConstPtr<Timespec>) -> AxResult<()> {
    if let Some(ts) = nullable!(timeout.get_as_ref())? {
        if ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1_000_000_000 {
            return Err(AxError::InvalidInput);
        }
    }
    Ok(())
}

fn get_mq(fd: c_int) -> AxResult<Arc<MqDescriptor>> {
    get_file_like(fd)?
        .downcast_arc::<MqDescriptor>()
        .map_err(|_| AxError::BadFileDescriptor)
}

impl MqDescriptor {
    fn new(queue: Arc<PosixQueue>, flags: u32) -> Self {
        Self { queue, flags: Mutex::new(flags) }
    }
}

impl FileLike for MqDescriptor {
    fn path(&self) -> Cow<'_, str> {
        Cow::Owned(format!("mqueue:/{}", self.queue.name))
    }

    fn nonblocking(&self) -> bool {
        *self.flags.lock() & O_NONBLOCK != 0
    }

    fn set_nonblocking(&self, nonblocking: bool) -> AxResult {
        let mut flags = self.flags.lock();
        if nonblocking {
            *flags |= O_NONBLOCK;
        } else {
            *flags &= !(O_NONBLOCK as u32);
        }
        Ok(())
    }
}

impl Pollable for MqDescriptor {
    fn poll(&self) -> IoEvents {
        let inner = self.queue.inner.lock();
        let mut events = IoEvents::empty();
        if !inner.messages.is_empty() {
            events |= IoEvents::IN;
        }
        if inner.messages.len() < self.queue.maxmsg {
            events |= IoEvents::OUT;
        }
        events
    }

    fn register(&self, _context: &mut Context<'_>, _events: IoEvents) {}
}

pub fn sys_mq_open(
    name: *const c_char,
    oflag: c_int,
    mode: u32,
    attr: UserConstPtr<MqAttr>,
) -> AxResult<isize> {
    let name = normalize_name(&vm_load_string(name)?)?;
    let flags = oflag as u32;
    let current_uid = sys_geteuid()? as u32;
    let current_gid = sys_getegid()? as u32;
    let mut manager = MQ_MANAGER.lock();

    if let Some(queue) = manager.get(&name) {
        if flags & O_CREAT != 0 && flags & O_EXCL != 0 {
            return Err(AxError::AlreadyExists);
        }
        let perm = super::IpcPerm {
            key: 0,
            uid: queue.uid,
            gid: queue.gid,
            cuid: queue.uid,
            cgid: queue.gid,
            mode: queue.mode,
            seq: 0,
            pad: 0,
            unused0: 0,
            unused1: 0,
        };
        if open_for_read(flags) && !has_ipc_permission(&perm, current_uid, current_gid, false) {
            return Err(AxError::PermissionDenied);
        }
        if open_for_write(flags) && !has_ipc_permission(&perm, current_uid, current_gid, true) {
            return Err(AxError::PermissionDenied);
        }
        let fd = add_file_like(Arc::new(MqDescriptor::new(queue.clone(), flags)), flags & O_CLOEXEC != 0)?;
        return Ok(fd as isize);
    }

    if flags & O_CREAT == 0 {
        return Err(AxError::NotFound);
    }
    if manager.len() >= MQ_QUEUES_MAX.load(Ordering::Relaxed) {
        return Err(AxError::StorageFull);
    }

    let attr = nullable!(attr.get_as_ref())?.copied().unwrap_or(MqAttr {
        mq_flags: 0,
        mq_maxmsg: DEFAULT_MAXMSG,
        mq_msgsize: DEFAULT_MSGSIZE,
        mq_curmsgs: 0,
        reserved: [0; 4],
    });
    if attr.mq_maxmsg <= 0 || attr.mq_msgsize <= 0 {
        return Err(AxError::InvalidInput);
    }

    let queue = Arc::new(PosixQueue {
        name: name.clone(),
        mode: mode & 0o777,
        uid: current_uid,
        gid: current_gid,
        maxmsg: attr.mq_maxmsg as usize,
        msgsize: attr.mq_msgsize as usize,
        inner: Mutex::new(QueueInner { messages: VecDeque::new(), notification: None }),
    });
    manager.insert(name, queue.clone());
    let fd = add_file_like(Arc::new(MqDescriptor::new(queue, flags)), flags & O_CLOEXEC != 0)?;
    Ok(fd as isize)
}

pub fn sys_mq_unlink(name: *const c_char) -> AxResult<isize> {
    let name = normalize_name(&vm_load_string(name)?)?;
    let current_uid = sys_geteuid()? as u32;
    let current_gid = sys_getegid()? as u32;
    let mut manager = MQ_MANAGER.lock();
    let queue = manager.get(&name).ok_or(AxError::NotFound)?;
    let perm = super::IpcPerm {
        key: 0,
        uid: queue.uid,
        gid: queue.gid,
        cuid: queue.uid,
        cgid: queue.gid,
        mode: queue.mode,
        seq: 0,
        pad: 0,
        unused0: 0,
        unused1: 0,
    };
    if !has_ipc_permission(&perm, current_uid, current_gid, true) {
        return Err(AxError::PermissionDenied);
    }
    manager.remove(&name);
    Ok(0)
}

pub fn sys_mq_timedsend(
    mqdes: c_int,
    msg_ptr: UserConstPtr<u8>,
    msg_len: usize,
    msg_prio: u32,
    abs_timeout: UserConstPtr<Timespec>,
) -> AxResult<isize> {
    validate_timeout(abs_timeout)?;
    if msg_prio >= MQ_PRIO_MAX {
        return Err(AxError::InvalidInput);
    }
    let desc = get_mq(mqdes)?;
    if !open_for_write(*desc.flags.lock()) {
        return Err(AxError::BadFileDescriptor);
    }
    if msg_len > desc.queue.msgsize {
        return Err(AxError::try_from(-90).unwrap());
    }
    let mut data = Some(msg_ptr.get_as_slice(msg_len)?.to_vec());

    loop {
        let mut inner = desc.queue.inner.lock();
        if inner.messages.len() < desc.queue.maxmsg {
            let pos = inner
                .messages
                .iter()
                .position(|msg| msg.prio < msg_prio)
                .unwrap_or(inner.messages.len());
            inner.messages.insert(pos, Message { prio: msg_prio, data: data.take().unwrap() });
            let notification = inner.notification.take();
            drop(inner);
            if let Some(notification) = notification {
                if notification.signo > 0 {
                    let _ = sys_kill(notification.pid, notification.signo as u32);
                }
            }
            return Ok(0);
        }
        if desc.nonblocking() {
            return Err(AxError::WouldBlock);
        }
        drop(inner);
        if !abs_timeout.is_null() {
            block_on(sleep(Duration::from_millis(1)));
            return Err(AxError::TimedOut);
        }
        yield_now();
    }
}

pub fn sys_mq_timedreceive(
    mqdes: c_int,
    msg_ptr: UserPtr<u8>,
    msg_len: usize,
    msg_prio: UserPtr<u32>,
    abs_timeout: UserConstPtr<Timespec>,
) -> AxResult<isize> {
    validate_timeout(abs_timeout)?;
    let desc = get_mq(mqdes)?;
    if !open_for_read(*desc.flags.lock()) {
        return Err(AxError::BadFileDescriptor);
    }
    if msg_len < desc.queue.msgsize {
        return Err(AxError::try_from(-90).unwrap());
    }

    loop {
        let mut inner = desc.queue.inner.lock();
        if let Some(message) = inner.messages.pop_front() {
            let len = message.data.len();
            msg_ptr.get_as_mut_slice(len)?.copy_from_slice(&message.data);
            if let Some(prio) = nullable!(msg_prio.get_as_mut())? {
                *prio = message.prio;
            }
            return Ok(len as isize);
        }
        if desc.nonblocking() {
            return Err(AxError::WouldBlock);
        }
        drop(inner);
        if !abs_timeout.is_null() {
            block_on(sleep(Duration::from_millis(1)));
            return Err(AxError::TimedOut);
        }
        yield_now();
    }
}

pub fn sys_mq_getsetattr(
    mqdes: c_int,
    newattr: UserConstPtr<MqAttr>,
    oldattr: UserPtr<MqAttr>,
) -> AxResult<isize> {
    let desc = get_mq(mqdes)?;
    if let Some(oldattr) = nullable!(oldattr.get_as_mut())? {
        *oldattr = queue_attr(&desc.queue, *desc.flags.lock());
    }
    if let Some(newattr) = nullable!(newattr.get_as_ref())? {
        let mut flags = desc.flags.lock();
        if newattr.mq_flags & O_NONBLOCK as c_long != 0 {
            *flags |= O_NONBLOCK;
        } else {
            *flags &= !(O_NONBLOCK as u32);
        }
    }
    Ok(0)
}

pub fn sys_mq_notify(mqdes: c_int, notification: usize) -> AxResult<isize> {
    let desc = get_mq(mqdes)?;
    let mut inner = desc.queue.inner.lock();
    if notification == 0 {
        inner.notification = None;
        return Ok(0);
    }
    let event = UserConstPtr::<SigeventHeader>::from(notification).get_as_ref()?;
    if inner.notification.is_some() {
        return Err(AxError::ResourceBusy);
    }
    let pid = axtask::current().as_thread().proc_data.proc.pid() as i32;
    let signo = if event.notify == SIGEV_NONE {
        0
    } else if event.signo > 0 {
        event.signo
    } else {
        SIGUSR1_FALLBACK
    };
    inner.notification = Some(Notification { pid, signo });
    Ok(0)
}
