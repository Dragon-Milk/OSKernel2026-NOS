use axerrno::{AxError, AxResult};
use axtask::current;
use starry_process::Pid;

use crate::task::{AsThread, get_process_data, get_process_group, register_process_group};

fn resolve_proc_pid(pid: Pid) -> Pid {
    if pid == 1 {
        starry_process::init_proc().pid()
    } else {
        pid
    }
}

pub fn sys_getsid(pid: Pid) -> AxResult<isize> {
    Ok(get_process_data(resolve_proc_pid(pid))?
        .proc
        .group()
        .session()
        .sid() as _)
}

pub fn sys_setsid() -> AxResult<isize> {
    let curr = current();
    let proc = &curr.as_thread().proc_data.proc;
    if get_process_group(proc.pid()).is_ok() {
        return Err(AxError::OperationNotPermitted);
    }

    if let Some((session, _)) = proc.create_session() {
        Ok(session.sid() as _)
    } else {
        Ok(proc.pid() as _)
    }
}

pub fn sys_getpgid(pid: Pid) -> AxResult<isize> {
    Ok(get_process_data(resolve_proc_pid(pid))?.proc.group().pgid() as _)
}

pub fn sys_setpgid(pid: i32, pgid: i32) -> AxResult<isize> {
    if pgid < 0 {
        return Err(AxError::InvalidInput);
    }

    let curr = current();
    let curr_proc = &curr.as_thread().proc_data.proc;
    let target_pid = if pid == 0 {
        curr_proc.pid()
    } else {
        resolve_proc_pid(pid.try_into().map_err(|_| AxError::NoSuchProcess)?)
    };
    let pgid: Pid = pgid.try_into().map_err(|_| AxError::InvalidInput)?;
    let proc_data = if pid == 0 {
        curr.as_thread().proc_data.clone()
    } else {
        get_process_data(target_pid)?
    };
    let proc = &proc_data.proc;

    if target_pid != curr_proc.pid()
        && proc
            .parent()
            .is_none_or(|parent| parent.pid() != curr_proc.pid())
    {
        return Err(AxError::NoSuchProcess);
    }
    if target_pid != curr_proc.pid() && proc_data.did_exec() {
        return Err(AxError::PermissionDenied);
    }

    if proc.group().session().sid() == proc.pid() {
        return Err(AxError::OperationNotPermitted);
    }

    if pgid == 0 {
        if let Some(group) = proc.create_group() {
            register_process_group(&group);
        }
    } else if pgid == proc.pid() {
        if let Some(group) = proc.create_group() {
            register_process_group(&group);
        }
    } else {
        let group = get_process_group(pgid).map_err(|_| AxError::OperationNotPermitted)?;
        if group.session().sid() != curr_proc.group().session().sid()
            || !proc.move_to_group(&group)
        {
            return Err(AxError::OperationNotPermitted);
        }
    }

    Ok(0)
}

// TODO: job control
