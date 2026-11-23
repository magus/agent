use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct ConversationRecord {
    pub id: String,
    pub title: String,
    pub creator: String,
    #[serde(default, skip_serializing_if = "super::ModelSettings::is_default")]
    pub model_settings: super::ModelSettings,
    pub allowed_target_list: Vec<String>,
    /// Unix seconds of creation or the latest user message; orders the conversation list.
    #[serde(default)]
    pub last_activity: u64,
}

/// Title a client gives a new conversation before it has a message.
pub(crate) fn default_title(id: &str) -> String {
    format!("Conversation {}", id.get(..8).unwrap_or(id))
}

/// Title taken from a conversation's first message: its first line, cut at a word near 60 characters.
pub(crate) fn title_from_message(prompt: &str) -> Option<String> {
    let line = prompt
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())?;
    let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.chars().count() <= 60 {
        return Some(line);
    }
    let cut: String = line.chars().take(60).collect();
    let cut = match cut.rfind(' ') {
        Some(index) if index >= 30 => &cut[..index],
        _ => cut.as_str(),
    };
    Some(format!(
        "{}…",
        cut.trim_end_matches([',', '.', ';', ':', ' '])
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_use_the_first_line_and_stop_at_a_word() {
        assert_eq!(default_title("314df3d2-aaaa"), "Conversation 314df3d2");
        assert_eq!(
            title_from_message("\n  Fix   the\tbuild \nmore"),
            Some("Fix the build".into())
        );
        assert_eq!(title_from_message("   \n "), None);
        let long = "Compare the battery life of these three laptops for travel, then summarize";
        let title = title_from_message(long).unwrap();
        assert_eq!(
            title,
            "Compare the battery life of these three laptops for travel…"
        );
        assert!(title.chars().count() <= 61);
    }
}
