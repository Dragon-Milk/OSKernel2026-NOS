#[cfg(feature = "perf-profile")]
use alloc::string::String;

#[cfg(feature = "perf-profile")]
mod enabled {
    use core::{mem::MaybeUninit, slice, str};

    use axhal::time::monotonic_time_nanos;
    use axsync::Mutex;
    use kernel_guard::NoPreemptIrqSave;
    use starry_vm::{VmPtr, vm_read_slice};

    use super::String;
    use crate::mm::IoVec;

    const MAX_GROUPS: usize = 64;
    const MAX_CASES: usize = 2048;
    const MAX_ACTIVE_CASES: usize = 128;
    const MAX_NAME_LEN: usize = 64;
    const WRITE_SCAN_LIMIT: usize = 256;
    const GROUP_START: &[u8] = b"#### OS COMP TEST GROUP START ";
    const GROUP_END: &[u8] = b"#### OS COMP TEST GROUP END ";
    static PERF_STATE: Mutex<PerfState> = Mutex::new(PerfState::new());

    #[derive(Clone, Copy)]
    struct Name {
        bytes: [u8; MAX_NAME_LEN],
        len: u8,
    }

    impl Name {
        const fn empty() -> Self {
            Self {
                bytes: [0; MAX_NAME_LEN],
                len: 0,
            }
        }

        fn set_bytes(&mut self, name: &[u8]) {
            let len = name.len().min(MAX_NAME_LEN);
            self.bytes = [0; MAX_NAME_LEN];
            self.bytes[..len].copy_from_slice(&name[..len]);
            self.len = len as u8;
        }

        fn eq_bytes(&self, name: &[u8]) -> bool {
            let len = self.len as usize;
            len == name.len() && self.bytes[..len] == *name
        }

        fn as_str(&self) -> &str {
            str::from_utf8(&self.bytes[..self.len as usize]).unwrap_or("<invalid>")
        }
    }

    #[derive(Clone, Copy)]
    struct GroupRecord {
        used: bool,
        finished: bool,
        name: Name,
        start_us: u64,
        end_us: u64,
    }

    impl GroupRecord {
        const fn empty() -> Self {
            Self {
                used: false,
                finished: false,
                name: Name::empty(),
                start_us: 0,
                end_us: 0,
            }
        }
    }

    #[derive(Clone, Copy)]
    struct CaseRecord {
        used: bool,
        finished: bool,
        group: Name,
        name: Name,
        start_us: u64,
        end_us: u64,
        pid: u64,
    }

    impl CaseRecord {
        const fn empty() -> Self {
            Self {
                used: false,
                finished: false,
                group: Name::empty(),
                name: Name::empty(),
                start_us: 0,
                end_us: 0,
                pid: 0,
            }
        }
    }

    #[derive(Clone, Copy)]
    struct ActiveCase {
        used: bool,
        pid: u64,
        record_index: usize,
    }

    impl ActiveCase {
        const fn empty() -> Self {
            Self {
                used: false,
                pid: 0,
                record_index: 0,
            }
        }
    }

    struct PerfState {
        groups: [GroupRecord; MAX_GROUPS],
        group_len: usize,
        cases: [CaseRecord; MAX_CASES],
        case_len: usize,
        active_cases: [ActiveCase; MAX_ACTIVE_CASES],
        active_group: Name,
        active_group_valid: bool,
        dropped_groups: u64,
        dropped_cases: u64,
        dropped_active_cases: u64,
    }

    impl PerfState {
        const fn new() -> Self {
            Self {
                groups: [GroupRecord::empty(); MAX_GROUPS],
                group_len: 0,
                cases: [CaseRecord::empty(); MAX_CASES],
                case_len: 0,
                active_cases: [ActiveCase::empty(); MAX_ACTIVE_CASES],
                active_group: Name::empty(),
                active_group_valid: false,
                dropped_groups: 0,
                dropped_cases: 0,
                dropped_active_cases: 0,
            }
        }

        fn begin_group(&mut self, name: &[u8], now_us: u64) {
            if self.group_len < MAX_GROUPS {
                let record = &mut self.groups[self.group_len];
                record.used = true;
                record.finished = false;
                record.name.set_bytes(name);
                record.start_us = now_us;
                record.end_us = 0;
                self.group_len += 1;
            } else {
                self.dropped_groups += 1;
            }

            self.active_group.set_bytes(name);
            self.active_group_valid = true;
        }

        fn end_group(&mut self, name: &[u8], now_us: u64) {
            for record in self.groups[..self.group_len].iter_mut().rev() {
                if record.used && !record.finished && record.name.eq_bytes(name) {
                    record.finished = true;
                    record.end_us = now_us;
                    break;
                }
            }

            if self.active_group.eq_bytes(name) {
                self.active_group_valid = false;
            }
        }

        fn begin_case(&mut self, group: &[u8], name: &[u8], pid: u64, now_us: u64) {
            self.finish_pid_case(pid, now_us);

            if self.case_len >= MAX_CASES {
                self.dropped_cases += 1;
                return;
            }

            let record_index = self.case_len;
            let record = &mut self.cases[record_index];
            record.used = true;
            record.finished = false;
            record.group.set_bytes(group);
            record.name.set_bytes(name);
            record.start_us = now_us;
            record.end_us = 0;
            record.pid = pid;
            self.case_len += 1;

            if let Some(active) = self.active_cases.iter_mut().find(|it| !it.used) {
                active.used = true;
                active.pid = pid;
                active.record_index = record_index;
            } else {
                self.dropped_active_cases += 1;
            }
        }

        #[allow(dead_code)]
        fn end_case(&mut self, group: &[u8], name: &[u8], now_us: u64) {
            let mut finished_pid = None;
            for record in self.cases[..self.case_len].iter_mut().rev() {
                if record.used
                    && !record.finished
                    && record.group.eq_bytes(group)
                    && record.name.eq_bytes(name)
                {
                    record.finished = true;
                    record.end_us = now_us;
                    finished_pid = Some(record.pid);
                    break;
                }
            }

            if let Some(pid) = finished_pid {
                if let Some(active) = self
                    .active_cases
                    .iter_mut()
                    .find(|it| it.used && it.pid == pid)
                {
                    active.used = false;
                }
            }
        }

        fn begin_process_case(&mut self, pid: u64, name: &str, now_us: u64) {
            if !self.active_group_valid {
                return;
            }

            let group = self.active_group;
            self.begin_case(
                &group.bytes[..group.len as usize],
                name.as_bytes(),
                pid,
                now_us,
            );
        }

        fn finish_pid_case(&mut self, pid: u64, now_us: u64) {
            let Some(active) = self
                .active_cases
                .iter_mut()
                .find(|it| it.used && it.pid == pid)
            else {
                return;
            };

            let record = &mut self.cases[active.record_index];
            if record.used && !record.finished {
                record.finished = true;
                record.end_us = now_us;
            }
            active.used = false;
        }

        fn group_count(&self) -> usize {
            self.group_len
        }

        fn case_count(&self) -> usize {
            self.case_len
        }

        fn group(&self, index: usize) -> GroupRecord {
            self.groups[index]
        }

        fn case(&self, index: usize) -> CaseRecord {
            self.cases[index]
        }
    }

    fn now_us() -> u64 {
        monotonic_time_nanos() / 1_000
    }

    fn perf_trace_enabled() -> bool {
        option_env!("AX_LOG") == Some("trace")
    }

    fn marker_name<'a>(bytes: &'a [u8], marker: &[u8]) -> Option<&'a [u8]> {
        let start = bytes
            .windows(marker.len())
            .position(|window| window == marker)?;
        let name_start = start + marker.len();
        let name_end = bytes[name_start..]
            .iter()
            .position(|byte| matches!(*byte, b' ' | b'\t' | b'\r' | b'\n' | b'#'))
            .map_or(bytes.len(), |offset| name_start + offset);
        (name_end > name_start).then_some(&bytes[name_start..name_end])
    }

    fn observe_group_markers(bytes: &[u8]) {
        if let Some(name) = marker_name(bytes, GROUP_START) {
            perf_begin_group_bytes(name);
        }
        if let Some(name) = marker_name(bytes, GROUP_END) {
            perf_end_group_bytes(name);
        }
    }

    fn exec_case_name<'a>(path: &'a str, args: &'a [String]) -> Option<&'a str> {
        let name = path.rsplit('/').next().unwrap_or(path);

        if name == "sh" || name.ends_with(".sh") {
            return None;
        }

        if name == "busybox" {
            let applet = args.get(1).map(String::as_str)?;
            if applet == "sh" {
                return None;
            }
            return Some(applet);
        }

        if name == "lmbench_all" {
            return args.get(1).map(String::as_str).or(Some(name));
        }

        Some(name)
    }

    fn print_group(record: GroupRecord, summary_us: u64) {
        let end_us = if record.finished {
            record.end_us
        } else {
            summary_us
        };
        trace!(
            "[PERF] GROUP name={} start_us={} end_us={} elapsed_us={} status={}",
            record.name.as_str(),
            record.start_us,
            end_us,
            end_us.saturating_sub(record.start_us),
            if record.finished { "ok" } else { "unfinished" }
        );
    }

    fn print_case(record: CaseRecord, summary_us: u64) {
        let end_us = if record.finished {
            record.end_us
        } else {
            summary_us
        };
        trace!(
            "[PERF] CASE group={} name={} start_us={} end_us={} elapsed_us={} status={}",
            record.group.as_str(),
            record.name.as_str(),
            record.start_us,
            end_us,
            end_us.saturating_sub(record.start_us),
            if record.finished { "ok" } else { "unfinished" }
        );
    }

    fn perf_begin_group_bytes(name: &[u8]) {
        PERF_STATE.lock().begin_group(name, now_us());
    }

    fn perf_end_group_bytes(name: &[u8]) {
        PERF_STATE.lock().end_group(name, now_us());
    }

    #[allow(dead_code)]
    pub fn perf_begin_group(name: &str) {
        perf_begin_group_bytes(name.as_bytes());
    }

    #[allow(dead_code)]
    pub fn perf_end_group(name: &str) {
        perf_end_group_bytes(name.as_bytes());
    }

    #[allow(dead_code)]
    pub fn perf_begin_case(group: &str, name: &str) {
        PERF_STATE
            .lock()
            .begin_case(group.as_bytes(), name.as_bytes(), 0, now_us());
    }

    #[allow(dead_code)]
    pub fn perf_end_case(group: &str, name: &str) {
        PERF_STATE
            .lock()
            .end_case(group.as_bytes(), name.as_bytes(), now_us());
    }

    pub fn perf_begin_process_exec(pid: u64, path: &str, args: &[String]) {
        let Some(name) = exec_case_name(path, args) else {
            return;
        };
        PERF_STATE.lock().begin_process_case(pid, name, now_us());
    }

    pub fn perf_end_process_case(pid: u64) {
        PERF_STATE.lock().finish_pid_case(pid, now_us());
    }

    pub fn perf_prepare_run() {
        if perf_trace_enabled() {
            axlog::set_max_level("warn");
        }
    }

    pub fn perf_observe_user_write(fd: i32, buf: *const u8, len: usize) {
        if !matches!(fd, 1 | 2) || len == 0 {
            return;
        }

        let scan_len = len.min(WRITE_SCAN_LIMIT);
        let mut bytes = [MaybeUninit::<u8>::uninit(); WRITE_SCAN_LIMIT];
        if vm_read_slice(buf, &mut bytes[..scan_len]).is_err() {
            return;
        }

        let bytes = unsafe { slice::from_raw_parts(bytes.as_ptr().cast::<u8>(), scan_len) };
        observe_group_markers(bytes);
    }

    pub fn perf_observe_user_writev(fd: i32, iov: *const IoVec, iovcnt: usize, written: usize) {
        if !matches!(fd, 1 | 2) {
            return;
        }

        let mut remaining = written;
        for index in 0..iovcnt.min(1024) {
            let Ok(iov) = iov.wrapping_add(index).vm_read() else {
                return;
            };
            if iov.iov_len <= 0 {
                continue;
            }
            let len = (iov.iov_len as usize).min(remaining);
            if len == 0 {
                break;
            }
            perf_observe_user_write(fd, iov.iov_base, len);
            remaining -= len;
        }
    }

    pub fn perf_print_summary() {
        if !perf_trace_enabled() {
            return;
        }

        let _guard = NoPreemptIrqSave::new();
        axlog::set_max_level("trace");

        let summary_us = now_us();
        let (group_count, case_count, dropped_groups, dropped_cases, dropped_active_cases) = {
            let state = PERF_STATE.lock();
            (
                state.group_count(),
                state.case_count(),
                state.dropped_groups,
                state.dropped_cases,
                state.dropped_active_cases,
            )
        };

        trace!("[PERF] SUMMARY START");
        for index in 0..group_count {
            print_group(PERF_STATE.lock().group(index), summary_us);
        }
        for index in 0..case_count {
            print_case(PERF_STATE.lock().case(index), summary_us);
        }
        if dropped_groups != 0 || dropped_cases != 0 || dropped_active_cases != 0 {
            trace!(
                "[PERF] DROPPED groups={} cases={} active_cases={}",
                dropped_groups, dropped_cases, dropped_active_cases
            );
        }
        trace!("[PERF] SUMMARY END");

        axlog::set_max_level("warn");
    }
}

#[cfg(feature = "perf-profile")]
pub use enabled::*;

#[cfg(not(feature = "perf-profile"))]
#[allow(dead_code)]
#[inline(always)]
pub fn perf_begin_group(_name: &str) {}

#[cfg(not(feature = "perf-profile"))]
#[allow(dead_code)]
#[inline(always)]
pub fn perf_end_group(_name: &str) {}

#[cfg(not(feature = "perf-profile"))]
#[allow(dead_code)]
#[inline(always)]
pub fn perf_begin_case(_group: &str, _name: &str) {}

#[cfg(not(feature = "perf-profile"))]
#[allow(dead_code)]
#[inline(always)]
pub fn perf_end_case(_group: &str, _name: &str) {}

#[cfg(not(feature = "perf-profile"))]
#[inline(always)]
pub fn perf_begin_process_exec(_pid: u64, _path: &str, _args: &[alloc::string::String]) {}

#[cfg(not(feature = "perf-profile"))]
#[inline(always)]
pub fn perf_end_process_case(_pid: u64) {}

#[cfg(not(feature = "perf-profile"))]
#[inline(always)]
pub fn perf_prepare_run() {}

#[cfg(not(feature = "perf-profile"))]
#[inline(always)]
pub fn perf_observe_user_write(_fd: i32, _buf: *const u8, _len: usize) {}

#[cfg(not(feature = "perf-profile"))]
#[inline(always)]
pub fn perf_observe_user_writev(
    _fd: i32,
    _iov: *const crate::mm::IoVec,
    _iovcnt: usize,
    _written: usize,
) {
}

#[cfg(not(feature = "perf-profile"))]
#[inline(always)]
pub fn perf_print_summary() {}
