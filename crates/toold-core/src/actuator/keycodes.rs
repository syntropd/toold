//! Linux evdev input event codes and ASCII character mapping.

pub const EV_SYN: u16 = 0x00;
pub const EV_KEY: u16 = 0x01;
pub const EV_REL: u16 = 0x02;
pub const EV_ABS: u16 = 0x03;

pub const SYN_REPORT: u16 = 0x00;

pub const REL_X: u16 = 0x00;
pub const REL_Y: u16 = 0x01;
pub const REL_WHEEL: u16 = 0x08;

pub const ABS_X: u16 = 0x00;
pub const ABS_Y: u16 = 0x01;

pub const BTN_LEFT: u16 = 0x110;
pub const BTN_RIGHT: u16 = 0x111;
pub const BTN_MIDDLE: u16 = 0x112;

pub const KEY_ESC: u16 = 1;
pub const KEY_1: u16 = 2;
pub const KEY_2: u16 = 3;
pub const KEY_3: u16 = 4;
pub const KEY_4: u16 = 5;
pub const KEY_5: u16 = 6;
pub const KEY_6: u16 = 7;
pub const KEY_7: u16 = 8;
pub const KEY_8: u16 = 9;
pub const KEY_9: u16 = 10;
pub const KEY_0: u16 = 11;
pub const KEY_MINUS: u16 = 12;
pub const KEY_EQUAL: u16 = 13;
pub const KEY_BACKSPACE: u16 = 14;
pub const KEY_TAB: u16 = 15;
pub const KEY_Q: u16 = 16;
pub const KEY_W: u16 = 17;
pub const KEY_E: u16 = 18;
pub const KEY_R: u16 = 19;
pub const KEY_T: u16 = 20;
pub const KEY_Y: u16 = 21;
pub const KEY_U: u16 = 22;
pub const KEY_I: u16 = 23;
pub const KEY_O: u16 = 24;
pub const KEY_P: u16 = 25;
pub const KEY_LEFTBRACE: u16 = 26;
pub const KEY_RIGHTBRACE: u16 = 27;
pub const KEY_ENTER: u16 = 28;
pub const KEY_LEFTCTRL: u16 = 29;
pub const KEY_A: u16 = 30;
pub const KEY_S: u16 = 31;
pub const KEY_D: u16 = 32;
pub const KEY_F: u16 = 33;
pub const KEY_G: u16 = 34;
pub const KEY_H: u16 = 35;
pub const KEY_J: u16 = 36;
pub const KEY_K: u16 = 37;
pub const KEY_L: u16 = 38;
pub const KEY_SEMICOLON: u16 = 39;
pub const KEY_APOSTROPHE: u16 = 40;
pub const KEY_GRAVE: u16 = 41;
pub const KEY_LEFTSHIFT: u16 = 42;
pub const KEY_BACKSLASH: u16 = 43;
pub const KEY_Z: u16 = 44;
pub const KEY_X: u16 = 45;
pub const KEY_C: u16 = 46;
pub const KEY_V: u16 = 47;
pub const KEY_B: u16 = 48;
pub const KEY_N: u16 = 49;
pub const KEY_M: u16 = 50;
pub const KEY_COMMA: u16 = 51;
pub const KEY_DOT: u16 = 52;
pub const KEY_SLASH: u16 = 53;
pub const KEY_RIGHTSHIFT: u16 = 54;
pub const KEY_SPACE: u16 = 57;

/// Maps a unicode/ASCII char to an evdev keycode and a shift modifier flag.
pub fn char_to_keycode(ch: char) -> Option<(u16, bool)> {
    match ch {
        'a'..='z' => {
            let offset = (ch as u32) - ('a' as u32);
            let code = match offset {
                0 => KEY_A,
                1 => KEY_B,
                2 => KEY_C,
                3 => KEY_D,
                4 => KEY_E,
                5 => KEY_F,
                6 => KEY_G,
                7 => KEY_H,
                8 => KEY_I,
                9 => KEY_J,
                10 => KEY_K,
                11 => KEY_L,
                12 => KEY_M,
                13 => KEY_N,
                14 => KEY_O,
                15 => KEY_P,
                16 => KEY_Q,
                17 => KEY_R,
                18 => KEY_S,
                19 => KEY_T,
                20 => KEY_U,
                21 => KEY_V,
                22 => KEY_W,
                23 => KEY_X,
                24 => KEY_Y,
                _ => KEY_Z,
            };
            Some((code, false))
        }
        'A'..='Z' => {
            let offset = (ch as u32) - ('A' as u32);
            let code = match offset {
                0 => KEY_A,
                1 => KEY_B,
                2 => KEY_C,
                3 => KEY_D,
                4 => KEY_E,
                5 => KEY_F,
                6 => KEY_G,
                7 => KEY_H,
                8 => KEY_I,
                9 => KEY_J,
                10 => KEY_K,
                11 => KEY_L,
                12 => KEY_M,
                13 => KEY_N,
                14 => KEY_O,
                15 => KEY_P,
                16 => KEY_Q,
                17 => KEY_R,
                18 => KEY_S,
                19 => KEY_T,
                20 => KEY_U,
                21 => KEY_V,
                22 => KEY_W,
                23 => KEY_X,
                24 => KEY_Y,
                _ => KEY_Z,
            };
            Some((code, true))
        }
        '1' => Some((KEY_1, false)),
        '2' => Some((KEY_2, false)),
        '3' => Some((KEY_3, false)),
        '4' => Some((KEY_4, false)),
        '5' => Some((KEY_5, false)),
        '6' => Some((KEY_6, false)),
        '7' => Some((KEY_7, false)),
        '8' => Some((KEY_8, false)),
        '9' => Some((KEY_9, false)),
        '0' => Some((KEY_0, false)),
        '!' => Some((KEY_1, true)),
        '@' => Some((KEY_2, true)),
        '#' => Some((KEY_3, true)),
        '$' => Some((KEY_4, true)),
        '%' => Some((KEY_5, true)),
        '^' => Some((KEY_6, true)),
        '&' => Some((KEY_7, true)),
        '*' => Some((KEY_8, true)),
        '(' => Some((KEY_9, true)),
        ')' => Some((KEY_0, true)),
        ' ' => Some((KEY_SPACE, false)),
        '\n' | '\r' => Some((KEY_ENTER, false)),
        '\t' => Some((KEY_TAB, false)),
        '-' => Some((KEY_MINUS, false)),
        '_' => Some((KEY_MINUS, true)),
        '=' => Some((KEY_EQUAL, false)),
        '+' => Some((KEY_EQUAL, true)),
        '[' => Some((KEY_LEFTBRACE, false)),
        '{' => Some((KEY_LEFTBRACE, true)),
        ']' => Some((KEY_RIGHTBRACE, false)),
        '}' => Some((KEY_RIGHTBRACE, true)),
        '\\' => Some((KEY_BACKSLASH, false)),
        '|' => Some((KEY_BACKSLASH, true)),
        ';' => Some((KEY_SEMICOLON, false)),
        ':' => Some((KEY_SEMICOLON, true)),
        '\'' => Some((KEY_APOSTROPHE, false)),
        '"' => Some((KEY_APOSTROPHE, true)),
        '`' => Some((KEY_GRAVE, false)),
        '~' => Some((KEY_GRAVE, true)),
        ',' => Some((KEY_COMMA, false)),
        '<' => Some((KEY_COMMA, true)),
        '.' => Some((KEY_DOT, false)),
        '>' => Some((KEY_DOT, true)),
        '/' => Some((KEY_SLASH, false)),
        '?' => Some((KEY_SLASH, true)),
        _ => None,
    }
}
