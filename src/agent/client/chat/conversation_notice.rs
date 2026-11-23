pub(crate) fn conversation_notice(status: &str) -> String {
    if let Some(detail) = status.strip_prefix("Disconnected. ") {
        return format!(
            "Can't reach the Agent service. Check that it's running and that this device is on \
             Tailscale. ({detail})"
        );
    }
    let notice = match status {
        "Ready"
        | "Running"
        | "Stopping…"
        | "Connected. Create a conversation."
        | "Results retrieved. Send a prompt to continue."
        | "A tool is still running. Retrieve again to check."
        | "Interrupted. Retrieve pending results before continuing."
        | "Stop requested. Submitted actions may already have acted; retrieve their results." => "",
        "Stopped. No further model turns or tools will be submitted. Submitted actions cannot be undone." => {
            "Stopped."
        }
        _ => status,
    };
    notice.into()
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
            conversation_notice(
                "Stopped. No further model turns or tools will be submitted. Submitted actions cannot be undone."
            ),
            "Stopped."
        );
        assert_eq!(
            conversation_notice("Disconnected. Connection refused"),
            "Can't reach the Agent service. Check that it's running and that this device is on \
             Tailscale. (Connection refused)"
        );
        assert_eq!(
            conversation_notice("Sign in with ChatGPT first."),
            "Sign in with ChatGPT first."
        );
    }
}
