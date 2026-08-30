/// Convert the daemon's canonical action format into a compact user-facing key label.
pub fn canonical_action_label(action: &str) -> String {
    let parts: Vec<_> = action.split('/').collect();
    match parts.as_slice() {
        ["function-layer"] => "Fn".to_string(),
        ["keyboard", usage, modifiers] => {
            let Ok(usage) = usage.parse::<u8>() else {
                return action.to_string();
            };
            let Ok(modifiers) = modifiers.parse::<u8>() else {
                return action.to_string();
            };
            keyboard_label(usage, modifiers)
        }
        ["consumer", usage] => usage
            .parse::<u16>()
            .ok()
            .map(consumer_label)
            .unwrap_or_else(|| action.to_string()),
        ["mouse-button", buttons, wheel] => mouse_button_label(buttons, wheel),
        ["mouse-move", x, y, wheel] => format!("Mouse {x},{y} / {wheel}"),
        ["macro", id] => format!("Macro {id}"),
        ["firmware", code] => code
            .parse::<u8>()
            .ok()
            .map(firmware_label)
            .unwrap_or_else(|| action.to_string()),
        ["power", "1"] => "Power".to_string(),
        ["power", "2"] => "Sleep".to_string(),
        ["power", "4"] => "Wake".to_string(),
        ["power", code] => format!("Power {code}"),
        ["raw", kind, _, _, _] => format!("Raw {kind}"),
        _ => action.to_string(),
    }
}

fn mouse_button_label(buttons: &str, wheel: &str) -> String {
    match (buttons, wheel) {
        ("1", "0") => "Mouse Left Click".to_string(),
        ("2", "0") => "Mouse Right Click".to_string(),
        ("4", "0") => "Mouse Middle Click".to_string(),
        ("8", "0") => "Mouse Back".to_string(),
        ("16", "0") => "Mouse Forward".to_string(),
        ("0", "1") => "Wheel Up".to_string(),
        ("0", "-1") => "Wheel Down".to_string(),
        _ => format!("Mouse {buttons} / Wheel {wheel}"),
    }
}

fn keyboard_label(usage: u8, modifiers: u8) -> String {
    let key = hid_usage_label(usage);
    let mut names = Vec::new();
    for (mask, name) in [
        (0x01, "Ctrl"),
        (0x02, "Shift"),
        (0x04, "Alt"),
        (0x08, "Super"),
        (0x10, "RCtrl"),
        (0x20, "RShift"),
        (0x40, "RAlt"),
        (0x80, "RSuper"),
    ] {
        if modifiers & mask != 0 {
            names.push(name);
        }
    }
    if usage != 0 || names.is_empty() {
        names.push(&key);
    }
    names.join(" + ")
}

fn hid_usage_label(usage: u8) -> String {
    match usage {
        0 => "Disabled".to_string(),
        0x04..=0x1D => ((b'A' + usage - 0x04) as char).to_string(),
        0x1E..=0x26 => (usage - 0x1D).to_string(),
        0x27 => "0".to_string(),
        0x28 => "Enter".to_string(),
        0x29 => "Esc".to_string(),
        0x2A => "Backspace".to_string(),
        0x2B => "Tab".to_string(),
        0x2C => "Space".to_string(),
        0x2D => "-".to_string(),
        0x2E => "=".to_string(),
        0x2F => "[".to_string(),
        0x30 => "]".to_string(),
        0x31 => "\\".to_string(),
        0x33 => ";".to_string(),
        0x34 => "'".to_string(),
        0x35 => "`".to_string(),
        0x36 => ",".to_string(),
        0x37 => ".".to_string(),
        0x38 => "/".to_string(),
        0x39 => "Caps Lock".to_string(),
        0x3A..=0x45 => format!("F{}", usage - 0x39),
        0x46 => "Print Screen".to_string(),
        0x47 => "Scroll Lock".to_string(),
        0x48 => "Pause".to_string(),
        0x49 => "Insert".to_string(),
        0x4A => "Home".to_string(),
        0x4B => "Page Up".to_string(),
        0x4C => "Delete".to_string(),
        0x4D => "End".to_string(),
        0x4E => "Page Down".to_string(),
        0x4F => "Right".to_string(),
        0x50 => "Left".to_string(),
        0x51 => "Down".to_string(),
        0x52 => "Up".to_string(),
        0x53 => "Num Lock".to_string(),
        0x65 => "Menu".to_string(),
        0x66 => "Power".to_string(),
        0x68..=0x73 => format!("F{}", usage - 0x5B),
        0xE0 => "Left Ctrl".to_string(),
        0xE1 => "Left Shift".to_string(),
        0xE2 => "Left Alt".to_string(),
        0xE3 => "Left Super".to_string(),
        0xE4 => "Right Ctrl".to_string(),
        0xE5 => "Right Shift".to_string(),
        0xE6 => "Right Alt".to_string(),
        0xE7 => "Right Super".to_string(),
        _ => format!("Key 0x{usage:02X}"),
    }
}

fn consumer_label(usage: u16) -> String {
    match usage {
        0x00B5 => "Next Track".to_string(),
        0x00B6 => "Previous Track".to_string(),
        0x00B7 => "Stop".to_string(),
        0x00CD => "Play / Pause".to_string(),
        0x00E2 => "Mute".to_string(),
        0x00E9 => "Volume Up".to_string(),
        0x00EA => "Volume Down".to_string(),
        0x006F => "Brightness Up".to_string(),
        0x0070 => "Brightness Down".to_string(),
        0x0183 => "Media Player".to_string(),
        0x018A => "Mail".to_string(),
        0x0192 => "Calculator".to_string(),
        0x0194 => "Computer".to_string(),
        0x0221 => "Search".to_string(),
        0x0223 => "Web Home".to_string(),
        0x0224 => "Back".to_string(),
        0x0225 => "Forward".to_string(),
        0x0226 => "Web Stop".to_string(),
        0x0227 => "Refresh".to_string(),
        0x022A => "Favorites".to_string(),
        _ => format!("Media 0x{usage:04X}"),
    }
}

fn firmware_label(code: u8) -> String {
    match code {
        0 => "LED On / Off".to_string(),
        1 => "LED Brightness +".to_string(),
        2 => "LED Brightness −".to_string(),
        3 => "Next LED Effect".to_string(),
        4 => "Next LED Colour".to_string(),
        5 => "LED Speed +".to_string(),
        6 => "LED Speed −".to_string(),
        7 => "Keyboard Lock".to_string(),
        19 => "F-key / Media Mode".to_string(),
        36 => "WASD / Arrow Mode".to_string(),
        _ => format!("Keyboard Control {code}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_actions_have_compact_human_labels() {
        let cases = [
            ("keyboard/82/0", "Up"),
            ("keyboard/74/0", "Home"),
            ("keyboard/69/0", "F12"),
            ("keyboard/115/0", "F24"),
            ("keyboard/224/0", "Left Ctrl"),
            ("keyboard/231/0", "Right Super"),
            ("keyboard/6/3", "Ctrl + Shift + C"),
            ("consumer/233", "Volume Up"),
            ("consumer/551", "Refresh"),
            ("mouse-button/1/0", "Mouse Left Click"),
            ("mouse-button/0/-1", "Wheel Down"),
            ("power/2", "Sleep"),
            ("function-layer", "Fn"),
            ("macro/4", "Macro 4"),
            ("firmware/0", "LED On / Off"),
            ("firmware/3", "Next LED Effect"),
            ("firmware/7", "Keyboard Lock"),
            ("firmware/19", "F-key / Media Mode"),
            ("firmware/36", "WASD / Arrow Mode"),
            ("firmware/99", "Keyboard Control 99"),
        ];
        for (canonical, expected) in cases {
            assert_eq!(canonical_action_label(canonical), expected);
        }
    }

    #[test]
    fn malformed_and_unknown_actions_remain_identifiable() {
        assert_eq!(canonical_action_label("keyboard/nope/0"), "keyboard/nope/0");
        assert_eq!(canonical_action_label("keyboard/255/0"), "Key 0xFF");
        assert_eq!(canonical_action_label("firmware/nope"), "firmware/nope");
        assert_eq!(canonical_action_label("future/action"), "future/action");
    }
}
