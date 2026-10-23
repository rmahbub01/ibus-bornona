#![allow(non_upper_case_globals, dead_code)]
//! C FFI bindings to libibus-1.0 and gobject/glib.

use std::ffi::{c_char, c_int, c_uint, c_void};

pub type IBusBus = c_void;
pub type IBusFactory = c_void;
pub type IBusEngine = c_void;
pub type IBusText = c_void;
pub type IBusComponent = c_void;
pub type IBusEngineDesc = c_void;
pub type GDBusConnection = c_void;
pub type GCallback = unsafe extern "C" fn();

// IBus Modifiers and Key Masks
pub const IBUS_SHIFT_MASK: u32 = 1 << 0;
pub const IBUS_LOCK_MASK: u32 = 1 << 1;
pub const IBUS_CONTROL_MASK: u32 = 1 << 2;
pub const IBUS_MOD1_MASK: u32 = 1 << 3; // Alt
pub const IBUS_MOD4_MASK: u32 = 1 << 6; // Super / Meta
pub const IBUS_MOD5_MASK: u32 = 1 << 7; // AltGr / Level 3
pub const IBUS_RELEASE_MASK: u32 = 1 << 30;

// Key constants
pub const IBUS_KEY_BackSpace: u32 = 0xff08;
pub const IBUS_KEY_Tab: u32 = 0xff09;
pub const IBUS_KEY_Return: u32 = 0xff0d;
pub const IBUS_KEY_Escape: u32 = 0xff1b;
pub const IBUS_KEY_Delete: u32 = 0xffff;
pub const IBUS_KEY_Home: u32 = 0xff50;
pub const IBUS_KEY_Left: u32 = 0xff51;
pub const IBUS_KEY_Up: u32 = 0xff52;
pub const IBUS_KEY_Right: u32 = 0xff53;
pub const IBUS_KEY_Down: u32 = 0xff54;
pub const IBUS_KEY_Page_Up: u32 = 0xff55;
pub const IBUS_KEY_Page_Down: u32 = 0xff56;
pub const IBUS_KEY_End: u32 = 0xff57;
pub const IBUS_KEY_space: u32 = 0x0020;

// Keypad
pub const IBUS_KEY_KP_Enter: u32 = 0xff8d;
pub const IBUS_KEY_KP_Space: u32 = 0xff80;
pub const IBUS_KEY_KP_Tab: u32 = 0xff89;
pub const IBUS_KEY_KP_Multiply: u32 = 0xffaa;
pub const IBUS_KEY_KP_Add: u32 = 0xffab;
pub const IBUS_KEY_KP_Subtract: u32 = 0xffad;
pub const IBUS_KEY_KP_Decimal: u32 = 0xffae;
pub const IBUS_KEY_KP_Divide: u32 = 0xffaf;
pub const IBUS_KEY_KP_0: u32 = 0xffb0;
pub const IBUS_KEY_KP_1: u32 = 0xffb1;
pub const IBUS_KEY_KP_2: u32 = 0xffb2;
pub const IBUS_KEY_KP_3: u32 = 0xffb3;
pub const IBUS_KEY_KP_4: u32 = 0xffb4;
pub const IBUS_KEY_KP_5: u32 = 0xffb5;
pub const IBUS_KEY_KP_6: u32 = 0xffb6;
pub const IBUS_KEY_KP_7: u32 = 0xffb7;
pub const IBUS_KEY_KP_8: u32 = 0xffb8;
pub const IBUS_KEY_KP_9: u32 = 0xffb9;

// Modifiers
pub const IBUS_KEY_Shift_L: u32 = 0xffe1;
pub const IBUS_KEY_Shift_R: u32 = 0xffe2;
pub const IBUS_KEY_Control_L: u32 = 0xffe3;
pub const IBUS_KEY_Control_R: u32 = 0xffe4;
pub const IBUS_KEY_Caps_Lock: u32 = 0xffe5;
pub const IBUS_KEY_Shift_Lock: u32 = 0xffe6;
pub const IBUS_KEY_Meta_L: u32 = 0xffe7;
pub const IBUS_KEY_Meta_R: u32 = 0xffe8;
pub const IBUS_KEY_Alt_L: u32 = 0xffe9;
pub const IBUS_KEY_Alt_R: u32 = 0xffea;
pub const IBUS_KEY_Super_L: u32 = 0xffeb;
pub const IBUS_KEY_Super_R: u32 = 0xffec;
pub const IBUS_KEY_ISO_Level3_Shift: u32 = 0xfe03; // AltGr

pub const IBUS_ATTR_TYPE_UNDERLINE: c_uint = 1;
pub const IBUS_ATTR_UNDERLINE_SINGLE: c_uint = 1;

unsafe extern "C" {
    pub fn ibus_init();
    pub fn ibus_main();
    pub fn ibus_quit();

    pub fn ibus_bus_new() -> *mut IBusBus;
    pub fn ibus_bus_is_connected(bus: *mut IBusBus) -> c_int;
    pub fn ibus_bus_get_connection(bus: *mut IBusBus) -> *mut GDBusConnection;
    pub fn ibus_bus_request_name(bus: *mut IBusBus, name: *const c_char, flags: c_uint) -> c_uint;
    pub fn ibus_bus_register_component(bus: *mut IBusBus, component: *mut IBusComponent) -> c_int;
    pub fn ibus_bus_set_global_engine_async(
        bus: *mut IBusBus,
        global_engine: *const c_char,
        timeout: c_int,
        cancellable: *mut c_void,
        callback: *mut c_void,
        user_data: *mut c_void,
    );

    pub fn ibus_factory_new(connection: *mut GDBusConnection) -> *mut IBusFactory;

    pub fn ibus_engine_new(
        engine_name: *const c_char,
        path: *const c_char,
        connection: *mut GDBusConnection,
    ) -> *mut IBusEngine;

    pub fn ibus_engine_commit_text(engine: *mut IBusEngine, text: *mut IBusText);
    pub fn ibus_engine_update_preedit_text(
        engine: *mut IBusEngine,
        text: *mut IBusText,
        cursor_pos: c_uint,
        visible: c_int,
    );
    pub fn ibus_engine_hide_preedit_text(engine: *mut IBusEngine);

    pub fn ibus_text_new_from_string(str: *const c_char) -> *mut IBusText;
    pub fn ibus_text_append_attribute(
        text: *mut IBusText,
        type_: c_uint,
        value: c_uint,
        start_index: c_uint,
        end_index: c_int,
    );

    pub fn ibus_component_new(
        name: *const c_char,
        description: *const c_char,
        version: *const c_char,
        license: *const c_char,
        author: *const c_char,
        homepage: *const c_char,
        command_line: *const c_char,
        textdomain: *const c_char,
    ) -> *mut IBusComponent;

    pub fn ibus_engine_desc_new(
        name: *const c_char,
        longname: *const c_char,
        description: *const c_char,
        language: *const c_char,
        license: *const c_char,
        author: *const c_char,
        icon: *const c_char,
        layout: *const c_char,
    ) -> *mut IBusEngineDesc;

    pub fn ibus_component_add_engine(component: *mut IBusComponent, engine: *mut IBusEngineDesc);

    // GLib / GObject
    pub fn g_signal_connect_data(
        instance: *mut c_void,
        detailed_signal: *const c_char,
        c_handler: GCallback,
        data: *mut c_void,
        destroy_data: *mut c_void,
        connect_flags: c_int,
    ) -> c_uint;

    pub fn g_object_ref_sink(object: *mut c_void) -> *mut c_void;
    pub fn g_object_unref(object: *mut c_void);
    pub fn g_free(mem: *mut c_void);
}
