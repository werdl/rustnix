const BUF_HEIGHT: usize = 25;
const BUF_WIDTH: usize = 80;

use core::fmt;

use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;
use x86_64::instructions::{interrupts, port::Port};

#[allow(dead_code)]
#[allow(missing_docs)] // no need to document this, only colours
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGrey = 7,
    DarkGrey = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

/// A combination of a foreground and background color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    /// Create a new ColorCode with the given foreground and background colors
    pub const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

/// A character in the VGA buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct VgaChar {
    ascii_char: u8,
    color_code: ColorCode,
}

/// The VGA buffer
#[repr(transparent)]
pub struct Buffer {
    chars: [[Volatile<VgaChar>; BUF_WIDTH]; BUF_HEIGHT],
}

/// The VGA writer
pub struct VgaWriter {
    col_pos: usize,
    color_code: ColorCode,
    buf: &'static mut Buffer,
}

impl VgaWriter {
    /// Create a new VgaWriter with the given foreground and background colors
    pub fn new(fg: Color, bg: Color) -> VgaWriter {
        let buf = unsafe { &mut *(0xb8000 as *mut Buffer) };

        VgaWriter {
            col_pos: 0,
            color_code: ColorCode::new(fg, bg),
            buf,
        }
    }

    /// Write a byte to the VGA buffer
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            0x08 => {
                if self.col_pos > 0 {
                    self.col_pos -= 1;
                    // manually write a space to clear the character
                    let row = BUF_HEIGHT - 1;
                    let col = self.col_pos;
                    self.buf.chars[row][col].write(VgaChar {
                        ascii_char: b' ',
                        color_code: self.color_code,
                    });
                }
            }
            byte => {
                if self.col_pos >= BUF_WIDTH {
                    self.new_line();
                }

                let row = BUF_HEIGHT - 1;
                let col = self.col_pos;

                self.buf.chars[row][col].write(VgaChar {
                    ascii_char: byte,
                    color_code: self.color_code,
                });

                self.col_pos += 1;
            }
        }
    }

    /// Clear a row in the VGA buffer
    pub fn clear_row(&mut self, row: usize) {
        let blank = VgaChar {
            ascii_char: b' ',
            color_code: self.color_code,
        };

        for col in 0..BUF_WIDTH {
            self.buf.chars[row][col].write(blank);
        }
    }

    /// Move to a new line in the VGA buffer
    pub fn new_line(&mut self) {
        for row in 1..BUF_HEIGHT {
            for col in 0..BUF_WIDTH {
                let character = &self.buf.chars[row][col];
                self.buf.chars[row - 1][col].write(character.read());
            }
        }

        self.clear_row(BUF_HEIGHT - 1);
        self.col_pos = 0;
    }

    /// Write a string to the VGA buffer
    pub fn write_str(&mut self, s: &str) {
        for c in s.chars() {
            match c as u8 {
                b'\n' => self.write_byte(b'\n'),
                0x20..=0x7e => self.write_byte(c as u8),
                _ => self.write_byte(0xfe), // ■ for unsupported characters
            }
        }
    }
}

impl fmt::Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

/// Write a character to the VGA buffer
pub fn write_char(c: char, fg: Color, bg: Color) {
    interrupts::without_interrupts(|| {
        let mut writer = VGA_WRITER.lock();
        let old_color = writer.color_code;
        writer.color_code = ColorCode::new(fg, bg);
        writer.write_byte(c as u8);
        writer.color_code = old_color;
    });
}

/// Write a string to the VGA buffer
pub fn write_str(s: &str, fg: Color, bg: Color) {
    interrupts::without_interrupts(|| {
        for c in s.chars() {
            write_char(c, fg, bg);
        }

        // TODO: cursor
    });
}

/// Clear the last character in the VGA buffer
pub fn clear_last_char() {
    interrupts::without_interrupts(|| {
        let mut writer = VGA_WRITER.lock();
        if writer.col_pos > 0 {
            writer.col_pos -= 1;
            writer.write_byte(b' ');
            writer.col_pos -= 1;
        }
    });
}

/// Clear the entire VGA buffer
pub fn clear_screen() {
    interrupts::without_interrupts(|| {
        let mut writer = VGA_WRITER.lock();
        for row in 0..BUF_HEIGHT {
            writer.clear_row(row);
        }
        writer.col_pos = 0;
    });
}

lazy_static! {
    /// The global VGA writer
    pub static ref VGA_WRITER: Mutex<VgaWriter> =
        spin::Mutex::new(VgaWriter::new(Color::White, Color::Black));
}

#[doc(hidden)] // needs to be public for the print! macro, but shouldn't be used directly
pub fn _kprint(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        VGA_WRITER.lock().write_fmt(args).unwrap();
    });
}

/// Print to the VGA buffer
#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => ($crate::internal::vga::_kprint(core::format_args!($($arg)*)));
}

/// Print to the VGA buffer with a newline
#[macro_export]
macro_rules! kprintln {
    () => ($crate::kprint!("\n"));
    ($($arg:tt)*) => ($crate::kprint!("{}\n", core::format_args!($($arg)*)));
}

/// test a single kprintln
#[test_case]
fn test_single_kprintln() {
    kprintln!("test_kprintln_simple output");
}

/// test 200 kprintlns
#[test_case]
fn test_many_kprintln() {
    for _ in 0..200 {
        kprintln!("test_kprintln_many output");
    }
}

/// test kprintln output is correct
#[test_case]
fn test_kprintln_output() {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    let s = "Hello world! Enjoy some numbers: 42 ";
    interrupts::without_interrupts(|| {
        let mut writer = VGA_WRITER.lock();
        writeln!(writer, "\n{}", s).expect("writeln failed");
        for (i, c) in s.chars().enumerate() {
            let screen_char = writer.buf.chars[BUF_HEIGHT - 2][i].read();
            assert_eq!(char::from(screen_char.ascii_char), c);
        }
    });
}
