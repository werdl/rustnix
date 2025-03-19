#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

pub use core::prelude::rust_2024::*;

extern crate alloc;
pub mod internal;
use internal::{ata, clk, gdt, interrupts, memory, vga};

use core::panic::PanicInfo;

#[allow(unused_imports)]
use bootloader::{entry_point, BootInfo};
use internal::vga::Color;

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

use log::{info, Level, Metadata, Record};
use alloc::format;

// Example: A custom logger that writes logs to a serial port.
pub struct SerialLogger;

impl log::Log for SerialLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        // You can filter logs based on level
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level = record.level();
            let args = record.args();
            let message = format!("{}", args);

            // Here you would send the message to a serial port or some output
            // For example, `serial_write(message.as_bytes())`

            kprint!("[ ");

            match level {
                Level::Error => {
                    vga::write_str("ERROR", Color::LightRed, Color::Black);
                }
                Level::Warn => {
                    vga::write_str("WARN", Color::Yellow, Color::Black);
                }
                Level::Info => {
                    vga::write_str("INFO", Color::LightBlue, Color::Black);
                }
                Level::Debug => {
                    vga::write_str("DEBUG", Color::LightGreen, Color::Black);
                }
                Level::Trace => {
                    vga::write_str("TRACE", Color::LightCyan, Color::Black);
                }
            }

            match level {
                Level::Warn | Level::Info => {
                    kprint!(" ");
                }
                _ => {}
            }

            kprint!("] {}\n", message);

        }
    }

    fn flush(&self) {
        // Optional: Flush logs if necessary
    }
}

pub fn init_logger() {
    log::set_logger(&SerialLogger)
        .map(|()| log::set_max_level(log::LevelFilter::Info))
        .unwrap();
}


pub fn init(boot_info: &'static BootInfo) {
    kprint!("[ ");
    vga::write_str("INFO", Color::LightBlue, Color::Black);
    kprint!(" ] Initializing memory...\n");
    memory::init(boot_info);
    init_logger();
    info!("Logger initialized");
    info!("Memory initialized");


    gdt::init();
    info!("GDT initialized");

    interrupts::init_idt();
    info!("IDT initialized");

    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
    info!("Interrupts enabled");



    ata::init();
    info!("ATA initialized");

    clk::pit::init();
    info!("PIT initialized");

    info!("Kernel initialized");
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
    init(_boot_info);
    test_main();
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}
