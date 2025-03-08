use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::{print, println};
use lazy_static::lazy_static;
use crate::gdt;

lazy_static! {

    pub static ref IRQ_HANDLERS: spin::Mutex<[fn(); 16]> = 
        spin::Mutex::new([|| {}; 16]);

    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_u8()].set_handler_fn(keyboard_interrupt_handler);
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
pub fn init_idt() {
    IDT.load();
}

pub fn set_irq_handler(irq: u8, handler: fn()) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut handlers = IRQ_HANDLERS.lock();
        handlers[irq as usize] = handler;
    });
}


macro_rules! irq_handler {
    ($handler:ident, $irq:expr) => {
        pub extern "x86-interrupt" fn $handler(_: InterruptStackFrame) {
            let handlers = IRQ_HANDLERS.lock();
            handlers[$irq]();
            unsafe {
                PICS.lock().notify_end_of_interrupt(
                    PIC_1_OFFSET + $irq
                );
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

use x86_64::structures::idt::PageFaultErrorCode;
use crate::hlt_loop;

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode)
{
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn keyboard_interrupt_handler(
    _stack_frame: InterruptStackFrame)
{
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    crate::task::keyboard::add_scancode(scancode);

    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    print!(".");
    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

#[test_case]
fn test_breakpoint_exception() {
    x86_64::instructions::interrupts::int3();
}

use pic8259::ChainedPics;
use spin;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;


pub static PICS: spin::Mutex<ChainedPics> = 
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }
    
    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

