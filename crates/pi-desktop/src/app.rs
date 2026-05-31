use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, SharedString, StatefulInteractiveElement as _, Styled as _,
    Subscription, Window, div, font, px, svg,
};
use gpui_component::group_box::GroupBoxVariant;
use gpui_component::input::{InputEvent, InputState};
use gpui_component::setting::{SettingGroup, SettingItem, SettingPage, Settings};
use gpui_component::{IconName, Sizable as _, Size, StyledExt as _};
use pi_bridge_types::ProviderAuthStatus;

use crate::backend::{BackendData, BackendSession, BackendSnapshot};
use crate::components::button::{PiButtonKind, pi_button, pi_icon_button};
use crate::components::input::pi_input;
use crate::components::surface;
use crate::design::theme;
use crate::ui;

pub struct PiDesktop {
    settings_drawer_open: bool,
    backend: Option<Arc<BackendSession>>,
    data: Option<BackendData>,
    selected_provider: Option<String>,
    api_key_input: Entity<InputState>,
    prompt_input: Entity<InputState>,
    status: SharedString,
    pending: bool,
    cwd: PathBuf,
    _subscriptions: Vec<Subscription>,
}

impl PiDesktop {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let api_key_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Paste API key")
                .masked(true)
        });
        let prompt_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Ask Pi anything")
                .auto_grow(1, 4)
        });
        let prompt_subscription =
            cx.subscribe_in(&prompt_input, window, |view, input, event, window, cx| {
                if matches!(event, InputEvent::PressEnter { secondary: false }) {
                    let text = input.read(cx).value().to_string();
                    if !text.trim().is_empty() {
                        input.update(cx, |input, cx| input.set_value("", window, cx));
                        view.send_prompt_text(text, cx);
                    }
                }
            });
        let cwd = std::env::current_dir().unwrap_or_else(|_error| PathBuf::from("."));
        let mut this = Self {
            settings_drawer_open: false,
            backend: None,
            data: None,
            selected_provider: None,
            api_key_input,
            prompt_input,
            status: "Starting embedded Pi backend…".into(),
            pending: true,
            cwd,
            _subscriptions: vec![prompt_subscription],
        };
        this.start_backend(cx);
        this
    }

    fn start_backend(&mut self, cx: &mut Context<Self>) {
        if self.backend.is_some() {
            self.run_backend(
                "Refreshing embedded Pi backend…",
                |backend| backend.refresh(),
                cx,
            );
            return;
        }
        self.pending = true;
        self.status = "Starting embedded Pi backend…".into();
        cx.notify();
        let cwd = self.cwd.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { BackendSession::connect(cwd) })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.pending = false;
                match result {
                    Ok(snapshot) => view.apply_snapshot(snapshot),
                    Err(error) => {
                        view.backend = None;
                        view.data = None;
                        view.status = format!("Backend unavailable: {error:#}").into();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn apply_snapshot(&mut self, snapshot: BackendSnapshot) {
        self.backend = Some(snapshot.session);
        self.apply_data(snapshot.data, "Embedded Pi backend ready.");
    }

    fn apply_data(&mut self, data: BackendData, status: impl Into<SharedString>) {
        self.data = Some(data);
        self.status = status.into();
    }

    fn run_backend(
        &mut self,
        pending_status: impl Into<SharedString>,
        operation: impl FnOnce(Arc<BackendSession>) -> Result<BackendData> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.backend.clone() else {
            self.status = "Backend is not ready yet.".into();
            cx.notify();
            return;
        };
        self.pending = true;
        self.status = pending_status.into();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { operation(session) }).await;
            let _ = this.update(cx, |view, cx| {
                view.pending = false;
                match result {
                    Ok(data) => view.apply_data(data, "Settings synchronized."),
                    Err(error) => view.status = format!("Backend error: {error:#}").into(),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn open_landing(&mut self, cx: &mut Context<Self>) {
        self.settings_drawer_open = false;
        cx.notify();
    }

    fn toggle_settings_drawer(&mut self, cx: &mut Context<Self>) {
        self.settings_drawer_open = !self.settings_drawer_open;
        cx.notify();
    }

    fn select_provider(&mut self, provider: String, cx: &mut Context<Self>) {
        self.selected_provider = Some(provider);
        cx.notify();
    }

    fn save_api_key(&mut self, persist: bool, cx: &mut Context<Self>) {
        let Some(provider) = self.selected_provider.clone() else {
            self.status = "Pick a provider first.".into();
            cx.notify();
            return;
        };
        let api_key = self.api_key_input.read(cx).value().to_string();
        if api_key.trim().is_empty() {
            self.status = "Paste an API key before saving.".into();
            cx.notify();
            return;
        }
        let label = if persist {
            "Saving API key to Pi auth storage…"
        } else {
            "Activating API key for this run…"
        };
        self.run_backend(
            label,
            move |backend| backend.save_api_key(provider, api_key, persist),
            cx,
        );
    }

    fn remove_selected_auth(&mut self, cx: &mut Context<Self>) {
        let Some(provider) = self.selected_provider.clone() else {
            self.status = "Pick a provider first.".into();
            cx.notify();
            return;
        };
        self.run_backend(
            "Removing provider auth…",
            move |backend| backend.remove_auth(provider),
            cx,
        );
    }

    fn send_prompt_text(&mut self, text: String, cx: &mut Context<Self>) {
        self.run_backend(
            "Sending prompt through the embedded Pi SDK…",
            move |backend| backend.prompt(text),
            cx,
        );
    }

    fn providers(&self) -> Vec<ProviderAuthStatus> {
        self.data
            .as_ref()
            .map(|data| data.auth.clone())
            .unwrap_or_default()
    }
}

impl Render for PiDesktop {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::app_bg())
            .text_color(theme::text())
            .child(self.render_status_bar(cx))
            .child(
                div()
                    .flex_1()
                    .relative()
                    .overflow_hidden()
                    .child(self.render_background_grid())
                    .child(self.render_landing())
                    .when(self.settings_drawer_open, |this| {
                        this.child(self.render_settings_drawer(cx))
                    }),
            )
            .child(self.render_chat_bar(cx))
    }
}

impl PiDesktop {
    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .h(px(28.0))
            .px_2()
            .child(
                div()
                    .id("home-logo")
                    .p_1()
                    .cursor_pointer()
                    .hover(|style| style.bg(theme::surface_hover()))
                    .on_click(cx.listener(|view, _, _, cx| view.open_landing(cx)))
                    .child(
                        svg()
                            .path(ui::logo_path())
                            .size_4()
                            .text_color(theme::text()),
                    ),
            )
            .child(
                pi_icon_button("settings", IconName::Settings, PiButtonKind::Ghost, cx)
                    .on_click(cx.listener(|view, _, _, cx| view.toggle_settings_drawer(cx))),
            )
    }

    fn render_landing(&self) -> impl IntoElement {
        div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .px_8()
            .child(
                div()
                    .mb(px(74.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_5()
                    .max_w(px(760.0))
                    .child(
                        svg()
                            .path(ui::logo_path())
                            .size(px(104.0))
                            .text_color(theme::text()),
                    )
                    .child(hero_title()),
            )
    }

    fn render_background_grid(&self) -> impl IntoElement {
        let vertical = (0..96).map(|index| {
            let major = index % 4 == 0;
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(px(index as f32 * 16.0))
                .w(px(1.0))
                .bg(if major {
                    theme::grid_major()
                } else {
                    theme::grid_minor()
                })
                .into_any_element()
        });
        let horizontal = (0..72).map(|index| {
            let major = index % 4 == 0;
            div()
                .absolute()
                .left_0()
                .right_0()
                .top(px(index as f32 * 16.0))
                .h(px(1.0))
                .bg(if major {
                    theme::grid_major()
                } else {
                    theme::grid_minor()
                })
                .into_any_element()
        });

        div()
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .left_0()
            .children(vertical.chain(horizontal))
    }

    fn render_settings_drawer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .w(px(520.0))
            .occlude()
            .border_l_1()
            .border_color(theme::hairline())
            .bg(theme::surface())
            .child(self.render_settings_component(cx))
    }

    fn render_settings_component(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let providers = self.providers();
        let selected_provider = self.selected_provider.clone();
        let api_key_input = self.api_key_input.clone();
        let status = self.status.clone();
        let pending = self.pending;
        let view = cx.entity().clone();

        Settings::new("pi-settings")
            .sidebar_width(px(148.0))
            .with_size(Size::Small)
            .with_group_variant(GroupBoxVariant::Outline)
            .pages(vec![
                SettingPage::new("Auth")
                    .default_open(true)
                    .resettable(false)
                    .group(SettingGroup::new().title("Providers").item(SettingItem::render(
                        move |_, _, cx| {
                            auth_settings_content(
                                providers.clone(),
                                selected_provider.clone(),
                                api_key_input.clone(),
                                status.clone(),
                                pending,
                                view.clone(),
                                cx,
                            )
                        },
                    ))),
                SettingPage::new("Theme")
                    .resettable(false)
                    .group(SettingGroup::new().title("Theme").item(SettingItem::render(
                        |_, _, _| {
                            settings_placeholder(
                                "Pi.dev dark, grid controls, accent, and font settings will live here.",
                            )
                        },
                    ))),
                SettingPage::new("Advanced")
                    .resettable(false)
                    .group(SettingGroup::new().title("Advanced").item(SettingItem::render(
                        |_, _, _| {
                            settings_placeholder(
                                "Runtime, workspace, SDK, and debugging controls will live here.",
                            )
                        },
                    ))),
            ])
    }

    fn render_chat_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .border_t_1()
            .border_color(theme::hairline())
            .bg(theme::surface())
            .p_3()
            .flex()
            .gap_3()
            .items_end()
            .child(
                div()
                    .flex_1()
                    .child(pi_input(&self.prompt_input).h(px(44.0))),
            )
            .child(
                pi_button("send-prompt", "Send", PiButtonKind::Primary, cx)
                    .loading(self.pending)
                    .on_click(cx.listener(|view, _, window, cx| {
                        let text = view.prompt_input.read(cx).value().to_string();
                        if text.trim().is_empty() {
                            view.status = "Type a prompt first.".into();
                            cx.notify();
                        } else {
                            view.prompt_input
                                .update(cx, |input, cx| input.set_value("", window, cx));
                            view.send_prompt_text(text, cx);
                        }
                    })),
            )
    }
}

fn auth_settings_content(
    providers: Vec<ProviderAuthStatus>,
    selected_provider: Option<String>,
    api_key_input: Entity<InputState>,
    status_text: SharedString,
    pending: bool,
    view: Entity<PiDesktop>,
    cx: &mut gpui::App,
) -> AnyElement {
    let selected_status = selected_provider.as_ref().and_then(|provider| {
        providers
            .iter()
            .find(|status| status.provider == *provider)
            .cloned()
    });
    let provider_icons = providers
        .into_iter()
        .map(|status| {
            provider_icon_cell(status, selected_provider.as_deref(), view.clone())
                .into_any_element()
        })
        .collect::<Vec<_>>();

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .id("provider-icon-scroll")
                .h(px(124.0))
                .overflow_y_scroll()
                .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                .border_1()
                .border_color(theme::hairline())
                .bg(theme::app_bg())
                .p_2()
                .flex()
                .flex_wrap()
                .justify_center()
                .gap_2()
                .children(provider_icons),
        )
        .when_some(selected_status, |this, status| {
            this.child(provider_auth_flow(
                status,
                api_key_input,
                status_text,
                pending,
                view,
                cx,
            ))
        })
        .into_any_element()
}

fn provider_icon_cell(
    status: ProviderAuthStatus,
    selected_provider: Option<&str>,
    view: Entity<PiDesktop>,
) -> impl IntoElement {
    let selected = selected_provider == Some(status.provider.as_str());
    let provider = status.provider.clone();
    div()
        .id(SharedString::from(format!("provider-{}", status.provider)))
        .size(px(42.0))
        .border_1()
        .border_color(if selected {
            theme::accent()
        } else {
            theme::hairline()
        })
        .bg(if selected {
            theme::surface_selected()
        } else {
            gpui::transparent_black()
        })
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|style| style.bg(theme::surface_selected()))
        .on_click(move |_, _, cx| {
            let provider = provider.clone();
            view.update(cx, |view, cx| view.select_provider(provider, cx));
        })
        .child(provider_logo(
            &status.provider,
            &status.display_name,
            px(24.0),
        ))
}

fn provider_auth_flow(
    status: ProviderAuthStatus,
    api_key_input: Entity<InputState>,
    status_text: SharedString,
    pending: bool,
    view: Entity<PiDesktop>,
    cx: &mut gpui::App,
) -> AnyElement {
    let provider = status.provider.clone();
    let configured = status.configured;
    let source = ui::auth_source_label(status.source);
    let browser_view = view.clone();
    let save_view = view.clone();
    let runtime_view = view.clone();
    let remove_view = view.clone();

    div()
        .border_1()
        .border_color(theme::hairline())
        .bg(theme::surface())
        .p_3()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(provider_logo(&provider, &status.display_name, px(28.0)))
                .child(
                    div()
                        .flex_1()
                        .child(div().font_semibold().child(status.display_name))
                        .child(
                            div()
                                .text_sm()
                                .text_color(if configured {
                                    theme::success()
                                } else {
                                    theme::text_muted()
                                })
                                .child(source),
                        ),
                ),
        )
        .child(auth_step(
            "1",
            auth_title(&provider),
            auth_detail(&provider),
        ))
        .when(provider_supports_browser_auth(&provider), |this| {
            this.child(
                pi_button(
                    "browser-auth",
                    "Open browser or device auth",
                    PiButtonKind::Secondary,
                    cx,
                )
                .on_click(move |_, _, cx| {
                    browser_view.update(cx, |view, cx| {
                        view.status =
                            "Browser/device auth UI is staged; backend auth endpoint comes next."
                                .into();
                        cx.notify();
                    });
                }),
            )
        })
        .child(auth_step(
            "2",
            "Fallback API key",
            &ui::provider_env_hint(&provider),
        ))
        .child(pi_input(&api_key_input).h(px(38.0)))
        .child(
            div()
                .flex()
                .gap_2()
                .child(
                    pi_button("save-key", "Save", PiButtonKind::Primary, cx)
                        .loading(pending)
                        .on_click(move |_, _, cx| {
                            save_view.update(cx, |view, cx| view.save_api_key(true, cx));
                        }),
                )
                .child(
                    pi_button("runtime-key", "Use once", PiButtonKind::Ghost, cx).on_click(
                        move |_, _, cx| {
                            runtime_view.update(cx, |view, cx| view.save_api_key(false, cx));
                        },
                    ),
                ),
        )
        .child(
            pi_button("remove-auth", "Remove auth", PiButtonKind::Danger, cx).on_click(
                move |_, _, cx| {
                    remove_view.update(cx, |view, cx| view.remove_selected_auth(cx));
                },
            ),
        )
        .child(status_box(status_text))
        .into_any_element()
}

fn settings_placeholder(message: &'static str) -> AnyElement {
    div()
        .w_full()
        .border_1()
        .border_color(theme::hairline())
        .bg(theme::app_bg())
        .p_3()
        .text_sm()
        .text_color(theme::text_muted())
        .child(message)
        .into_any_element()
}

fn status_box(status: SharedString) -> impl IntoElement {
    surface::quiet_panel()
        .p_2()
        .text_sm()
        .text_color(theme::text_muted())
        .child(status)
}

fn hero_title() -> impl IntoElement {
    div()
        .font(font(theme::SERIF_FONT).italic())
        .text_size(px(44.0))
        .line_height(px(48.0))
        .font_weight(gpui::FontWeight(400.0))
        .text_color(theme::text())
        .text_center()
        .child(div().child("There are many agent harnesses,"))
        .child(
            div()
                .flex()
                .justify_center()
                .gap_2()
                .child("but this one is")
                .child(div().text_color(theme::accent()).child("yours.")),
        )
}

fn provider_supports_browser_auth(provider: &str) -> bool {
    matches!(provider, "openai" | "github-copilot")
}

fn auth_title(provider: &str) -> &'static str {
    match provider {
        "openai" => "Sign in with ChatGPT",
        "github-copilot" => "Sign in with GitHub",
        _ => "Authenticate",
    }
}

fn auth_detail(provider: &str) -> &'static str {
    match provider {
        "openai" => "Use browser/device auth for ChatGPT subscription access when available.",
        "github-copilot" => "Use the GitHub device flow when available.",
        _ => "Use the provider auth flow when available.",
    }
}

fn auth_step(number: &'static str, title: &'static str, detail: &str) -> impl IntoElement {
    div()
        .flex()
        .gap_2()
        .items_start()
        .child(
            div()
                .size_5()
                .bg(theme::accent())
                .text_color(theme::app_bg())
                .flex()
                .items_center()
                .justify_center()
                .text_xs()
                .font_semibold()
                .child(number),
        )
        .child(
            div()
                .flex_1()
                .child(div().text_sm().font_semibold().child(title))
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::text_muted())
                        .child(detail.to_owned()),
                ),
        )
}

fn provider_logo(provider: &str, display_name: &str, size: gpui::Pixels) -> AnyElement {
    if let Some(path) = ui::provider_logo_path(provider) {
        svg()
            .path(path)
            .size(size)
            .text_color(theme::text())
            .into_any_element()
    } else {
        div()
            .size(size)
            .border_1()
            .border_color(theme::hairline())
            .flex()
            .items_center()
            .justify_center()
            .text_xs()
            .text_color(theme::text_muted())
            .child(ui::provider_initials(display_name))
            .into_any_element()
    }
}
