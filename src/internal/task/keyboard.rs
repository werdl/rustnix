use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use log::warn;

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
pub static PRESSED_KEYS: spin::Mutex<alloc::collections::BTreeSet<u8>> = spin::Mutex::new(alloc::collections::BTreeSet::new());

pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            warn!("WARNING: scancode queue full; dropping keyboard input");
        } else {
            WAKER.wake();
        }
    } else {
        warn!("WARNING: scancode queue uninitialized");
    }
}
pub struct ScancodeStream {
    /// this way, the only way to create a ScancodeStream is through the new function
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");

        ScancodeStream { _private: () }
    }
}

use core::{pin::Pin, task::{Context, Poll}};
use futures_util::stream::Stream;

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("scancode queue not initialized");

        // fast path
        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(&cx.waker());
        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}
use futures_util::task::AtomicWaker;

static WAKER: AtomicWaker = AtomicWaker::new();

use futures_util::stream::StreamExt;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use crate::{kprint, internal::vga};

pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(ScancodeSet1::new(),
        layouts::Uk105Key, HandleControl::Ignore);

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => kprint!("{}", character),
                    DecodedKey::RawKey(key) => kprint!("{:?}", key),
                }
            }
        }
    }
}

pub async fn handle_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(ScancodeSet1::new(),
        layouts::Uk105Key, HandleControl::Ignore);

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => {
                        if character as u8  == 0x08 {
                            vga::clear_last_char();
                            continue;
                        }
                        kprint!("{}", character);
                        PRESSED_KEYS.lock().insert(character as u8);
                    },
                    DecodedKey::RawKey(key) => {
                        kprint!("{:?}", key as u8);
                    },

                }
            }
        }
    }
}
