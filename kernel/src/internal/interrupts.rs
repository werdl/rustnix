use crate::internal::gdt;
use crate::internal::memory::physical_memory_offset;
use crate::internal::{interrupts, syscall};
use lazy_static::lazy_static;
use log::{error, warn};
use x86_64::registers::control::Cr2;
use x86_64::structures::paging::OffsetPageTable;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

lazy_static! {
    /// Array of interrupt handlers
    pub static ref IRQ_HANDLERS: spin::Mutex<[fn(); 16]> = spin::Mutex::new([|| {}; 16]);
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault.
                set_handler_fn(double_fault_handler).
                set_stack_index(gdt::DOUBLE_FAULT_IST);
            idt.page_fault.
                set_handler_fn(page_fault_handler).
                set_stack_index(gdt::PAGE_FAULT_IST);
            idt.general_protection_fault.
                set_handler_fn(general_protection_fault_handler).
                set_stack_index(gdt::GENERAL_PROTECTION_FAULT_IST);

            let f = wrapped_syscall_handler as *mut fn();
            idt[0x80].
                set_handler_fn(core::mem::transmute(f)).
                set_privilege_level(x86_64::PrivilegeLevel::Ring3);
        }
        idt[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_interrupt_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt[PIC_1_OFFSET].set_handler_fn(irq0_handler);
        idt[PIC_1_OFFSET + 1].set_handler_fn(irq1_handler);
        idt[PIC_1_OFFSET + 2].set_handler_fn(irq2_handler);
        idt[PIC_1_OFFSET + 3].set_handler_fn(irq3_handler);
        idt[PIC_1_OFFSET + 4].set_handler_fn(irq4_handler);
        idt[PIC_1_OFFSET + 5].set_handler_fn(irq5_handler);
        idt[PIC_1_OFFSET + 6].set_handler_fn(irq6_handler);
        idt[PIC_1_OFFSET + 7].set_handler_fn(irq7_handler);
        idt[PIC_1_OFFSET + 8].set_handler_fn(irq8_handler);
        idt[PIC_1_OFFSET + 9].set_handler_fn(irq9_handler);
        idt[PIC_1_OFFSET + 10].set_handler_fn(irq10_handler);
        idt[PIC_1_OFFSET + 11].set_handler_fn(irq11_handler);
        idt[PIC_1_OFFSET + 12].set_handler_fn(irq12_handler);
        idt[PIC_1_OFFSET + 13].set_handler_fn(irq13_handler);
        idt[PIC_1_OFFSET + 14].set_handler_fn(irq14_handler);
        idt[PIC_1_OFFSET + 15].set_handler_fn(irq15_handler);
        idt
    };
}

/// Initialize the Interrupt Descriptor Table
pub fn init_idt() {
    crate::internal::vga::trace("Initializing IDT");
    IDT.load();
}

/// Initialize the interrupt system
pub fn init() {
    crate::internal::vga::trace("Initializing interrupts");
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
}

/// Set the handler for an IRQ
pub fn set_irq_handler(irq: u8, handler: fn()) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut handlers = IRQ_HANDLERS.lock();
        handlers[irq as usize] = handler;
    });
}

macro_rules! irq_handler {
    ($handler:ident, $irq:expr) => {
        /// Handler for IRQ $irq
        pub extern "x86-interrupt" fn $handler(_: InterruptStackFrame) {
            let handlers = IRQ_HANDLERS.lock();
            handlers[$irq]();
            unsafe {
                PICS.lock().notify_end_of_interrupt(PIC_1_OFFSET + $irq);
            }
        }
    };
}

irq_handler!(irq0_handler, 0);
irq_handler!(irq1_handler, 1);
irq_handler!(irq2_handler, 2);
irq_handler!(irq3_handler, 3);
irq_handler!(irq4_handler, 4);
irq_handler!(irq5_handler, 5);
irq_handler!(irq6_handler, 6);
irq_handler!(irq7_handler, 7);
irq_handler!(irq8_handler, 8);
irq_handler!(irq9_handler, 9);
irq_handler!(irq10_handler, 10);
irq_handler!(irq11_handler, 11);
irq_handler!(irq12_handler, 12);
irq_handler!(irq13_handler, 13);
irq_handler!(irq14_handler, 14);
irq_handler!(irq15_handler, 15);

use crate::hlt_loop;
use x86_64::structures::idt::PageFaultErrorCode;

extern "x86-interrupt" fn page_fault_handler(
    _stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let addr = Cr2::read().unwrap().as_u64();

    let page_table = unsafe { crate::internal::process::page_table() };
    let mut mapper = unsafe {
        OffsetPageTable::new(page_table, physical_memory_offset())
    };

    if error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE) {
        if crate::internal::memory::alloc_pages(&mut mapper, addr, 1).is_err() {
            if error_code.contains(PageFaultErrorCode::USER_MODE) {
                warn!(
                    "User Mode Error (exiting): Page fault at {:#X} with error code {:#X}\n",
                    addr,
                    error_code.bits()
                );
                crate::internal::process::exit();
            } else {
                error!(
                    "Error: Could not allocate page at {:#X}\n",
                    addr
                );
                hlt_loop();
            }
        }
    } else if error_code.contains(PageFaultErrorCode::USER_MODE) {
        let start = (addr / 4096) * 4096;
        if crate::internal::memory::alloc_pages(&mut mapper, start, 4096).is_ok() {
            if crate::internal::process::is_userspace(start) {
                let code_addr = crate::internal::process::get_code_addr();
                let src = (code_addr + start) as *mut u8;
                let dst = start as *mut u8;
                unsafe {
                    core::ptr::copy_nonoverlapping(src, dst, 4096);
                }
            }
        }
    } else {
        panic!(
            "Error: Page fault at {:#X} with error code {:#X}\n",
            addr,
            error_code.bits()
        );
    }
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    warn!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: GENERAL PROTECTION FAULT\n{:#?}\nError code: {:#X}",
        stack_frame, error_code
    );
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

/// Test the breakpoint exception
#[test_case]
fn test_breakpoint_exception() {
    x86_64::instructions::interrupts::int3();
}

use pic8259::ChainedPics;
use spin;

use super::process;

/// Offset for the controller PIC
pub const PIC_1_OFFSET: u8 = 32;
/// Offset for the worker PIC
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

/// The Programmable Interrupt Controller
pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

/// interrupt types
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    /// Timer interrupt
    Timer = PIC_1_OFFSET,
    /// Keyboard interrupt
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }
}

// Naked function wrapper saving all scratch registers to the stack
// See: https://os.phil-opp.com/returning-from-exceptions/
macro_rules! wrap {
    ($fn: ident => $w:ident) => {
        #[naked]
        /// must be sysv64 calling convention, as x86-interrupt would overwrite rax, with the return value we are setting
        pub unsafe extern "sysv64" fn $w() {
            unsafe {core::arch::naked_asm!(
                "push rax",
                "push rcx",
                "push rdx",
                "push rsi",
                "push rdi",
                "push r8",
                "push r9",
                "push r10",
                "push r11",
                "mov rsi, rsp", // Arg #2: register list
                "mov rdi, rsp", // Arg #1: interupt frame
                "add rdi, 9 * 8", // 9 registers * 8 bytes
                "call {}",
                "pop r11",
                "pop r10",
                "pop r9",
                "pop r8",
                "pop rdi",
                "pop rsi",
                "pop rdx",
                "pop rcx",
                "pop rax",
                "iretq",
                sym $fn
            );}
        }
    };
}

wrap!(syscall_handler => wrapped_syscall_handler);

extern "sysv64" fn syscall_handler(
    _stack_frame: &mut InterruptStackFrame,
    regs: &mut process::Registers,
) {
    let n = regs.rax;

    // The registers order follow the System V ABI convention
    let arg1 = regs.rdi;
    let arg2 = regs.rsi;
    let arg3 = regs.rdx;
    let arg4 = regs.r8;

    // Backup CPU context before spawning a process - not needed right now

    let res = syscall::dispatch(n as usize, arg1, arg2, arg3, arg4);

    regs.rax = res as usize;

    // Restore CPU context before exiting a process - not needed right now

    unsafe { PICS.lock().notify_end_of_interrupt(0x80) };
}
