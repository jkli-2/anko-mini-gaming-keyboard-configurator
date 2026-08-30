//! Semantic types shared by the daemon and its clients.
//!
//! This crate deliberately contains no HID or wire-protocol details.

mod action_label;
mod layout;

pub use action_label::canonical_action_label;
pub use layout::{PHYSICAL_KEYS, PhysicalKey, physical_key_by_id};

/// Configuration information reported by the keyboard.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyboardConfig {
    pub protocol_version: u16,
    pub product_id: u16,
    pub firmware_version: u16,
    pub work_mode: u8,
    pub link_status: u8,
    pub battery: u8,
    pub charge: u8,
    pub profile_count: u8,
    pub profile: u8,
    pub layer_count: u8,
    pub layer: u8,
    pub auto_sleep_seconds: Option<u16>,
    pub serial_number: Option<String>,
}

/// The two effective keymap banks exposed by this keyboard.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyBank {
    Base,
    Fn,
}

/// A semantic key assignment. Unknown/non-canonical records remain lossless as `Raw`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyAction {
    Keyboard { modifiers: u8, usage: u8 },
    FunctionLayer,
    Consumer { usage: u16 },
    MouseButton { buttons: u8, vertical_wheel: i8 },
    MouseMove { x: i8, y: i8, wheel: i8 },
    Macro { id: u8 },
    Firmware { code: u8 },
    Power { code: u8 },
    Raw { kind: u8, codes: [u8; 3] },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Device-normalized HSV components. Each component occupies the full 0..=255 range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Hsv {
    pub hue: u8,
    pub saturation: u8,
    pub value: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LightingState {
    pub kind: u8,
    pub effect: u8,
    pub brightness: u8,
    pub speed: u8,
    pub direction: u8,
    pub color_enabled: bool,
    pub single_color_index: u8,
    pub hsv: Hsv,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacroEventKind {
    MouseButton,
    Keyboard,
    VerticalScroll,
    HorizontalScroll,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacroEventAction {
    Press,
    Release,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MacroEvent {
    pub delay_ms: u16,
    pub kind: MacroEventKind,
    pub action: MacroEventAction,
    pub code: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Macro {
    pub id: u8,
    pub events: Vec<MacroEvent>,
}
