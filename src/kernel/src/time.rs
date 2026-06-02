//! 时间类型转换模块。
//!
//! 本模块负责：
//! - 提供 `TimeValueLike` trait，统一内核中各种时间结构体与 `TimeValue` 的互转
//! - 支持 `timespec`、`__kernel_timespec`、`__kernel_old_timespec`（纳秒精度）
//! - 支持 `timeval`、`__kernel_old_timeval`、`__kernel_sock_timeval`（微秒精度）
//!
//! 核心不变量：
//! - 所有时间类型的秒字段必须非负
//! - 纳秒字段必须在 [0, 999_999_999] 范围内
//! - 微秒字段必须在 [0, 999_999] 范围内
//!
//! 修改注意：
//! - 新增时间类型时需同时实现 `from_time_value` 和 `try_into_time_value`
//! - `try_into_time_value` 必须进行范围校验，防止非法时间值传入

use axerrno::{AxError, AxResult};
use axhal::time::TimeValue;
use linux_raw_sys::general::{
    __kernel_old_timespec, __kernel_old_timeval, __kernel_sock_timeval, __kernel_timespec,
    timespec, timeval,
};

/// 时间值转换 trait。
///
/// 为各种 Linux 时间结构体提供与内核 `TimeValue` 之间的双向转换能力。
/// `from_time_value` 不会失败，`try_into_time_value` 在时间值非法时返回 `InvalidInput`。
pub trait TimeValueLike {
    /// 从 `TimeValue` 构造本类型。
    fn from_time_value(tv: TimeValue) -> Self;

    /// 尝试将本类型转换为 `TimeValue`。
    fn try_into_time_value(self) -> AxResult<TimeValue>;
}

impl TimeValueLike for TimeValue {
    fn from_time_value(tv: TimeValue) -> Self {
        tv
    }

    fn try_into_time_value(self) -> AxResult<TimeValue> {
        Ok(self)
    }
}

/// `timespec` 与 `TimeValue` 的互转。
///
/// 对应 Linux `struct timespec`，精度为纳秒。
impl TimeValueLike for timespec {
    fn from_time_value(tv: TimeValue) -> Self {
        Self {
            tv_sec: tv.as_secs() as _,
            tv_nsec: tv.subsec_nanos() as _,
        }
    }

    fn try_into_time_value(self) -> AxResult<TimeValue> {
        if self.tv_nsec < 0 || self.tv_nsec > 999_999_999 || self.tv_sec < 0 {
            return Err(AxError::InvalidInput);
        }
        Ok(TimeValue::new(self.tv_sec as u64, self.tv_nsec as u32))
    }
}

/// `__kernel_timespec` 与 `TimeValue` 的互转。
///
/// 对应 Linux 内核内部使用的 `__kernel_timespec`，精度为纳秒。
impl TimeValueLike for __kernel_timespec {
    fn from_time_value(tv: TimeValue) -> Self {
        Self {
            tv_sec: tv.as_secs() as _,
            tv_nsec: tv.subsec_nanos() as _,
        }
    }

    fn try_into_time_value(self) -> AxResult<TimeValue> {
        if self.tv_nsec < 0 || self.tv_nsec > 999_999_999 || self.tv_sec < 0 {
            return Err(AxError::InvalidInput);
        }
        Ok(TimeValue::new(self.tv_sec as u64, self.tv_nsec as u32))
    }
}

/// `__kernel_old_timespec` 与 `TimeValue` 的互转。
///
/// 对应 Linux 旧版 `__kernel_old_timespec`，用于兼容 32 位 ABI。
impl TimeValueLike for __kernel_old_timespec {
    fn from_time_value(tv: TimeValue) -> Self {
        Self {
            tv_sec: tv.as_secs() as _,
            tv_nsec: tv.subsec_nanos() as _,
        }
    }

    fn try_into_time_value(self) -> AxResult<TimeValue> {
        if self.tv_nsec < 0 || self.tv_nsec > 999_999_999 || self.tv_sec < 0 {
            return Err(AxError::InvalidInput);
        }
        Ok(TimeValue::new(self.tv_sec as u64, self.tv_nsec as u32))
    }
}

/// `timeval` 与 `TimeValue` 的互转。
///
/// 对应 Linux `struct timeval`，精度为微秒。转换时微秒乘以 1000 得到纳秒。
impl TimeValueLike for timeval {
    fn from_time_value(tv: TimeValue) -> Self {
        Self {
            tv_sec: tv.as_secs() as _,
            tv_usec: tv.subsec_micros() as _,
        }
    }

    fn try_into_time_value(self) -> AxResult<TimeValue> {
        if self.tv_usec < 0 || self.tv_usec > 999_999 || self.tv_sec < 0 {
            return Err(AxError::InvalidInput);
        }
        Ok(TimeValue::new(
            self.tv_sec as u64,
            self.tv_usec as u32 * 1000,
        ))
    }
}

/// `__kernel_old_timeval` 与 `TimeValue` 的互转。
///
/// 对应 Linux 旧版 `__kernel_old_timeval`，用于兼容 32 位 ABI，精度为微秒。
impl TimeValueLike for __kernel_old_timeval {
    fn from_time_value(tv: TimeValue) -> Self {
        Self {
            tv_sec: tv.as_secs() as _,
            tv_usec: tv.subsec_micros() as _,
        }
    }

    fn try_into_time_value(self) -> AxResult<TimeValue> {
        if self.tv_usec < 0 || self.tv_usec > 999_999 || self.tv_sec < 0 {
            return Err(AxError::InvalidInput);
        }
        Ok(TimeValue::new(
            self.tv_sec as u64,
            self.tv_usec as u32 * 1000,
        ))
    }
}

/// `__kernel_sock_timeval` 与 `TimeValue` 的互转。
///
/// 对应 Linux 套接字层使用的 `__kernel_sock_timeval`，精度为微秒。
impl TimeValueLike for __kernel_sock_timeval {
    fn from_time_value(tv: TimeValue) -> Self {
        Self {
            tv_sec: tv.as_secs() as _,
            tv_usec: tv.subsec_micros() as _,
        }
    }

    fn try_into_time_value(self) -> AxResult<TimeValue> {
        if self.tv_usec < 0 || self.tv_usec > 999_999 || self.tv_sec < 0 {
            return Err(AxError::InvalidInput);
        }
        Ok(TimeValue::new(
            self.tv_sec as u64,
            self.tv_usec as u32 * 1000,
        ))
    }
}
