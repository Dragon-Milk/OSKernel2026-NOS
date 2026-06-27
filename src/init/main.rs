#![no_std]
#![no_main]
#![doc = include_str!("../../README.md")]

extern crate alloc;

const INIT_SCRIPT: &str = concat!(
    "LTP_SAFE_DATA='", include_str!("ltp-cases/ltp-safe.txt"), "'\n",
    include_str!("ltp-cases.sh"), "\n",
    include_str!("init.sh")
);

pub const CMDLINES: &[&[&str]] = &[
    &["/bin/sh", "-c", INIT_SCRIPT],
    &["/busybox", "sh", "-c", INIT_SCRIPT],
    &["/musl/busybox", "sh", "-c", INIT_SCRIPT],
    &["/glibc/busybox", "sh", "-c", INIT_SCRIPT],
];

include!("env.rs");

#[unsafe(no_mangle)]
fn main() {
    let envs = [
        TEST_PROFILE_ENV,
        LTP_LIBC_ENV,
        LTP_TIMEOUT_ENV,
        LTP_CASE_LIST_ENV,
    ];

    starry_kernel::entry::init(CMDLINES, &envs);
}

#[cfg(feature = "vf2")]
extern crate axplat_riscv64_visionfive2;
