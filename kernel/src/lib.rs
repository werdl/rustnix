#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![feature(naked_functions)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

/// import the allocator crate
extern crate alloc;

/// internal modules, not exposed to userspace
pub mod internal;
use internal::{acpi, ata, clk, fs, gdt, interrupts, keyboard, memory, syscall, user, vga};
pub use {
    syscall::ALLOC, syscall::CLOSE, syscall::EXEC, syscall::EXIT, syscall::FLUSH, syscall::FREE,
    syscall::GETERRNO, syscall::GETPID, syscall::KIND, syscall::OPEN, syscall::READ,
    syscall::SLEEP, syscall::WAIT, syscall::WRITE,
};

use core::panic::PanicInfo;

#[allow(unused_imports)]
use bootloader::{BootInfo, entry_point};
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

use alloc::format;
use log::{Level, Metadata, Record, info};

// Example: A custom logger that writes logs to a serial port.
pub struct SerialLogger;

impl log::Log for SerialLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        // You can filter logs based on level
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level = record.level();
            let args = record.args();
            let message = format!("{}", args);

            // Here you would send the message to a serial port or some output
            // For example, `serial_write(message.as_bytes())`

            // print time since boot
            kprint!("[{:.6}]", clk::get_time_since_boot());
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

use log::LevelFilter;

#[cfg(feature = "trace_log")]
const LOG_LEVEL: LevelFilter = LevelFilter::Trace;

#[cfg(feature = "debug_log")]
const LOG_LEVEL: LevelFilter = LevelFilter::Debug;

#[cfg(feature = "warn_log")]
const LOG_LEVEL: LevelFilter = LevelFilter::Warn;

#[cfg(feature = "error_log")]
const LOG_LEVEL: LevelFilter = LevelFilter::Error;

#[cfg(all(
    feature = "info_log",
    not(any(
        feature = "trace_log",
        feature = "debug_log",
        feature = "warn_log",
        feature = "error_log"
    ))
))]
const LOG_LEVEL: LevelFilter = LevelFilter::Info;

pub fn init_logger() {
    log::set_logger(&SerialLogger)
        .map(|()| log::set_max_level(LOG_LEVEL))
        .unwrap();
}

pub static ASCII_ART: &str = r"______          _         _
| ___ \        | |       (_)
| |_/ /   _ ___| |_ _ __  ___  __
|    / | | / __| __| '_ \| \ \/ /
| |\ \ |_| \__ \ |_| | | | |>  <
\_| \_\__,_|___/\__|_| |_|_/_/\_\";

fn write_str_rainbow(s: &str) {
    let mut fg = Color::Red;
    for c in s.chars() {
        crate::vga::write_char(c, fg, Color::Black);
        if c == '\n' || c == ' ' {
            continue;
        }
        fg = match fg {
            Color::Red => Color::LightRed,
            Color::LightRed => Color::Yellow,
            Color::Yellow => Color::LightGreen,
            Color::LightGreen => Color::LightCyan,
            Color::LightCyan => Color::LightBlue,
            Color::LightBlue => Color::Magenta,
            Color::Magenta => Color::Pink,
            _ => Color::Red,
        };
    }
}

pub fn init(boot_info: &'static BootInfo) {
    system_msg!("Initializing kernel...");

    gdt::init();
    vga::info("GDT initialized");


    interrupts::init_idt();
    vga::info("IDT initialized");

    interrupts::init();
    vga::info("Interrupts enabled");

    clk::pit::init();
    vga::info("PIT initialized");

    memory::init(boot_info);
    vga::info("Memory initialized");

    init_logger();
    info!("Logger initialized");

    syscall::init();
    info!("Syscalls initialized");

    keyboard::init();
    info!("Console initialized");


    ata::init();
    info!("ATA initialized");

    #[cfg(not(test))] // tests don't have attached disk
    fs::init();
    info!("Filesystem initialized");

    acpi::init();
    info!("ACPI initialized");

    user::init();
    info!("Users initialized");

    serial_print!("Kernel initialized in {} ms", clk::get_boot_time_ns() / 1_000_000);

    #[cfg(feature = "ascii-art")]
    {
        write_str_rainbow(ASCII_ART);
        kprintln!();
    }
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
