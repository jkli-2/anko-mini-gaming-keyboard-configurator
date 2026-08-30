#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicalKey {
    pub id: &'static str,
    pub label: &'static str,
    pub index: u16,
}

macro_rules! key {
    ($id:literal, $label:literal, $index:literal) => {
        PhysicalKey {
            id: $id,
            label: $label,
            index: $index,
        }
    };
}

/// The 63 physical keys in stable UI order, with their sparse matrix positions.
pub const PHYSICAL_KEYS: &[PhysicalKey] = &[
    key!("esc", "Esc", 0),
    key!("digit-1", "1", 1),
    key!("digit-2", "2", 2),
    key!("digit-3", "3", 3),
    key!("digit-4", "4", 4),
    key!("digit-5", "5", 5),
    key!("digit-6", "6", 6),
    key!("digit-7", "7", 7),
    key!("digit-8", "8", 8),
    key!("digit-9", "9", 9),
    key!("digit-0", "0", 10),
    key!("minus", "-", 11),
    key!("equal", "+", 12),
    key!("backspace", "Back", 13),
    key!("tab", "Tab", 15),
    key!("q", "Q", 16),
    key!("w", "W", 17),
    key!("e", "E", 18),
    key!("r", "R", 19),
    key!("t", "T", 20),
    key!("y", "Y", 21),
    key!("u", "U", 22),
    key!("i", "I", 23),
    key!("o", "O", 24),
    key!("p", "P", 25),
    key!("bracket-left", "[", 26),
    key!("bracket-right", "]", 27),
    key!("backslash", "\\", 28),
    key!("caps", "Caps", 30),
    key!("a", "A", 31),
    key!("s", "S", 32),
    key!("d", "D", 33),
    key!("f", "F", 34),
    key!("g", "G", 35),
    key!("h", "H", 36),
    key!("j", "J", 37),
    key!("k", "K", 38),
    key!("l", "L", 39),
    key!("semicolon", ";", 40),
    key!("quote", "'", 41),
    key!("enter", "Enter", 43),
    key!("shift-left", "LShift", 45),
    key!("z", "Z", 47),
    key!("x", "X", 48),
    key!("c", "C", 49),
    key!("v", "V", 50),
    key!("b", "B", 51),
    key!("n", "N", 52),
    key!("m", "M", 53),
    key!("comma", ",", 54),
    key!("period", ".", 55),
    key!("slash", "/", 56),
    key!("arrow-up", "Up", 57),
    key!("shift-right", "RShift", 58),
    key!("ctrl-left", "LCtrl", 60),
    key!("meta-left", "Win", 61),
    key!("alt-left", "LAlt", 62),
    key!("space", "Space", 66),
    key!("alt-right", "RAlt", 69),
    key!("arrow-left", "Left", 70),
    key!("arrow-down", "Down", 71),
    key!("arrow-right", "Right", 72),
    key!("fn", "Fn", 73),
];

pub fn physical_key_by_id(id: &str) -> Option<&'static PhysicalKey> {
    PHYSICAL_KEYS.iter().find(|key| key.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_has_unique_ids_and_matrix_positions() {
        assert_eq!(PHYSICAL_KEYS.len(), 63);
        for (position, key) in PHYSICAL_KEYS.iter().enumerate() {
            assert!(key.index < 75);
            assert!(
                !PHYSICAL_KEYS[..position]
                    .iter()
                    .any(|previous| { previous.index == key.index || previous.id == key.id })
            );
        }
    }
}
