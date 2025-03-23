#![warn(missing_docs)]

/// acpi module, handles acpi tables (basically allows shutdown and logs some system info)
pub mod acpi;
/// allocator module, handles heap allocation
pub mod allocator;
/// ata module, handles ata devices
pub mod ata;
/// clk module, handles clock and related interrupts
pub mod clk;
/// console module, handles console input
pub mod console;
/// devices module, handles devices
pub mod devices;
/// file module, handles file types and trait definitions
pub mod file;
/// fs module, handles file system operations
pub mod fs;
/// gdt module, handles global descriptor table
pub mod gdt;
/// interrupts module, handles interrupt handling
pub mod interrupts;
/// io module, handles io operations
pub mod io;
/// keyboard module, handles keyboard input and related interrupts
pub mod keyboard;
/// memory module, handles memory operations
pub mod memory;
/// process module, not yet implemented
pub mod process;
/// serial module, handles serial output
pub mod serial;
/// syscall module, handles system calls
pub mod syscall;
/// task module, handles task scheduling and execution
pub mod task;
/// vga module, handles vga output
pub mod vga;
