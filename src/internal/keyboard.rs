use core::sync::atomic::{AtomicBool, Ordering};

use pc_keyboard::{layouts::Uk105Key, DecodedKey, HandleControl, KeyCode, KeyState, Keyboard, ScancodeSet1};
use x86_64::instructions::port::Port;

use super::console::handle_key;

pub static CTRL: AtomicBool = AtomicBool::new(false);
pub static ALT: AtomicBool = AtomicBool::new(false);
pub static SHIFT: AtomicBool = AtomicBool::new(false);
pub static CAPS: AtomicBool = AtomicBool::new(false);

fn read_scancode() -> u8 {
    let mut port = Port::new(0x60);
    unsafe { port.read() }
}

fn handle_csi(code: &str) {
    handle_key('\x1B'); // ESC
    handle_key('[');
    for c in code.chars() {
        handle_key(c);
    }
}

fn interrupt_handler() {
    let scancode = read_scancode();
    let mut kb = Keyboard::<Uk105Key, ScancodeSet1>::new(
        ScancodeSet1::new(),
        Uk105Key,
        HandleControl::MapLettersToUnicode,
    );

    if let Ok(Some(event)) = kb.add_byte(scancode) {
        // first, ctrl, alt, deleteshift
        let ord = Ordering::Relaxed;
        match event.code {
            KeyCode::LAlt | KeyCode::RAltGr => ALT.store(event.state == KeyState::Down, ord),
            KeyCode::LShift | KeyCode::RShift => SHIFT.store(event.state == KeyState::Down, ord),
            KeyCode::LControl | KeyCode::RControl => CTRL.store(event.state == KeyState::Down, ord),
            _ => {}
        }
        let is_alt = ALT.load(ord);
        let is_ctrl = CTRL.load(ord);
        let is_shift = SHIFT.load(ord);

        if let Some(key) = kb.process_keyevent(event) {
            match key {
                DecodedKey::RawKey(KeyCode::PageUp) => handle_csi("5~"),
                DecodedKey::RawKey(KeyCode::PageDown) => handle_csi("6~"),
                DecodedKey::RawKey(KeyCode::ArrowUp) => handle_csi("A"),
                DecodedKey::RawKey(KeyCode::ArrowDown) => handle_csi("B"),
                DecodedKey::RawKey(KeyCode::ArrowRight) => handle_csi("C"),
                DecodedKey::RawKey(KeyCode::ArrowLeft) => handle_csi("D"),

                // Convert Shift-Tab into Backtab
                DecodedKey::Unicode('\t') if is_shift => handle_csi("Z"),

                DecodedKey::Unicode(c) => handle_key(c),

                _ => {}
            }
        }
    }
}

pub fn init() {
    crate::internal::interrupts::set_irq_handler(1, interrupt_handler);
}
