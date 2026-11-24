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

    /// Text that streamed but never completed, as entries that say why it ended.
    pub fn take_unfinished(&mut self, stopped: bool) -> Vec<Entry> {
        let reason = if stopped { "Stopped" } else { "Interrupted" };
        std::mem::take(&mut self.text_by_index)
            .into_values()
            .filter(|text| !text.trim().is_empty())
            .map(|text| Entry::text("Agent", format!("{text}\n\n*{reason} before finishing.*")))
            .collect()
    }

    /// Marks a message that got no reply at all, so it doesn't look ignored when scrolling back.
    pub fn unanswered(stopped: bool) -> Entry {
        let reason = if stopped { "Stopped" } else { "Interrupted" };
        Entry::text("Agent", format!("*{reason} before replying.*"))
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

    #[test]
    fn unanswered_messages_say_why() {
        assert_eq!(
            ResponsePreview::unanswered(true).text,
            "*Stopped before replying.*"
        );
        assert_eq!(ResponsePreview::unanswered(false).role, "Agent");
    }

    #[test]
    fn unfinished_text_is_kept_with_its_reason() {
        let mut preview = ResponsePreview::default();
        preview.append(0, "Rivers carve");
        preview.append(1, "  ");
        let kept = preview.take_unfinished(true);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].role, "Agent");
        assert_eq!(kept[0].text, "Rivers carve\n\n*Stopped before finishing.*");
        assert!(preview.take_unfinished(false).is_empty());

        preview.append(0, "Partial");
        assert!(
            preview.take_unfinished(false)[0]
                .text
                .ends_with("*Interrupted before finishing.*")
        );
    }
}
