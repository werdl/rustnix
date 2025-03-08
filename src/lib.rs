#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

pub use core::prelude::rust_2024::*;

extern crate alloc;
pub mod language_features;
pub mod file;
pub mod vga;
pub mod serial;
pub mod interrupts;
pub mod gdt;
pub mod memory;
pub mod allocator;
pub mod task;
pub mod ata;
pub mod clk;

use core::{panic::PanicInfo, prelude::rust_2024::*};

use alloc::string::String;
use bootloader::{entry_point, BootInfo};
use memory::BootInfoFrameAllocator;
use vga::Color;
use x86_64::VirtAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    hlt_loop();
}

fn ok_message(message: &str) {
    print!("[  ");
    vga::write_str("OK", Color::Green, Color::Black);
    print!("  ] {}\n", message);
}

pub fn init(boot_info: &'static BootInfo) {
    gdt::init();
    ok_message("GDT initialized");

    interrupts::init_idt();
    ok_message("IDT initialized");

    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
    ok_message("Interrupts enabled");

    memory::init(boot_info);
    ok_message("Memory initialized");
    
    ata::init();
    ok_message("ATA initialized");

    clk::pit::init();
    ok_message("PIT initialized");

    println!("rustnix: {}", clk::get_time());
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[cfg(test)]
entry_point!(test_kmain);

/// Entry point for `cargo test`
#[cfg(test)]
fn test_kmain(_boot_info: &'static BootInfo) -> ! {
    use bootloader::BootInfo;

    init(_boot_info);
    test_main();
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}
