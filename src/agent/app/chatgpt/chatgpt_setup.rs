use super::login_timing::LoginTiming;
use super::remote_account::RemoteAccount;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{ActiveTheme, Disableable, Icon};
use gpui_kit::{prelude::*, *};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) struct ChatGptSetup {
    client: Arc<RemoteAccount>,
    account: Option<String>,
    busy: bool,
    signing_in: bool,
    message: String,
    cancelled: Arc<AtomicBool>,
    login_timing: Option<LoginTiming>,
    ready_rendered: bool,
    login_hovered: bool,
}

impl ChatGptSetup {
    pub(crate) fn open(client: Arc<RemoteAccount>, cx: &mut App) {
        if crate::app::core::window::activate(cx.default_global::<SetupWindow>().0, cx) {
            return;
        }

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(560.), px(430.)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("Agent · ChatGPT".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        match gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| ChatGptSetup::new(client, window, cx))
        }) {
            Ok((handle, view)) => {
                cx.global_mut::<SetupWindow>().0 = Some(handle);
                crate::app::core::window::observe(handle, &view, cx);
            }
            Err(error) => eprintln!("Could not open ChatGPT setup. {error}"),
        }
        cx.activate(true);
    }

    fn new(client: Arc<RemoteAccount>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.observe_window_activation(window, |view, window, _| {
            if window.is_window_active()
                && let Some(timing) = view.login_timing
            {
                timing.mark("window_activated");
            }
        })
        .detach();
        let load_client = client.clone();
        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { load_client.account() })
                .await;
            let _ = view.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(account) => view.account = account,
                    Err(error) => view.message = error.to_string(),
                }
                cx.notify();
            });
        })
        .detach();

        Self {
            client,
            account: None,
            busy: true,
            signing_in: false,
            message: "Sign in once for all devices connected to your service.".into(),
            cancelled: Arc::new(AtomicBool::new(false)),
            login_timing: None,
            ready_rendered: false,
            login_hovered: false,
        }
    }

    fn run(&mut self, action: AccountAction, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.signing_in = matches!(action, AccountAction::SignIn);
        let timing = LoginTiming::new();
        self.login_timing = self.signing_in.then_some(timing);
        self.ready_rendered = false;
        if self.signing_in {
            timing.mark("started");
        }
        self.cancelled = Arc::new(AtomicBool::new(false));
        self.message = match action {
            AccountAction::SignIn => {
                "Finish signing in through your browser, or cancel login here."
            }
            AccountAction::CheckModel => {
                "Sending 9 × 221 as a test. No screenshot or desktop action is sent."
            }
            AccountAction::SignOut => "Signing out…",
        }
        .into();
        cx.notify();

        let client = self.client.clone();
        let cancelled = self.cancelled.clone();
        cx.spawn(async move |view, cx| {
            let (result, account) = cx
                .background_executor()
                .spawn(async move {
                    let result = match action {
                        AccountAction::SignIn => client
                            .sign_in(&cancelled, &timing)
                            .map(|_| "Signed in with ChatGPT.".into()),
                        AccountAction::CheckModel => client.check_model(),
                        AccountAction::SignOut => client.sign_out(),
                    };
                    let account = client.account();
                    if matches!(action, AccountAction::SignIn) {
                        timing.mark("background_finished");
                    }
                    (result, account)
                })
                .await;
            let activate = view.update(cx, |view, cx| {
                view.busy = false;
                view.signing_in = false;
                view.message = result.unwrap_or_else(|error| error.to_string());
                match account {
                    Ok(account) => view.account = account,
                    Err(error) => view.message = error.to_string(),
                }
                cx.notify();
                if matches!(action, AccountAction::SignIn) {
                    timing.mark("ui_ready");
                }
                matches!(action, AccountAction::SignIn) && !view.cancelled.load(Ordering::Relaxed)
            });

            if activate.unwrap_or(false) {
                timing.mark("focus_requested");
                cx.update(|cx| {
                    let handle = cx.global::<SetupWindow>().0;
                    crate::app::core::window::activate(handle, cx);
                });
            }
        })
        .detach();
    }
}

impl Render for ChatGptSetup {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.busy
            && !self.ready_rendered
            && let Some(timing) = self.login_timing
        {
            timing.mark("enabled_button_rendered");
            self.ready_rendered = true;
        }
        let connected = self.account.is_some();
        let account = self
            .account
            .clone()
            .unwrap_or_else(|| "Not signed in".into());
        div()
            .id("chatgpt-setup")
            .size_full()
            .overflow_y_scroll()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .p_6()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("ChatGPT"),
            )
            .child(div().text_sm().child(account))
            .child(
                div().flex().gap_2().child(
                    Button::new("chatgpt-login")
                        .min_w(px(220.))
                        .label(if self.signing_in && self.login_hovered {
                            "Cancel"
                        } else if self.signing_in {
                            "Connecting ChatGPT…"
                        } else {
                            "Login with ChatGPT"
                        })
                        .icon(if self.signing_in && self.login_hovered {
                            Icon::new(IconName::X)
                        } else {
                            Icon::default().data(include_bytes!(concat!(
                                env!("CARGO_MANIFEST_DIR"),
                                "/assets/openai.svg"
                            )))
                        })
                        .primary()
                        .disabled(self.busy && !self.signing_in)
                        .on_hover(cx.listener(|view, hovered, _, cx| {
                            view.login_hovered = *hovered;
                            cx.notify();
                        }))
                        .on_click(cx.listener(|view, _, _, cx| {
                            if view.signing_in {
                                view.cancelled.store(true, Ordering::Relaxed);
                                view.message = "Cancelling login…".into();
                                cx.notify();
                            } else {
                                view.run(AccountAction::SignIn, cx);
                            }
                        })),
                ),
            )
            .when(connected, |view| {
                view.child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            Button::new("check-model")
                                .label("Test model")
                                .disabled(self.busy)
                                .on_click(cx.listener(|view, _, _, cx| {
                                    view.run(AccountAction::CheckModel, cx)
                                })),
                        )
                        .child(
                            Button::new("sign-out")
                                .label("Sign out")
                                .disabled(self.busy)
                                .on_click(cx.listener(|view, _, _, cx| {
                                    view.run(AccountAction::SignOut, cx)
                                })),
                        ),
                )
            })
            .child(div().text_sm().child(self.message.clone()))
            .child(
                Button::new("chatgpt-usage")
                    .label("ChatGPT usage")
                    .on_click(|_, _, cx| cx.open_url("https://chatgpt.com/settings/usage")),
            )
    }
}

impl Drop for ChatGptSetup {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

#[derive(Clone, Copy)]
enum AccountAction {
    SignIn,
    CheckModel,
    SignOut,
}

#[derive(Default)]
struct SetupWindow(Option<AnyWindowHandle>);

impl Global for SetupWindow {}
