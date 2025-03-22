use core::sync::atomic::AtomicBool;

use alloc::string::String;
use spin::Mutex;
use x86_64::instructions::interrupts;

use crate::kprint;

use crate::internal::file::Stream;

pub const BACKSPACE: char = '\x08';
pub const END_OF_TEXT: char = '\x03';
pub const END_OF_TRANSMISSION: char = '\x04';
pub const ESC: char = '\x1B';

#[derive(Debug)]
pub struct Console {
    stdin_index: usize,
}

pub static ECHO: AtomicBool = AtomicBool::new(true);
pub static STDIN: Mutex<String> = Mutex::new(String::new());
pub static RAW_MODE: AtomicBool = AtomicBool::new(false);

impl Console {
    pub fn new() -> Self {
        Console { stdin_index: 0 }
    }
}

pub fn read_single_char() -> char {
    loop {
        let res = interrupts::without_interrupts(|| STDIN.lock().pop());

        if let Some(c) = res {
            return c;
        }
    }
}

pub fn read_line() -> String {
    loop {
        let res = interrupts::without_interrupts(|| {
            let mut stdin = STDIN.lock();
            match stdin.chars().next_back() {
                Some('\n') => {
                    let line = stdin.clone();
                    stdin.clear();
                    Some(line)
                }
                _ => None,
            }
        });
        if let Some(line) = res {
            return line;
        }
    }
}

pub fn enable_raw_mode() {
    RAW_MODE.store(true, core::sync::atomic::Ordering::SeqCst);
}

pub fn disable_raw_mode() {
    RAW_MODE.store(false, core::sync::atomic::Ordering::SeqCst);
}

pub fn is_raw_mode() -> bool {
    RAW_MODE.load(core::sync::atomic::Ordering::SeqCst)
}

pub fn enable_echo() {
    ECHO.store(true, core::sync::atomic::Ordering::SeqCst);
}

pub fn disable_echo() {
    ECHO.store(false, core::sync::atomic::Ordering::SeqCst);
}

pub fn is_echo() -> bool {
    ECHO.load(core::sync::atomic::Ordering::SeqCst)
}

/// handler function or keyboard interrupts
pub fn handle_key(key: char) {
    if key == BACKSPACE && !is_raw_mode() {
        // ^C - two backspaces to remove the ^C
        if let Some(c) = interrupts::without_interrupts(|| STDIN.lock().pop()) {
            let n_bs = match c {
                END_OF_TEXT | END_OF_TRANSMISSION | ESC => 2,
                _ => {
                    if (c as u32) < 0xFF {
                        1
                    } else {
                        c.len_utf8()
                    }
                }
            };

            for _ in 0..n_bs {
                kprint!("{}", "\x08".repeat(n_bs));
            }
        }
    } else {
        STDIN.lock().push(key);

        if is_echo() {
            match key {
                END_OF_TEXT => {
                    kprint!("{}", "^C");
                }
                END_OF_TRANSMISSION => {
                    kprint!("{}", "^D");
                }
                ESC => {
                    kprint!("{}", "^[");
                }
                _ => {
                    kprint!("{}", key);
                }
            }
        }
    }
}

impl Stream for Console {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, crate::internal::file::FileError> {
        // read buf.len() bytes from stdin
        let mut i = 0;
        while i < buf.len() {
            buf[i] = read_single_char() as u8;
            i += 1;
        }
        self.stdin_index += i;
        Ok(i)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, crate::internal::file::FileError> {
        let string = String::from_utf8_lossy(buf);
        kprint!("{}", string);
        Ok(buf.len())
    }

    fn close(&mut self) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), crate::internal::file::FileError> {
        Ok(())
    }

    fn poll(&mut self, _event: crate::internal::file::IOEvent) -> bool {
        true
    }
}
