#![no_std]
#![no_main]
#![doc = include_str!("../../README.md")]

extern crate alloc;

pub const CMDLINES: &[&[&str]] = &[
    &["/bin/sh", "-c", include_str!("init.sh")],
    &["/busybox", "sh", "-c", include_str!("init.sh")],
    &["/musl/busybox", "sh", "-c", include_str!("init.sh")],
    &["/glibc/busybox", "sh", "-c", include_str!("init.sh")],
];

#[unsafe(no_mangle)]
fn main() {
    let envs: [&str; 0] = [];

    starry_kernel::entry::init(CMDLINES, &envs);
}

#[cfg(feature = "vf2")]
extern crate axplat_riscv64_visionfive2;
