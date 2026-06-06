//! The core functionality of a monolithic kernel, including loading user
//! programs and managing processes.
//!
//! 宏内核的核心功能，包括加载用户程序和管理进程。
//! 本文件是内核 crate 的根模块，负责聚合导出所有核心子系统模块：配置、入口、
//! 文件系统、内存管理、伪文件系统、系统调用和任务管理。

#![no_std]
#![feature(likely_unlikely)]
#![feature(bstr)]
#![allow(missing_docs)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

extern crate alloc;
extern crate axruntime;

#[macro_use]
extern crate axlog;

// 内核入口
pub mod entry;

mod config;
mod file;
mod mm;
mod perf;
mod pseudofs;
mod syscall;
mod task;
mod time;
