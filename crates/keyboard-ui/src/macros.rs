//! Macro codec for the Anko / Z64 firmware.
//!
//! Reverse-engineered from the official client:
//! - 4096-byte macro blob
//! - 16 macro slots
//! - first 64 bytes reserved for slot metadata; first 32 bytes hold
//!   16 little-endian offsets
//! - event data begins at byte 64
//! - each event is 4 bytes:
//!   [delay_lo, delay_hi, flags, code]
//!
//! flags:
//! - bit 7: last event in macro
//! - bit 6: action == Press
//! - low 6 bits:
//!   0x03 = mouse button
//!   0x02 = keyboard
//!   0x04 = vertical wheel
//!   0x05 = horizontal wheel
//!
//! The official client treats action bit clear as Release. Wheel events are
//! emitted as Release by the recorder, but the firmware format itself still
//! carries the same action bit.

// The codec is deliberately staged ahead of its D-Bus wiring. Keep it tested
// without making the unfinished Macro page pretend those paths are live.
#![allow(dead_code)]
#![allow(clippy::items_after_test_module)]

use std::collections::HashMap;

use adw::prelude::*;
use keyboard_core::canonical_action_label;
use serde::{Deserialize, Serialize};

use crate::chord::{
    CapturedChord, MOD_ALT, MOD_CTRL, MOD_SHIFT, MOD_SUPER, chord_label, install_capture,
    modifiers_label,
};
use crate::dbus::{MacroEvent as DeviceMacroEvent, Macros};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub(crate) enum MacroStep {
    Chord {
        modifiers: u8,
        key: u8,
        delay_ms: u16,
    },
    Wait(u16),
    MouseClick {
        button: u8,
        delay_ms: u16,
    },
    Scroll {
        horizontal: bool,
        negative: bool,
        delay_ms: u16,
    },
    Raw(DeviceMacroEvent),
}

type MacroSteps = std::rc::Rc<std::cell::RefCell<Vec<(u8, Vec<MacroStep>)>>>;
type SavedMacroSteps = std::rc::Rc<std::cell::RefCell<HashMap<u8, Vec<MacroStep>>>>;
type PendingMacroSave = std::rc::Rc<std::cell::RefCell<Option<(u8, Vec<MacroStep>)>>>;

fn modifier_usage(bit: u8) -> u8 {
    match bit {
        MOD_CTRL => 224,
        MOD_SHIFT => 225,
        MOD_ALT => 226,
        MOD_SUPER => 227,
        _ => 0,
    }
}

fn modifier_bit(usage: u8) -> Option<u8> {
    match usage {
        224 | 228 => Some(MOD_CTRL),
        225 | 229 => Some(MOD_SHIFT),
        226 | 230 => Some(MOD_ALT),
        227 | 231 => Some(MOD_SUPER),
        _ => None,
    }
}

fn transition_modifiers(events: &mut Vec<DeviceMacroEvent>, held: &mut u8, desired: u8) {
    for bit in [MOD_SUPER, MOD_ALT, MOD_SHIFT, MOD_CTRL] {
        if *held & bit != 0 && desired & bit == 0 {
            events.push((0, 1, false, modifier_usage(bit)));
            *held &= !bit;
        }
    }
    for bit in [MOD_CTRL, MOD_SHIFT, MOD_ALT, MOD_SUPER] {
        if desired & bit != 0 && *held & bit == 0 {
            events.push((0, 1, true, modifier_usage(bit)));
            *held |= bit;
        }
    }
}

fn compile_steps(steps: &[MacroStep]) -> Vec<DeviceMacroEvent> {
    let mut events = Vec::new();
    let mut held_modifiers = 0;

    for step in steps {
        match *step {
            MacroStep::Chord {
                modifiers,
                key,
                delay_ms,
            } => {
                transition_modifiers(&mut events, &mut held_modifiers, modifiers);
                events.push((0, 1, true, key));
                events.push((delay_ms, 1, false, key));
            }
            MacroStep::Wait(delay_ms) => {
                transition_modifiers(&mut events, &mut held_modifiers, 0);
                if let Some(last) = events.last_mut() {
                    last.0 = last.0.saturating_add(delay_ms).min(MAX_DELAY_MS);
                }
            }
            MacroStep::MouseClick { button, delay_ms } => {
                transition_modifiers(&mut events, &mut held_modifiers, 0);
                events.push((0, 0, true, button));
                events.push((delay_ms, 0, false, button));
            }
            MacroStep::Scroll {
                horizontal,
                negative,
                delay_ms,
            } => {
                transition_modifiers(&mut events, &mut held_modifiers, 0);
                events.push((
                    delay_ms,
                    if horizontal { 3 } else { 2 },
                    false,
                    if negative { 0xFF } else { 1 },
                ));
            }
            MacroStep::Raw(event) => {
                transition_modifiers(&mut events, &mut held_modifiers, 0);
                events.push(event);
            }
        }
    }
    transition_modifiers(&mut events, &mut held_modifiers, 0);
    events
}

fn decompile_events(events: &[DeviceMacroEvent]) -> Vec<MacroStep> {
    let mut steps = Vec::new();
    let mut held_modifiers = 0;
    let mut index = 0;
    while index < events.len() {
        let event = events[index];
        if event.1 == 1 {
            if let Some(bit) = modifier_bit(event.3) {
                if event.2 {
                    held_modifiers |= bit;
                } else {
                    held_modifiers &= !bit;
                    if event.0 > 0 {
                        steps.push(MacroStep::Wait(event.0));
                    }
                }
                index += 1;
                continue;
            }
            if event.2
                && events
                    .get(index + 1)
                    .is_some_and(|next| next.1 == 1 && !next.2 && next.3 == event.3)
            {
                let release = events[index + 1];
                steps.push(MacroStep::Chord {
                    modifiers: held_modifiers,
                    key: event.3,
                    delay_ms: release.0,
                });
                index += 2;
                continue;
            }
        }
        if event.1 == 0
            && event.2
            && events
                .get(index + 1)
                .is_some_and(|next| next.1 == 0 && !next.2 && next.3 == event.3)
        {
            let release = events[index + 1];
            steps.push(MacroStep::MouseClick {
                button: event.3,
                delay_ms: release.0,
            });
            index += 2;
            continue;
        }
        if matches!(event.1, 2 | 3) {
            steps.push(MacroStep::Scroll {
                horizontal: event.1 == 3,
                negative: event.3 == 0xFF,
                delay_ms: event.0,
            });
        } else {
            steps.push(MacroStep::Raw(event));
        }
        index += 1;
    }
    steps
}

fn macro_names_path() -> std::path::PathBuf {
    gtk::glib::user_data_dir()
        .join("anko-keyboard")
        .join("macro-names.json")
}

fn macro_steps_path() -> std::path::PathBuf {
    gtk::glib::user_data_dir()
        .join("anko-keyboard")
        .join("macro-steps.json")
}

fn load_macro_names() -> HashMap<u8, String> {
    let Ok(contents) = std::fs::read_to_string(macro_names_path()) else {
        return HashMap::new();
    };
    serde_json::from_str::<HashMap<u8, String>>(&contents).unwrap_or_default()
}

fn save_macro_names(names: &HashMap<u8, String>) -> Result<(), String> {
    let path = macro_names_path();
    let parent = path
        .parent()
        .ok_or_else(|| "macro name path has no parent".to_string())?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let contents = serde_json::to_string_pretty(names).map_err(|error| error.to_string())?;
    std::fs::write(path, contents).map_err(|error| error.to_string())
}

fn load_saved_steps() -> HashMap<u8, Vec<MacroStep>> {
    let Ok(contents) = std::fs::read_to_string(macro_steps_path()) else {
        return HashMap::new();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

fn save_steps(steps: &HashMap<u8, Vec<MacroStep>>) -> Result<(), String> {
    let path = macro_steps_path();
    let parent = path
        .parent()
        .ok_or_else(|| "macro step path has no parent".to_string())?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let contents = serde_json::to_string_pretty(steps).map_err(|error| error.to_string())?;
    std::fs::write(path, contents).map_err(|error| error.to_string())
}

fn clean_macro_name(value: &str) -> String {
    value.trim().chars().take(48).collect()
}

pub const MACRO_BLOB_SIZE: usize = 4096;
pub const MACRO_SLOT_COUNT: usize = 16;
pub const MACRO_HEADER_SIZE: usize = 64;
pub const MACRO_EVENT_SIZE: usize = 4;
pub const MAX_DELAY_MS: u16 = 10_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacroAction {
    Press,
    Release,
}

impl MacroAction {
    fn from_flags(flags: u8) -> Self {
        if flags & 0x40 != 0 {
            Self::Press
        } else {
            Self::Release
        }
    }

    fn flag(self) -> u8 {
        match self {
            Self::Press => 0x40,
            Self::Release => 0x00,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacroEventType {
    MouseButton,
    Keyboard,
    WheelVertical,
    WheelHorizontal,
}

impl MacroEventType {
    fn from_flags(flags: u8) -> Self {
        match flags & 0x3f {
            0x02 => Self::Keyboard,
            0x04 => Self::WheelVertical,
            0x05 => Self::WheelHorizontal,
            // The official parser defaults unknown/other values to type 1.
            _ => Self::MouseButton,
        }
    }

    fn flag(self) -> u8 {
        match self {
            Self::MouseButton => 0x03,
            Self::Keyboard => 0x02,
            Self::WheelVertical => 0x04,
            Self::WheelHorizontal => 0x05,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MacroEvent {
    pub event_type: MacroEventType,
    pub action: MacroAction,
    pub code: u8,
    pub delay_ms: u16,
}

impl MacroEvent {
    pub const fn keyboard(code: u8, action: MacroAction, delay_ms: u16) -> Self {
        Self {
            event_type: MacroEventType::Keyboard,
            action,
            code,
            delay_ms,
        }
    }

    pub const fn mouse_button(code: u8, action: MacroAction, delay_ms: u16) -> Self {
        Self {
            event_type: MacroEventType::MouseButton,
            action,
            code,
            delay_ms,
        }
    }

    pub const fn wheel_vertical(code: u8, delay_ms: u16) -> Self {
        Self {
            event_type: MacroEventType::WheelVertical,
            action: MacroAction::Release,
            code,
            delay_ms,
        }
    }

    pub const fn wheel_horizontal(code: u8, delay_ms: u16) -> Self {
        Self {
            event_type: MacroEventType::WheelHorizontal,
            action: MacroAction::Release,
            code,
            delay_ms,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MacroSlot {
    pub name: String,
    pub events: Vec<MacroEvent>,
}

impl MacroSlot {
    pub fn new(index: usize) -> Self {
        Self {
            name: format!("M{index}"),
            events: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MacroBank {
    pub slots: Vec<MacroSlot>,
}

impl Default for MacroBank {
    fn default() -> Self {
        Self {
            slots: (0..MACRO_SLOT_COUNT).map(MacroSlot::new).collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MacroCodecError {
    WrongBlobSize { expected: usize, actual: usize },
    TooManySlots { actual: usize },
    InvalidOffset { slot: usize, offset: usize },
    UnterminatedMacro { slot: usize },
    BlobOverflow,
}

impl std::fmt::Display for MacroCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongBlobSize { expected, actual } => {
                write!(f, "macro blob must be {expected} bytes, got {actual}")
            }
            Self::TooManySlots { actual } => {
                write!(
                    f,
                    "macro bank has {actual} slots; maximum is {MACRO_SLOT_COUNT}"
                )
            }
            Self::InvalidOffset { slot, offset } => {
                write!(f, "macro slot {slot} has invalid offset {offset}")
            }
            Self::UnterminatedMacro { slot } => {
                write!(f, "macro slot {slot} has no terminating event")
            }
            Self::BlobOverflow => write!(f, "macro data exceeds 4096-byte firmware region"),
        }
    }
}

impl std::error::Error for MacroCodecError {}

pub fn decode_macro_blob(blob: &[u8]) -> Result<MacroBank, MacroCodecError> {
    if blob.len() != MACRO_BLOB_SIZE {
        return Err(MacroCodecError::WrongBlobSize {
            expected: MACRO_BLOB_SIZE,
            actual: blob.len(),
        });
    }

    let mut bank = MacroBank::default();

    for slot_index in 0..MACRO_SLOT_COUNT {
        let lo = blob[slot_index * 2];
        let hi = blob[slot_index * 2 + 1];

        // Official client considers both FF FF and a zero first offset byte
        // to mean "empty slot".
        if (lo == 0xff && hi == 0xff) || lo == 0 {
            continue;
        }

        let mut offset = u16::from_le_bytes([lo, hi]) as usize;
        if !(MACRO_HEADER_SIZE..MACRO_BLOB_SIZE).contains(&offset) {
            return Err(MacroCodecError::InvalidOffset {
                slot: slot_index,
                offset,
            });
        }

        let mut events = Vec::new();

        loop {
            if offset + MACRO_EVENT_SIZE > MACRO_BLOB_SIZE {
                return Err(MacroCodecError::UnterminatedMacro { slot: slot_index });
            }

            let delay_ms = u16::from_le_bytes([blob[offset], blob[offset + 1]]);
            let flags = blob[offset + 2];
            let code = blob[offset + 3];

            events.push(MacroEvent {
                event_type: MacroEventType::from_flags(flags),
                action: MacroAction::from_flags(flags),
                code,
                delay_ms,
            });

            let is_last = flags & 0x80 != 0 || flags == 0;
            offset += MACRO_EVENT_SIZE;

            if is_last {
                break;
            }
        }

        bank.slots[slot_index].events = events;
    }

    Ok(bank)
}

pub fn encode_macro_blob(bank: &MacroBank) -> Result<Vec<u8>, MacroCodecError> {
    if bank.slots.len() > MACRO_SLOT_COUNT {
        return Err(MacroCodecError::TooManySlots {
            actual: bank.slots.len(),
        });
    }

    // This matches the official client:
    // first 64 bytes = 0xff, remaining bytes = 0x00.
    let mut blob = vec![0u8; MACRO_BLOB_SIZE];
    blob[..MACRO_HEADER_SIZE].fill(0xff);

    let mut write_offset = MACRO_HEADER_SIZE;

    for slot_index in 0..MACRO_SLOT_COUNT {
        let Some(slot) = bank.slots.get(slot_index) else {
            continue;
        };

        if slot.events.is_empty() {
            continue;
        }

        let needed = slot.events.len() * MACRO_EVENT_SIZE;
        if write_offset + needed > MACRO_BLOB_SIZE {
            return Err(MacroCodecError::BlobOverflow);
        }

        let [off_lo, off_hi] = (write_offset as u16).to_le_bytes();
        blob[slot_index * 2] = off_lo;
        blob[slot_index * 2 + 1] = off_hi;

        for (event_index, event) in slot.events.iter().enumerate() {
            let base = write_offset + event_index * MACRO_EVENT_SIZE;
            let delay = event.delay_ms.min(MAX_DELAY_MS);
            let [delay_lo, delay_hi] = delay.to_le_bytes();

            let mut flags = event.event_type.flag() | event.action.flag();
            if event_index + 1 == slot.events.len() {
                flags |= 0x80;
            }

            blob[base] = delay_lo;
            blob[base + 1] = delay_hi;
            blob[base + 2] = flags;
            blob[base + 3] = event.code;
        }

        write_offset += needed;
    }

    Ok(blob)
}

/// Mouse button codes used by the official client recorder.
pub mod mouse_code {
    pub const LEFT: u8 = 1;
    pub const RIGHT: u8 = 2;
    pub const MIDDLE: u8 = 4;
    pub const FORWARD: u8 = 8;
    pub const BACKWARD: u8 = 16;
}

/// Wheel codes used by the official client recorder.
pub mod wheel_code {
    pub const POSITIVE: u8 = 1;
    pub const NEGATIVE: u8 = 255;

    pub const UP: u8 = POSITIVE;
    pub const DOWN: u8 = NEGATIVE;
    pub const RIGHT: u8 = NEGATIVE;
    pub const LEFT: u8 = POSITIVE;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_bank_matches_official_layout() {
        let blob = encode_macro_blob(&MacroBank::default()).unwrap();

        assert_eq!(blob.len(), MACRO_BLOB_SIZE);
        assert!(blob[..MACRO_HEADER_SIZE].iter().all(|b| *b == 0xff));
        assert!(blob[MACRO_HEADER_SIZE..].iter().all(|b| *b == 0x00));
    }

    #[test]
    fn one_keyboard_press_round_trips() {
        let mut bank = MacroBank::default();
        bank.slots[0]
            .events
            .push(MacroEvent::keyboard(0x04, MacroAction::Press, 123));

        let blob = encode_macro_blob(&bank).unwrap();

        assert_eq!(&blob[0..2], &64u16.to_le_bytes());
        assert_eq!(&blob[64..68], &[123, 0, 0x80 | 0x40 | 0x02, 0x04]);

        let decoded = decode_macro_blob(&blob).unwrap();
        assert_eq!(decoded.slots[0].events, bank.slots[0].events);
    }

    #[test]
    fn multiple_slots_are_packed_sequentially() {
        let mut bank = MacroBank::default();

        bank.slots[0].events = vec![
            MacroEvent::keyboard(0x04, MacroAction::Press, 10),
            MacroEvent::keyboard(0x04, MacroAction::Release, 20),
        ];

        bank.slots[3].events = vec![MacroEvent::mouse_button(
            mouse_code::LEFT,
            MacroAction::Press,
            30,
        )];

        let blob = encode_macro_blob(&bank).unwrap();

        assert_eq!(
            u16::from_le_bytes([blob[0], blob[1]]) as usize,
            MACRO_HEADER_SIZE
        );
        assert_eq!(
            u16::from_le_bytes([blob[6], blob[7]]) as usize,
            MACRO_HEADER_SIZE + 8
        );

        let decoded = decode_macro_blob(&blob).unwrap();
        assert_eq!(decoded.slots[0].events, bank.slots[0].events);
        assert_eq!(decoded.slots[3].events, bank.slots[3].events);
    }

    #[test]
    fn event_type_flags_match_official_client() {
        assert_eq!(MacroEventType::MouseButton.flag(), 0x03);
        assert_eq!(MacroEventType::Keyboard.flag(), 0x02);
        assert_eq!(MacroEventType::WheelVertical.flag(), 0x04);
        assert_eq!(MacroEventType::WheelHorizontal.flag(), 0x05);
    }

    #[test]
    fn delay_is_clamped_to_official_ui_limit() {
        let mut bank = MacroBank::default();
        bank.slots[0]
            .events
            .push(MacroEvent::keyboard(0x04, MacroAction::Press, u16::MAX));

        let blob = encode_macro_blob(&bank).unwrap();
        let delay = u16::from_le_bytes([blob[64], blob[65]]);

        assert_eq!(delay, MAX_DELAY_MS);
    }

    #[test]
    fn semantic_chords_share_common_modifiers() {
        let steps = vec![
            MacroStep::Chord {
                modifiers: MOD_CTRL,
                key: 4,
                delay_ms: 50,
            },
            MacroStep::Chord {
                modifiers: MOD_CTRL,
                key: 6,
                delay_ms: 90,
            },
        ];
        let events = compile_steps(&steps);
        assert_eq!(
            events,
            vec![
                (0, 1, true, 224),
                (0, 1, true, 4),
                (50, 1, false, 4),
                (0, 1, true, 6),
                (90, 1, false, 6),
                (0, 1, false, 224),
            ]
        );
        assert_eq!(decompile_events(&events), steps);
    }

    #[test]
    fn semantic_compiler_transitions_changed_modifiers_and_expands_clicks() {
        let steps = vec![
            MacroStep::Chord {
                modifiers: MOD_CTRL,
                key: 4,
                delay_ms: 20,
            },
            MacroStep::Chord {
                modifiers: MOD_SHIFT,
                key: 5,
                delay_ms: 30,
            },
            MacroStep::MouseClick {
                button: 1,
                delay_ms: 40,
            },
        ];
        let events = compile_steps(&steps);
        assert_eq!(events[0], (0, 1, true, 224));
        assert!(events.contains(&(0, 1, false, 224)));
        assert!(events.contains(&(0, 1, true, 225)));
        assert_eq!(
            &events[events.len() - 2..],
            &[(0, 0, true, 1), (40, 0, false, 1)]
        );
        assert_eq!(decompile_events(&events), steps);
    }

    #[test]
    fn semantic_compiler_keeps_only_shared_modifiers_held() {
        let steps = vec![
            MacroStep::Chord {
                modifiers: MOD_CTRL | MOD_SHIFT,
                key: 4,
                delay_ms: 50,
            },
            MacroStep::Chord {
                modifiers: MOD_CTRL,
                key: 5,
                delay_ms: 90,
            },
        ];

        assert_eq!(
            compile_steps(&steps),
            vec![
                (0, 1, true, 224),
                (0, 1, true, 225),
                (0, 1, true, 4),
                (50, 1, false, 4),
                (0, 1, false, 225),
                (0, 1, true, 5),
                (90, 1, false, 5),
                (0, 1, false, 224),
            ]
        );
    }

    #[test]
    fn wait_is_applied_after_releasing_held_modifiers() {
        let steps = vec![
            MacroStep::Chord {
                modifiers: MOD_CTRL,
                key: 4,
                delay_ms: 50,
            },
            MacroStep::Wait(200),
            MacroStep::Chord {
                modifiers: 0,
                key: 40,
                delay_ms: 20,
            },
        ];
        let events = compile_steps(&steps);

        assert_eq!(events[3], (200, 1, false, 224));
        assert_eq!(decompile_events(&events), steps);
    }

    #[test]
    fn wait_is_compiled_into_the_preceding_firmware_delay() {
        let steps = vec![
            MacroStep::Chord {
                modifiers: 0,
                key: 4,
                delay_ms: 20,
            },
            MacroStep::Wait(500),
            MacroStep::Chord {
                modifiers: 0,
                key: 25,
                delay_ms: 20,
            },
        ];

        assert_eq!(
            compile_steps(&steps),
            vec![
                (0, 1, true, 4),
                (520, 1, false, 4),
                (0, 1, true, 25),
                (20, 1, false, 25),
            ]
        );
    }

    #[test]
    fn semantic_steps_round_trip_through_the_client_sidecar() {
        let steps = vec![
            MacroStep::Chord {
                modifiers: MOD_CTRL,
                key: 6,
                delay_ms: 20,
            },
            MacroStep::Wait(500),
        ];
        let json = serde_json::to_string(&steps).unwrap();

        assert_eq!(
            serde_json::from_str::<Vec<MacroStep>>(&json).unwrap(),
            steps
        );
    }

    #[test]
    fn client_macro_names_are_trimmed_and_bounded() {
        assert_eq!(clean_macro_name("  Copy All  "), "Copy All");
        assert_eq!(clean_macro_name(&"x".repeat(60)).chars().count(), 48);
    }

    #[test]
    fn mini_keyboard_excludes_modifier_key_usages() {
        assert!(
            MACRO_KEY_ROWS
                .iter()
                .flat_map(|row| row.iter())
                .all(|(_, usage)| !(224..=231).contains(usage))
        );
    }
}

#[derive(Clone)]
pub(crate) struct MacroPage {
    pub root: gtk::Box,
    slots: gtk::ListBox,
    selected_macro: gtk::Label,
    summary: gtk::Label,
    events: gtk::Box,
    raw_events: gtk::Box,
    steps: MacroSteps,
    original: MacroSteps,
    names: std::rc::Rc<std::cell::RefCell<HashMap<u8, String>>>,
    saved_steps: SavedMacroSteps,
    pending_save: PendingMacroSave,
    slot_labels: Vec<gtk::Label>,
    clear: gtk::Button,
    revert: gtk::Button,
    save: gtk::Button,
    kind: gtk::DropDown,
    capture: gtk::Button,
    keyboard_key: gtk::MenuButton,
    keyboard_usage: std::rc::Rc<std::cell::Cell<u8>>,
    ctrl: gtk::ToggleButton,
    shift: gtk::ToggleButton,
    alt: gtk::ToggleButton,
    super_key: gtk::ToggleButton,
    mouse_button: gtk::DropDown,
    scroll_direction: gtk::DropDown,
    delay: gtk::SpinButton,
    editing_index: std::rc::Rc<std::cell::Cell<Option<usize>>>,
    add: gtk::Button,
}

impl MacroPage {
    pub(crate) fn profile_metadata(&self) -> crate::profile::ClientProfileMetadata {
        crate::profile::ClientProfileMetadata {
            macro_names: self
                .names
                .borrow()
                .iter()
                .map(|(&id, name)| (id, name.clone()))
                .collect(),
            macro_steps: self
                .saved_steps
                .borrow()
                .iter()
                .map(|(&id, steps)| (id, steps.clone()))
                .collect(),
            layout: if crate::kle::custom_kle_path().exists() {
                "custom".to_string()
            } else {
                "default".to_string()
            },
        }
    }

    pub(crate) fn restore_profile_metadata(
        &self,
        metadata: &crate::profile::ClientProfileMetadata,
    ) -> Result<(), String> {
        let names: HashMap<_, _> = metadata
            .macro_names
            .iter()
            .map(|(&id, name)| (id, name.clone()))
            .collect();
        let steps: HashMap<_, _> = metadata
            .macro_steps
            .iter()
            .map(|(&id, steps)| (id, steps.clone()))
            .collect();
        save_macro_names(&names)?;
        save_steps(&steps)?;
        *self.names.borrow_mut() = names;
        *self.saved_steps.borrow_mut() = steps;
        self.refresh_names();
        self.render_selected();
        Ok(())
    }

    pub fn set_macros(&self, macros: Macros) {
        let pending = self.pending_save.borrow_mut().take();
        let mut saved = self.saved_steps.borrow().clone();
        if let Some((id, steps)) = pending {
            let device_events = macros
                .iter()
                .find(|(macro_id, _)| *macro_id == id)
                .map(|(_, events)| events.as_slice())
                .unwrap_or_default();
            if compile_steps(&steps) == device_events {
                if steps.is_empty() {
                    saved.remove(&id);
                } else {
                    saved.insert(id, steps);
                }
            }
        }
        saved.retain(|id, steps| {
            macros
                .iter()
                .find(|(macro_id, _)| macro_id == id)
                .is_some_and(|(_, events)| compile_steps(steps) == events.as_slice())
        });
        let steps: Vec<_> = macros
            .into_iter()
            .map(|(id, events)| {
                let steps = saved
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| decompile_events(&events));
                (id, steps)
            })
            .collect();
        *self.saved_steps.borrow_mut() = saved;
        if let Err(error) = save_steps(&self.saved_steps.borrow()) {
            eprintln!("keyboard-ui: could not save semantic macro steps: {error}");
        }
        *self.original.borrow_mut() = steps.clone();
        *self.steps.borrow_mut() = steps;
        self.render_selected();
    }

    pub fn connect_save<F: Fn(u8, Vec<DeviceMacroEvent>) + 'static>(&self, callback: F) {
        let page = self.clone();
        self.save.connect_clicked(move |_| {
            let id = page.selected_id();
            let steps = page.selected_steps();
            *page.pending_save.borrow_mut() = Some((id, steps.clone()));
            callback(id, compile_steps(&steps));
        });
    }

    fn selected_id(&self) -> u8 {
        self.slots
            .selected_row()
            .map(|row| row.index() as u8)
            .unwrap_or(0)
    }

    fn selected_steps(&self) -> Vec<MacroStep> {
        let id = self.selected_id();
        self.steps
            .borrow()
            .iter()
            .find(|(macro_id, _)| *macro_id == id)
            .map(|(_, steps)| steps.clone())
            .unwrap_or_default()
    }

    fn set_selected_steps(&self, steps: Vec<MacroStep>) {
        self.reset_editor();
        let id = self.selected_id();
        let mut all_steps = self.steps.borrow_mut();
        all_steps.retain(|(macro_id, _)| *macro_id != id);
        if !steps.is_empty() {
            all_steps.push((id, steps));
            all_steps.sort_by_key(|(macro_id, _)| *macro_id);
        }
        drop(all_steps);
        self.render_selected();
    }

    fn display_name(&self, id: u8) -> String {
        self.names
            .borrow()
            .get(&id)
            .map(|name| format!("M{id} · {name}"))
            .unwrap_or_else(|| format!("M{id}"))
    }

    fn refresh_names(&self) {
        for (id, label) in self.slot_labels.iter().enumerate() {
            label.set_label(&self.display_name(id as u8));
        }
        self.selected_macro
            .set_label(&self.display_name(self.selected_id()));
    }

    fn begin_edit(&self, index: usize) {
        let Some(step) = self.selected_steps().get(index).cloned() else {
            return;
        };
        if matches!(step, MacroStep::Raw(_)) {
            return;
        }
        self.editing_index.set(Some(index));
        self.add.set_label("Update Step");
        match step {
            MacroStep::Chord {
                modifiers,
                key,
                delay_ms,
            } => {
                self.kind.set_selected(0);
                self.keyboard_usage.set(key);
                self.keyboard_key
                    .set_label(&canonical_action_label(&format!("keyboard/{key}/0")));
                self.capture.set_label("⌨ Capture");
                self.ctrl.set_active(modifiers & MOD_CTRL != 0);
                self.shift.set_active(modifiers & MOD_SHIFT != 0);
                self.alt.set_active(modifiers & MOD_ALT != 0);
                self.super_key.set_active(modifiers & MOD_SUPER != 0);
                self.delay.set_value(f64::from(delay_ms));
            }
            MacroStep::MouseClick { button, delay_ms } => {
                self.kind.set_selected(1);
                let selected = [1, 2, 4, 8, 16]
                    .iter()
                    .position(|candidate| *candidate == button)
                    .unwrap_or(0);
                self.mouse_button.set_selected(selected as u32);
                self.delay.set_value(f64::from(delay_ms));
            }
            MacroStep::Scroll {
                horizontal,
                negative,
                delay_ms,
            } => {
                self.kind.set_selected(if horizontal { 3 } else { 2 });
                self.scroll_direction.set_selected(u32::from(negative));
                self.delay.set_value(f64::from(delay_ms));
            }
            MacroStep::Wait(delay_ms) => {
                self.kind.set_selected(4);
                self.delay.set_value(f64::from(delay_ms));
            }
            MacroStep::Raw(_) => {}
        }
    }

    fn reset_editor(&self) {
        self.editing_index.set(None);
        self.add.set_label("Add Step");
    }

    fn update_add_availability(&self) {
        let wait_needs_predecessor = self.kind.selected() == 4
            && self.selected_steps().is_empty()
            && self.editing_index.get().is_none();
        self.add.set_sensitive(!wait_needs_predecessor);
        self.add.set_tooltip_text(
            wait_needs_predecessor.then_some("Wait must follow an action on this keyboard"),
        );
    }

    fn move_step(&self, from: usize, to: usize) {
        let mut steps = self.selected_steps();
        if from >= steps.len() || to >= steps.len() || from == to {
            return;
        }
        let step = steps.remove(from);
        steps.insert(to, step);
        self.set_selected_steps(steps);
    }

    fn delete_step(&self, index: usize) {
        let mut steps = self.selected_steps();
        if index < steps.len() {
            steps.remove(index);
            self.set_selected_steps(steps);
        }
    }

    fn build_step_row(&self, index: usize, step: &MacroStep, count: usize) -> gtk::Widget {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let handle = gtk::Image::from_icon_name("list-drag-handle-symbolic");
        handle.add_css_class("dim-label");
        row.append(&handle);

        let edit = gtk::Button::new();
        edit.add_css_class("flat");
        edit.set_hexpand(true);
        edit.set_child(Some(&macro_step_row(index, step)));
        let page = self.clone();
        edit.connect_clicked(move |_| page.begin_edit(index));
        row.append(&edit);

        let up = gtk::Button::from_icon_name("go-up-symbolic");
        up.add_css_class("flat");
        up.set_sensitive(index > 0);
        let page = self.clone();
        up.connect_clicked(move |_| page.move_step(index, index.saturating_sub(1)));
        row.append(&up);

        let down = gtk::Button::from_icon_name("go-down-symbolic");
        down.add_css_class("flat");
        down.set_sensitive(index + 1 < count);
        let page = self.clone();
        down.connect_clicked(move |_| page.move_step(index, index + 1));
        row.append(&down);

        let delete = gtk::Button::from_icon_name("user-trash-symbolic");
        delete.add_css_class("flat");
        let page = self.clone();
        delete.connect_clicked(move |_| page.delete_step(index));
        row.append(&delete);

        let drag = gtk::DragSource::new();
        drag.set_actions(gtk::gdk::DragAction::MOVE);
        drag.connect_prepare(move |_, _, _| {
            Some(gtk::gdk::ContentProvider::for_value(
                &(index as u32).to_value(),
            ))
        });
        row.add_controller(drag);

        let drop_target = gtk::DropTarget::new(u32::static_type(), gtk::gdk::DragAction::MOVE);
        let page = self.clone();
        drop_target.connect_drop(move |_, value, _, _| {
            let Ok(from) = value.get::<u32>() else {
                return false;
            };
            page.move_step(from as usize, index);
            true
        });
        row.add_controller(drop_target);
        row.upcast()
    }

    fn render_selected(&self) {
        let id = self.selected_id();
        self.selected_macro.set_label(&self.display_name(id));
        let all_steps = self.steps.borrow();
        let steps = all_steps
            .iter()
            .find(|(macro_id, _)| *macro_id == id)
            .map(|(_, steps)| steps.as_slice())
            .unwrap_or_default();
        let original_steps = self
            .original
            .borrow()
            .iter()
            .find(|(macro_id, _)| *macro_id == id)
            .map(|(_, steps)| steps.clone())
            .unwrap_or_default();
        let dirty = steps != original_steps;
        self.clear.set_sensitive(!steps.is_empty());
        self.revert.set_sensitive(dirty);
        let compiled = compile_steps(steps);
        let valid = steps.is_empty()
            || (!compiled.is_empty() && !matches!(steps.first(), Some(MacroStep::Wait(_))));
        self.save.set_sensitive(dirty && valid);
        self.save.set_tooltip_text(
            (!valid).then_some("A Wait needs an action before it; move or remove the first Wait"),
        );
        self.update_add_availability();
        let duration: u32 = compiled.iter().map(|event| u32::from(event.0)).sum();
        self.summary
            .set_label(&format!("{} steps · {duration} ms", steps.len()));

        while let Some(child) = self.events.first_child() {
            self.events.remove(&child);
        }
        while let Some(child) = self.raw_events.first_child() {
            self.raw_events.remove(&child);
        }
        for event in &compiled {
            self.raw_events.append(&macro_event_row(event));
        }
        if steps.is_empty() {
            let empty_title = gtk::Label::new(Some("No macro steps"));
            empty_title.add_css_class("title-4");
            self.events.append(&empty_title);
            let hint = gtk::Label::new(Some("This macro slot is empty on the keyboard."));
            hint.add_css_class("dim-label");
            self.events.append(&hint);
            return;
        }

        for (index, step) in steps.iter().enumerate() {
            self.events
                .append(&self.build_step_row(index, step, steps.len()));
        }
    }
}

fn macro_event_row(event: &DeviceMacroEvent) -> gtk::Widget {
    let (delay_ms, kind, pressed, code) = *event;
    let row = gtk::Grid::builder()
        .column_spacing(12)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(12)
        .margin_end(12)
        .build();
    let key = match kind {
        0 => match code {
            1 => "Left mouse".to_string(),
            2 => "Right mouse".to_string(),
            4 => "Middle mouse".to_string(),
            8 => "Forward mouse".to_string(),
            16 => "Back mouse".to_string(),
            _ => format!("Mouse 0x{code:02X}"),
        },
        1 => canonical_action_label(&format!("keyboard/{code}/0")),
        2 => if code == 0xFF {
            "Scroll down"
        } else {
            "Scroll up"
        }
        .to_string(),
        3 => if code == 0xFF {
            "Scroll right"
        } else {
            "Scroll left"
        }
        .to_string(),
        _ => format!("Unknown {kind}:0x{code:02X}"),
    };
    let key = gtk::Label::new(Some(&key));
    key.set_xalign(0.0);
    key.set_hexpand(true);
    let action = gtk::Label::new(Some(if pressed { "Press" } else { "Release" }));
    action.set_xalign(0.0);
    action.set_hexpand(true);
    let delay = gtk::Label::new(Some(&format!("{delay_ms} ms")));
    delay.set_xalign(1.0);
    row.attach(&key, 0, 0, 1, 1);
    row.attach(&action, 1, 0, 1, 1);
    row.attach(&delay, 2, 0, 1, 1);
    row.upcast()
}

fn mouse_button_label(button: u8) -> &'static str {
    match button {
        1 => "Mouse Left Click",
        2 => "Mouse Right Click",
        4 => "Mouse Middle Click",
        8 => "Mouse Forward Click",
        16 => "Mouse Back Click",
        _ => "Mouse Click",
    }
}

fn macro_step_row(index: usize, step: &MacroStep) -> gtk::Widget {
    let row = gtk::Grid::builder()
        .column_spacing(12)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(12)
        .margin_end(12)
        .build();
    let number = gtk::Label::new(Some(&(index + 1).to_string()));
    number.add_css_class("dim-label");
    let (description, delay_ms) = match *step {
        MacroStep::Chord {
            modifiers,
            key,
            delay_ms,
        } => (
            chord_label(CapturedChord {
                modifiers,
                usage: key,
            }),
            Some(delay_ms),
        ),
        MacroStep::Wait(delay_ms) => (format!("Wait {delay_ms} ms"), None),
        MacroStep::MouseClick { button, delay_ms } => {
            (mouse_button_label(button).to_string(), Some(delay_ms))
        }
        MacroStep::Scroll {
            horizontal,
            negative,
            delay_ms,
        } => {
            let direction = match (horizontal, negative) {
                (false, false) => "Scroll Up",
                (false, true) => "Scroll Down",
                (true, false) => "Scroll Left",
                (true, true) => "Scroll Right",
            };
            (direction.to_string(), Some(delay_ms))
        }
        MacroStep::Raw(event) => (
            format!("Raw event kind {} code {}", event.1, event.3),
            Some(event.0),
        ),
    };
    let description = gtk::Label::new(Some(&description));
    description.set_xalign(0.0);
    description.set_hexpand(true);
    let delay = gtk::Label::new(delay_ms.map(|value| format!("{value} ms")).as_deref());
    delay.set_xalign(1.0);
    delay.add_css_class("dim-label");
    row.attach(&number, 0, 0, 1, 1);
    row.attach(&description, 1, 0, 1, 1);
    row.attach(&delay, 2, 0, 1, 1);
    row.upcast()
}

type MacroKey = (&'static str, u8);

const MACRO_KEY_ROWS: &[&[MacroKey]] = &[
    &[
        ("Esc", 41),
        ("F1", 58),
        ("F2", 59),
        ("F3", 60),
        ("F4", 61),
        ("F5", 62),
        ("F6", 63),
        ("F7", 64),
        ("F8", 65),
        ("F9", 66),
        ("F10", 67),
        ("F11", 68),
        ("F12", 69),
    ],
    &[
        ("1", 30),
        ("2", 31),
        ("3", 32),
        ("4", 33),
        ("5", 34),
        ("6", 35),
        ("7", 36),
        ("8", 37),
        ("9", 38),
        ("0", 39),
        ("−", 45),
        ("=", 46),
        ("Bksp", 42),
    ],
    &[
        ("Tab", 43),
        ("Q", 20),
        ("W", 26),
        ("E", 8),
        ("R", 21),
        ("T", 23),
        ("Y", 28),
        ("U", 24),
        ("I", 12),
        ("O", 18),
        ("P", 19),
        ("[", 47),
        ("]", 48),
        ("\\", 49),
    ],
    &[
        ("Caps", 57),
        ("A", 4),
        ("S", 22),
        ("D", 7),
        ("F", 9),
        ("G", 10),
        ("H", 11),
        ("J", 13),
        ("K", 14),
        ("L", 15),
        (";", 51),
        ("'", 52),
        ("Enter", 40),
    ],
    &[
        ("Z", 29),
        ("X", 27),
        ("C", 6),
        ("V", 25),
        ("B", 5),
        ("N", 17),
        ("M", 16),
        (",", 54),
        (".", 55),
        ("/", 56),
        ("Space", 44),
        ("←", 80),
        ("↓", 81),
        ("↑", 82),
        ("→", 79),
    ],
];

fn macro_keyboard_picker() -> (gtk::MenuButton, std::rc::Rc<std::cell::Cell<u8>>) {
    let selected_usage = std::rc::Rc::new(std::cell::Cell::new(4));
    let picker = gtk::MenuButton::new();
    picker.set_label("A");
    picker.set_tooltip_text(Some("Choose a keyboard key"));

    let popover = gtk::Popover::new();
    let keyboard = gtk::Box::new(gtk::Orientation::Vertical, 4);
    keyboard.set_margin_top(8);
    keyboard.set_margin_bottom(8);
    keyboard.set_margin_start(8);
    keyboard.set_margin_end(8);
    for keys in MACRO_KEY_ROWS {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 3);
        row.set_halign(gtk::Align::Center);
        for &(label, usage) in *keys {
            let button = gtk::Button::with_label(label);
            button.add_css_class("palette-button");
            let selected_usage = selected_usage.clone();
            let picker = picker.clone();
            let popover = popover.clone();
            button.connect_clicked(move |_| {
                selected_usage.set(usage);
                picker.set_label(label);
                popover.popdown();
            });
            row.append(&button);
        }
        keyboard.append(&row);
    }
    popover.set_child(Some(&keyboard));
    picker.set_popover(Some(&popover));
    (picker, selected_usage)
}

/// Macro editor backed by the physically verified FDA1 read/write path.
pub(crate) fn macros_page() -> MacroPage {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.set_margin_top(18);
    page.set_margin_bottom(18);
    page.set_margin_start(24);
    page.set_margin_end(24);

    //let title = gtk::Label::new(Some("Macros"));
    //title.add_css_class("title-2");
    //title.set_xalign(0.0);
    //page.append(&title);

    //let subtitle = gtk::Label::new(Some(
    //"Create and edit the keyboard's 16 macro slots. Device loading and saving will be wired up next.",
    //));
    //subtitle.add_css_class("dim-label");
    //subtitle.set_wrap(true);
    //subtitle.set_xalign(0.0);
    //page.append(&subtitle);

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 18);
    content.set_hexpand(true);
    content.set_vexpand(true);

    // ---- Macro slot list -------------------------------------------------
    let slots_section = gtk::Box::new(gtk::Orientation::Vertical, 8);
    slots_section.set_size_request(170, -1);

    let slots_title = gtk::Label::new(Some("Macro list"));
    slots_title.add_css_class("heading");
    slots_title.set_xalign(0.0);
    slots_section.append(&slots_title);

    let slots_card = gtk::Box::new(gtk::Orientation::Vertical, 0);
    slots_card.add_css_class("card");
    slots_card.set_vexpand(true);

    let slots_scroll = gtk::ScrolledWindow::new();
    slots_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    slots_scroll.set_vexpand(true);

    let slots_list = gtk::ListBox::new();
    slots_list.set_selection_mode(gtk::SelectionMode::Single);
    slots_list.add_css_class("navigation-sidebar");

    let mut slot_labels = Vec::with_capacity(MACRO_SLOT_COUNT);
    for index in 0..MACRO_SLOT_COUNT {
        let row = gtk::ListBoxRow::new();

        let label = gtk::Label::new(Some(&format!("M{index}")));
        label.set_xalign(0.0);
        label.set_margin_top(8);
        label.set_margin_bottom(8);
        label.set_margin_start(12);
        label.set_margin_end(12);

        row.set_child(Some(&label));
        slots_list.append(&row);
        slot_labels.push(label);
    }

    if let Some(row) = slots_list.row_at_index(0) {
        slots_list.select_row(Some(&row));
    }

    slots_scroll.set_child(Some(&slots_list));
    slots_card.append(&slots_scroll);
    slots_section.append(&slots_card);

    // ---- Event editor ----------------------------------------------------
    let editor_section = gtk::Box::new(gtk::Orientation::Vertical, 8);
    editor_section.set_hexpand(true);
    editor_section.set_vexpand(true);

    let editor_heading = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let selected_macro = gtk::Label::new(Some("M0"));
    selected_macro.add_css_class("heading");
    selected_macro.set_xalign(0.0);

    let rename = gtk::MenuButton::new();
    rename.set_icon_name("document-edit-symbolic");
    rename.add_css_class("flat");
    rename.set_tooltip_text(Some("Rename macro locally"));
    let rename_popover = gtk::Popover::new();
    let rename_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    rename_box.set_margin_top(8);
    rename_box.set_margin_bottom(8);
    rename_box.set_margin_start(8);
    rename_box.set_margin_end(8);
    let rename_entry = gtk::Entry::builder()
        .placeholder_text("Macro name")
        .max_length(48)
        .build();
    let rename_save = gtk::Button::with_label("Save Name");
    rename_save.add_css_class("suggested-action");
    rename_box.append(&rename_entry);
    rename_box.append(&rename_save);
    rename_popover.set_child(Some(&rename_box));
    rename.set_popover(Some(&rename_popover));

    let summary = gtk::Label::new(Some("0 steps · 0 ms"));
    summary.add_css_class("dim-label");
    summary.set_hexpand(true);
    summary.set_halign(gtk::Align::End);

    editor_heading.append(&selected_macro);
    editor_heading.append(&rename);
    editor_heading.append(&summary);
    editor_section.append(&editor_heading);

    let events_card = gtk::Box::new(gtk::Orientation::Vertical, 0);
    events_card.add_css_class("card");
    events_card.set_hexpand(true);
    events_card.set_vexpand(true);

    // Step rows are self-explanatory; omit column headers to keep the list clean.

    let events = gtk::Box::new(gtk::Orientation::Vertical, 8);
    events.set_halign(gtk::Align::Fill);
    events.set_valign(gtk::Align::Start);
    events.set_hexpand(true);
    events.set_vexpand(true);

    // Keep step rows away from the rounded card border. This is especially
    // noticeable for the selected/hovered row background.
    events.set_margin_top(8);
    events.set_margin_bottom(8);
    events.set_margin_start(8);
    events.set_margin_end(8);
    let events_scroll = gtk::ScrolledWindow::new();
    events_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    events_scroll.set_vexpand(true);
    events_scroll.set_child(Some(&events));
    events_card.append(&events_scroll);

    editor_section.append(&events_card);

    // Raw firmware events live in a popup instead of taking vertical space in
    // the normal editor. Keep the backing box here so render_selected() can
    // continue rebuilding it whenever the semantic steps change.
    let raw_events = gtk::Box::new(gtk::Orientation::Vertical, 0);

    // ---- Step composer ---------------------------------------------------
    // The composer is a single rounded card aligned with the macro-step card
    // above. Controls inside it affect one step; macro-level actions remain
    // outside the card.
    let footer = gtk::Box::new(gtk::Orientation::Vertical, 8);
    footer.set_margin_top(8);

    let composer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    composer.add_css_class("card");
    composer.set_hexpand(true);

    let composer_content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    composer_content.set_margin_top(12);
    composer_content.set_margin_bottom(12);
    composer_content.set_margin_start(12);
    composer_content.set_margin_end(12);

    // Three semantic rows keeps the controls available to power users without
    // presenting them as one dense toolbar:
    //   1. step type
    //   2. input/action parameters
    //   3. timing + commit
    let type_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let input_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let timing_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let save_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    save_row.set_margin_top(8);

    type_row.set_halign(gtk::Align::Fill);
    input_row.set_halign(gtk::Align::Fill);
    timing_row.set_halign(gtk::Align::Fill);
    save_row.set_halign(gtk::Align::Fill);

    let kind =
        gtk::DropDown::from_strings(&["Keystroke", "Mouse click", "V scroll", "H scroll", "Wait"]);
    kind.set_tooltip_text(Some("Step type"));

    let (keyboard_key, keyboard_usage) = macro_keyboard_picker();
    let capture = gtk::Button::with_label("⌨ Capture");
    capture.set_tooltip_text(Some("Capture a key combination from the keyboard"));
    let modifier_menu = gtk::MenuButton::new();
    modifier_menu.set_label("No modifiers");
    let modifier_popover = gtk::Popover::new();
    let modifier_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    modifier_box.set_margin_top(6);
    modifier_box.set_margin_bottom(6);
    modifier_box.set_margin_start(6);
    modifier_box.set_margin_end(6);
    let ctrl = gtk::ToggleButton::with_label("Ctrl");
    let shift = gtk::ToggleButton::with_label("Shift");
    let alt = gtk::ToggleButton::with_label("Alt");
    let super_key = gtk::ToggleButton::with_label("Super");
    for button in [&ctrl, &shift, &alt, &super_key] {
        modifier_box.append(button);
    }
    modifier_popover.set_child(Some(&modifier_box));
    modifier_menu.set_popover(Some(&modifier_popover));

    {
        let keyboard_key = keyboard_key.clone();
        let keyboard_usage = keyboard_usage.clone();
        let ctrl = ctrl.clone();
        let shift = shift.clone();
        let alt = alt.clone();
        let super_key = super_key.clone();
        install_capture(&capture, move |chord| {
            keyboard_usage.set(chord.usage);
            keyboard_key.set_label(&canonical_action_label(&format!(
                "keyboard/{}/0",
                chord.usage
            )));
            ctrl.set_active(chord.modifiers & MOD_CTRL != 0);
            shift.set_active(chord.modifiers & MOD_SHIFT != 0);
            alt.set_active(chord.modifiers & MOD_ALT != 0);
            super_key.set_active(chord.modifiers & MOD_SUPER != 0);
        });
    }

    for button in [&ctrl, &shift, &alt, &super_key] {
        let modifier_menu = modifier_menu.clone();
        let ctrl = ctrl.clone();
        let shift = shift.clone();
        let alt = alt.clone();
        let super_key = super_key.clone();
        button.connect_toggled(move |_| {
            let modifiers = (if ctrl.is_active() { MOD_CTRL } else { 0 })
                | (if shift.is_active() { MOD_SHIFT } else { 0 })
                | (if alt.is_active() { MOD_ALT } else { 0 })
                | (if super_key.is_active() { MOD_SUPER } else { 0 });
            modifier_menu.set_label(&modifiers_label(modifiers));
        });
    }
    let mouse_button = gtk::DropDown::from_strings(&["Left", "Right", "Middle", "Forward", "Back"]);
    mouse_button.set_visible(false);

    let scroll_direction = gtk::DropDown::from_strings(&["Up", "Down"]);
    scroll_direction.set_visible(false);

    let delay = gtk::SpinButton::with_range(0.0, f64::from(MAX_DELAY_MS), 1.0);
    delay.set_value(20.0);
    delay.set_width_chars(5);

    let timing_label = gtk::Label::new(Some("After"));
    let ms = gtk::Label::new(Some("ms"));
    ms.add_css_class("dim-label");

    {
        let keyboard_key = keyboard_key.clone();
        let capture = capture.clone();
        let modifier_menu = modifier_menu.clone();
        let mouse_button = mouse_button.clone();
        let scroll_direction = scroll_direction.clone();
        let timing_label = timing_label.clone();
        let delay = delay.clone();
        let previous_kind = std::rc::Rc::new(std::cell::Cell::new(0));
        kind.connect_selected_notify(move |kind| {
            let selected = kind.selected();
            let previous = previous_kind.replace(selected);
            keyboard_key.set_visible(selected == 0);
            capture.set_visible(selected == 0);
            modifier_menu.set_visible(selected == 0);
            mouse_button.set_visible(selected == 1);
            scroll_direction.set_visible(matches!(selected, 2 | 3));
            timing_label.set_label(if selected == 4 { "Duration" } else { "After" });
            if selected == 4 && previous != 4 {
                delay.set_value(500.0);
            } else if selected != 4 && previous == 4 {
                delay.set_value(20.0);
            }
            if matches!(selected, 2 | 3) {
                let labels = if selected == 2 {
                    ["Up", "Down"]
                } else {
                    ["Left", "Right"]
                };
                scroll_direction.set_model(Some(&gtk::StringList::new(&labels)));
                scroll_direction.set_selected(0);
            }
        });
    }

    let timing_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    timing_spacer.set_hexpand(true);

    let save_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    save_spacer.set_hexpand(true);

    let add = gtk::Button::with_label("Add Step");
    add.add_css_class("suggested-action");

    let raw_events_button = gtk::MenuButton::new();
    raw_events_button.set_label("Show Raw Events");
    raw_events_button.set_tooltip_text(Some("Show the compiled firmware press/release sequence"));

    let raw_popover = gtk::Popover::new();
    raw_popover.set_has_arrow(true);

    let raw_popup_content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    raw_popup_content.set_margin_top(12);
    raw_popup_content.set_margin_bottom(12);
    raw_popup_content.set_margin_start(12);
    raw_popup_content.set_margin_end(12);
    raw_popup_content.set_size_request(430, 260);

    let raw_title = gtk::Label::new(Some("Raw events"));
    raw_title.add_css_class("heading");
    raw_title.set_xalign(0.0);
    raw_popup_content.append(&raw_title);

    let raw_hint = gtk::Label::new(Some(
        "Compiled firmware press/release sequence for the selected macro.",
    ));
    raw_hint.add_css_class("dim-label");
    raw_hint.set_xalign(0.0);
    raw_hint.set_wrap(true);
    raw_popup_content.append(&raw_hint);

    let raw_scroll = gtk::ScrolledWindow::new();
    raw_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    raw_scroll.set_vexpand(true);
    raw_scroll.set_min_content_height(180);
    raw_scroll.set_child(Some(&raw_events));
    raw_popup_content.append(&raw_scroll);

    raw_popover.set_child(Some(&raw_popup_content));
    raw_events_button.set_popover(Some(&raw_popover));

    let clear = gtk::Button::with_label("Clear");
    clear.set_sensitive(false);

    let revert = gtk::Button::with_label("Revert");
    revert.set_sensitive(false);

    let save = gtk::Button::with_label("Save");
    save.add_css_class("suggested-action");
    save.set_sensitive(false);

    // Row 1: choose the semantic step type.
    type_row.append(&kind);

    // Row 2: configure the action. Only the controls relevant to the selected
    // step type are visible; nothing is hidden merely for compactness.
    input_row.append(&capture);
    input_row.append(&modifier_menu);
    input_row.append(&keyboard_key);
    input_row.append(&mouse_button);
    input_row.append(&scroll_direction);

    // Row 3: timing belongs to the step, while Add Step is the commit action.
    // Keeping them apart makes "After 20 ms" read as configuration rather than
    // as part of the Add button.
    timing_row.append(&timing_label);
    timing_row.append(&delay);
    timing_row.append(&ms);
    timing_row.append(&timing_spacer);
    timing_row.append(&add);

    // Macro-level actions get their own row. Raw Events sits at the left of
    // this row as an inspector, while destructive/commit actions stay grouped
    // at the right.
    save_row.append(&raw_events_button);
    save_row.append(&save_spacer);
    save_row.append(&clear);
    save_row.append(&revert);
    save_row.append(&save);

    composer_content.append(&type_row);
    composer_content.append(&input_row);
    composer_content.append(&timing_row);
    composer.append(&composer_content);

    footer.append(&composer);
    footer.append(&save_row);

    editor_section.append(&footer);

    let macro_page = MacroPage {
        root: page.clone(),
        slots: slots_list.clone(),
        selected_macro,
        summary,
        events,
        raw_events,
        steps: Default::default(),
        original: Default::default(),
        names: std::rc::Rc::new(std::cell::RefCell::new(load_macro_names())),
        saved_steps: std::rc::Rc::new(std::cell::RefCell::new(load_saved_steps())),
        pending_save: Default::default(),
        slot_labels,
        clear,
        revert,
        save,
        kind: kind.clone(),
        capture: capture.clone(),
        keyboard_key: keyboard_key.clone(),
        keyboard_usage: keyboard_usage.clone(),
        ctrl: ctrl.clone(),
        shift: shift.clone(),
        alt: alt.clone(),
        super_key: super_key.clone(),
        mouse_button: mouse_button.clone(),
        scroll_direction: scroll_direction.clone(),
        delay: delay.clone(),
        editing_index: Default::default(),
        add: add.clone(),
    };

    macro_page.refresh_names();

    {
        let macro_page = macro_page.clone();
        kind.connect_selected_notify(move |_| macro_page.update_add_availability());
    }

    {
        let macro_page = macro_page.clone();
        let rename_entry = rename_entry.clone();
        rename_popover.connect_show(move |_| {
            let id = macro_page.selected_id();
            rename_entry.set_text(
                macro_page
                    .names
                    .borrow()
                    .get(&id)
                    .map(String::as_str)
                    .unwrap_or_default(),
            );
            rename_entry.grab_focus();
        });
    }
    {
        let macro_page = macro_page.clone();
        let rename_entry = rename_entry.clone();
        let rename_popover = rename_popover.clone();
        rename_save.connect_clicked(move |_| {
            let id = macro_page.selected_id();
            let name = clean_macro_name(rename_entry.text().as_str());
            if name.is_empty() {
                macro_page.names.borrow_mut().remove(&id);
            } else {
                macro_page.names.borrow_mut().insert(id, name);
            }
            if let Err(error) = save_macro_names(&macro_page.names.borrow()) {
                eprintln!("keyboard-ui: could not save macro names: {error}");
            }
            macro_page.refresh_names();
            rename_popover.popdown();
        });
    }

    {
        let macro_page = macro_page.clone();
        let kind = kind.clone();
        let keyboard_usage = keyboard_usage.clone();
        let ctrl = ctrl.clone();
        let shift = shift.clone();
        let alt = alt.clone();
        let super_key = super_key.clone();
        let mouse_button = mouse_button.clone();
        let scroll_direction = scroll_direction.clone();
        let delay = delay.clone();
        add.connect_clicked(move |_| {
            let mut steps = macro_page.selected_steps();
            let selected_kind = kind.selected();
            let delay_ms = delay.value_as_int() as u16;
            let step = match selected_kind {
                0 => MacroStep::Chord {
                    modifiers: (if ctrl.is_active() { MOD_CTRL } else { 0 })
                        | (if shift.is_active() { MOD_SHIFT } else { 0 })
                        | (if alt.is_active() { MOD_ALT } else { 0 })
                        | (if super_key.is_active() { MOD_SUPER } else { 0 }),
                    key: keyboard_usage.get(),
                    delay_ms,
                },
                1 => MacroStep::MouseClick {
                    button: [1, 2, 4, 8, 16]
                        .get(mouse_button.selected() as usize)
                        .copied()
                        .unwrap_or(1),
                    delay_ms,
                },
                2 | 3 => MacroStep::Scroll {
                    horizontal: selected_kind == 3,
                    negative: scroll_direction.selected() != 0,
                    delay_ms,
                },
                4 => MacroStep::Wait(delay_ms),
                _ => return,
            };
            if let Some(index) = macro_page
                .editing_index
                .get()
                .filter(|index| !matches!(steps.get(*index), Some(MacroStep::Raw(_))))
            {
                if index < steps.len() {
                    steps[index] = step;
                } else {
                    steps.push(step);
                }
            } else {
                steps.push(step);
            }
            macro_page.set_selected_steps(steps);
        });
    }
    {
        let macro_page = macro_page.clone();
        macro_page.clear.clone().connect_clicked(move |_| {
            macro_page.set_selected_steps(Vec::new());
        });
    }
    {
        let macro_page = macro_page.clone();
        macro_page.revert.clone().connect_clicked(move |_| {
            let id = macro_page.selected_id();
            let steps = macro_page
                .original
                .borrow()
                .iter()
                .find(|(macro_id, _)| *macro_id == id)
                .map(|(_, steps)| steps.clone())
                .unwrap_or_default();
            macro_page.set_selected_steps(steps);
        });
    }

    // Switching slots updates the read-only view. No device state is touched.
    {
        let macro_page = macro_page.clone();
        slots_list.connect_row_selected(move |_, row| {
            if row.is_some() {
                macro_page.reset_editor();
                macro_page.render_selected();
            }
        });
    }

    content.append(&slots_section);
    content.append(&editor_section);
    let content_clamp = adw::Clamp::builder()
        .maximum_size(900)
        .tightening_threshold(700)
        .child(&content)
        .build();
    content_clamp.set_hexpand(true);
    content_clamp.set_vexpand(true);
    page.append(&content_clamp);

    macro_page.render_selected();
    macro_page
}
