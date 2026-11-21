use super::{LoginState, LoginView};
use std::io;
use std::sync::atomic::Ordering;

#[test]
fn cancellation_releases_the_ui_before_the_worker_finishes() {
    let mut state = pending_state();
    let worker = state.cancelled.clone();
    state
        .publish_url(&worker, "https://auth.openai.com/first")
        .unwrap();

    state.cancel();

    assert!(!state.view.pending);
    assert!(state.view.url.is_none());
    assert!(worker.load(Ordering::SeqCst));
    assert!(
        state
            .publish_url(&worker, "https://auth.openai.com/late")
            .is_err()
    );

    state.finish(&worker, Ok("old account".into()));
    assert!(!state.view.completed);
    assert!(state.view.account.is_none());
}

#[test]
fn old_worker_cannot_overwrite_a_new_attempt_even_with_the_same_id() {
    let mut state = pending_state();
    let old_worker = state.cancelled.clone();
    state.cancel();

    state = pending_state();
    let new_worker = state.cancelled.clone();
    state
        .publish_url(&new_worker, "https://auth.openai.com/new")
        .unwrap();

    assert!(
        state
            .publish_url(&old_worker, "https://auth.openai.com/old")
            .is_err()
    );
    state.finish(&old_worker, Err(io::Error::other("old failure")));
    state.finish(&old_worker, Ok("old account".into()));

    assert!(state.view.pending);
    assert_eq!(
        state.view.url.as_deref(),
        Some("https://auth.openai.com/new")
    );
    assert!(state.view.error.is_none());
    assert!(state.view.account.is_none());

    state.finish(&new_worker, Ok("new account".into()));
    assert!(!state.view.pending);
    assert!(state.view.completed);
    assert!(state.view.url.is_none());
    assert_eq!(state.view.account.as_deref(), Some("new account"));
}

fn pending_state() -> LoginState {
    LoginState {
        view: LoginView {
            id: "attempt".into(),
            pending: true,
            ..LoginView::default()
        },
        ..LoginState::default()
    }
}
