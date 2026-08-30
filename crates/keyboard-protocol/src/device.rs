use std::error::Error;
use std::ffi::CStr;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use hidapi::{DeviceInfo, HidApi, HidDevice, HidError};
use keyboard_core::{
    HARDWARE_SNAPSHOT_VERSION, HardwareSnapshot, KeyAction, KeyBank, KeyboardConfig, LightingState,
    Macro, Rgb,
};

use crate::{
    BLOCK_DATA_SIZE, KEY_INDEX_COUNT, MACRO_STORAGE_SIZE, MacroCodecError, PacketError,
    ProtocolError, decode_block_response, decode_key_action, decode_macro_storage,
    decode_rgb_values, encode_config_request, encode_factory_reset, encode_key_action,
    encode_key_color_write, encode_key_write, encode_keymap_bulk_block, encode_keymap_read,
    encode_lighting_effect_request, encode_lighting_read_request, encode_lighting_write,
    encode_macro_read, encode_macro_storage, encode_macro_write, encode_rgb_bulk_block,
    encode_rgb_read, encode_rgb_values, parse_config_response, parse_lighting_config_response,
    parse_lighting_response, validate_macro_storage_layout,
};

pub const VENDOR_ID: u16 = 0x36AE;
pub const PRODUCT_ID: u16 = 0xFDA1;
pub const TARGET_USAGE_PAGE: u16 = 0xFF00;
pub const TARGET_USAGE: u16 = 0x0002;
pub const REPORT_ID: u8 = 0;
pub const REPORT_SIZE: usize = 64;
const RESPONSE_TIMEOUT: Duration = Duration::from_millis(200);
const LOCK_PATH: &str = "/tmp/anko-keyboard-36ae-fda1.lock";

#[derive(Debug)]
pub enum TransportError {
    Hid(HidError),
    TargetNotFound,
    ShortWrite {
        expected: usize,
        actual: usize,
    },
    Timeout,
    Protocol(ProtocolError),
    Packet(PacketError),
    Macro(MacroCodecError),
    Io(io::Error),
    LockPoisoned,
    InvalidKeyIndex(u16),
    InvalidKeymapLength {
        expected: usize,
        actual: usize,
    },
    InvalidColorIndex(u16),
    InvalidColorMapLength {
        expected: usize,
        actual: usize,
    },
    InvalidSnapshot(String),
    VerificationFailed(&'static str),
    LightingVerificationFailed {
        expected: LightingState,
        actual: LightingState,
    },
    RestoreFailed {
        operation: String,
        restore: String,
    },
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hid(error) => write!(f, "HID error: {error}"),
            Self::TargetNotFound => write!(
                f,
                "target 36AE:FDA1 vendor interface FF00:0002 was not found"
            ),
            Self::ShortWrite { expected, actual } => {
                write!(
                    f,
                    "short HID write: expected {expected} bytes, wrote {actual}"
                )
            }
            Self::Timeout => write!(f, "timed out waiting for keyboard response"),
            Self::Protocol(error) => write!(f, "invalid keyboard response: {error}"),
            Self::Packet(error) => write!(f, "invalid packet data: {error}"),
            Self::Macro(error) => write!(f, "invalid macro storage: {error}"),
            Self::Io(error) => write!(f, "device lock error: {error}"),
            Self::LockPoisoned => write!(f, "in-process HID transaction lock was poisoned"),
            Self::InvalidKeyIndex(index) => write!(f, "key index {index} is outside the matrix"),
            Self::InvalidKeymapLength { expected, actual } => write!(
                f,
                "keymap has the wrong length: expected {expected} records, got {actual}"
            ),
            Self::InvalidColorIndex(index) => {
                write!(f, "color index {index} is outside the matrix")
            }
            Self::InvalidColorMapLength { expected, actual } => write!(
                f,
                "color map has the wrong length: expected {expected} records, got {actual}"
            ),
            Self::InvalidSnapshot(error) => write!(f, "invalid hardware snapshot: {error}"),
            Self::VerificationFailed(operation) => {
                write!(f, "hardware readback did not match {operation}")
            }
            Self::LightingVerificationFailed { expected, actual } => write!(
                f,
                "hardware lighting readback mismatch: expected {expected:?}, got {actual:?}"
            ),
            Self::RestoreFailed { operation, restore } => write!(
                f,
                "write failed ({operation}) and restoring the original state also failed ({restore})"
            ),
        }
    }
}

impl Error for TransportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Hid(error) => Some(error),
            Self::Protocol(error) => Some(error),
            Self::Packet(error) => Some(error),
            Self::Macro(error) => Some(error),
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<HidError> for TransportError {
    fn from(value: HidError) -> Self {
        Self::Hid(value)
    }
}

impl From<ProtocolError> for TransportError {
    fn from(value: ProtocolError) -> Self {
        Self::Protocol(value)
    }
}

impl From<PacketError> for TransportError {
    fn from(value: PacketError) -> Self {
        Self::Packet(value)
    }
}

impl From<MacroCodecError> for TransportError {
    fn from(value: MacroCodecError) -> Self {
        Self::Macro(value)
    }
}

impl From<io::Error> for TransportError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

/// Find the exact vendor configuration collection. VID/PID alone is insufficient.
pub fn find_target(api: &HidApi) -> Option<&DeviceInfo> {
    api.device_list().find(|device| {
        device.vendor_id() == VENDOR_ID
            && device.product_id() == PRODUCT_ID
            && device.usage_page() == TARGET_USAGE_PAGE
            && device.usage() == TARGET_USAGE
    })
}

pub fn read_keyboard_config(api: &HidApi) -> Result<KeyboardConfig, TransportError> {
    KeyboardDevice::open(api)?.read_config()
}

/// One open, serialized HID owner for read-only diagnostic transactions.
pub struct KeyboardDevice {
    device: Mutex<HidDevice>,
    _process_lock: File,
}

impl KeyboardDevice {
    pub fn open(api: &HidApi) -> Result<Self, TransportError> {
        let process_lock = acquire_process_lock()?;
        let target = find_target(api).ok_or(TransportError::TargetNotFound)?;
        Ok(Self {
            device: Mutex::new(open_path(api, target.path())?),
            _process_lock: process_lock,
        })
    }

    pub fn read_config(&self) -> Result<KeyboardConfig, TransportError> {
        let response = self.transact(&encode_config_request())?;
        parse_config_response(&response).map_err(Into::into)
    }

    pub fn read_keymap(&self, bank: KeyBank) -> Result<Vec<KeyAction>, TransportError> {
        let byte_count = KEY_INDEX_COUNT * 4;
        let bytes = self.read_blocks(byte_count, |offset| encode_keymap_read(bank, offset))?;
        Ok(bytes
            .chunks_exact(4)
            .map(|record| decode_key_action([record[0], record[1], record[2], record[3]]))
            .collect())
    }

    pub fn read_colors(&self) -> Result<Vec<Rgb>, TransportError> {
        let bytes = self.read_blocks(KEY_INDEX_COUNT * 3, encode_rgb_read)?;
        decode_rgb_values(&bytes).map_err(Into::into)
    }

    /// Read and decode all 16 macro slots without modifying device state.
    pub fn read_macros(&self) -> Result<Vec<Macro>, TransportError> {
        decode_macro_storage(&self.read_macro_storage_raw()?).map_err(Into::into)
    }

    /// Replace the macro store, verify exact readback, and restore the original
    /// bytes if either the write or verification fails.
    pub fn write_macros(&self, macros: &[Macro]) -> Result<(), TransportError> {
        let expected = encode_macro_storage(macros)?;
        let original = self.read_macro_storage_raw()?;
        if original == expected {
            return Ok(());
        }

        let operation = self
            .write_macro_storage_once(&expected)
            .and_then(|()| self.verify_macro_storage(&expected));
        if let Err(error) = operation {
            let restore = self
                .write_macro_storage_once(&original)
                .and_then(|()| self.verify_macro_storage(&original));
            return match restore {
                Ok(()) => Err(error),
                Err(restore_error) => Err(TransportError::RestoreFailed {
                    operation: error.to_string(),
                    restore: restore_error.to_string(),
                }),
            };
        }
        Ok(())
    }

    /// Restore factory values. This operation is destructive and callers must
    /// obtain explicit user confirmation before invoking it.
    pub fn factory_reset(&self) -> Result<(), TransportError> {
        let response = self.transact(&encode_factory_reset())?;
        validate_write_reply(&response)
    }

    /// Read the complete macro region without semantic decoding.
    pub fn read_macro_storage_raw(&self) -> Result<Vec<u8>, TransportError> {
        let mut storage = Vec::with_capacity(MACRO_STORAGE_SIZE);
        for offset in (0..MACRO_STORAGE_SIZE).step_by(BLOCK_DATA_SIZE) {
            let requested_len = BLOCK_DATA_SIZE.min(MACRO_STORAGE_SIZE - offset);
            let request = encode_macro_read(requested_len, offset as u16)?;
            let response = self.transact(&request)?;
            storage.extend_from_slice(decode_block_response(&response, requested_len)?);
        }
        Ok(storage)
    }

    /// Capture all persistent regions needed for a lossless device backup.
    pub fn capture_hardware_snapshot(&self) -> Result<HardwareSnapshot, TransportError> {
        let config = self.read_config()?;
        let base_keymap = self
            .read_keymap(KeyBank::Base)?
            .into_iter()
            .map(encode_key_action)
            .collect();
        let fn_keymap = self
            .read_keymap(KeyBank::Fn)?
            .into_iter()
            .map(encode_key_action)
            .collect();
        let lighting = self.read_lighting()?;
        let key_rgb = self
            .read_colors()?
            .into_iter()
            .map(|color| [color.r, color.g, color.b])
            .collect();
        let macro_storage = self.read_macro_storage_raw()?;

        Ok(HardwareSnapshot {
            version: HARDWARE_SNAPSHOT_VERSION,
            vendor_id: VENDOR_ID,
            product_id: config.product_id,
            firmware_version: config.firmware_version,
            protocol_version: config.protocol_version,
            base_keymap,
            fn_keymap,
            lighting,
            key_rgb,
            macro_storage,
        })
    }

    /// Restore a complete snapshot as one logical operation. If any section
    /// fails, every already-written section is restored from a fresh baseline.
    pub fn restore_hardware_snapshot(
        &self,
        target: &HardwareSnapshot,
    ) -> Result<(), TransportError> {
        let config = self.read_config()?;
        validate_snapshot(target, &config)?;
        let original = self.capture_hardware_snapshot()?;
        if original == *target {
            return Ok(());
        }

        if let Err(error) = self.apply_hardware_snapshot(target) {
            return match self.apply_hardware_snapshot(&original) {
                Ok(()) => Err(error),
                Err(restore_error) => Err(TransportError::RestoreFailed {
                    operation: error.to_string(),
                    restore: restore_error.to_string(),
                }),
            };
        }
        Ok(())
    }

    fn apply_hardware_snapshot(&self, snapshot: &HardwareSnapshot) -> Result<(), TransportError> {
        let base_keymap: Vec<_> = snapshot
            .base_keymap
            .iter()
            .copied()
            .map(decode_key_action)
            .collect();
        let fn_keymap: Vec<_> = snapshot
            .fn_keymap
            .iter()
            .copied()
            .map(decode_key_action)
            .collect();
        let colors: Vec<_> = snapshot
            .key_rgb
            .iter()
            .map(|rgb| Rgb {
                r: rgb[0],
                g: rgb[1],
                b: rgb[2],
            })
            .collect();

        self.write_macro_storage_raw(&snapshot.macro_storage)?;
        self.write_keymap(KeyBank::Base, &base_keymap)?;
        self.write_keymap(KeyBank::Fn, &fn_keymap)?;
        self.write_colors(&colors)?;
        self.write_lighting(snapshot.lighting)
    }

    pub fn read_lighting(&self) -> Result<LightingState, TransportError> {
        let response = self.transact(&encode_lighting_read_request())?;
        if response.len() >= 16 {
            trace_lighting_bytes("06 0A response body", &response[5..16]);
        }
        parse_lighting_response(&response).map_err(Into::into)
    }

    /// Read the firmware's stored defaults for an effect without committing it.
    pub fn read_lighting_effect_config(&self, effect: u8) -> Result<LightingState, TransportError> {
        let request = encode_lighting_effect_request(effect);
        trace_lighting_bytes("06 16 probe request", &request[1..9]);
        let response = self.transact(&request)?;
        if response.len() >= 16 {
            trace_lighting_bytes("06 16 probe response body", &response[5..16]);
        }
        parse_lighting_config_response(&response).map_err(Into::into)
    }

    /// Write global lighting state, verify it, and restore the original on failure.
    pub fn write_lighting(&self, state: LightingState) -> Result<(), TransportError> {
        let original = self.read_lighting()?;
        if original == state {
            return Ok(());
        }

        let operation = self
            .write_lighting_once(state, original.effect != state.effect)
            .and_then(|()| self.verify_lighting(state));
        if let Err(error) = operation {
            let restore = self
                .write_lighting_once(original, original.effect != state.effect)
                .and_then(|()| self.verify_lighting(original));
            return match restore {
                Ok(()) => Err(error),
                Err(restore_error) => Err(TransportError::RestoreFailed {
                    operation: error.to_string(),
                    restore: restore_error.to_string(),
                }),
            };
        }
        Ok(())
    }

    /// Write one matrix RGB record, verify it, and restore the original on failure.
    pub fn write_color(&self, key_index: u16, color: Rgb) -> Result<(), TransportError> {
        if key_index as usize >= KEY_INDEX_COUNT {
            return Err(TransportError::InvalidColorIndex(key_index));
        }
        let original = self.read_colors()?[key_index as usize];
        if original == color {
            return Ok(());
        }

        let operation = self
            .write_color_once(key_index, color)
            .and_then(|()| self.verify_color(key_index, color));
        if let Err(error) = operation {
            let restore = self
                .write_color_once(key_index, original)
                .and_then(|()| self.verify_color(key_index, original));
            return match restore {
                Ok(()) => Err(error),
                Err(restore_error) => Err(TransportError::RestoreFailed {
                    operation: error.to_string(),
                    restore: restore_error.to_string(),
                }),
            };
        }
        Ok(())
    }

    /// Replace all 75 matrix RGB records, verifying and restoring them on failure.
    pub fn write_colors(&self, colors: &[Rgb]) -> Result<(), TransportError> {
        if colors.len() != KEY_INDEX_COUNT {
            return Err(TransportError::InvalidColorMapLength {
                expected: KEY_INDEX_COUNT,
                actual: colors.len(),
            });
        }
        let original = self.read_colors()?;
        if original == colors {
            return Ok(());
        }

        let operation = self
            .write_colors_once(colors)
            .and_then(|()| self.verify_colors(colors));
        if let Err(error) = operation {
            let restore = self
                .write_colors_once(&original)
                .and_then(|()| self.verify_colors(&original));
            return match restore {
                Ok(()) => Err(error),
                Err(restore_error) => Err(TransportError::RestoreFailed {
                    operation: error.to_string(),
                    restore: restore_error.to_string(),
                }),
            };
        }
        Ok(())
    }

    /// Write one matrix record, verify it by readback, and restore the original on failure.
    pub fn write_key(
        &self,
        bank: KeyBank,
        key_index: u16,
        action: KeyAction,
    ) -> Result<(), TransportError> {
        if key_index as usize >= KEY_INDEX_COUNT {
            return Err(TransportError::InvalidKeyIndex(key_index));
        }
        let original_map = self.read_keymap(bank)?;
        let original = original_map[key_index as usize];
        if original == action {
            return Ok(());
        }

        let operation = self
            .write_key_once(bank, key_index, action)
            .and_then(|()| self.verify_key(bank, key_index, action));
        if let Err(error) = operation {
            let restore = self
                .write_key_once(bank, key_index, original)
                .and_then(|()| self.verify_key(bank, key_index, original));
            return match restore {
                Ok(()) => Err(error),
                Err(restore_error) => Err(TransportError::RestoreFailed {
                    operation: error.to_string(),
                    restore: restore_error.to_string(),
                }),
            };
        }
        Ok(())
    }

    /// Replace one complete 75-record bank, verifying and restoring it on failure.
    pub fn write_keymap(&self, bank: KeyBank, actions: &[KeyAction]) -> Result<(), TransportError> {
        if actions.len() != KEY_INDEX_COUNT {
            return Err(TransportError::InvalidKeymapLength {
                expected: KEY_INDEX_COUNT,
                actual: actions.len(),
            });
        }
        let original = self.read_keymap(bank)?;
        if original == actions {
            return Ok(());
        }

        let operation = self
            .write_keymap_once(bank, actions)
            .and_then(|()| self.verify_keymap(bank, actions));
        if let Err(error) = operation {
            let restore = self
                .write_keymap_once(bank, &original)
                .and_then(|()| self.verify_keymap(bank, &original));
            return match restore {
                Ok(()) => Err(error),
                Err(restore_error) => Err(TransportError::RestoreFailed {
                    operation: error.to_string(),
                    restore: restore_error.to_string(),
                }),
            };
        }
        Ok(())
    }

    fn write_key_once(
        &self,
        bank: KeyBank,
        key_index: u16,
        action: KeyAction,
    ) -> Result<(), TransportError> {
        let response = self.transact(&encode_key_write(bank, key_index, action)?)?;
        validate_write_reply(&response)
    }

    fn verify_key(
        &self,
        bank: KeyBank,
        key_index: u16,
        expected: KeyAction,
    ) -> Result<(), TransportError> {
        let actual = self.read_keymap(bank)?[key_index as usize];
        if actual == expected {
            Ok(())
        } else {
            Err(TransportError::VerificationFailed("single-key write"))
        }
    }

    fn write_keymap_once(
        &self,
        bank: KeyBank,
        actions: &[KeyAction],
    ) -> Result<(), TransportError> {
        let bytes: Vec<u8> = actions
            .iter()
            .flat_map(|action| encode_key_action(*action))
            .collect();
        for offset in (0..bytes.len()).step_by(BLOCK_DATA_SIZE) {
            let end = (offset + BLOCK_DATA_SIZE).min(bytes.len());
            let report = encode_keymap_bulk_block(bank, offset as u16, &bytes[offset..end])?;
            let response = self.transact(&report)?;
            validate_write_reply(&response)?;
        }
        Ok(())
    }

    fn write_lighting_once(
        &self,
        state: LightingState,
        select_effect: bool,
    ) -> Result<(), TransportError> {
        if select_effect {
            let request = encode_lighting_effect_request(state.effect);
            trace_lighting_bytes("06 16 request", &request[1..9]);
            let response = self.transact(&request)?;
            if response.len() >= 16 {
                trace_lighting_bytes("06 16 response body", &response[5..16]);
            }
            parse_lighting_config_response(&response)?;
        }
        let request = encode_lighting_write(state);
        trace_lighting_bytes("06 0B request", &request[1..17]);
        let response = self.transact(&request)?;
        if response.len() >= 16 {
            trace_lighting_bytes("06 0B response body", &response[5..16]);
        }
        validate_write_reply(&response)
    }

    fn verify_lighting(&self, expected: LightingState) -> Result<(), TransportError> {
        let actual = self.read_lighting()?;
        if lighting_readback_matches(expected, actual) {
            Ok(())
        } else {
            Err(TransportError::LightingVerificationFailed { expected, actual })
        }
    }

    fn write_color_once(&self, key_index: u16, color: Rgb) -> Result<(), TransportError> {
        let response = self.transact(&encode_key_color_write(key_index, color)?)?;
        validate_write_reply(&response)
    }

    fn verify_color(&self, key_index: u16, expected: Rgb) -> Result<(), TransportError> {
        if self.read_colors()?[key_index as usize] == expected {
            Ok(())
        } else {
            Err(TransportError::VerificationFailed("single-key color write"))
        }
    }

    fn write_colors_once(&self, colors: &[Rgb]) -> Result<(), TransportError> {
        let bytes = encode_rgb_values(colors);
        for offset in (0..bytes.len()).step_by(BLOCK_DATA_SIZE) {
            let end = (offset + BLOCK_DATA_SIZE).min(bytes.len());
            let response =
                self.transact(&encode_rgb_bulk_block(offset as u16, &bytes[offset..end])?)?;
            validate_write_reply(&response)?;
        }
        Ok(())
    }

    fn verify_colors(&self, expected: &[Rgb]) -> Result<(), TransportError> {
        if self.read_colors()? == expected {
            Ok(())
        } else {
            Err(TransportError::VerificationFailed("bulk color write"))
        }
    }

    fn verify_keymap(&self, bank: KeyBank, expected: &[KeyAction]) -> Result<(), TransportError> {
        if self.read_keymap(bank)? == expected {
            Ok(())
        } else {
            Err(TransportError::VerificationFailed("bulk keymap write"))
        }
    }

    fn write_macro_storage_once(&self, storage: &[u8]) -> Result<(), TransportError> {
        const WRITE_BLOCK_SIZE: usize = 59;
        for offset in (0..storage.len()).step_by(WRITE_BLOCK_SIZE) {
            let end = (offset + WRITE_BLOCK_SIZE).min(storage.len());
            let response =
                self.transact(&encode_macro_write(offset as u16, &storage[offset..end])?)?;
            validate_write_reply(&response)?;
        }
        Ok(())
    }

    fn verify_macro_storage(&self, expected: &[u8]) -> Result<(), TransportError> {
        if self.read_macro_storage_raw()? == expected {
            Ok(())
        } else {
            Err(TransportError::VerificationFailed("macro storage write"))
        }
    }

    /// Replace the complete macro storage byte-for-byte with verified readback.
    pub fn write_macro_storage_raw(&self, storage: &[u8]) -> Result<(), TransportError> {
        if storage.len() != MACRO_STORAGE_SIZE {
            return Err(TransportError::InvalidSnapshot(format!(
                "macro storage has {} bytes; expected {MACRO_STORAGE_SIZE}",
                storage.len()
            )));
        }
        let original = self.read_macro_storage_raw()?;
        if original == storage {
            return Ok(());
        }
        let operation = self
            .write_macro_storage_once(storage)
            .and_then(|()| self.verify_macro_storage(storage));
        if let Err(error) = operation {
            let restore = self
                .write_macro_storage_once(&original)
                .and_then(|()| self.verify_macro_storage(&original));
            return match restore {
                Ok(()) => Err(error),
                Err(restore_error) => Err(TransportError::RestoreFailed {
                    operation: error.to_string(),
                    restore: restore_error.to_string(),
                }),
            };
        }
        Ok(())
    }

    fn read_blocks(
        &self,
        total_len: usize,
        request: impl Fn(u16) -> [u8; REPORT_SIZE + 1],
    ) -> Result<Vec<u8>, TransportError> {
        let mut data = Vec::with_capacity(total_len);
        for offset in (0..total_len).step_by(BLOCK_DATA_SIZE) {
            let requested_len = BLOCK_DATA_SIZE.min(total_len - offset);
            let response = self.transact(&request(offset as u16))?;
            data.extend_from_slice(decode_block_response(&response, requested_len)?);
        }
        Ok(data)
    }

    fn transact(&self, request: &[u8; REPORT_SIZE + 1]) -> Result<Vec<u8>, TransportError> {
        let device = self
            .device
            .lock()
            .map_err(|_| TransportError::LockPoisoned)?;
        Self::drain_pending_reports(&device)?;
        device.set_blocking_mode(true)?;

        let written = device.write(request)?;
        if written != request.len() {
            return Err(TransportError::ShortWrite {
                expected: request.len(),
                actual: written,
            });
        }

        let started = Instant::now();
        loop {
            let remaining = RESPONSE_TIMEOUT.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return Err(TransportError::Timeout);
            }

            let mut response = [0; REPORT_SIZE];
            let read = device.read_timeout(&mut response, duration_millis_i32(remaining))?;
            if read == 0 {
                return Err(TransportError::Timeout);
            }

            // AA FA is an unsolicited lighting event in the vendor frontend. It does not
            // complete the active request, so continue waiting within the original timeout.
            if read >= 2 && response[..2] == [0xAA, 0xFA] {
                continue;
            }

            return Ok(response[..read].to_vec());
        }
    }

    fn drain_pending_reports(device: &HidDevice) -> Result<(), HidError> {
        device.set_blocking_mode(false)?;
        let mut response = [0; REPORT_SIZE];
        while device.read(&mut response)? != 0 {}
        Ok(())
    }
}

fn validate_snapshot(
    snapshot: &HardwareSnapshot,
    config: &KeyboardConfig,
) -> Result<(), TransportError> {
    let invalid = |message: String| TransportError::InvalidSnapshot(message);
    if snapshot.version != HARDWARE_SNAPSHOT_VERSION {
        return Err(invalid(format!(
            "schema version {} is unsupported; expected {HARDWARE_SNAPSHOT_VERSION}",
            snapshot.version
        )));
    }
    if snapshot.vendor_id != VENDOR_ID || snapshot.product_id != PRODUCT_ID {
        return Err(invalid(format!(
            "profile targets {:04X}:{:04X}, expected {VENDOR_ID:04X}:{PRODUCT_ID:04X}",
            snapshot.vendor_id, snapshot.product_id
        )));
    }
    if config.product_id != snapshot.product_id {
        return Err(invalid(format!(
            "connected product {:04X} does not match profile product {:04X}",
            config.product_id, snapshot.product_id
        )));
    }
    if config.protocol_version != snapshot.protocol_version {
        return Err(invalid(format!(
            "connected protocol {} does not match profile protocol {}",
            config.protocol_version, snapshot.protocol_version
        )));
    }
    for (name, records) in [
        ("Base keymap", &snapshot.base_keymap),
        ("Fn keymap", &snapshot.fn_keymap),
    ] {
        if records.len() != KEY_INDEX_COUNT {
            return Err(invalid(format!(
                "{name} has {} records; expected {KEY_INDEX_COUNT}",
                records.len()
            )));
        }
    }
    if snapshot.key_rgb.len() != KEY_INDEX_COUNT {
        return Err(invalid(format!(
            "RGB map has {} records; expected {KEY_INDEX_COUNT}",
            snapshot.key_rgb.len()
        )));
    }
    if snapshot.macro_storage.len() != MACRO_STORAGE_SIZE {
        return Err(invalid(format!(
            "macro storage has {} bytes; expected {MACRO_STORAGE_SIZE}",
            snapshot.macro_storage.len()
        )));
    }
    validate_macro_storage_layout(&snapshot.macro_storage)?;
    let lighting = snapshot.lighting;
    if lighting.kind != 1 || lighting.effect > 19 {
        return Err(invalid(
            "lighting kind/effect is outside the verified FDA1 range".to_string(),
        ));
    }
    if lighting.effect == 0 && lighting.color_enabled {
        return Err(invalid(
            "lighting color mode must be disabled when effect is Off".to_string(),
        ));
    }
    Ok(())
}

fn open_path(api: &HidApi, path: &CStr) -> Result<HidDevice, HidError> {
    api.open_path(path)
}

fn validate_write_reply(response: &[u8]) -> Result<(), TransportError> {
    if response.is_empty() {
        return Err(ProtocolError::ResponseTooShort {
            expected: 1,
            actual: 0,
        }
        .into());
    }
    if response[0] != 0xAA {
        return Err(ProtocolError::UnexpectedReplyPrefix(response[0]).into());
    }
    Ok(())
}

fn acquire_process_lock() -> Result<File, io::Error> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(LOCK_PATH)?;
    // SAFETY: `file` owns a valid descriptor for the duration of this call and remains
    // stored in KeyboardDevice for the lifetime of the acquired advisory lock.
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
    if result == 0 {
        Ok(file)
    } else {
        Err(io::Error::last_os_error())
    }
}

fn duration_millis_i32(duration: Duration) -> i32 {
    duration.as_millis().min(i32::MAX as u128) as i32
}

fn trace_lighting_bytes(label: &str, bytes: &[u8]) {
    if std::env::var_os("ANKO_KEYBOARD_TRACE_LIGHTING").is_none() {
        return;
    }
    let hex = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    eprintln!("keyboard-protocol: {label}: {hex}");
}

fn lighting_readback_matches(expected: LightingState, actual: LightingState) -> bool {
    if expected.kind != actual.kind || expected.effect != actual.effect {
        return false;
    }

    // Off has no configurable parameters. The encoder forces its colour flag off,
    // while the firmware is free to retain or normalize all inactive values.
    if expected.effect == 0 {
        return !actual.color_enabled;
    }

    if expected.brightness != actual.brightness || expected.color_enabled != actual.color_enabled {
        return false;
    }

    // Static does not use speed; all animated effects do.
    if expected.effect != 1 && expected.speed != actual.speed {
        return false;
    }

    // Only Stream, Bloom, UD Wave, and effect 14 expose a direction setting.
    if matches!(expected.effect, 5 | 6 | 7 | 14) && expected.direction != actual.direction {
        return false;
    }

    // Dynamic RGB does not display HSV directly, but the official client preserves
    // it and the firmware echoes it. Verify it so future single-colour switching
    // retains the exact stored values.
    expected.hsv == actual.hsv
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot_fixture() -> (HardwareSnapshot, KeyboardConfig) {
        let mut macro_storage = vec![0; MACRO_STORAGE_SIZE];
        macro_storage[..64].fill(0xff);
        let config = KeyboardConfig {
            protocol_version: 1,
            product_id: PRODUCT_ID,
            firmware_version: 3,
            work_mode: 1,
            link_status: 1,
            battery: 50,
            charge: 1,
            profile_count: 0,
            profile: 0,
            layer_count: 2,
            layer: 0,
            auto_sleep_seconds: None,
            serial_number: None,
        };
        let snapshot = HardwareSnapshot {
            version: HARDWARE_SNAPSHOT_VERSION,
            vendor_id: VENDOR_ID,
            product_id: PRODUCT_ID,
            firmware_version: 3,
            protocol_version: 1,
            base_keymap: vec![[0x20, 0, 0x29, 0]; KEY_INDEX_COUNT],
            fn_keymap: vec![[0x20, 0, 0xff, 0]; KEY_INDEX_COUNT],
            lighting: lighting(
                1,
                4,
                0,
                1,
                true,
                keyboard_core::Hsv {
                    hue: 1,
                    saturation: 2,
                    value: 3,
                },
            ),
            key_rgb: vec![[0, 0, 0]; KEY_INDEX_COUNT],
            macro_storage,
        };
        (snapshot, config)
    }

    #[test]
    fn timeout_conversion_is_bounded() {
        assert_eq!(duration_millis_i32(Duration::from_millis(200)), 200);
        assert_eq!(duration_millis_i32(Duration::MAX), i32::MAX);
    }

    #[test]
    fn write_reply_requires_ack_prefix() {
        assert!(validate_write_reply(&[0xAA, 0x10]).is_ok());
        assert!(matches!(
            validate_write_reply(&[0x06]),
            Err(TransportError::Protocol(
                ProtocolError::UnexpectedReplyPrefix(0x06)
            ))
        ));
    }

    #[test]
    fn hardware_snapshot_validation_is_strict_before_writes() {
        let (snapshot, config) = snapshot_fixture();
        assert!(validate_snapshot(&snapshot, &config).is_ok());

        let mut wrong_length = snapshot.clone();
        wrong_length.fn_keymap.pop();
        assert!(matches!(
            validate_snapshot(&wrong_length, &config),
            Err(TransportError::InvalidSnapshot(_))
        ));

        let mut wrong_protocol = snapshot;
        wrong_protocol.protocol_version = 2;
        assert!(matches!(
            validate_snapshot(&wrong_protocol, &config),
            Err(TransportError::InvalidSnapshot(_))
        ));
    }

    fn lighting(
        effect: u8,
        brightness: u8,
        speed: u8,
        direction: u8,
        color_enabled: bool,
        hsv: keyboard_core::Hsv,
    ) -> LightingState {
        LightingState {
            kind: 1,
            effect,
            brightness,
            speed,
            direction,
            color_enabled,
            single_color_index: 0,
            hsv,
        }
    }

    #[test]
    fn lighting_verification_ignores_only_inapplicable_fields() {
        let hsv = keyboard_core::Hsv {
            hue: 10,
            saturation: 20,
            value: 30,
        };
        let other_hsv = keyboard_core::Hsv {
            hue: 40,
            saturation: 50,
            value: 60,
        };

        let off = lighting(0, 4, 3, 1, false, hsv);
        assert!(lighting_readback_matches(
            off,
            lighting(0, 0, 0, 0, false, other_hsv)
        ));
        assert!(!lighting_readback_matches(
            off,
            lighting(1, 0, 0, 0, false, other_hsv)
        ));

        let static_dynamic = lighting(1, 4, 3, 1, false, hsv);
        assert!(!lighting_readback_matches(
            static_dynamic,
            lighting(1, 4, 0, 0, false, other_hsv)
        ));
        assert!(!lighting_readback_matches(
            static_dynamic,
            lighting(1, 4, 0, 0, true, other_hsv)
        ));

        let directed_single = lighting(5, 4, 2, 1, true, hsv);
        assert!(lighting_readback_matches(directed_single, directed_single));
        assert!(!lighting_readback_matches(
            directed_single,
            lighting(5, 4, 2, 0, true, hsv)
        ));
        assert!(!lighting_readback_matches(
            directed_single,
            lighting(5, 4, 2, 1, true, other_hsv)
        ));
    }
}
