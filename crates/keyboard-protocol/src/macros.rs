use std::error::Error;
use std::fmt;

use keyboard_core::{Macro, MacroEvent, MacroEventAction, MacroEventKind};

pub const MACRO_STORAGE_SIZE: usize = 4096;
const POINTER_TABLE_SIZE: usize = 64;
const USABLE_MACRO_COUNT: u8 = 16;
const EVENT_SIZE: usize = 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MacroCodecError {
    StorageSize { expected: usize, actual: usize },
    InvalidMacroId(u8),
    DuplicateMacroId(u8),
    EmptyMacro(u8),
    StorageOverflow,
    InvalidPointer { id: u8, offset: u16 },
    UnterminatedMacro(u8),
    UnknownEventKind(u8),
}

impl fmt::Display for MacroCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for MacroCodecError {}

pub fn encode_macro_storage(macros: &[Macro]) -> Result<[u8; MACRO_STORAGE_SIZE], MacroCodecError> {
    let mut storage = [0; MACRO_STORAGE_SIZE];
    storage[..POINTER_TABLE_SIZE].fill(0xFF);
    let mut cursor = POINTER_TABLE_SIZE;
    let mut used = [false; USABLE_MACRO_COUNT as usize];

    for definition in macros {
        if definition.id >= USABLE_MACRO_COUNT {
            return Err(MacroCodecError::InvalidMacroId(definition.id));
        }
        if used[definition.id as usize] {
            return Err(MacroCodecError::DuplicateMacroId(definition.id));
        }
        if definition.events.is_empty() {
            return Err(MacroCodecError::EmptyMacro(definition.id));
        }
        used[definition.id as usize] = true;

        let needed = definition
            .events
            .len()
            .checked_mul(EVENT_SIZE)
            .ok_or(MacroCodecError::StorageOverflow)?;
        if cursor + needed > MACRO_STORAGE_SIZE {
            return Err(MacroCodecError::StorageOverflow);
        }
        storage[definition.id as usize * 2..definition.id as usize * 2 + 2]
            .copy_from_slice(&(cursor as u16).to_le_bytes());

        for (index, event) in definition.events.iter().enumerate() {
            let mut flags = event_kind_bits(event.kind);
            if event.action == MacroEventAction::Press {
                flags |= 0x40;
            }
            if index + 1 == definition.events.len() {
                flags |= 0x80;
            }
            storage[cursor..cursor + 2].copy_from_slice(&event.delay_ms.to_le_bytes());
            storage[cursor + 2] = flags;
            storage[cursor + 3] = event.code;
            cursor += EVENT_SIZE;
        }
    }
    Ok(storage)
}

pub fn decode_macro_storage(storage: &[u8]) -> Result<Vec<Macro>, MacroCodecError> {
    if storage.len() != MACRO_STORAGE_SIZE {
        return Err(MacroCodecError::StorageSize {
            expected: MACRO_STORAGE_SIZE,
            actual: storage.len(),
        });
    }
    let mut macros = Vec::new();
    for id in 0..USABLE_MACRO_COUNT {
        let pointer_index = id as usize * 2;
        let pointer_bytes = [storage[pointer_index], storage[pointer_index + 1]];
        if pointer_bytes == [0xFF, 0xFF] || pointer_bytes == [0, 0] {
            continue;
        }
        let offset = u16::from_le_bytes(pointer_bytes);
        let mut cursor = offset as usize;
        if cursor < POINTER_TABLE_SIZE || cursor + EVENT_SIZE > storage.len() {
            return Err(MacroCodecError::InvalidPointer { id, offset });
        }
        let mut events = Vec::new();
        loop {
            if cursor + EVENT_SIZE > storage.len() {
                return Err(MacroCodecError::UnterminatedMacro(id));
            }
            let flags = storage[cursor + 2];
            let kind_bits = flags & 0x3F;
            let kind = match kind_bits {
                3 => MacroEventKind::MouseButton,
                2 => MacroEventKind::Keyboard,
                4 => MacroEventKind::VerticalScroll,
                5 => MacroEventKind::HorizontalScroll,
                other => return Err(MacroCodecError::UnknownEventKind(other)),
            };
            events.push(MacroEvent {
                delay_ms: u16::from_le_bytes([storage[cursor], storage[cursor + 1]]),
                kind,
                action: if flags & 0x40 != 0 {
                    MacroEventAction::Press
                } else {
                    MacroEventAction::Release
                },
                code: storage[cursor + 3],
            });
            cursor += EVENT_SIZE;
            if flags & 0x80 != 0 || flags == 0 {
                break;
            }
        }
        macros.push(Macro { id, events });
    }
    Ok(macros)
}

fn event_kind_bits(kind: MacroEventKind) -> u8 {
    match kind {
        MacroEventKind::MouseButton => 3,
        MacroEventKind::Keyboard => 2,
        MacroEventKind::VerticalScroll => 4,
        MacroEventKind::HorizontalScroll => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_macros() -> Vec<Macro> {
        vec![
            Macro {
                id: 0,
                events: vec![
                    MacroEvent {
                        delay_ms: 25,
                        kind: MacroEventKind::Keyboard,
                        action: MacroEventAction::Press,
                        code: 4,
                    },
                    MacroEvent {
                        delay_ms: 10,
                        kind: MacroEventKind::Keyboard,
                        action: MacroEventAction::Release,
                        code: 4,
                    },
                ],
            },
            Macro {
                id: 15,
                events: vec![MacroEvent {
                    delay_ms: 300,
                    kind: MacroEventKind::VerticalScroll,
                    action: MacroEventAction::Release,
                    code: 0xFF,
                }],
            },
        ]
    }

    #[test]
    fn macro_storage_has_exact_pointer_and_event_encoding() {
        let storage = encode_macro_storage(&fixture_macros()).unwrap();
        assert_eq!(&storage[0..2], &[64, 0]);
        assert!(storage[2..30].iter().all(|byte| *byte == 0xFF));
        assert_eq!(&storage[30..32], &[72, 0]);
        assert_eq!(&storage[64..72], &[25, 0, 0x42, 4, 10, 0, 0x82, 4]);
        assert_eq!(&storage[72..76], &[0x2C, 0x01, 0x84, 0xFF]);
    }

    #[test]
    fn understood_macro_events_round_trip() {
        let macros = fixture_macros();
        let storage = encode_macro_storage(&macros).unwrap();
        assert_eq!(decode_macro_storage(&storage).unwrap(), macros);
    }

    #[test]
    fn malformed_macro_storage_is_rejected() {
        assert_eq!(
            decode_macro_storage(&[0; 10]),
            Err(MacroCodecError::StorageSize {
                expected: 4096,
                actual: 10
            })
        );
        let mut storage = [0xFF; MACRO_STORAGE_SIZE];
        storage[0..2].copy_from_slice(&4095u16.to_le_bytes());
        assert_eq!(
            decode_macro_storage(&storage),
            Err(MacroCodecError::InvalidPointer {
                id: 0,
                offset: 4095
            })
        );
    }
}
