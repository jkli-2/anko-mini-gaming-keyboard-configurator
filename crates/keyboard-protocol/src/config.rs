use std::error::Error;
use std::fmt;

use keyboard_core::KeyboardConfig;

use crate::{PRODUCT_ID, REPORT_ID, REPORT_SIZE};

const CONFIG_FAMILY: u8 = 0x06;
const READ_CONFIG_COMMAND: u8 = 0x05;
const CONFIG_BODY_OFFSET: usize = 5;
const BASE_CONFIG_BODY_LEN: usize = 14;
const AUTO_SLEEP_BODY_LEN: usize = 16;
const EXTENDED_RESPONSE_LEN: u8 = 40;
const SERIAL_START: usize = 21;
const SERIAL_END: usize = 43;

/// Errors produced while decoding a response from the keyboard.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    ResponseTooShort { expected: usize, actual: usize },
    UnexpectedReplyPrefix(u8),
    WrongProductId(u16),
    InvalidSerialNumber,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResponseTooShort { expected, actual } => {
                write!(
                    f,
                    "response is too short: expected at least {expected} bytes, got {actual}"
                )
            }
            Self::UnexpectedReplyPrefix(value) => {
                write!(f, "unexpected response prefix 0x{value:02X}; expected 0xAA")
            }
            Self::WrongProductId(value) => write!(
                f,
                "response is for product 0x{value:04X}, not target 0x{PRODUCT_ID:04X}"
            ),
            Self::InvalidSerialNumber => write!(f, "serial number is not valid UTF-8"),
        }
    }
}

impl Error for ProtocolError {}

/// Encode the report-ID-prefixed buffer expected by hidapi for `06 05`.
pub fn encode_config_request() -> [u8; REPORT_SIZE + 1] {
    let mut report = [0; REPORT_SIZE + 1];
    report[0] = REPORT_ID;
    report[1] = CONFIG_FAMILY;
    report[2] = READ_CONFIG_COMMAND;
    report
}

/// Decode a `06 05` response as laid out by the stock frontend.
pub fn parse_config_response(response: &[u8]) -> Result<KeyboardConfig, ProtocolError> {
    require_len(response, CONFIG_BODY_OFFSET + BASE_CONFIG_BODY_LEN)?;

    if response[0] != 0xAA {
        return Err(ProtocolError::UnexpectedReplyPrefix(response[0]));
    }

    let body = &response[CONFIG_BODY_OFFSET..];
    let product_id = le_u16(&body[2..4]);
    if product_id != PRODUCT_ID {
        return Err(ProtocolError::WrongProductId(product_id));
    }

    // The vendor frontend uses response byte 2 as the response-layout length/version.
    let layout_len = response[2];
    let auto_sleep_seconds = if layout_len >= AUTO_SLEEP_BODY_LEN as u8 {
        require_len(response, CONFIG_BODY_OFFSET + AUTO_SLEEP_BODY_LEN)?;
        Some(le_u16(&body[14..16]))
    } else {
        None
    };

    let serial_number = if layout_len >= EXTENDED_RESPONSE_LEN {
        require_len(response, SERIAL_END)?;
        let bytes: Vec<u8> = response[SERIAL_START..SERIAL_END]
            .iter()
            .copied()
            .filter(|byte| *byte != 0)
            .collect();
        let serial = String::from_utf8(bytes).map_err(|_| ProtocolError::InvalidSerialNumber)?;
        (!serial.is_empty()).then_some(serial)
    } else {
        None
    };

    Ok(KeyboardConfig {
        protocol_version: le_u16(&body[0..2]),
        product_id,
        firmware_version: le_u16(&body[4..6]),
        work_mode: body[6],
        link_status: body[7],
        battery: body[8],
        charge: body[9],
        profile_count: body[10],
        profile: body[11],
        layer_count: body[12],
        layer: body[13],
        auto_sleep_seconds,
        serial_number,
    })
}

fn require_len(response: &[u8], expected: usize) -> Result<(), ProtocolError> {
    if response.len() < expected {
        Err(ProtocolError::ResponseTooShort {
            expected,
            actual: response.len(),
        })
    } else {
        Ok(())
    }
}

fn le_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VENDOR_ID;

    fn known_response(layout_len: u8) -> [u8; REPORT_SIZE] {
        let mut response = [0; REPORT_SIZE];
        response[0] = 0xAA;
        response[2] = layout_len;
        response[5..21].copy_from_slice(&[
            0x01, 0x00, // protocol version
            0xA1, 0xFD, // PID
            0x03, 0x00, // firmware
            0x01, // work mode
            0x01, // link status
            50,   // battery (generic field; meaning on wired FDA1 is unknown)
            0x01, // charge
            0x00, // profile count
            0x00, // profile
            0x02, // layer count
            0x00, // layer
            0x2C, 0x01, // 300 seconds
        ]);
        response
    }

    #[test]
    fn config_request_has_report_id_and_64_byte_payload() {
        let request = encode_config_request();
        assert_eq!(request.len(), 65);
        assert_eq!(&request[..3], &[0x00, 0x06, 0x05]);
        assert!(request[3..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn decodes_known_device_values_and_optional_sleep() {
        let response = known_response(16);
        let config = parse_config_response(&response).unwrap();

        assert_eq!(config.protocol_version, 1);
        assert_eq!(config.product_id, PRODUCT_ID);
        assert_eq!(config.firmware_version, 3);
        assert_eq!(config.work_mode, 1);
        assert_eq!(config.link_status, 1);
        assert_eq!(config.battery, 50);
        assert_eq!(config.charge, 1);
        assert_eq!(config.profile_count, 0);
        assert_eq!(config.profile, 0);
        assert_eq!(config.layer_count, 2);
        assert_eq!(config.layer, 0);
        assert_eq!(config.auto_sleep_seconds, Some(300));
        assert_eq!(config.serial_number, None);
    }

    #[test]
    fn decodes_extended_serial_number() {
        let mut response = known_response(40);
        response[21..31].copy_from_slice(b"Z64-0001\0\0");

        let config = parse_config_response(&response).unwrap();
        assert_eq!(config.serial_number.as_deref(), Some("Z64-0001"));
    }

    #[test]
    fn rejects_a_response_for_another_product() {
        let mut response = known_response(16);
        response[7..9].copy_from_slice(&0xFEBBu16.to_le_bytes());

        assert_eq!(
            parse_config_response(&response),
            Err(ProtocolError::WrongProductId(0xFEBB))
        );
    }

    #[test]
    fn rejects_short_responses_without_panicking() {
        assert_eq!(
            parse_config_response(&[0xAA; 4]),
            Err(ProtocolError::ResponseTooShort {
                expected: 19,
                actual: 4,
            })
        );
    }

    #[test]
    fn constants_identify_only_the_supported_keyboard() {
        assert_eq!(VENDOR_ID, 0x36AE);
        assert_eq!(PRODUCT_ID, 0xFDA1);
    }
}
