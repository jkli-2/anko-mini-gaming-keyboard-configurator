use keyboard_core::KeyAction;

pub fn encode_key_action(action: KeyAction) -> [u8; 4] {
    match action {
        KeyAction::Keyboard { modifiers, usage } => [0x20, modifiers, usage, 0],
        KeyAction::FunctionLayer => [0x20, 0, 0xFF, 0],
        KeyAction::Consumer { usage } => {
            let [low, high] = usage.to_le_bytes();
            [0x30, low, high, 0]
        }
        KeyAction::MouseButton {
            buttons,
            vertical_wheel,
        } => [0x10, buttons, 0, vertical_wheel as u8],
        KeyAction::MouseMove { x, y, wheel } => [0x11, x as u8, y as u8, wheel as u8],
        KeyAction::Macro { id } => [0x60, id, 1, 0],
        KeyAction::Firmware { code } => [0x1F, code, 0, 0],
        KeyAction::Power { code } => [0x40, code, 0, 0],
        KeyAction::Raw { kind, codes } => [kind, codes[0], codes[1], codes[2]],
    }
}

pub fn decode_key_action(record: [u8; 4]) -> KeyAction {
    let [kind, code1, code2, code3] = record;
    match record {
        [0x20, 0, 0xFF, 0] => KeyAction::FunctionLayer,
        [0x20, modifiers, usage, 0] => KeyAction::Keyboard { modifiers, usage },
        [0x30, low, high, 0] => KeyAction::Consumer {
            usage: u16::from_le_bytes([low, high]),
        },
        [0x10, buttons, 0, wheel] => KeyAction::MouseButton {
            buttons,
            vertical_wheel: wheel as i8,
        },
        [0x11, x, y, wheel] => KeyAction::MouseMove {
            x: x as i8,
            y: y as i8,
            wheel: wheel as i8,
        },
        [0x60, id, 1, 0] => KeyAction::Macro { id },
        [0x1F, code, 0, 0] => KeyAction::Firmware { code },
        [0x40, code, 0, 0] => KeyAction::Power { code },
        _ => KeyAction::Raw {
            kind,
            codes: [code1, code2, code3],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_actions_have_exact_records_and_round_trip() {
        let cases = [
            (
                KeyAction::Keyboard {
                    modifiers: 0,
                    usage: 0x29,
                },
                [0x20, 0, 0x29, 0],
            ),
            (KeyAction::FunctionLayer, [0x20, 0, 0xFF, 0]),
            (
                KeyAction::Consumer { usage: 0x01_83 },
                [0x30, 0x83, 0x01, 0],
            ),
            (
                KeyAction::MouseButton {
                    buttons: 1,
                    vertical_wheel: -1,
                },
                [0x10, 1, 0, 0xFF],
            ),
            (
                KeyAction::MouseMove {
                    x: -2,
                    y: 3,
                    wheel: 0,
                },
                [0x11, 0xFE, 3, 0],
            ),
            (KeyAction::Macro { id: 15 }, [0x60, 15, 1, 0]),
            (KeyAction::Firmware { code: 7 }, [0x1F, 7, 0, 0]),
            (KeyAction::Power { code: 2 }, [0x40, 2, 0, 0]),
        ];

        for (action, record) in cases {
            assert_eq!(encode_key_action(action), record);
            assert_eq!(decode_key_action(record), action);
        }
    }

    #[test]
    fn unknown_and_noncanonical_records_are_lossless() {
        let records = [[0x13, 1, 2, 3], [0x60, 2, 9, 0], [0x20, 1, 2, 3]];
        for record in records {
            assert_eq!(encode_key_action(decode_key_action(record)), record);
        }
    }
}
