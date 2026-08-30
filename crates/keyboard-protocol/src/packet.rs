use std::error::Error;
use std::fmt;

use keyboard_core::{KeyAction, KeyBank, Rgb};

use crate::{ProtocolError, REPORT_ID, REPORT_SIZE, encode_key_action};

const FAMILY: u8 = 0x06;
pub const BLOCK_DATA_SIZE: usize = 56;
const MACRO_WRITE_DATA_SIZE: usize = 59;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PacketError {
    EmptyPayload,
    PayloadTooLong { maximum: usize, actual: usize },
    MisalignedPayload { record_size: usize, actual: usize },
    OffsetOverflow,
}

impl fmt::Display for PacketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPayload => write!(f, "packet payload must not be empty"),
            Self::PayloadTooLong { maximum, actual } => {
                write!(
                    f,
                    "packet payload is too long: maximum {maximum}, got {actual}"
                )
            }
            Self::MisalignedPayload {
                record_size,
                actual,
            } => write!(
                f,
                "payload length {actual} is not a multiple of record size {record_size}"
            ),
            Self::OffsetOverflow => write!(f, "protocol byte offset exceeds 16 bits"),
        }
    }
}

impl Error for PacketError {}

pub fn encode_le_u16(value: u16) -> [u8; 2] {
    value.to_le_bytes()
}

pub fn decode_le_u16(bytes: [u8; 2]) -> u16 {
    u16::from_le_bytes(bytes)
}

pub fn encode_keymap_read(bank: KeyBank, offset: u16) -> [u8; REPORT_SIZE + 1] {
    let mut report = command(0x08);
    report[3] = 0x3A;
    report[4..6].copy_from_slice(&offset.to_le_bytes());
    report[7] = wire_layer(bank);
    report
}

pub fn encode_keymap_bulk_block(
    bank: KeyBank,
    offset: u16,
    data: &[u8],
) -> Result<[u8; REPORT_SIZE + 1], PacketError> {
    validate_payload(data, BLOCK_DATA_SIZE)?;
    if data.len() % 4 != 0 {
        return Err(PacketError::MisalignedPayload {
            record_size: 4,
            actual: data.len(),
        });
    }
    encode_bulk_block(0x09, Some(bank), offset, data)
}

pub fn encode_key_write(
    bank: KeyBank,
    key_index: u16,
    action: KeyAction,
) -> Result<[u8; REPORT_SIZE + 1], PacketError> {
    let offset = key_index
        .checked_mul(4)
        .ok_or(PacketError::OffsetOverflow)?;
    let mut report = command(0x10);
    report[3] = 7;
    report[4..6].copy_from_slice(&offset.to_le_bytes());
    report[7] = wire_layer(bank);
    report[9..13].copy_from_slice(&encode_key_action(action));
    Ok(report)
}

pub fn encode_rgb_read(offset: u16) -> [u8; REPORT_SIZE + 1] {
    let mut report = command(0x13);
    report[3] = 0x3A;
    report[4..6].copy_from_slice(&offset.to_le_bytes());
    report
}

pub fn encode_rgb_bulk_block(
    offset: u16,
    data: &[u8],
) -> Result<[u8; REPORT_SIZE + 1], PacketError> {
    encode_bulk_block(0x12, None, offset, data)
}

pub fn encode_rgb_values(colors: &[Rgb]) -> Vec<u8> {
    colors
        .iter()
        .flat_map(|color| [color.r, color.g, color.b])
        .collect()
}

pub fn decode_rgb_values(bytes: &[u8]) -> Result<Vec<Rgb>, PacketError> {
    if bytes.len() % 3 != 0 {
        return Err(PacketError::MisalignedPayload {
            record_size: 3,
            actual: bytes.len(),
        });
    }
    Ok(bytes
        .chunks_exact(3)
        .map(|rgb| Rgb {
            r: rgb[0],
            g: rgb[1],
            b: rgb[2],
        })
        .collect())
}

/// Extract the data area shared by keymap and per-key RGB block replies.
pub fn decode_block_response(
    response: &[u8],
    requested_len: usize,
) -> Result<&[u8], ProtocolError> {
    const DATA_START: usize = 8;
    let required = DATA_START + requested_len;
    if response.len() < required {
        return Err(ProtocolError::ResponseTooShort {
            expected: required,
            actual: response.len(),
        });
    }
    if response[0] != 0xAA {
        return Err(ProtocolError::UnexpectedReplyPrefix(response[0]));
    }
    Ok(&response[DATA_START..required])
}

pub fn encode_key_color_write(
    key_index: u16,
    rgb: Rgb,
) -> Result<[u8; REPORT_SIZE + 1], PacketError> {
    let offset = key_index
        .checked_mul(3)
        .ok_or(PacketError::OffsetOverflow)?;
    let mut report = command(0x14);
    report[3] = 3;
    report[4..6].copy_from_slice(&offset.to_le_bytes());
    report[9..12].copy_from_slice(&[rgb.r, rgb.g, rgb.b]);
    Ok(report)
}

pub fn encode_macro_read(length: usize, offset: u16) -> Result<[u8; REPORT_SIZE + 1], PacketError> {
    if length == 0 {
        return Err(PacketError::EmptyPayload);
    }
    if length > BLOCK_DATA_SIZE {
        return Err(PacketError::PayloadTooLong {
            maximum: BLOCK_DATA_SIZE,
            actual: length,
        });
    }
    let mut report = command(0x0C);
    report[3] = length as u8;
    report[4..6].copy_from_slice(&offset.to_le_bytes());
    Ok(report)
}

pub fn encode_macro_write(offset: u16, data: &[u8]) -> Result<[u8; REPORT_SIZE + 1], PacketError> {
    validate_payload(data, MACRO_WRITE_DATA_SIZE)?;
    let mut report = command(0x0D);
    report[3] = data.len() as u8;
    report[4..6].copy_from_slice(&offset.to_le_bytes());
    report[6..6 + data.len()].copy_from_slice(data);
    Ok(report)
}

pub fn encode_factory_reset() -> [u8; REPORT_SIZE + 1] {
    let mut report = command(0x0F);
    report[3] = 0xFF;
    report
}

fn command(command_byte: u8) -> [u8; REPORT_SIZE + 1] {
    let mut report = [0; REPORT_SIZE + 1];
    report[0] = REPORT_ID;
    report[1] = FAMILY;
    report[2] = command_byte;
    report
}

fn encode_bulk_block(
    command_byte: u8,
    bank: Option<KeyBank>,
    offset: u16,
    data: &[u8],
) -> Result<[u8; REPORT_SIZE + 1], PacketError> {
    validate_payload(data, BLOCK_DATA_SIZE)?;
    let mut report = command(command_byte);
    report[3] = (data.len() + 3) as u8;
    report[4..6].copy_from_slice(&offset.to_le_bytes());
    if let Some(bank) = bank {
        report[7] = wire_layer(bank);
    }
    report[9..9 + data.len()].copy_from_slice(data);
    Ok(report)
}

fn validate_payload(data: &[u8], maximum: usize) -> Result<(), PacketError> {
    if data.is_empty() {
        Err(PacketError::EmptyPayload)
    } else if data.len() > maximum {
        Err(PacketError::PayloadTooLong {
            maximum,
            actual: data.len(),
        })
    } else {
        Ok(())
    }
}

fn wire_layer(bank: KeyBank) -> u8 {
    match bank {
        KeyBank::Base => 0,
        KeyBank::Fn => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn little_endian_u16_round_trips() {
        for value in [0, 1, 0x1234, u16::MAX] {
            assert_eq!(decode_le_u16(encode_le_u16(value)), value);
        }
    }

    #[test]
    fn base_and_fn_map_only_to_wire_layers_zero_and_one() {
        assert_eq!(encode_keymap_read(KeyBank::Base, 0)[7], 0);
        assert_eq!(encode_keymap_read(KeyBank::Fn, 0)[7], 1);
    }

    #[test]
    fn keymap_read_has_exact_vendor_payload() {
        let report = encode_keymap_read(KeyBank::Fn, 0x1234);
        assert_eq!(&report[1..9], &[0x06, 0x08, 0x3A, 0x34, 0x12, 0, 1, 0]);
        assert!(report[9..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn bulk_keymap_blocks_encode_full_and_final_lengths() {
        let full = encode_keymap_bulk_block(KeyBank::Base, 0, &[0xA5; 56]).unwrap();
        assert_eq!(&full[1..9], &[0x06, 0x09, 59, 0, 0, 0, 0, 0]);
        assert_eq!(&full[9..65], &[0xA5; 56]);

        let final_block = encode_keymap_bulk_block(KeyBank::Fn, 56, &[1, 2, 3, 4]).unwrap();
        assert_eq!(
            &final_block[1..13],
            &[0x06, 0x09, 7, 56, 0, 0, 1, 0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn single_key_write_uses_sparse_key_index_offset() {
        let report = encode_key_write(
            KeyBank::Fn,
            73,
            KeyAction::Keyboard {
                modifiers: 0,
                usage: 0x45,
            },
        )
        .unwrap();
        assert_eq!(
            &report[1..13],
            &[0x06, 0x10, 7, 0x24, 0x01, 0, 1, 0, 0x20, 0, 0x45, 0]
        );
    }

    #[test]
    fn rgb_packets_use_three_byte_offsets() {
        let read = encode_rgb_read(56);
        assert_eq!(&read[1..9], &[0x06, 0x13, 0x3A, 56, 0, 0, 0, 0]);

        let single = encode_key_color_write(
            73,
            Rgb {
                r: 255,
                g: 84,
                b: 0,
            },
        )
        .unwrap();
        assert_eq!(
            &single[1..12],
            &[0x06, 0x14, 3, 0xDB, 0, 0, 0, 0, 255, 84, 0]
        );
    }

    #[test]
    fn rgb_bulk_block_uses_the_same_56_byte_chunk_rule() {
        let report = encode_rgb_bulk_block(0x0102, &[9, 8, 7]).unwrap();
        assert_eq!(
            &report[1..12],
            &[0x06, 0x12, 6, 0x02, 0x01, 0, 0, 0, 9, 8, 7]
        );
    }

    #[test]
    fn semantic_rgb_values_pack_and_unpack_in_rgb_order() {
        let colors = [
            Rgb {
                r: 255,
                g: 84,
                b: 0,
            },
            Rgb {
                r: 0,
                g: 255,
                b: 159,
            },
        ];
        let bytes = encode_rgb_values(&colors);
        assert_eq!(bytes, [255, 84, 0, 0, 255, 159]);
        assert_eq!(decode_rgb_values(&bytes).unwrap(), colors);
        assert_eq!(
            decode_rgb_values(&[1, 2]),
            Err(PacketError::MisalignedPayload {
                record_size: 3,
                actual: 2
            })
        );
    }

    #[test]
    fn block_response_data_starts_at_byte_eight() {
        let response = [0xAA, 0x07, 0x3A, 0, 0, 0, 1, 0, 0x20, 0, 0x29, 0];
        assert_eq!(
            decode_block_response(&response, 4).unwrap(),
            &[0x20, 0, 0x29, 0]
        );
    }

    #[test]
    fn macro_read_and_write_match_frontend_payloads() {
        let read = encode_macro_read(56, 0x0234).unwrap();
        assert_eq!(&read[1..6], &[0x06, 0x0C, 56, 0x34, 0x02]);

        let write = encode_macro_write(59, &[1, 2, 3]).unwrap();
        assert_eq!(&write[1..9], &[0x06, 0x0D, 3, 59, 0, 1, 2, 3]);
    }

    #[test]
    fn factory_reset_matches_frontend_command() {
        let report = encode_factory_reset();
        assert_eq!(&report[..5], &[0, 0x06, 0x0F, 0xFF, 0]);
        assert!(report[5..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn payload_limits_are_enforced() {
        assert_eq!(
            encode_rgb_bulk_block(0, &[]),
            Err(PacketError::EmptyPayload)
        );
        assert_eq!(
            encode_keymap_bulk_block(KeyBank::Base, 0, &[0; 57]),
            Err(PacketError::PayloadTooLong {
                maximum: 56,
                actual: 57
            })
        );
        assert_eq!(
            encode_keymap_bulk_block(KeyBank::Base, 0, &[0; 3]),
            Err(PacketError::MisalignedPayload {
                record_size: 4,
                actual: 3
            })
        );
        assert_eq!(
            encode_macro_write(0, &[0; 60]),
            Err(PacketError::PayloadTooLong {
                maximum: 59,
                actual: 60
            })
        );
        assert_eq!(
            encode_macro_read(57, 0),
            Err(PacketError::PayloadTooLong {
                maximum: 56,
                actual: 57
            })
        );
    }
}
