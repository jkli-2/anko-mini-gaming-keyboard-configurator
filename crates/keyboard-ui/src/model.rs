#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KeymapDraft {
    original: Vec<(String, String)>,
    draft: Vec<(String, String)>,
}

impl KeymapDraft {
    pub fn new(assignments: Vec<(String, String)>) -> Self {
        Self {
            original: assignments.clone(),
            draft: assignments,
        }
    }

    pub fn action(&self, key_id: &str) -> Option<&str> {
        self.draft
            .iter()
            .find(|(id, _)| id == key_id)
            .map(|(_, action)| action.as_str())
    }

    pub fn set_action(&mut self, key_id: &str, action: String) -> bool {
        let Some((_, current)) = self.draft.iter_mut().find(|(id, _)| id == key_id) else {
            return false;
        };
        *current = action;
        true
    }

    pub fn assignments(&self) -> Vec<(String, String)> {
        self.draft.clone()
    }

    pub fn is_dirty(&self) -> bool {
        self.original != self.draft
    }

    pub fn revert(&mut self) {
        self.draft.clone_from(&self.original);
    }

    pub fn commit(&mut self) {
        self.original.clone_from(&self.draft);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drafts_edit_revert_and_commit_locally() {
        let original = vec![
            ("esc".to_string(), "keyboard/41/0".to_string()),
            ("a".to_string(), "keyboard/4/0".to_string()),
        ];
        let mut draft = KeymapDraft::new(original.clone());
        assert!(!draft.is_dirty());
        assert!(draft.set_action("esc", "keyboard/69/0".to_string()));
        assert!(draft.is_dirty());
        assert_eq!(draft.action("esc"), Some("keyboard/69/0"));
        assert!(!draft.set_action("unknown", "keyboard/0/0".to_string()));

        draft.revert();
        assert_eq!(draft.assignments(), original);
        assert!(!draft.is_dirty());

        draft.set_action("a", "keyboard/5/0".to_string());
        draft.commit();
        assert!(!draft.is_dirty());
        assert_eq!(draft.action("a"), Some("keyboard/5/0"));
    }
}
