pub(crate) fn conversation_notice(status: &str) -> &str {
    match status {
        "Ready"
        | "Running"
        | "Stopping…"
        | "Connected. Create a conversation."
        | "Results retrieved. Send a prompt to continue."
        | "A tool is still running. Retrieve again to check."
        | "Interrupted. Retrieve pending results before continuing."
        | "Stop requested. Submitted actions may already have acted; retrieve their results."
        | "Stopped. No further model turns or tools will be submitted. Submitted actions cannot be undone." => {
            ""
        }
        _ => status,
    }
}

#[cfg(test)]
mod tests {
    use super::conversation_notice;

    #[test]
    fn routine_status_is_silent_but_errors_remain_visible() {
        assert_eq!(
            conversation_notice("Results retrieved. Send a prompt to continue."),
            ""
        );
        assert_eq!(conversation_notice("Stopping…"), "");
        assert_eq!(
            conversation_notice("Disconnected. Connection refused"),
            "Disconnected. Connection refused"
        );
        assert_eq!(
            conversation_notice("Sign in with ChatGPT first."),
            "Sign in with ChatGPT first."
        );
    }
}
