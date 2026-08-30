pub use keyboard_core::{PHYSICAL_KEYS, PhysicalKey, physical_key_by_id};

/// Safe key/RGB storage extent: the complete 5 x 15 matrix index space.
pub const KEY_INDEX_COUNT: usize = 75;

/// Generic array extent declared by the vendor JSON. This is not a safe storage extent.
pub const DECLARED_KEY_INDEX_COUNT: usize = 126;

pub fn physical_key(index: u16) -> Option<&'static PhysicalKey> {
    PHYSICAL_KEYS.iter().find(|key| key.index == index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_has_63_unique_sparse_keys() {
        assert_eq!(PHYSICAL_KEYS.len(), 63);
        assert_eq!(KEY_INDEX_COUNT, 5 * 15);
        assert_eq!(DECLARED_KEY_INDEX_COUNT, 126);
    }

    #[test]
    fn important_indices_match_device_definition() {
        let cases = [
            (0, "esc"),
            (30, "caps"),
            (66, "space"),
            (70, "arrow-left"),
            (71, "arrow-down"),
            (72, "arrow-right"),
            (73, "fn"),
        ];
        for (index, id) in cases {
            assert_eq!(physical_key(index).map(|key| key.id), Some(id));
        }
        assert_eq!(physical_key(14), None);
        assert_eq!(physical_key_by_id("space").map(|key| key.index), Some(66));
        assert_eq!(physical_key_by_id("unknown"), None);
    }
}
