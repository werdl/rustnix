use crate::internal::interrupts;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use x86_64::instructions::port::Port;

use x86_64::instructions::interrupts as x86_interrupts;

// At boot the PIT starts with a frequency divider of 0 (equivalent to 65536)
// which will result in about 54.926 ms between ticks.
// During init we will change the divider to 1193 to have about 1.000 ms
// between ticks to improve time measurements accuracy.
const PIT_FREQUENCY: f64 = 3_579_545.0 / 3.0; // 1_193_181.666 Hz
const PIT_DIVIDER: u16 = 1193;
/// Interval between PIT ticks in seconds
pub const PIT_INTERVAL: f64 = (PIT_DIVIDER as f64) / PIT_FREQUENCY;

static PIT_TICKS: AtomicUsize = AtomicUsize::new(0);
static TSC_FREQUENCY: AtomicU64 = AtomicU64::new(0);

/// Initialize the PIT
pub fn init() {
    crate::internal::vga::trace("Initializing PIT");
    unsafe {
        let mut port = Port::new(0x43);
        port.write(0x36u8); // Channel 0, lobyte/hibyte, rate generator, binary
        let mut port = Port::new(0x40);
        port.write((PIT_DIVIDER & 0xFF) as u8);
        let mut port = Port::new(0x40);
        port.write((PIT_DIVIDER >> 8) as u8);
    }

    interrupts::set_irq_handler(0, pit_handler);
    calibrate_tsc();
}

// now setup the interrupt handler
fn pit_handler() {
    PIT_TICKS.fetch_add(1, Ordering::SeqCst);
}

/// Get the current TSC value
pub fn get_tsc() -> u64 {
    unsafe {
        core::arch::x86_64::_mm_lfence(); // prevent instruction reordering
        core::arch::x86_64::_rdtsc() // do the actual read
    }
}

/// Get the number of PIT ticks since boot
pub fn get_ticks() -> usize {
    PIT_TICKS.load(Ordering::Relaxed)
}

/// Get the current time since boot in nanoseconds
pub fn get_boot_time_ns() -> u64 {
    let ticks = get_ticks() as f64;
    let tsc = get_tsc() as f64;
    let tsc_freq = TSC_FREQUENCY.load(Ordering::Relaxed) as f64;
    let seconds = ticks * PIT_INTERVAL;
    let nanos = (tsc / tsc_freq * 1_000_000_000.0) as u64;
    nanos + (seconds * 1_000_000_000.0) as u64
}

/// Sleep for a given number of seconds (based on PIT)
pub fn sleep(seconds: f64) {
    let start = get_ticks();
    let ticks = (seconds) / PIT_INTERVAL;
    while get_ticks() - start < ticks as usize {
        hlt();
    }
}

/// Calibrate the TSC frequency
pub fn calibrate_tsc() {
    let start = get_tsc();
    sleep(1.0);
    let end = get_tsc();
    TSC_FREQUENCY.store(end - start, Ordering::Relaxed);
}

/// Wait for a given number of nanoseconds (based on TSC)
pub fn wait(ns: u64) {
    let start = get_tsc();
    let tsc_freq = TSC_FREQUENCY.load(Ordering::Relaxed) as f64;
    let ticks = (ns as f64) / 1_000_000_000.0 * tsc_freq;
    while get_tsc() - start < ticks as u64 {
        hlt();
    }
}

/// Halt the CPU
pub fn hlt() {
    let disabled = !x86_interrupts::are_enabled();
    x86_interrupts::enable_and_hlt();
    if disabled {
        x86_interrupts::disable();
    }
}
