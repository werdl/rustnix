use core::sync::atomic::AtomicBool;

use alloc::string::String;
use spin::Mutex;
use x86_64::instructions::interrupts;

use crate::kprint;

use crate::internal::file::Stream;

/// Backspace character
pub const BACKSPACE: char = '\x08';

/// End of text character (Ctrl+C)
pub const END_OF_TEXT: char = '\x03';

/// End of transmission character (Ctrl+D)
pub const END_OF_TRANSMISSION: char = '\x04';

/// Escape character
pub const ESC: char = '\x1B';

/// Console struct
#[derive(Debug)]
pub struct Console;

/// should input be echoed to the screen
pub static ECHO: AtomicBool = AtomicBool::new(true);

/// stdin buffer
pub static STDIN: Mutex<String> = Mutex::new(String::new());

/// raw mode flag
pub static RAW_MODE: AtomicBool = AtomicBool::new(false);

impl Console {
    /// Create a new Console
    pub fn new() -> Self {
        Console {}
    }
}

/// Read a single character from stdin
pub fn read_single_char() -> char {
    loop {
        let res = interrupts::without_interrupts(|| STDIN.lock().pop());

        if let Some(c) = res {
            return c;
        }
    }
}

/// enable raw mode
pub fn enable_raw_mode() {
    RAW_MODE.store(true, core::sync::atomic::Ordering::SeqCst);
}

/// disable raw mode
pub fn disable_raw_mode() {
    RAW_MODE.store(false, core::sync::atomic::Ordering::SeqCst);
}

/// check if raw mode is enabled
pub fn is_raw_mode() -> bool {
    RAW_MODE.load(core::sync::atomic::Ordering::SeqCst)
}

/// enable echo
pub fn enable_echo() {
    ECHO.store(true, core::sync::atomic::Ordering::SeqCst);
}

/// disable echo
pub fn disable_echo() {
    ECHO.store(false, core::sync::atomic::Ordering::SeqCst);
}

/// check if echo is enabled
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
