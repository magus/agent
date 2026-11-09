use crate::protocol::ConversationView;

#[derive(Clone)]
pub(super) struct PendingMessage {
    pub service_host: String,
    pub conversation_id: String,
    pub message_id: String,
    pub prompt: String,
    pub model: String,
}

impl PendingMessage {
    pub fn confirmed(&self, service_host: &str, conversation: &ConversationView) -> bool {
        self.service_host == service_host
            && self.conversation_id == conversation.id
            && conversation
                .entry_list
                .iter()
                .any(|entry| entry.message_id.as_deref() == Some(&self.message_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Entry;

    #[test]
    fn only_the_matching_receipt_confirms_a_message() {
        let pending = PendingMessage {
            service_host: "service".into(),
            conversation_id: "chat".into(),
            message_id: "receipt".into(),
            prompt: "Same text".into(),
            model: "model".into(),
        };
        let mut conversation = ConversationView {
            id: "chat".into(),
            ..Default::default()
        };
        assert!(!pending.confirmed("service", &conversation));

        let mut entry = Entry::text("You", "Same text");
        entry.message_id = Some("earlier-message".into());
        conversation.entry_list.push(entry);
        assert!(!pending.confirmed("service", &conversation));

        conversation.entry_list[0].message_id = Some("receipt".into());
        assert!(pending.confirmed("service", &conversation));
        assert!(!pending.confirmed("another-service", &conversation));
        conversation.id = "another-chat".into();
        assert!(!pending.confirmed("service", &conversation));
    }
}
