use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use keyboard_core::{HARDWARE_SNAPSHOT_VERSION, HardwareSnapshot};
use serde::{Deserialize, Serialize};

use crate::macros::{MACRO_SLOT_COUNT, MacroStep};

pub(crate) const PROFILE_FORMAT: &str = "anko-fda1-profile";
pub(crate) const PROFILE_VERSION: u32 = 1;
const VENDOR_ID: u16 = 0x36ae;
const PRODUCT_ID: u16 = 0xfda1;
const KEY_INDEX_COUNT: usize = 75;
const MACRO_STORAGE_SIZE: usize = 4096;
const MACRO_POINTER_TABLE_SIZE: usize = 64;
const MACRO_EVENT_SIZE: usize = 4;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ClientProfileMetadata {
    pub macro_names: BTreeMap<u8, String>,
    pub macro_steps: BTreeMap<u8, Vec<MacroStep>>,
    pub layout: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct DeviceProfile {
    pub format: String,
    pub version: u32,
    pub name: String,
    pub created_unix: u64,
    pub hardware: HardwareSnapshot,
    pub client: ClientProfileMetadata,
}

impl DeviceProfile {
    pub(crate) fn new(
        name: impl Into<String>,
        hardware: HardwareSnapshot,
        client: ClientProfileMetadata,
    ) -> Result<Self, String> {
        let profile = Self {
            format: PROFILE_FORMAT.to_string(),
            version: PROFILE_VERSION,
            name: clean_profile_name(&name.into()),
            created_unix: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| error.to_string())?
                .as_secs(),
            hardware,
            client,
        };
        profile.validate()?;
        Ok(profile)
    }

    pub(crate) fn from_json(source: &str) -> Result<Self, String> {
        let profile: Self = serde_json::from_str(source)
            .map_err(|error| format!("Invalid profile JSON: {error}"))?;
        profile.validate()?;
        Ok(profile)
    }

    pub(crate) fn to_json(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(|error| error.to_string())
    }

    fn validate(&self) -> Result<(), String> {
        if self.format != PROFILE_FORMAT || self.version != PROFILE_VERSION {
            return Err(format!(
                "Unsupported profile format/version: {}/{}",
                self.format, self.version
            ));
        }
        if self.name.trim().is_empty() || self.name.chars().count() > 64 {
            return Err("Profile name must contain 1 to 64 characters".to_string());
        }
        let hardware = &self.hardware;
        if hardware.version != HARDWARE_SNAPSHOT_VERSION {
            return Err(format!(
                "Unsupported hardware snapshot version {}",
                hardware.version
            ));
        }
        if hardware.vendor_id != VENDOR_ID || hardware.product_id != PRODUCT_ID {
            return Err(format!(
                "Profile targets {:04X}:{:04X}, expected {VENDOR_ID:04X}:{PRODUCT_ID:04X}",
                hardware.vendor_id, hardware.product_id
            ));
        }
        for (name, count) in [
            ("Base keymap", hardware.base_keymap.len()),
            ("Fn keymap", hardware.fn_keymap.len()),
            ("RGB map", hardware.key_rgb.len()),
        ] {
            if count != KEY_INDEX_COUNT {
                return Err(format!(
                    "{name} contains {count} records; expected {KEY_INDEX_COUNT}"
                ));
            }
        }
        if hardware.macro_storage.len() != MACRO_STORAGE_SIZE {
            return Err(format!(
                "Macro storage contains {} bytes; expected {MACRO_STORAGE_SIZE}",
                hardware.macro_storage.len()
            ));
        }
        validate_macro_storage_layout(&hardware.macro_storage)?;
        if hardware.lighting.kind != 1 {
            return Err(format!(
                "Unsupported lighting record kind {}",
                hardware.lighting.kind
            ));
        }
        if hardware.lighting.effect > 19 {
            return Err(format!(
                "Unsupported lighting effect {}",
                hardware.lighting.effect
            ));
        }
        if hardware.lighting.effect == 0 && hardware.lighting.color_enabled {
            return Err("The Off lighting effect cannot use single-colour mode".to_string());
        }
        for (&id, name) in &self.client.macro_names {
            if id as usize >= MACRO_SLOT_COUNT
                || name.trim().is_empty()
                || name.chars().count() > 48
            {
                return Err(format!("Invalid local name for macro M{id}"));
            }
        }
        if let Some(id) = self
            .client
            .macro_steps
            .keys()
            .find(|&&id| id as usize >= MACRO_SLOT_COUNT)
        {
            return Err(format!("Invalid semantic steps for macro M{id}"));
        }
        if !matches!(self.client.layout.as_str(), "default" | "custom") {
            return Err("Profile layout metadata must be 'default' or 'custom'".to_string());
        }
        Ok(())
    }
}

fn validate_macro_storage_layout(storage: &[u8]) -> Result<(), String> {
    for id in 0..MACRO_SLOT_COUNT {
        let pointer_index = id * 2;
        let pointer_bytes = [storage[pointer_index], storage[pointer_index + 1]];
        if pointer_bytes == [0xFF, 0xFF] || pointer_bytes == [0, 0] {
            continue;
        }
        let offset = u16::from_le_bytes(pointer_bytes) as usize;
        if offset < MACRO_POINTER_TABLE_SIZE || offset + MACRO_EVENT_SIZE > storage.len() {
            return Err(format!("Macro M{id} has an invalid storage pointer"));
        }
        let mut cursor = offset;
        loop {
            if cursor + MACRO_EVENT_SIZE > storage.len() {
                return Err(format!("Macro M{id} is not terminated"));
            }
            let flags = storage[cursor + 2];
            cursor += MACRO_EVENT_SIZE;
            if flags & 0x80 != 0 || flags == 0 {
                break;
            }
        }
    }
    Ok(())
}

pub(crate) fn clean_profile_name(value: &str) -> String {
    let name: String = value.trim().chars().take(64).collect();
    if name.is_empty() {
        "Keyboard Backup".to_string()
    } else {
        name
    }
}

pub(crate) fn profiles_dir() -> PathBuf {
    gtk::glib::user_data_dir()
        .join("anko-keyboard")
        .join("profiles")
}

pub(crate) fn active_profile_path() -> PathBuf {
    profiles_dir().join("active.json")
}

pub(crate) fn load_active_profile() -> Result<Option<DeviceProfile>, String> {
    let path = active_profile_path();
    match std::fs::read_to_string(path) {
        Ok(source) => DeviceProfile::from_json(&source).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

pub(crate) fn save_active_profile(profile: &DeviceProfile) -> Result<(), String> {
    let path = active_profile_path();
    let parent = path
        .parent()
        .ok_or_else(|| "Profile path has no parent".to_string())?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, profile.to_json()?).map_err(|error| error.to_string())?;
    std::fs::rename(&temporary, path).map_err(|error| error.to_string())
}

pub(crate) fn write_profile_file(path: &Path, profile: &DeviceProfile) -> Result<(), String> {
    std::fs::write(path, profile.to_json()?).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyboard_core::{Hsv, LightingState};

    fn fixture() -> DeviceProfile {
        let mut macro_storage = vec![0; MACRO_STORAGE_SIZE];
        macro_storage[..64].fill(0xff);
        DeviceProfile::new(
            "  Work  ",
            HardwareSnapshot {
                version: HARDWARE_SNAPSHOT_VERSION,
                vendor_id: VENDOR_ID,
                product_id: PRODUCT_ID,
                firmware_version: 3,
                protocol_version: 1,
                base_keymap: vec![[0x20, 0, 0x29, 0]; KEY_INDEX_COUNT],
                fn_keymap: vec![[0x20, 0, 0xff, 0]; KEY_INDEX_COUNT],
                lighting: LightingState {
                    kind: 1,
                    effect: 1,
                    brightness: 4,
                    speed: 0,
                    direction: 1,
                    color_enabled: true,
                    single_color_index: 7,
                    hsv: Hsv {
                        hue: 4,
                        saturation: 193,
                        value: 244,
                    },
                },
                key_rgb: vec![[0, 0, 0]; KEY_INDEX_COUNT],
                macro_storage,
            },
            ClientProfileMetadata {
                layout: "default".to_string(),
                ..ClientProfileMetadata::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn profile_round_trips_and_normalizes_name() {
        let profile = fixture();
        assert_eq!(profile.name, "Work");
        assert_eq!(
            DeviceProfile::from_json(&profile.to_json().unwrap()).unwrap(),
            profile
        );
    }

    #[test]
    fn profile_validation_rejects_incomplete_hardware_state() {
        let mut profile = fixture();
        profile.hardware.base_keymap.pop();
        assert!(profile.to_json().unwrap_err().contains("Base keymap"));
    }

    #[test]
    fn profile_validation_rejects_invalid_macro_layout() {
        let mut profile = fixture();
        profile.hardware.macro_storage[0..2].copy_from_slice(&4095u16.to_le_bytes());
        assert!(profile.to_json().unwrap_err().contains("storage pointer"));
    }
}
