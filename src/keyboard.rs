use pc_keyboard::*;

use crate::{print, println};

use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::port::Port;

lazy_static! {
    static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
        Mutex::new(
            Keyboard::new(
                layouts::Us104Key, ScancodeSet1,
                HandleControl::Ignore
            )
        );
}

// called directly from the interrupt.
pub fn handle_keycode(scancode: u8)
{
    let mut keyboard = KEYBOARD.lock();

    if let Ok(Some(ev)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(ev) {
            match key {
                DecodedKey::Unicode(character) => println!("{}", character),
                DecodedKey::RawKey(key) => println!("{:?}", key),
            }
        }
    }
}
