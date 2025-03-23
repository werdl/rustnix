// todo
#[repr(align(8), C)]
#[derive(Debug, Clone, Copy, Default)]
#[allow(missing_docs)]
pub struct Registers {
    pub r11: usize,
    pub r10: usize,
    pub r9: usize,
    pub r8: usize,
    pub rdi: usize,
    pub rsi: usize,
    pub rdx: usize,
    pub rcx: usize,
    pub rax: isize,
}
