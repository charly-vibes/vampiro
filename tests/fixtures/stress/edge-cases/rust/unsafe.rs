#![allow(unused)]

static mut COUNTER: u32 = 0;

fn increment() -> u32 {
    unsafe {
        COUNTER += 1;
        COUNTER
    }
}

extern "C" {
    fn malloc(size: usize) -> *mut u8;
}

fn main() {
    let _ = increment();
    let _p = unsafe { malloc(64) };
    let _r: &u32 = unsafe { &*(&42 as *const u32) };
}