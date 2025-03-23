use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;

lazy_static! {
    /// Serial port 1
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(0x3f8) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

#[doc(hidden)] // needs to be public for the serial_print! macro, but shouldn't be used directly
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        SERIAL1
            .lock()
            .write_fmt(args)
            .expect("Printing to serial failed");
    });
}

/// Print to the serial port
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::internal::serial::_print(core::format_args!($($arg)*));
    };
}

/// Print to the serial port with a newline
#[macro_export]
macro_rules! serial_println {
    () => {
        $crate::serial_print!("\n");
    };
    ($($arg:tt)*) => {
        $crate::serial_print!("{}\n", core::format_args!($($arg)*));
    };
}
