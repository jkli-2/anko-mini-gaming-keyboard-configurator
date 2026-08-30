use serde::{Deserialize, Serialize};

use crate::LightingState;

/// Version of the daemon-owned, lossless hardware snapshot schema.
pub const HARDWARE_SNAPSHOT_VERSION: u32 = 1;

/// Exact persistent state captured from one supported keyboard.
///
/// Keymaps contain all sparse matrix records, not only the physical keys shown
/// by the UI. Macro storage is retained byte-for-byte so a backup does not lose
/// data merely because the current semantic decoder does not understand it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HardwareSnapshot {
    pub version: u32,
    pub vendor_id: u16,
    pub product_id: u16,
    pub firmware_version: u16,
    pub protocol_version: u16,
    pub base_keymap: Vec<[u8; 4]>,
    pub fn_keymap: Vec<[u8; 4]>,
    pub lighting: LightingState,
    pub key_rgb: Vec<[u8; 3]>,
    pub macro_storage: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Hsv, LightingState};

    #[test]
    fn snapshot_schema_round_trips_losslessly() {
        let snapshot = HardwareSnapshot {
            version: HARDWARE_SNAPSHOT_VERSION,
            vendor_id: 0x36ae,
            product_id: 0xfda1,
            firmware_version: 3,
            protocol_version: 1,
            base_keymap: vec![[0x20, 0, 0x29, 0]],
            fn_keymap: vec![[0x1f, 0x24, 0, 0]],
            lighting: LightingState {
                kind: 1,
                effect: 5,
                brightness: 4,
                speed: 2,
                direction: 3,
                color_enabled: true,
                single_color_index: 7,
                hsv: Hsv {
                    hue: 255,
                    saturation: 255,
                    value: 60,
                },
            },
            key_rgb: vec![[255, 0, 0]],
            macro_storage: vec![0xff, 0xff, 0, 0],
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: HardwareSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, snapshot);
    }
}
