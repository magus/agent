use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy)]
pub(crate) struct LoginTiming {
    started: Instant,
    attempt: u128,
}

impl LoginTiming {
    pub(crate) fn new() -> Self {
        Self {
            started: Instant::now(),
            attempt: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        }
    }

    pub(crate) fn mark(&self, event: &str) {
        eprintln!(
            "login_timing attempt={} elapsed_ms={} event={event}",
            self.attempt,
            self.started.elapsed().as_millis()
        );
    }
}
