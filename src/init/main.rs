#![no_std]
#![no_main]
#![doc = include_str!("../../README.md")]

extern crate alloc;

const INIT_SCRIPT: &str = concat!(include_str!("ltp-cases.sh"), "\n", include_str!("init.sh"));

pub const CMDLINES: &[&[&str]] = &[
    &["/bin/sh", "-c", INIT_SCRIPT],
    &["/busybox", "sh", "-c", INIT_SCRIPT],
    &["/musl/busybox", "sh", "-c", INIT_SCRIPT],
    &["/glibc/busybox", "sh", "-c", INIT_SCRIPT],
];

#[unsafe(no_mangle)]
fn main() {
    let envs = [
        concat!("TEST_PROFILE=", env!("TEST_PROFILE")),
        concat!("LTP_CATEGORY=", env!("LTP_CATEGORY")),
        concat!("LTP_BATCH=", env!("LTP_BATCH")),
        concat!("LTP_LIBC=", env!("LTP_LIBC")),
        concat!("LTP_TIMEOUT=", env!("LTP_TIMEOUT")),
    ];

    starry_kernel::entry::init(CMDLINES, &envs);
}

#[cfg(feature = "vf2")]
extern crate axplat_riscv64_visionfive2;
