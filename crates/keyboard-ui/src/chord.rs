use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;
use keyboard_core::canonical_action_label;

pub(crate) const MOD_CTRL: u8 = 1;
pub(crate) const MOD_SHIFT: u8 = 2;
pub(crate) const MOD_ALT: u8 = 4;
pub(crate) const MOD_SUPER: u8 = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CapturedChord {
    pub modifiers: u8,
    pub usage: u8,
}

pub(crate) fn chord_label(chord: CapturedChord) -> String {
    canonical_action_label(&format!("keyboard/{}/{}", chord.usage, chord.modifiers))
}

pub(crate) fn modifiers_label(modifiers: u8) -> String {
    let mut parts = Vec::new();
    for (bit, label) in [
        (MOD_CTRL, "Ctrl"),
        (MOD_SHIFT, "Shift"),
        (MOD_ALT, "Alt"),
        (MOD_SUPER, "Super"),
    ] {
        if modifiers & bit != 0 {
            parts.push(label);
        }
    }
    if parts.is_empty() {
        "No modifiers".to_string()
    } else {
        parts.join(" + ")
    }
}

pub(crate) fn install_capture<F>(button: &gtk::Button, on_captured: F)
where
    F: Fn(CapturedChord) + 'static,
{
    let capturing = Rc::new(Cell::new(false));
    {
        let capturing = capturing.clone();
        button.connect_clicked(move |button| {
            capturing.set(true);
            button.set_label("Press a key…");
            button.grab_focus();
        });
    }

    let key_controller = gtk::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let button = button.clone();
        let capturing = capturing.clone();
        key_controller.connect_key_pressed(move |_, key, _, state| {
            if !capturing.get() {
                return gtk::glib::Propagation::Proceed;
            }
            let Some(usage) = hid_usage_from_gdk_key(key) else {
                return gtk::glib::Propagation::Stop;
            };
            on_captured(CapturedChord {
                modifiers: chord_modifiers(state),
                usage,
            });
            capturing.set(false);
            button.set_label("⌨ Capture");
            gtk::glib::Propagation::Stop
        });
    }
    button.add_controller(key_controller);

    {
        let button = button.clone();
        let button_on_leave = button.clone();
        let capturing = capturing.clone();
        let focus = gtk::EventControllerFocus::new();
        focus.connect_leave(move |_| {
            if capturing.replace(false) {
                button_on_leave.set_label("⌨ Capture");
            }
        });
        button.add_controller(focus);
    }
}

fn hid_usage_from_gdk_key(key: gtk::gdk::Key) -> Option<u8> {
    if let Some(character) = key.to_unicode() {
        let character = character.to_ascii_lowercase();
        if character.is_ascii_lowercase() {
            return Some(character as u8 - b'a' + 4);
        }
        if let Some(index) = "1234567890".find(character) {
            return Some(30 + index as u8);
        }
        if let Some((_, usage)) = [
            ('-', 45),
            ('=', 46),
            ('[', 47),
            (']', 48),
            ('\\', 49),
            (';', 51),
            ('\'', 52),
            ('`', 53),
            (',', 54),
            ('.', 55),
            ('/', 56),
            (' ', 44),
            ('!', 30),
            ('@', 31),
            ('#', 32),
            ('$', 33),
            ('%', 34),
            ('^', 35),
            ('&', 36),
            ('*', 37),
            ('(', 38),
            (')', 39),
            ('_', 45),
            ('+', 46),
            ('{', 47),
            ('}', 48),
            ('|', 49),
            (':', 51),
            ('"', 52),
            ('~', 53),
            ('<', 54),
            ('>', 55),
            ('?', 56),
        ]
        .into_iter()
        .find(|(candidate, _)| *candidate == character)
        {
            return Some(usage);
        }
    }
    let name = key.name()?;
    match name.as_str() {
        "Escape" => Some(41),
        "Return" | "KP_Enter" => Some(40),
        "BackSpace" => Some(42),
        "Tab" | "ISO_Left_Tab" => Some(43),
        "Caps_Lock" => Some(57),
        "Insert" => Some(73),
        "Home" => Some(74),
        "Page_Up" => Some(75),
        "Delete" => Some(76),
        "End" => Some(77),
        "Page_Down" => Some(78),
        "Right" => Some(79),
        "Left" => Some(80),
        "Down" => Some(81),
        "Up" => Some(82),
        name if name.starts_with('F') => name[1..]
            .parse::<u8>()
            .ok()
            .filter(|number| (1..=24).contains(number))
            .map(|number| {
                if number <= 12 {
                    57 + number
                } else {
                    91 + number
                }
            }),
        _ => None,
    }
}

fn chord_modifiers(state: gtk::gdk::ModifierType) -> u8 {
    (if state.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
        MOD_CTRL
    } else {
        0
    }) | (if state.contains(gtk::gdk::ModifierType::SHIFT_MASK) {
        MOD_SHIFT
    } else {
        0
    }) | (if state.contains(gtk::gdk::ModifierType::ALT_MASK) {
        MOD_ALT
    } else {
        0
    }) | (if state.contains(gtk::gdk::ModifierType::SUPER_MASK) {
        MOD_SUPER
    } else {
        0
    })
}
