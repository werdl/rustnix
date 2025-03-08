const BUF_HEIGHT: usize = 25;
const BUF_WIDTH: usize = 80;

use core::fmt;

use volatile::Volatile;
use lazy_static::lazy_static;
use spin::Mutex;

#[allow(dead_code)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    pub const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct VgaChar {
    ascii_char: u8,
    color_code: ColorCode,
}

#[repr(transparent)]
pub struct Buffer {
    chars: [[Volatile<VgaChar>; BUF_WIDTH]; BUF_HEIGHT],
}

pub struct VgaWriter {
    col_pos: usize,
    color_code: ColorCode,
    buf: &'static mut Buffer,
}

impl VgaWriter {
    pub fn new(fg: Color, bg: Color) -> VgaWriter {
        let buf = unsafe { &mut *(0xb8000 as *mut Buffer) };

        VgaWriter {
            col_pos: 0,
            color_code: ColorCode::new(fg, bg),
            buf,
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
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

    pub fn clear_row(&mut self, row: usize) {
        let blank = VgaChar {
            ascii_char: b' ',
            color_code: self.color_code,
        };

        for col in 0..BUF_WIDTH {
            self.buf.chars[row][col].write(blank);
        }
    }

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

    pub fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                _ => self.write_byte(0xfe), // ■
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

lazy_static! {
    pub static ref VGA_WRITER: Mutex<VgaWriter> = spin::Mutex::new(VgaWriter::new(Color::White, Color::Black));
}

#[doc(hidden)] // needs to be public for the print! macro, but shouldn't be used directly
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    VGA_WRITER.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}