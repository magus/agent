use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy)]
pub(super) struct LoginTiming {
    started: Instant,
    attempt: u128,
}

impl LoginTiming {
    pub(super) fn new() -> Self {
        Self {
            started: Instant::now(),
            attempt: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        }
    }

    pub(super) fn mark(&self, event: &str) {
        eprintln!(
            "login_timing attempt={} elapsed_ms={} event={event}",
            self.attempt,
            self.started.elapsed().as_millis()
        );
    }
}
