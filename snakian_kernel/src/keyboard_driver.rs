// A keyboard driver for the OS. Handles keyboard input such as key presses and key releases.

use lazy_static::lazy_static;
use pc_keyboard::{layouts, DecodedKey, KeyCode, KeyEvent, KeyState, Keyboard, ScancodeSet1};
use spin::Mutex;

use crate::println;

lazy_static! {
    pub static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
        Mutex::new(Keyboard::new(
            ScancodeSet1::new(),
            layouts::Us104Key,
            pc_keyboard::HandleControl::Ignore
        ));
}

pub static KEYBOARD_DRIVER: Mutex<KeyboardDriver> = Mutex::new(KeyboardDriver::new());

/// The keyboard driver. Handles keyboard input such as key presses and key releases.
pub struct KeyboardDriver {
    /// The array of keys that are currently pressed; true if pressed, false if not pressed.
    pub pressed_keys: [bool; 128],
    pub current_char: Option<char>,
    pub current_char_as_key: Option<KeyCode>,
    pub current_key: Option<KeyCode>,
    pub is_unicode: bool,
}

impl KeyboardDriver {
    const fn new() -> KeyboardDriver {
        KeyboardDriver {
            pressed_keys: [false; 128],
            current_char: None,
            current_char_as_key: None,
            current_key: None,
            is_unicode: false,
        }
    }

    /// Handles a keyboard interrupt. This function is called by the interrupt handler.
    pub fn handle_key_event(&mut self, event: KeyEvent) {
        let decode = KEYBOARD.lock().process_keyevent(event.clone());
        if let Some(key) = decode {
            self.set_char_or_key(key, event.code); // never nesting :)
            self.pressed_keys[event.code as usize] = true;
        } else {
            self.pressed_keys[event.code as usize] = false;
        }
        self.update_pressed(event);
    }

    pub fn handle_byte(&mut self, byte: u8) {
        let decode = KEYBOARD
            .lock()
            .add_byte(byte)
            .expect("Failed to add byte to keyboard buffer");
        if let Some(key) = decode {
            self.handle_key_event(key);
        }
    }

    fn set_char_or_key(&mut self, event: DecodedKey, code: KeyCode) {
        match event {
            pc_keyboard::DecodedKey::Unicode(character) => {
                self.current_char = Some(character);
                self.current_key = None; // make sure we arent printing stale data
                self.current_char_as_key = Some(code);
                self.is_unicode = true;
            }
            pc_keyboard::DecodedKey::RawKey(key) => {
                self.current_key = Some(key);
                self.current_char = None;
                self.is_unicode = false;
            }
        }
    }

    fn update_pressed(&mut self, event: KeyEvent) {
        let is_key_up = event.state == KeyState::Up;
        self.pressed_keys[event.code as usize] = !is_key_up;

        if is_key_up {
            self.current_char = None;
            self.current_key = None;
        }
    }
}
