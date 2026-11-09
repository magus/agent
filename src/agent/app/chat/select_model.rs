pub(super) fn select_model(saved: &str, current: &str, model_list: &[String]) -> String {
    for candidate in [saved, current] {
        if model_list.iter().any(|model| model == candidate) {
            return candidate.into();
        }
    }
    model_list.first().cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::select_model;

    #[test]
    fn new_conversations_keep_the_current_available_model() {
        let model_list = vec!["first".into(), "chosen".into()];
        assert_eq!(select_model("", "chosen", &model_list), "chosen");
        assert_eq!(select_model("first", "chosen", &model_list), "first");
        assert_eq!(select_model("removed", "chosen", &model_list), "chosen");
        assert_eq!(select_model("", "", &model_list), "first");
        assert_eq!(select_model("old", "old", &[]), "");
    }
}
