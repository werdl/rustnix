/// syscall.rs - implements system calls, including the handler (which will be invoked from the interrupt file)

pub const READ: u64 = 0x1;
pub const WRITE: u64 = 0x2;
pub const OPEN: u64 = 0x3;
pub const CLOSE: u64 = 0x4;
pub const EXIT: u64 = 0x5;
pub const SLEEP: u64 = 0x6;
pub const WAIT: u64 = 0x7;
pub const GETPID: u64 = 0x8;
pub const EXEC: u64 = 0x9;
pub const FORK: u64 = 0xA;
pub const GETTID: u64 = 0xB;
pub const STOP: u64 = 0xC;
pub const WAITPID: u64 = 0xD;
pub const CONNECT: u64 = 0xE;
pub const ACCEPT: u64 = 0xF;
pub const LISTEN : u64 = 0x10;
pub const ALLOC : u64 = 0x11;
pub const FREE : u64 = 0x12;
pub const KIND: u64 = 0x13;

pub fn alloc(size: usize) -> *mut u8 {
    let layout = core::alloc::Layout::from_size_align(size, 1).unwrap();
    unsafe { alloc::alloc::alloc(layout) }
}

pub fn free(ptr: *mut u8, size: usize) {
    let layout = core::alloc::Layout::from_size_align(size, 1).unwrap();
    unsafe { alloc::alloc::dealloc(ptr, layout) }
}

#[test_case]
fn test_alloc_free() {
    let heap_value = alloc(1024);
    assert!(!heap_value.is_null());
    free(heap_value, 1024);
}
