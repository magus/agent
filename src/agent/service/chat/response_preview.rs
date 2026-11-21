use crate::protocol::Entry;
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct ResponsePreview {
    text_by_index: BTreeMap<usize, String>,
}

impl ResponsePreview {
    pub fn append(&mut self, index: usize, text: &str) {
        self.text_by_index.entry(index).or_default().push_str(text);
    }

    pub fn append_to(&self, entry_list: &mut Vec<Entry>) {
        for text in self.text_by_index.values() {
            if !text.is_empty() {
                entry_list.push(Entry::text("Agent", text));
            }
        }
    }

    pub fn clear(&mut self) {
        self.text_by_index.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previews_append_in_output_order_without_mutating_saved_history() {
        let history = vec![Entry::text("You", "Question")];
        let mut preview = ResponsePreview::default();
        preview.append(2, "Second");
        preview.append(0, "Fir");
        preview.append(0, "st");
        let mut visible = history.clone();
        preview.append_to(&mut visible);

        let text_list: Vec<_> = visible.iter().map(|entry| entry.text.as_str()).collect();
        assert_eq!(text_list, ["Question", "First", "Second"]);
        assert_eq!(history.len(), 1);

        preview.clear();
        let mut visible = history.clone();
        preview.append_to(&mut visible);
        assert_eq!(visible.len(), 1);
    }
}
