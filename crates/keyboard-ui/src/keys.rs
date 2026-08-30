use std::collections::HashMap;

use adw::prelude::*;
use keyboard_core::PHYSICAL_KEYS;

use crate::chord::{CapturedChord, chord_label, install_capture};
use crate::kle::{
    KLE_GRID_UNITS_PER_KEY, KLE_KEY_UNIT, KleKey, KleLayout, active_kle_layout,
    install_kle_key_css, kle_primary_legend,
};

type PaletteAction = (&'static str, &'static str);

const LETTER_ACTIONS: &[PaletteAction] = &[
    ("Q", "keyboard/20/0"),
    ("W", "keyboard/26/0"),
    ("E", "keyboard/8/0"),
    ("R", "keyboard/21/0"),
    ("T", "keyboard/23/0"),
    ("Y", "keyboard/28/0"),
    ("U", "keyboard/24/0"),
    ("I", "keyboard/12/0"),
    ("O", "keyboard/18/0"),
    ("P", "keyboard/19/0"),
    ("A", "keyboard/4/0"),
    ("S", "keyboard/22/0"),
    ("D", "keyboard/7/0"),
    ("F", "keyboard/9/0"),
    ("G", "keyboard/10/0"),
    ("H", "keyboard/11/0"),
    ("J", "keyboard/13/0"),
    ("K", "keyboard/14/0"),
    ("L", "keyboard/15/0"),
    ("Z", "keyboard/29/0"),
    ("X", "keyboard/27/0"),
    ("C", "keyboard/6/0"),
    ("V", "keyboard/25/0"),
    ("B", "keyboard/5/0"),
    ("N", "keyboard/17/0"),
    ("M", "keyboard/16/0"),
];

const DIGIT_ACTIONS: &[PaletteAction] = &[
    ("1", "keyboard/30/0"),
    ("2", "keyboard/31/0"),
    ("3", "keyboard/32/0"),
    ("4", "keyboard/33/0"),
    ("5", "keyboard/34/0"),
    ("6", "keyboard/35/0"),
    ("7", "keyboard/36/0"),
    ("8", "keyboard/37/0"),
    ("9", "keyboard/38/0"),
    ("0", "keyboard/39/0"),
];

const SYMBOL_ACTIONS: &[PaletteAction] = &[
    ("-", "keyboard/45/0"),
    ("=", "keyboard/46/0"),
    ("[", "keyboard/47/0"),
    ("]", "keyboard/48/0"),
    ("\\", "keyboard/49/0"),
    (";", "keyboard/51/0"),
    ("'", "keyboard/52/0"),
    ("`", "keyboard/53/0"),
    (",", "keyboard/54/0"),
    (".", "keyboard/55/0"),
    ("/", "keyboard/56/0"),
    ("!", "keyboard/30/2"),
    ("@", "keyboard/31/2"),
    ("#", "keyboard/32/2"),
    ("$", "keyboard/33/2"),
    ("%", "keyboard/34/2"),
    ("^", "keyboard/35/2"),
    ("&", "keyboard/36/2"),
    ("*", "keyboard/37/2"),
    ("(", "keyboard/38/2"),
    (")", "keyboard/39/2"),
    ("_", "keyboard/45/2"),
    ("+", "keyboard/46/2"),
    ("{", "keyboard/47/2"),
    ("}", "keyboard/48/2"),
    ("|", "keyboard/49/2"),
    (":", "keyboard/51/2"),
    ("\"", "keyboard/52/2"),
    ("~", "keyboard/53/2"),
    ("<", "keyboard/54/2"),
    (">", "keyboard/55/2"),
    ("?", "keyboard/56/2"),
];

const CONTROL_ACTIONS: &[PaletteAction] = &[
    ("Esc", "keyboard/41/0"),
    ("Tab", "keyboard/43/0"),
    ("Enter", "keyboard/40/0"),
    ("Space", "keyboard/44/0"),
    ("Backspace", "keyboard/42/0"),
    ("Delete", "keyboard/76/0"),
    ("Caps Lock", "keyboard/57/0"),
    ("Insert", "keyboard/73/0"),
];

const NAVIGATION_ACTIONS: &[PaletteAction] = &[
    ("Left", "keyboard/80/0"),
    ("Right", "keyboard/79/0"),
    ("Up", "keyboard/82/0"),
    ("Down", "keyboard/81/0"),
    ("Home", "keyboard/74/0"),
    ("End", "keyboard/77/0"),
    ("Page Up", "keyboard/75/0"),
    ("Page Down", "keyboard/78/0"),
];

const MODIFIER_ACTIONS: &[PaletteAction] = &[
    ("Left Shift", "keyboard/0/2"),
    ("Right Shift", "keyboard/0/32"),
    ("Left Ctrl", "keyboard/0/1"),
    ("Right Ctrl", "keyboard/0/16"),
    ("Left Alt", "keyboard/0/4"),
    ("Right Alt", "keyboard/0/64"),
    ("Left Super", "keyboard/0/8"),
    ("Right Super", "keyboard/0/128"),
    ("Fn", "function-layer"),
];

const FKEY_ACTIONS: &[PaletteAction] = &[
    ("F1", "keyboard/58/0"),
    ("F2", "keyboard/59/0"),
    ("F3", "keyboard/60/0"),
    ("F4", "keyboard/61/0"),
    ("F5", "keyboard/62/0"),
    ("F6", "keyboard/63/0"),
    ("F7", "keyboard/64/0"),
    ("F8", "keyboard/65/0"),
    ("F9", "keyboard/66/0"),
    ("F10", "keyboard/67/0"),
    ("F11", "keyboard/68/0"),
    ("F12", "keyboard/69/0"),
    ("F13", "keyboard/104/0"),
    ("F14", "keyboard/105/0"),
    ("F15", "keyboard/106/0"),
    ("F16", "keyboard/107/0"),
    ("F17", "keyboard/108/0"),
    ("F18", "keyboard/109/0"),
    ("F19", "keyboard/110/0"),
    ("F20", "keyboard/111/0"),
    ("F21", "keyboard/112/0"),
    ("F22", "keyboard/113/0"),
    ("F23", "keyboard/114/0"),
    ("F24", "keyboard/115/0"),
];

const MEDIA_ACTIONS: &[PaletteAction] = &[
    ("Volume +", "consumer/233"),
    ("Volume −", "consumer/234"),
    ("Mute", "consumer/226"),
    ("Play / Pause", "consumer/205"),
    ("Stop", "consumer/183"),
    ("Previous", "consumer/182"),
    ("Next", "consumer/181"),
    ("Media Player", "consumer/387"),
    ("Web Home", "consumer/547"),
    ("Refresh", "consumer/551"),
    ("Web Stop", "consumer/550"),
    ("Forward", "consumer/549"),
    ("Back", "consumer/548"),
    ("Favorites", "consumer/554"),
    ("Search", "consumer/545"),
    ("Calculator", "consumer/402"),
    ("Computer", "consumer/404"),
    ("Mail", "consumer/394"),
    ("Brightness −", "consumer/112"),
    ("Brightness +", "consumer/111"),
];

const MOUSE_ACTIONS: &[PaletteAction] = &[
    ("Left Click", "mouse-button/1/0"),
    ("Right Click", "mouse-button/2/0"),
    ("Middle Click", "mouse-button/4/0"),
    ("Back", "mouse-button/8/0"),
    ("Forward", "mouse-button/16/0"),
    ("Wheel Up", "mouse-button/0/1"),
    ("Wheel Down", "mouse-button/0/-1"),
];

const SYSTEM_ACTIONS: &[PaletteAction] = &[
    ("Power", "power/1"),
    ("Sleep", "power/2"),
    ("Wake", "power/4"),
];

const KEYBOARD_CONTROL_ACTIONS: &[PaletteAction] = &[
    ("LED On / Off", "firmware/0"),
    ("LED Brightness +", "firmware/1"),
    ("LED Brightness −", "firmware/2"),
    ("Next LED Effect", "firmware/3"),
    ("Next LED Colour", "firmware/4"),
    ("LED Speed +", "firmware/5"),
    ("LED Speed −", "firmware/6"),
    ("Keyboard Lock", "firmware/7"),
];

const MACRO_ACTIONS: &[PaletteAction] = &[
    ("M0", "macro/0"),
    ("M1", "macro/1"),
    ("M2", "macro/2"),
    ("M3", "macro/3"),
    ("M4", "macro/4"),
    ("M5", "macro/5"),
    ("M6", "macro/6"),
    ("M7", "macro/7"),
    ("M8", "macro/8"),
    ("M9", "macro/9"),
    ("M10", "macro/10"),
    ("M11", "macro/11"),
    ("M12", "macro/12"),
    ("M13", "macro/13"),
    ("M14", "macro/14"),
    ("M15", "macro/15"),
];

pub(crate) fn compact_keycap_label(label: &str) -> String {
    // Keep the physical KLE geometry stable even when an assignment has a long
    // human-readable name. This is display-only: the canonical action string
    // and full label remain unchanged in the summary, tooltip, and draft.
    match label {
        "LED Brightness +" => return "LED+".into(),
        "LED Brightness −" => return "LED−".into(),
        "LED Speed +" => return "Spd+".into(),
        "LED Speed −" => return "Spd−".into(),
        _ => {}
    }

    let compact: String = label
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();

    match compact.as_str() {
        "escape" | "esc" => "Esc".into(),
        "backspace" | "bksp" => "Bksp".into(),
        "capslock" | "caps" => "Caps".into(),
        "printscreen" | "prtsc" => "PrtSc".into(),
        "pageup" | "pgup" => "PgUp".into(),
        "pagedown" | "pgdn" => "PgDn".into(),
        "insert" | "ins" => "Ins".into(),
        "delete" | "del" => "Del".into(),
        "left" | "leftarrow" => "←".into(),
        "right" | "rightarrow" => "→".into(),
        "up" | "uparrow" => "↑".into(),
        "down" | "downarrow" => "↓".into(),
        "volumeup" | "volup" => "Vol+".into(),
        "volumedown" | "voldown" => "Vol−".into(),
        "mute" | "volumemute" => "Mute".into(),
        "playpause" | "mediaplaypause" => "▶/⏸".into(),
        "next" | "nexttrack" | "medianexttrack" => "⏭".into(),
        "previous" | "prev" | "previoustrack" | "mediaprevioustrack" => "⏮".into(),
        "stop" | "mediastop" => "■".into(),
        "brightnessup" | "brightup" => "Brt+".into(),
        "brightnessdown" | "brightdown" => "Brt−".into(),
        "ledonoff" => "LED".into(),
        "nextledeffect" => "Fx+".into(),
        "nextledcolour" | "nextledcolor" => "Col+".into(),
        "keyboardlock" => "Lock".into(),
        "mouseleftclick" => "MLeft".into(),
        "mouserightclick" => "MRight".into(),
        "mousemiddleclick" => "MMid".into(),
        "mouseback" => "MBack".into(),
        "mouseforward" => "MFwd".into(),
        "wheelup" => "Wh↑".into(),
        "wheeldown" => "Wh↓".into(),
        "leftcontrol" | "leftctrl" | "lctrl" => "Ctrl".into(),
        "rightcontrol" | "rightctrl" | "rctrl" => "RCtrl".into(),
        "leftalt" | "lalt" => "Alt".into(),
        "rightalt" | "ralt" => "RAlt".into(),
        "leftshift" | "lshift" => "Shift".into(),
        "rightshift" | "rshift" => "RShift".into(),
        "leftsuper" | "lsuper" | "leftmeta" | "lmeta" | "windows" | "win" => "Super".into(),
        "rightsuper" | "rsuper" | "rightmeta" | "rmeta" => "RSuper".into(),
        _ => label.to_string(),
    }
}

pub(crate) type KeysPage = (
    gtk::Box,
    gtk::DropDown,
    HashMap<String, gtk::ToggleButton>,
    HashMap<String, gtk::Label>,
    HashMap<String, gtk::Label>,
    gtk::Label,
    gtk::Label,
    gtk::Entry,
    gtk::Button,
    gtk::Button,
    Vec<(gtk::Button, &'static str)>,
    ShortcutControl,
);

#[derive(Clone)]
pub(crate) struct ShortcutControl {
    capture: gtk::Button,
    label: gtk::Label,
    assign: gtk::Button,
    chord: std::rc::Rc<std::cell::Cell<Option<CapturedChord>>>,
    key_selected: std::rc::Rc<std::cell::Cell<bool>>,
}

impl ShortcutControl {
    fn refresh(&self, empty_text: &str) {
        let selected = self.key_selected.get();
        self.capture.set_sensitive(selected);
        if let Some(chord) = self.chord.get() {
            self.label.set_text(&chord_label(chord));
            self.assign.set_sensitive(selected);
        } else {
            self.label.set_text(empty_text);
            self.assign.set_sensitive(false);
        }
    }

    pub(crate) fn set_selected_action(&self, selected: bool, action: &str) {
        self.key_selected.set(selected);
        self.chord.set(parse_shortcut_action(action));
        self.refresh(if selected {
            "Capture a key with Ctrl, Shift, Alt, or Super"
        } else {
            "Select a key above first"
        });
    }

    pub(crate) fn connect_assign<F: Fn(String) + 'static>(&self, callback: F) {
        let chord = self.chord.clone();
        self.assign.connect_clicked(move |_| {
            if let Some(action) = chord.get().and_then(shortcut_action) {
                callback(action);
            }
        });
    }
}

fn shortcut_action(chord: CapturedChord) -> Option<String> {
    (chord.modifiers != 0 && chord.usage != 0)
        .then(|| format!("keyboard/{}/{}", chord.usage, chord.modifiers))
}

fn parse_shortcut_action(action: &str) -> Option<CapturedChord> {
    let mut parts = action.split('/');
    if parts.next()? != "keyboard" {
        return None;
    }
    let usage = parts.next()?.parse::<u8>().ok()?;
    let modifiers = parts.next()?.parse::<u8>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    let chord = CapturedChord { modifiers, usage };
    shortcut_action(chord).map(|_| chord)
}

fn palette_button(label: &'static str, action: &'static str) -> gtk::Button {
    let button = gtk::Button::with_label(label);
    button.add_css_class("palette-button");
    button.set_tooltip_text(Some(action));
    button
}

fn keyboard_geometry(key_id: &str, label: &str) -> Option<(i32, i32, i32)> {
    // Positions are expressed in half-key units. This lets the UI mirror the
    // actual keycap proportions instead of forcing every physical key into a
    // generic 15-column matrix.
    let id = key_id.to_ascii_lowercase();
    let text = label.trim();

    let special = match id.as_str() {
        "escape" | "esc" => Some((0, 0, 2)),
        "backspace" => Some((26, 0, 4)),
        "tab" => Some((0, 1, 3)),
        "caps-lock" | "capslock" | "caps" => Some((0, 2, 3)),
        "enter" | "return" => Some((25, 2, 4)),
        "shift-left" => Some((0, 3, 4)),
        "shift-right" => Some((26, 3, 4)),
        "control-left" | "ctrl-left" => Some((0, 4, 2)),
        "meta-left" | "super-left" | "gui-left" => Some((2, 4, 2)),
        "alt-left" => Some((4, 4, 2)),
        "space" => Some((6, 4, 14)),
        "alt-right" => Some((20, 4, 2)),
        "left" | "arrow-left" => Some((22, 4, 2)),
        "down" | "arrow-down" => Some((24, 4, 2)),
        "right" | "arrow-right" => Some((26, 4, 2)),
        "fn" => Some((28, 4, 2)),
        "up" | "arrow-up" => Some((24, 3, 2)),
        _ => None,
    };
    if special.is_some() {
        return special;
    }

    // Fall back to the printed physical legend for ordinary keys. This keeps
    // the geometry independent of whatever action is currently assigned.
    if text.len() == 1 {
        let ch = text.chars().next()?;
        if let Some(i) = "1234567890".find(ch) {
            return Some((2 + (i as i32 * 2), 0, 2));
        }
        if let Some(i) = "QWERTYUIOP".find(ch.to_ascii_uppercase()) {
            return Some((3 + (i as i32 * 2), 1, 2));
        }
        if let Some(i) = "ASDFGHJKL".find(ch.to_ascii_uppercase()) {
            return Some((3 + (i as i32 * 2), 2, 2));
        }
        if let Some(i) = "ZXCVBNM".find(ch.to_ascii_uppercase()) {
            return Some((4 + (i as i32 * 2), 3, 2));
        }
        return match ch {
            '-' => Some((22, 0, 2)),
            '=' => Some((24, 0, 2)),
            '[' => Some((23, 1, 2)),
            ']' => Some((25, 1, 2)),
            '\\' => Some((27, 1, 2)),
            ';' => Some((21, 2, 2)),
            '\'' => Some((23, 2, 2)),
            ',' => Some((18, 3, 2)),
            '.' => Some((20, 3, 2)),
            '/' => Some((22, 3, 2)),
            _ => None,
        };
    }

    // Some physical legends intentionally use shorter human labels.
    match text.to_ascii_lowercase().as_str() {
        "back" | "delete" => Some((26, 0, 4)),
        "caps" | "caps lock" => Some((0, 2, 3)),
        "shift" => Some((0, 3, 4)),
        "rshift" => Some((26, 3, 4)),
        "ctrl" | "control" => Some((0, 4, 2)),
        "super" | "win" => Some((2, 4, 2)),
        "alt" => Some((4, 4, 2)),
        "ralt" => Some((20, 4, 2)),
        "space" => Some((6, 4, 14)),
        "left" => Some((22, 4, 2)),
        "down" => Some((24, 4, 2)),
        "right" => Some((26, 4, 2)),
        "up" => Some((24, 3, 2)),
        "fn" => Some((28, 4, 2)),
        "esc" => Some((0, 0, 2)),
        "tab" => Some((0, 1, 3)),
        "enter" | "return" => Some((25, 2, 4)),
        _ => None,
    }
}

fn normalized_key_identity(text: &str) -> String {
    let trimmed = text.trim().to_ascii_lowercase();
    match trimmed.as_str() {
        "escape" => "esc".to_string(),
        "return" => "enter".to_string(),
        "delete" | "back" | "bksp" => "backspace".to_string(),
        "control" | "lctrl" | "left ctrl" => "ctrl".to_string(),
        "left alt" | "lalt" => "alt".to_string(),
        "right alt" => "ralt".to_string(),
        "caps" => "caps lock".to_string(),
        "left shift" | "lshift" => "shift".to_string(),
        "right shift" => "rshift".to_string(),
        "win" | "meta" => "super".to_string(),
        other => other.to_string(),
    }
}

fn kle_legend_identities(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("<i "))
        // Ignore Fn-layer legends when an ordinary physical legend is also
        // present. Exact matching only needs one useful identity candidate.
        .map(normalized_key_identity)
        .collect()
}

fn match_kle_keys_to_physical(layout: &KleLayout) -> HashMap<String, usize> {
    // Prefer semantic legend matching whenever it is unique. This is much more
    // reliable for ordinary keys than relying on PHYSICAL_KEYS iteration order or
    // approximate geometry (and guarantees keys such as '=' receive the KLE slot
    // whose colour/size belongs to '=').
    //
    // Geometry remains the fallback for blank/icon-only/ambiguous KLE keys such
    // as Space, Super, and some modifier keys.
    let kle_identities: Vec<Vec<String>> = layout
        .keys
        .iter()
        .map(|key| kle_legend_identities(&key.legend))
        .collect();

    let mut matches = HashMap::new();
    let mut claimed: HashMap<usize, String> = HashMap::new();

    for key in PHYSICAL_KEYS {
        let wanted = normalized_key_identity(key.label);

        let semantic_candidates: Vec<usize> = kle_identities
            .iter()
            .enumerate()
            .filter_map(|(index, ids)| ids.iter().any(|id| id == &wanted).then_some(index))
            .collect();

        let semantic_match = if semantic_candidates.len() == 1 {
            Some(semantic_candidates[0])
        } else {
            None
        };

        let geometry_match = || {
            let (column, row, span) = keyboard_geometry(key.id, key.label)?;
            let expected_x = f64::from(column) / 2.0;
            let expected_y = f64::from(row);
            let expected_w = f64::from(span) / 2.0;

            layout
                .keys
                .iter()
                .enumerate()
                .filter(|(index, _)| !claimed.contains_key(index))
                .min_by(|(_, a), (_, b)| {
                    let score = |candidate: &KleKey| {
                        (candidate.y - expected_y).abs() * 1000.0
                            + (candidate.x - expected_x).abs() * 10.0
                            + (candidate.w - expected_w).abs() * 2.0
                    };
                    score(a)
                        .partial_cmp(&score(b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(index, _)| index)
        };

        let index = semantic_match
            .filter(|index| !claimed.contains_key(index))
            .or_else(geometry_match);

        if let Some(index) = index {
            claimed.insert(index, key.id.to_string());
            matches.insert(key.id.to_string(), index);
        } else {
            eprintln!("No KLE key matched physical key {} ({})", key.id, key.label);
        }
    }

    matches
}

pub(crate) fn keys_page() -> KeysPage {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 7);
    page.set_margin_start(18);
    page.set_margin_end(18);
    page.set_margin_top(8);
    page.set_margin_bottom(14);

    // Keep DropDown internally because the rest of the application already uses it,
    // but present it as two visible layer buttons so the active layer is always obvious.
    let layer_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    layer_row.set_halign(gtk::Align::Center);
    let bank = gtk::DropDown::from_strings(&["Base", "Fn"]);
    bank.set_visible(false);
    let base_layer = gtk::ToggleButton::with_label("Base");
    let fn_layer = gtk::ToggleButton::with_label("Fn");
    fn_layer.set_group(Some(&base_layer));
    base_layer.set_active(true);
    {
        let bank = bank.clone();
        base_layer.connect_toggled(move |button| {
            if button.is_active() && bank.selected() != 0 {
                bank.set_selected(0);
            }
        });
    }
    {
        let bank = bank.clone();
        fn_layer.connect_toggled(move |button| {
            if button.is_active() && bank.selected() != 1 {
                bank.set_selected(1);
            }
        });
    }
    {
        let base_layer = base_layer.clone();
        let fn_layer = fn_layer.clone();
        bank.connect_selected_notify(move |dropdown| {
            let base = dropdown.selected() == 0;
            base_layer.set_active(base);
            fn_layer.set_active(!base);
        });
    }
    layer_row.append(&base_layer);
    layer_row.append(&fn_layer);
    page.append(&layer_row);

    // Render the physical board from the bundled Keyboard Layout Editor JSON.
    // PHYSICAL_KEYS remains the source of key identity; KLE controls geometry.
    //
    // Use a homogeneous GTK grid instead of absolute pixel placement. KLE units
    // are converted to quarter-key grid cells, so the whole board can shrink with
    // the window while keeping the relative widths of Tab/Caps/Space/etc.
    let kle_layout = active_kle_layout();
    if let Some(layout) = &kle_layout {
        install_kle_key_css(layout);
    }
    let kle_matches = kle_layout
        .as_ref()
        .map(match_kle_keys_to_physical)
        .unwrap_or_default();

    let keyboard = gtk::Grid::builder()
        .column_homogeneous(true)
        .row_homogeneous(true)
        .hexpand(true)
        .vexpand(false)
        .build();

    let mut buttons = HashMap::new();
    let mut assignments = HashMap::new();
    let mut legends = HashMap::new();
    for key in PHYSICAL_KEYS {
        let button = gtk::ToggleButton::new();
        button.add_css_class("keyboard-key");
        button.set_hexpand(true);
        button.set_vexpand(true);
        button.set_margin_start(2);
        button.set_margin_end(2);
        button.set_margin_top(2);
        button.set_margin_bottom(2);
        button.set_tooltip_text(Some(key.id));

        // Keep text out of the key's size negotiation.  The KLE geometry should
        // decide the key size, not whichever layer happens to have the longest
        // assignment label.  Overlay children do not contribute to the overlay's
        // preferred size, so the tiny measuring child is the only thing GTK uses
        // when calculating the grid.
        let cap_overlay = gtk::Overlay::new();
        let cap_measure = gtk::Box::new(gtk::Orientation::Vertical, 0);
        cap_measure.set_size_request(0, 34);
        cap_overlay.set_child(Some(&cap_measure));

        let cap = gtk::Box::new(gtk::Orientation::Vertical, 0);
        cap.set_halign(gtk::Align::Fill);
        cap.set_valign(gtk::Align::Center);
        let assignment = gtk::Label::new(Some(&compact_keycap_label(key.label)));
        assignment.add_css_class("keyboard-assignment");
        assignment.set_hexpand(true);
        assignment.set_halign(gtk::Align::Fill);
        assignment.set_valign(gtk::Align::Center);
        assignment.set_margin_top(1);
        assignment.set_margin_bottom(1);
        assignment.set_ellipsize(gtk::pango::EllipsizeMode::End);
        let legend = gtk::Label::new(Some(&compact_keycap_label(key.label)));
        legend.add_css_class("keyboard-legend");
        legend.set_hexpand(true);
        legend.set_halign(gtk::Align::Fill);
        legend.set_ellipsize(gtk::pango::EllipsizeMode::End);
        legend.set_visible(false);
        cap.append(&assignment);
        cap.append(&legend);
        cap_overlay.add_overlay(&cap);
        button.set_child(Some(&cap_overlay));

        if let Some((kle_index, kle_key)) = kle_layout.as_ref().and_then(|layout| {
            kle_matches
                .get(key.id)
                .and_then(|index| layout.keys.get(*index).map(|kle_key| (*index, kle_key)))
        }) {
            button.add_css_class(&format!("kle-key-{kle_index}"));

            if let Some(kle_legend) = kle_primary_legend(&kle_key.legend) {
                button.set_tooltip_text(Some(&format!("{} · KLE: {kle_legend}", key.id)));
            }

            let x = (kle_key.x * KLE_GRID_UNITS_PER_KEY).round() as i32;
            let y = (kle_key.y * KLE_GRID_UNITS_PER_KEY).round() as i32;
            let w = (kle_key.w * KLE_GRID_UNITS_PER_KEY).round().max(1.0) as i32;
            let h = (kle_key.h * KLE_GRID_UNITS_PER_KEY).round().max(1.0) as i32;
            keyboard.attach(&button, x, y, w, h);
        } else {
            // Built-in fallback keeps the configurator usable if the bundled KLE
            // file is malformed or no longer matches PHYSICAL_KEYS. Fallback
            // geometry is expressed in half-key units, so multiply by two to get
            // the quarter-key grid used above.
            let (column, row, span) = keyboard_geometry(key.id, key.label)
                .unwrap_or_else(|| (i32::from(key.index % 15) * 2, i32::from(key.index / 15), 2));
            keyboard.attach(&button, column * 2, row * 4, span * 2, 4);
        }

        buttons.insert(key.id.to_string(), button);
        assignments.insert(key.id.to_string(), assignment);
        legends.insert(key.id.to_string(), legend);
    }

    // Scale the board as one physical object.  KLE geometry has a stable aspect
    // ratio, so preserve it while letting the whole board grow/shrink with the
    // window.  This also prevents a longer Base/Fn assignment label from making
    // one grid column wider than another.
    let keyboard_ratio = kle_layout
        .as_ref()
        .map(|layout| (layout.width / layout.height) as f32)
        .unwrap_or(3.0);
    let keyboard_frame = gtk::AspectFrame::new(0.5, 0.5, keyboard_ratio, false);
    keyboard_frame.set_child(Some(&keyboard));
    keyboard_frame.set_hexpand(true);
    keyboard_frame.set_vexpand(false);
    keyboard_frame.set_valign(gtk::Align::Start);

    // Keep a comfortable desktop maximum, but below it the AspectFrame scales
    // proportionally to whatever width the page has available.
    let keyboard_max_width = kle_layout
        .as_ref()
        .map(|layout| (layout.width * KLE_KEY_UNIT).round() as i32)
        .unwrap_or(960);
    let keyboard_clamp = adw::Clamp::builder()
        .maximum_size(keyboard_max_width)
        .child(&keyboard_frame)
        .build();
    keyboard_clamp.set_hexpand(true);
    keyboard_clamp.set_vexpand(false);
    keyboard_clamp.set_valign(gtk::Align::Start);
    page.append(&keyboard_clamp);

    // Give the physical board and editor a little visual separation without
    // reintroducing the large vertical gap from earlier iterations.
    let keyboard_separator = gtk::Separator::new(gtk::Orientation::Horizontal);
    keyboard_separator.set_margin_top(5);
    keyboard_separator.set_margin_bottom(5);
    page.append(&keyboard_separator);

    // Everything below the keyboard is the flexible editor region.  Let this
    // area absorb extra window height while the physical keyboard keeps its
    // natural KLE-driven size.
    let editor_area = gtk::Box::new(gtk::Orientation::Vertical, 8);
    editor_area.set_vexpand(true);
    editor_area.set_valign(gtk::Align::Fill);
    page.append(&editor_area);

    // Selection summary: separate “what am I editing?” from “what should it do?”.
    let summary = gtk::Box::new(gtk::Orientation::Vertical, 2);
    summary.set_halign(gtk::Align::Center);
    let selected = gtk::Label::new(Some("Select a key"));
    selected.add_css_class("key-summary-title");
    let current = gtk::Label::new(Some("Choose a key above to edit its assignment"));
    current.add_css_class("key-summary-detail");
    summary.append(&selected);
    summary.append(&current);
    editor_area.append(&summary);

    // Assignment palette. Keep the common keyboard vocabulary discoverable,
    // while protocol-dependent categories can exist now as explicit placeholders.
    // Action buttons use a small number of centered rows. This keeps the palette
    // compact while avoiding horizontal scrolling for longer categories.
    let palette_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    palette_header.set_halign(gtk::Align::Center);
    let palette_category = gtk::DropDown::from_strings(&[
        "Letters",
        "Digits",
        "Symbols",
        "Control",
        "Navigation",
        "Modifiers",
        "Shortcuts",
        "F-keys",
        "Media",
        "Mouse",
        "Keyboard Controls",
        "System",
        "Macros",
    ]);
    palette_category.add_css_class("palette-category");
    palette_header.append(&palette_category);

    // Advanced editing is a mode, not another action category. Keep it beside
    // the category picker so the normal palette stays compact.
    let advanced_toggle = gtk::ToggleButton::builder()
        .icon_name("document-edit-symbolic")
        .tooltip_text("Advanced action")
        .build();
    advanced_toggle.add_css_class("flat");
    palette_header.append(&advanced_toggle);
    editor_area.append(&palette_header);

    let palette_stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .vhomogeneous(true)
        .build();
    palette_stack.add_css_class("palette-strip");
    editor_area.append(&palette_stack);

    let mut palette_actions: Vec<(gtk::Button, &'static str)> = Vec::new();

    let shortcut_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    shortcut_row.set_halign(gtk::Align::Center);
    shortcut_row.set_margin_top(8);
    shortcut_row.set_margin_bottom(8);
    let shortcut_capture = gtk::Button::with_label("⌨ Capture");
    shortcut_capture.set_tooltip_text(Some("Capture one shortcut from the keyboard"));
    shortcut_capture.set_sensitive(false);
    let shortcut_label = gtk::Label::new(Some("Select a key above first"));
    shortcut_label.set_width_chars(28);
    shortcut_label.set_xalign(0.0);
    let shortcut_assign = gtk::Button::with_label("Assign");
    shortcut_assign.add_css_class("suggested-action");
    shortcut_assign.set_sensitive(false);
    shortcut_row.append(&shortcut_capture);
    shortcut_row.append(&shortcut_label);
    shortcut_row.append(&shortcut_assign);

    let shortcut = ShortcutControl {
        capture: shortcut_capture.clone(),
        label: shortcut_label.clone(),
        assign: shortcut_assign,
        chord: Default::default(),
        key_selected: Default::default(),
    };
    {
        let shortcut = shortcut.clone();
        install_capture(&shortcut_capture, move |chord| {
            if chord.modifiers == 0 {
                shortcut.chord.set(None);
                shortcut.refresh("Include Ctrl, Shift, Alt, or Super");
            } else {
                shortcut.chord.set(Some(chord));
                shortcut.refresh("");
            }
        });
    }

    fn action_rows(
        actions: &[(&'static str, &'static str)],
        per_row: usize,
    ) -> (gtk::Box, Vec<(gtk::Button, &'static str)>) {
        let rows = gtk::Box::new(gtk::Orientation::Vertical, 6);
        rows.set_halign(gtk::Align::Fill);
        rows.set_margin_start(8);
        rows.set_margin_end(8);
        rows.set_margin_top(3);
        rows.set_margin_bottom(3);

        let mut buttons = Vec::new();
        for chunk in actions.chunks(per_row.max(1)) {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            row.set_halign(gtk::Align::Center);

            for &(label, action_name) in chunk {
                let button = palette_button(label, action_name);
                button.set_sensitive(false);
                row.append(&button);
                buttons.push((button, action_name));
            }

            rows.append(&row);
        }

        (rows, buttons)
    }

    // Pick balanced row lengths for the built-in categories. Letters become
    // 13 + 13, Symbols become two equal rows, and the smaller categories stay
    // on a single centered row.
    for (name, title, actions, per_row) in [
        ("letters", "Letters", LETTER_ACTIONS, 13),
        ("digits", "Digits", DIGIT_ACTIONS, 10),
        ("symbols", "Symbols", SYMBOL_ACTIONS, 16),
        ("control", "Control", CONTROL_ACTIONS, 8),
        ("navigation", "Navigation", NAVIGATION_ACTIONS, 8),
        ("modifiers", "Modifiers", MODIFIER_ACTIONS, 8),
    ] {
        let (rows, buttons) = action_rows(actions, per_row);
        palette_actions.extend(buttons);
        palette_stack.add_titled(&rows, Some(name), title);
    }

    palette_stack.add_titled(&shortcut_row, Some("shortcuts"), "Shortcuts");

    for (name, title, actions, per_row) in [
        ("fkeys", "F-keys", FKEY_ACTIONS, 12),
        ("media", "Media", MEDIA_ACTIONS, 7),
        ("mouse", "Mouse", MOUSE_ACTIONS, 7),
        (
            "keyboard-controls",
            "Keyboard Controls",
            KEYBOARD_CONTROL_ACTIONS,
            4,
        ),
        ("system", "System", SYSTEM_ACTIONS, 3),
        ("macros", "Macros", MACRO_ACTIONS, 8),
    ] {
        let (rows, buttons) = action_rows(actions, per_row);
        palette_actions.extend(buttons);
        palette_stack.add_titled(&rows, Some(name), title);
    }
    // Keep the raw editor out of the normal workflow. The icon button above
    // enters/exits advanced mode; while active the normal category picker is
    // disabled and the action palette is replaced by this single input row.
    let advanced = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    advanced.set_margin_top(8);
    advanced.set_margin_bottom(8);
    advanced.set_margin_start(12);
    advanced.set_margin_end(12);
    advanced.set_halign(gtk::Align::Fill);
    advanced.set_visible(false);
    let advanced_label = gtk::Label::new(Some("Raw action"));
    advanced.append(&advanced_label);
    let action = gtk::Entry::builder()
        .placeholder_text("keyboard/usage/modifiers")
        .hexpand(true)
        .sensitive(false)
        .build();
    advanced.append(&action);
    editor_area.append(&advanced);

    // The dropdown is navigation, not a value being edited. Keep the stack
    // names stable so adding categories later does not affect action handling.
    {
        let palette_stack = palette_stack.clone();
        palette_category.connect_selected_notify(move |dropdown| {
            const PAGE_NAMES: &[&str] = &[
                "letters",
                "digits",
                "symbols",
                "control",
                "navigation",
                "modifiers",
                "shortcuts",
                "fkeys",
                "media",
                "mouse",
                "keyboard-controls",
                "system",
                "macros",
            ];
            if let Some(name) = PAGE_NAMES.get(dropdown.selected() as usize) {
                palette_stack.set_visible_child_name(name);
            }
        });
    }
    {
        let palette_category = palette_category.clone();
        let palette_stack = palette_stack.clone();
        let advanced = advanced.clone();
        advanced_toggle.connect_toggled(move |toggle| {
            let enabled = toggle.is_active();
            palette_category.set_sensitive(!enabled);
            palette_stack.set_visible(!enabled);
            advanced.set_visible(enabled);
        });
    }
    palette_stack.set_visible_child_name("letters");

    // Let the editor region absorb all spare height above the map-level actions.
    // This pins the footer to the true bottom of the Keys page at any window
    // height without stretching the keyboard itself.
    let editor_spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    editor_spacer.set_vexpand(true);
    editor_area.append(&editor_spacer);

    // Saving is a map-level operation, so label it as such instead of implying the
    // button only applies the currently selected key.
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    footer.set_margin_top(8);
    footer.set_halign(gtk::Align::Fill);

    let footer_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    footer_spacer.set_hexpand(true);
    footer.append(&footer_spacer);

    let revert = gtk::Button::with_label("Discard Changes");
    revert.set_sensitive(false);
    footer.append(&revert);

    let apply = gtk::Button::with_label("Save to Keyboard");
    apply.add_css_class("suggested-action");
    apply.set_sensitive(false);
    footer.append(&apply);
    footer.set_valign(gtk::Align::End);
    editor_area.append(&footer);

    (
        page,
        bank,
        buttons,
        assignments,
        legends,
        selected,
        current,
        action,
        apply,
        revert,
        palette_actions,
        shortcut,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_palette_actions_use_canonical_syntax() {
        for &(label, action) in LETTER_ACTIONS
            .iter()
            .chain(DIGIT_ACTIONS)
            .chain(SYMBOL_ACTIONS)
            .chain(CONTROL_ACTIONS)
            .chain(NAVIGATION_ACTIONS)
            .chain(MODIFIER_ACTIONS)
            .chain(FKEY_ACTIONS)
        {
            if action == "function-layer" {
                assert_eq!(label, "Fn");
                continue;
            }
            let parts: Vec<_> = action.split('/').collect();
            assert_eq!(parts.len(), 3, "{label} has noncanonical action {action}");
            assert_eq!(parts[0], "keyboard", "{label} has wrong action kind");
            parts[1]
                .parse::<u8>()
                .unwrap_or_else(|_| panic!("{label} has invalid HID usage in {action}"));
            parts[2]
                .parse::<u8>()
                .unwrap_or_else(|_| panic!("{label} has invalid modifier mask in {action}"));
        }

        assert!(LETTER_ACTIONS.contains(&("Q", "keyboard/20/0")));
        assert!(SYMBOL_ACTIONS.contains(&("!", "keyboard/30/2")));
        assert!(NAVIGATION_ACTIONS.contains(&("Home", "keyboard/74/0")));
        assert!(MODIFIER_ACTIONS.contains(&("Right Alt", "keyboard/0/64")));
        assert!(MODIFIER_ACTIONS.contains(&("Fn", "function-layer")));
        assert!(FKEY_ACTIONS.contains(&("F24", "keyboard/115/0")));
    }

    #[test]
    fn media_and_system_palette_actions_use_canonical_syntax() {
        for &(label, action) in MEDIA_ACTIONS {
            let usage = action
                .strip_prefix("consumer/")
                .unwrap_or_else(|| panic!("{label} has noncanonical action {action}"));
            usage
                .parse::<u16>()
                .unwrap_or_else(|_| panic!("{label} has invalid consumer usage in {action}"));
        }
        for &(label, action) in SYSTEM_ACTIONS {
            let usage = action
                .strip_prefix("power/")
                .unwrap_or_else(|| panic!("{label} has noncanonical action {action}"));
            usage
                .parse::<u8>()
                .unwrap_or_else(|_| panic!("{label} has invalid power usage in {action}"));
        }

        assert!(MEDIA_ACTIONS.contains(&("Play / Pause", "consumer/205")));
        for &(label, action) in MOUSE_ACTIONS {
            let parts: Vec<_> = action.split('/').collect();
            assert_eq!(parts.len(), 3, "{label} has noncanonical action {action}");
            assert_eq!(parts[0], "mouse-button", "{label} has wrong action kind");
            parts[1]
                .parse::<u8>()
                .unwrap_or_else(|_| panic!("{label} has invalid button mask in {action}"));
            parts[2]
                .parse::<i8>()
                .unwrap_or_else(|_| panic!("{label} has invalid wheel value in {action}"));
        }
        assert!(MOUSE_ACTIONS.contains(&("Left Click", "mouse-button/1/0")));
        assert!(MOUSE_ACTIONS.contains(&("Wheel Down", "mouse-button/0/-1")));
        assert!(SYSTEM_ACTIONS.contains(&("Sleep", "power/2")));
        for &(label, action) in KEYBOARD_CONTROL_ACTIONS {
            let code = action
                .strip_prefix("firmware/")
                .unwrap_or_else(|| panic!("{label} has noncanonical action {action}"));
            code.parse::<u8>()
                .unwrap_or_else(|_| panic!("{label} has invalid firmware code in {action}"));
        }
        assert!(KEYBOARD_CONTROL_ACTIONS.contains(&("LED On / Off", "firmware/0")));
        assert!(KEYBOARD_CONTROL_ACTIONS.contains(&("Next LED Effect", "firmware/3")));
        assert!(KEYBOARD_CONTROL_ACTIONS.contains(&("Keyboard Lock", "firmware/7")));
        for &(label, action) in MACRO_ACTIONS {
            let id = action
                .strip_prefix("macro/")
                .unwrap_or_else(|| panic!("{label} has noncanonical action {action}"))
                .parse::<u8>()
                .unwrap_or_else(|_| panic!("{label} has invalid macro id in {action}"));
            assert!(id <= 15, "{label} has out-of-range macro id in {action}");
        }
    }

    #[test]
    fn shortcuts_are_direct_keyboard_actions_without_macro_slots() {
        let chord = CapturedChord {
            modifiers: 3,
            usage: 6,
        };
        assert_eq!(shortcut_action(chord).as_deref(), Some("keyboard/6/3"));
        assert_eq!(parse_shortcut_action("keyboard/6/3"), Some(chord));
        assert_eq!(parse_shortcut_action("macro/0"), None);
        assert_eq!(parse_shortcut_action("keyboard/6/0"), None);
        assert_eq!(parse_shortcut_action("keyboard/0/1"), None);
    }

    #[test]
    fn keyboard_control_labels_stay_compact_and_directional() {
        assert_eq!(compact_keycap_label("LED On / Off"), "LED");
        assert_eq!(compact_keycap_label("LED Brightness +"), "LED+");
        assert_eq!(compact_keycap_label("LED Brightness −"), "LED−");
        assert_eq!(compact_keycap_label("LED Speed +"), "Spd+");
        assert_eq!(compact_keycap_label("LED Speed −"), "Spd−");
        assert_eq!(compact_keycap_label("Keyboard Lock"), "Lock");
    }
}
