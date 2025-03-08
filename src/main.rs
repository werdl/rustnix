#![no_std]
#![no_main]

mod language_features;
mod file;
mod vga;

use vga::{Color, ColorCode, Buffer, VgaChar, VgaWriter};

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("Hello, World!");

    loop {}
}