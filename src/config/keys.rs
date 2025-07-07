use serde::{Deserialize, Serialize};
use rdev::Key;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyWrapper(pub String);

impl KeyWrapper {
    pub fn as_key(&self) -> Option<Key> {
        string_to_key(&self.0)
    }

    pub fn from_key(key: Key) -> Self {
        KeyWrapper(key_to_string(&key))
    }
}

impl Default for KeyWrapper {
    fn default() -> Self {
        KeyWrapper("".to_string())
    }
}

pub fn string_to_key(s: &str) -> Option<Key> {
    use Key::*;
    Some(match s.to_uppercase().as_str() {
        "A" => Key::KeyA,
        "B" => Key::KeyB,
        "C" => Key::KeyC,
        "D" => Key::KeyD,
        "E" => Key::KeyE,
        "F" => Key::KeyF,
        "G" => Key::KeyG,
        "H" => Key::KeyH,
        "I" => Key::KeyI,
        "J" => Key::KeyJ,
        "K" => Key::KeyK,
        "L" => Key::KeyL,
        "M" => Key::KeyM,
        "N" => Key::KeyN,
        "O" => Key::KeyO,
        "P" => Key::KeyP,
        "Q" => Key::KeyQ,
        "R" => Key::KeyR,
        "S" => Key::KeyS,
        "T" => Key::KeyT,
        "U" => Key::KeyU,
        "V" => Key::KeyV,
        "W" => Key::KeyW,
        "X" => Key::KeyX,
        "Y" => Key::KeyY,
        "Z" => Key::KeyZ,
        "1" => Key::Num1,
        "2" => Key::Num2,
        "3" => Key::Num3,
        "4" => Key::Num4,
        "5" => Key::Num5,
        "6" => Key::Num6,
        "7" => Key::Num7,
        "8" => Key::Num8,
        "9" => Key::Num9,
        "0" => Key::Num0,
        "ENTER" => Return,
        "ESC" | "ESCAPE" => Escape,
        "BACKSPACE" => Backspace,
        "TAB" => Tab,
        "SPACE" => Space,
        "MINUS" => Minus,
        "EQUAL" => Equal,
        "LEFTBRACKET" => LeftBracket,
        "RIGHTBRACKET" => RightBracket,
        "BACKSLASH" => BackSlash,
        "SEMICOLON" => SemiColon,
        "APOSTROPHE" => Quote,
        "GRAVE" => BackQuote,
        "COMMA" => Comma,
        "DOT" | "PERIOD" => Dot,
        "SLASH" => Slash,
        "CAPSLOCK" => CapsLock,
        "F1" => F1,
        "F2" => F2,
        "F3" => F3,
        "F4" => F4,
        "F5" => F5,
        "F6" => F6,
        "F7" => F7,
        "F8" => F8,
        "F9" => F9,
        "F10" => F10,
        "F11" => F11,
        "F12" => F12,
        "INSERT" => Insert,
        "DELETE" => Delete,
        "HOME" => Home,
        "END" => End,
        "PAGEUP" => PageUp,
        "PAGEDOWN" => PageDown,
        "LEFT" => LeftArrow,
        "RIGHT" => RightArrow,
        "UP" => UpArrow,
        "DOWN" => DownArrow,
        _ => return None,
    })
}

pub fn key_to_string(key: &Key) -> String {
    format!("{:?}", key).to_uppercase()
}
