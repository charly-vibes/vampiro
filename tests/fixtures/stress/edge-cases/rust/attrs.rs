#![allow(unused)]
#![crate_name = "attrs_test"]

#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
struct Packed {
    #[cfg(target_os = "linux")]
    os_specific: u32,
    common: u64,
}

#[inline(always)]
fn fast(x: u32) -> u32 {
    x + 1
}

#[cold]
fn cold_path() -> ! {
    loop {}
}

fn main() {
    let _p = Packed { common: 42 };
    let _ = fast(1);
}