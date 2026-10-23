#![allow(non_upper_case_globals)]
//! IBus key event classification.

use super::ffi::*;

pub enum KeyAction {
    CommitAndPassThrough,
    Backspace,
    Space,
    Return,
    Escape,
    Modifier,
    TypeKeypad(u32),
    TypeChar(char),
    PassThrough,
}

pub fn classify_key(keyval: u32, state: u32) -> KeyAction {
    let ctrl = state & IBUS_CONTROL_MASK != 0;
    let alt = state & IBUS_MOD1_MASK != 0;
    let alt_gr = state & IBUS_MOD5_MASK != 0;
    if (ctrl && !alt) || (alt && !alt_gr) || state & IBUS_MOD4_MASK != 0 {
        return KeyAction::CommitAndPassThrough;
    }

    match keyval {
        IBUS_KEY_BackSpace => KeyAction::Backspace,
        IBUS_KEY_space => KeyAction::Space,
        IBUS_KEY_Return | IBUS_KEY_KP_Enter => KeyAction::Return,
        IBUS_KEY_Escape => KeyAction::Escape,
        IBUS_KEY_Tab | IBUS_KEY_KP_Tab => KeyAction::CommitAndPassThrough,
        IBUS_KEY_Left
        | IBUS_KEY_Right
        | IBUS_KEY_Up
        | IBUS_KEY_Down
        | IBUS_KEY_Home
        | IBUS_KEY_End
        | IBUS_KEY_Page_Up
        | IBUS_KEY_Page_Down
        | IBUS_KEY_Delete => KeyAction::CommitAndPassThrough,
        IBUS_KEY_Shift_L
        | IBUS_KEY_Shift_R
        | IBUS_KEY_Control_L
        | IBUS_KEY_Control_R
        | IBUS_KEY_Alt_L
        | IBUS_KEY_Alt_R
        | IBUS_KEY_Meta_L
        | IBUS_KEY_Meta_R
        | IBUS_KEY_Super_L
        | IBUS_KEY_Super_R
        | IBUS_KEY_Caps_Lock
        | IBUS_KEY_Shift_Lock
        | IBUS_KEY_ISO_Level3_Shift => KeyAction::Modifier,
        IBUS_KEY_KP_0..=IBUS_KEY_KP_9
        | IBUS_KEY_KP_Add
        | IBUS_KEY_KP_Subtract
        | IBUS_KEY_KP_Multiply
        | IBUS_KEY_KP_Divide
        | IBUS_KEY_KP_Decimal => KeyAction::TypeKeypad(keyval),
        0x0021..=0x007e => char::from_u32(keyval)
            .map(KeyAction::TypeChar)
            .unwrap_or(KeyAction::PassThrough),
        _ => KeyAction::PassThrough,
    }
}
