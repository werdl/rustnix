use core::ptr::addr_of;
use lazy_static::lazy_static;
use x86_64::instructions::segmentation::{Segment, CS, DS};
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::{
    Descriptor, GlobalDescriptorTable, SegmentSelector
};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

const STACK_SIZE: usize = 1024 * 8 * 16;
/// Double fault interrupt stack table index
pub const DOUBLE_FAULT_IST: u16 = 0;
/// Page fault interrupt stack table index
pub const PAGE_FAULT_IST: u16 = 1;
/// General protection fault interrupt stack table index
pub const GENERAL_PROTECTION_FAULT_IST: u16 = 2;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.privilege_stack_table[0] = {
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            VirtAddr::from_ptr(addr_of!(STACK)) + STACK_SIZE as u64
        };
        tss.interrupt_stack_table[DOUBLE_FAULT_IST as usize] = {
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            VirtAddr::from_ptr(addr_of!(STACK)) + STACK_SIZE as u64
        };
        tss.interrupt_stack_table[PAGE_FAULT_IST as usize] = {
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            VirtAddr::from_ptr(addr_of!(STACK)) + STACK_SIZE as u64
        };
        tss.interrupt_stack_table[GENERAL_PROTECTION_FAULT_IST as usize] = {
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            VirtAddr::from_ptr(addr_of!(STACK)) + STACK_SIZE as u64
        };
        tss
    };
}

lazy_static! {
    /// Global Descriptor Table
    pub static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();

        let tss = gdt.append(Descriptor::tss_segment(&TSS));
        let code = gdt.append(Descriptor::kernel_code_segment());
        let data = gdt.append(Descriptor::kernel_data_segment());
        let user_code = gdt.append(Descriptor::user_code_segment());
        let user_data = gdt.append(Descriptor::user_data_segment());

        (
            gdt,
            Selectors {
                tss,
                code,
                data,
                user_code,
                user_data,
            },
        )
    };
}

/// Segment selectors for the GDT
pub struct Selectors {
    tss: SegmentSelector,
    code: SegmentSelector,
    data: SegmentSelector,
    /// User code segment
    pub user_code: SegmentSelector,
    /// User data segment
    pub user_data: SegmentSelector,
}

/// Initialize the GDT and load it into the CPU
pub fn init() {
    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.code);
        DS::set_reg(GDT.1.data);
        load_tss(GDT.1.tss);
    }
}
