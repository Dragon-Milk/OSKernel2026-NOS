use alloc::{sync::Arc, vec::Vec};
use core::{fmt, time::Duration};

use axerrno::{AxError, AxResult};
use axpoll::IoEvents;
use axtask::future::{self, block_on, poll_io};
use bitmaps::Bitmap;
use linux_raw_sys::{
    general::*,
    select_macros::{FD_ISSET, FD_SET, FD_ZERO},
};
use starry_signal::SignalSet;

use super::FdPollSet;
use crate::{
    file::{FD_TABLE, FileLike},
    mm::{UserConstPtr, UserPtr, nullable},
    syscall::signal::check_sigset_size,
    task::with_blocked_signals,
    time::TimeValueLike,
};

struct FdSet(Bitmap<{ __FD_SETSIZE as usize }>);

impl FdSet {
    fn new(nfds: usize, fds: Option<&__kernel_fd_set>) -> Self {
        let mut bitmap = Bitmap::new();
        if let Some(fds) = fds {
            for i in 0..nfds {
                if unsafe { FD_ISSET(i as _, fds) } {
                    bitmap.set(i, true);
                }
            }
        }
        Self(bitmap)
    }
}

impl fmt::Debug for FdSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&self.0).finish()
    }
}

/// Poll all fds without allocation. Returns ready count and fills fd_sets.
fn poll_fds_direct(
    fd_bitmap: &Bitmap<{ __FD_SETSIZE as usize }>,
    read_set: &FdSet,
    write_set: &FdSet,
    except_set: &FdSet,
    readfds: &mut Option<&mut __kernel_fd_set>,
    writefds: &mut Option<&mut __kernel_fd_set>,
    exceptfds: &mut Option<&mut __kernel_fd_set>,
) -> AxResult<(usize, Vec<(Arc<dyn FileLike>, IoEvents, usize)>)> {
    let mut res = 0usize;
    let fd_table = FD_TABLE.read();
    let fd_count = fd_bitmap.len();
    let mut ready_fds = Vec::with_capacity(fd_count);

    for fd in fd_bitmap.into_iter() {
        let f = fd_table
            .get(fd)
            .ok_or(AxError::BadFileDescriptor)?
            .inner
            .clone();
        let mut interested = IoEvents::empty();
        interested.set(IoEvents::IN, read_set.0.get(fd));
        interested.set(IoEvents::OUT, write_set.0.get(fd));
        interested.set(IoEvents::ERR, except_set.0.get(fd));

        if interested.is_empty() {
            continue;
        }

        let events = f.poll() & interested;
        let mut fd_ready = false;
        if events.contains(IoEvents::IN)
            && let Some(set) = readfds.as_deref_mut()
        {
            fd_ready = true;
            unsafe { FD_SET(fd as _, set) };
        }
        if events.contains(IoEvents::OUT)
            && let Some(set) = writefds.as_deref_mut()
        {
            fd_ready = true;
            unsafe { FD_SET(fd as _, set) };
        }
        if events.contains(IoEvents::ERR)
            && let Some(set) = exceptfds.as_deref_mut()
        {
            fd_ready = true;
            unsafe { FD_SET(fd as _, set) };
        }
        if fd_ready {
            res += 1;
        }
        ready_fds.push((f, interested, fd));
    }

    drop(fd_table);
    Ok((res, ready_fds))
}

fn do_select(
    nfds: u32,
    readfds: UserPtr<__kernel_fd_set>,
    writefds: UserPtr<__kernel_fd_set>,
    exceptfds: UserPtr<__kernel_fd_set>,
    timeout: Option<Duration>,
    sigmask: UserConstPtr<SignalSetWithSize>,
) -> AxResult<isize> {
    if nfds > __FD_SETSIZE {
        return Err(AxError::InvalidInput);
    }
    let sigmask = if let Some(sigmask) = nullable!(sigmask.get_as_ref())? {
        check_sigset_size(sigmask.sigsetsize)?;
        let set = sigmask.set;
        nullable!(set.get_as_ref())?
    } else {
        None
    };

    let mut readfds = nullable!(readfds.get_as_mut())?;
    let mut writefds = nullable!(writefds.get_as_mut())?;
    let mut exceptfds = nullable!(exceptfds.get_as_mut())?;

    let read_set = FdSet::new(nfds as _, readfds.as_deref());
    let write_set = FdSet::new(nfds as _, writefds.as_deref());
    let except_set = FdSet::new(nfds as _, exceptfds.as_deref());

    debug!(
        "sys_select <= nfds: {nfds} sets: [read: {read_set:?}, write: {write_set:?}, except: \
         {except_set:?}] timeout: {timeout:?}"
    );

    let fd_bitmap = read_set.0 | write_set.0 | except_set.0;
    if fd_bitmap.is_empty() {
        if let Some(timeout) = timeout {
            block_on(future::sleep(timeout));
        }
        return Ok(0);
    }

    // Clear all sets before filling
    let clear_sets = |readfds: &mut Option<&mut __kernel_fd_set>,
                      writefds: &mut Option<&mut __kernel_fd_set>,
                      exceptfds: &mut Option<&mut __kernel_fd_set>| {
        if let Some(readfds) = readfds.as_deref_mut() {
            unsafe { FD_ZERO(readfds) };
        }
        if let Some(writefds) = writefds.as_deref_mut() {
            unsafe { FD_ZERO(writefds) };
        }
        if let Some(exceptfds) = exceptfds.as_deref_mut() {
            unsafe { FD_ZERO(exceptfds) };
        }
    };

    clear_sets(&mut readfds, &mut writefds, &mut exceptfds);

    // First pass: poll all fds directly
    let (ready, all_fds) = poll_fds_direct(
        &fd_bitmap,
        &read_set,
        &write_set,
        &except_set,
        &mut readfds,
        &mut writefds,
        &mut exceptfds,
    )?;

    if ready > 0 || timeout == Some(Duration::ZERO) {
        return Ok(ready as _);
    }

    if let Some(timeout) = timeout {
        return with_blocked_signals(sigmask.copied(), || {
            block_on(future::sleep(timeout));
            clear_sets(&mut readfds, &mut writefds, &mut exceptfds);
            let (ready, _) = poll_fds_direct(
                &fd_bitmap,
                &read_set,
                &write_set,
                &except_set,
                &mut readfds,
                &mut writefds,
                &mut exceptfds,
            )?;
            Ok(ready as _)
        });
    }

    // Nothing ready yet, need to wait. Build FdPollSet from all_fds.
    let fd_indices: Vec<usize> = all_fds.iter().map(|(_, _, idx)| *idx).collect();
    let poll_fds: Vec<(Arc<dyn FileLike>, IoEvents)> =
        all_fds.into_iter().map(|(f, e, _)| (f, e)).collect();
    let fds = FdPollSet(poll_fds);

    with_blocked_signals(sigmask.copied(), || {
        match block_on(future::timeout(
            timeout,
            poll_io(&fds, IoEvents::empty(), false, || {
                clear_sets(&mut readfds, &mut writefds, &mut exceptfds);
                let mut res = 0usize;
                for ((fd, interested), index) in fds.0.iter().zip(fd_indices.iter().copied()) {
                    let events = fd.poll() & *interested;
                    if events.contains(IoEvents::IN)
                        && let Some(set) = readfds.as_deref_mut()
                    {
                        res += 1;
                        unsafe { FD_SET(index as _, set) };
                    }
                    if events.contains(IoEvents::OUT)
                        && let Some(set) = writefds.as_deref_mut()
                    {
                        res += 1;
                        unsafe { FD_SET(index as _, set) };
                    }
                    if events.contains(IoEvents::ERR)
                        && let Some(set) = exceptfds.as_deref_mut()
                    {
                        res += 1;
                        unsafe { FD_SET(index as _, set) };
                    }
                }
                if res > 0 {
                    Ok(res as _)
                } else {
                    Err(AxError::WouldBlock)
                }
            }),
        )) {
            Ok(r) => r,
            Err(_) => Ok(0),
        }
    })
}

#[cfg(target_arch = "x86_64")]
pub fn sys_select(
    nfds: u32,
    readfds: UserPtr<__kernel_fd_set>,
    writefds: UserPtr<__kernel_fd_set>,
    exceptfds: UserPtr<__kernel_fd_set>,
    timeout: UserConstPtr<timeval>,
) -> AxResult<isize> {
    do_select(
        nfds,
        readfds,
        writefds,
        exceptfds,
        nullable!(timeout.get_as_ref())?
            .map(|it| it.try_into_time_value())
            .transpose()?,
        0.into(),
    )
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SignalSetWithSize {
    set: UserConstPtr<SignalSet>,
    sigsetsize: usize,
}

pub fn sys_pselect6(
    nfds: u32,
    readfds: UserPtr<__kernel_fd_set>,
    writefds: UserPtr<__kernel_fd_set>,
    exceptfds: UserPtr<__kernel_fd_set>,
    timeout: UserConstPtr<timespec>,
    sigmask: UserConstPtr<SignalSetWithSize>,
) -> AxResult<isize> {
    do_select(
        nfds,
        readfds,
        writefds,
        exceptfds,
        nullable!(timeout.get_as_ref())?
            .map(|ts| ts.try_into_time_value())
            .transpose()?,
        sigmask,
    )
}
