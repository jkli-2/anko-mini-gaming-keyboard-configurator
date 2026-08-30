//! HID transport and packet codec for the Anko/Kmart Z64 keyboard.

mod action;
mod config;
mod device;
mod layout;
mod lighting;
mod macros;
mod packet;

pub use action::{decode_key_action, encode_key_action};
pub use config::{ProtocolError, encode_config_request, parse_config_response};
pub use device::{
    KeyboardDevice, PRODUCT_ID, REPORT_ID, REPORT_SIZE, TARGET_USAGE, TARGET_USAGE_PAGE,
    TransportError, VENDOR_ID, find_target, read_keyboard_config,
};
pub use layout::{
    DECLARED_KEY_INDEX_COUNT, KEY_INDEX_COUNT, PHYSICAL_KEYS, PhysicalKey, physical_key,
    physical_key_by_id,
};
pub use lighting::{
    encode_lighting_effect_request, encode_lighting_read_request, encode_lighting_write,
    parse_lighting_config_response, parse_lighting_response,
};
pub use macros::{MACRO_STORAGE_SIZE, MacroCodecError, decode_macro_storage, encode_macro_storage};
pub use packet::{
    BLOCK_DATA_SIZE, PacketError, decode_block_response, decode_le_u16, decode_rgb_values,
    encode_factory_reset, encode_key_color_write, encode_key_write, encode_keymap_bulk_block,
    encode_keymap_read, encode_le_u16, encode_macro_read, encode_macro_write,
    encode_rgb_bulk_block, encode_rgb_read, encode_rgb_values,
};
