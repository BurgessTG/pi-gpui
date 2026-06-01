use std::time::Duration;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    Animation, AnimationExt as _, AnyElement, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, SharedString, Styled as _, div, px,
};
use gpui_component::animation::cubic_bezier;
use gpui_component::input::InputState;
use gpui_component::tag::Tag;
use gpui_component::{Sizable as _, Size, StyledExt as _};
use pi_bridge_types::{OAuthLoginMethod, ProviderAuthStatus};

use crate::app::{AuthFlow, PiDesktop, oauth_methods_for};
use crate::components::button::{PiButtonKind, pi_button};
use crate::components::input::pi_input;
use crate::components::provider_logo::{provider_hover_card, provider_icon_cell, provider_logo};
use crate::design::theme;

pub(crate) struct AuthSettingsState {
    pub providers: Vec<ProviderAuthStatus>,
    pub selected_provider: Option<String>,
    pub hover_card_provider: Option<String>,
    pub auth_flow: AuthFlow,
    pub pending: bool,
}

fn provider_card_animation() -> Animation {
    Animation::new(Duration::from_millis(180)).with_easing(cubic_bezier(0.32, 0.72, 0.0, 1.0))
}

pub(crate) fn auth_settings_content(
    state: AuthSettingsState,
    api_key_input: Entity<InputState>,
    view: Entity<PiDesktop>,
    cx: &mut gpui::App,
) -> AnyElement {
    let selected_status = state.selected_provider.as_ref().and_then(|provider| {
        state
            .providers
            .iter()
            .find(|status| status.provider == *provider)
            .cloned()
    });
    let hover_card = state.hover_card_provider.as_ref().and_then(|provider| {
        state
            .providers
            .iter()
            .enumerate()
            .find(|(_, status)| status.provider == *provider)
            .map(|(index, status)| (index, status.display_name.clone()))
    });
    let provider_count = state.providers.len();
    let rows = provider_count.saturating_add(4) / 5;
    let icon_window_height = px(16.0 + rows as f32 * 42.0 + rows.saturating_sub(1) as f32 * 8.0);
    let provider_icons = state
        .providers
        .into_iter()
        .map(|status| {
            provider_icon_cell(status, state.selected_provider.as_deref(), view.clone())
                .into_any_element()
        })
        .collect::<Vec<_>>();

    div()
        .w_full()
        .flex()
        .flex_col()
        .items_center()
        .gap_3()
        .child(
            div()
                .id("provider-icon-scroll")
                .w(px(292.0))
                .h(icon_window_height)
                .border_1()
                .border_color(theme::hairline())
                .p_2()
                .flex()
                .flex_wrap()
                .justify_center()
                .gap_2()
                .children(provider_icons)
                .when_some(hover_card, |this, (index, display_name)| {
                    this.child(provider_hover_card(&display_name, index))
                }),
        )
        .when_some(selected_status, |this, status| {
            this.child(provider_auth_flow(
                status,
                state.auth_flow,
                api_key_input,
                state.pending,
                view,
                cx,
            ))
        })
        .into_any_element()
}

fn provider_auth_flow(
    status: ProviderAuthStatus,
    auth_flow: AuthFlow,
    api_key_input: Entity<InputState>,
    pending: bool,
    view: Entity<PiDesktop>,
    cx: &mut gpui::App,
) -> AnyElement {
    let provider = status.provider.clone();
    let display_name = status.display_name.clone();
    let configured = status.configured;
    let oauth_methods = oauth_methods_for(&provider);
    let show_api_key = oauth_methods.is_empty() || auth_flow == AuthFlow::ApiKey;
    let show_auth_progress =
        pending && !configured && !oauth_methods.is_empty() && auth_flow == AuthFlow::Choose;
    let remove_view = view.clone();

    div()
        .w(px(292.0))
        .border_1()
        .border_color(theme::hairline())
        .bg(theme::surface())
        .relative()
        .overflow_hidden()
        .flex()
        .flex_col()
        .when(show_auth_progress, |this| {
            this.child(auth_verification_progress())
        })
        .child(
            div()
                .w_full()
                .p_3()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div().w_full().flex().flex_col().gap_1().child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(provider_logo(&provider, &display_name, px(28.0)))
                            .child(div().w(px(1.0)).h(px(34.0)).bg(theme::hairline()))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(div().font_semibold().line_clamp(2).child(display_name))
                                    .child(
                                        div()
                                            .w_full()
                                            .mt(px(-3.0))
                                            .flex()
                                            .justify_start()
                                            .child(provider_status_tag(configured)),
                                    ),
                            ),
                    ),
                )
                .when(configured, |this| {
                    this.child(
                        pi_button("remove-auth", "Remove auth", PiButtonKind::Danger, cx)
                            .w_full()
                            .on_click(move |_, _, cx| {
                                remove_view.update(cx, |view, cx| view.remove_selected_auth(cx));
                            }),
                    )
                })
                .when(
                    !configured && !oauth_methods.is_empty() && auth_flow == AuthFlow::Choose,
                    |this| this.child(subscription_auth_picker(oauth_methods, view.clone(), cx)),
                )
                .when(!configured && show_api_key, |this| {
                    this.child(api_key_auth_form(api_key_input, pending, view, cx))
                }),
        )
        .with_animation(
            SharedString::from(format!("provider-auth-card-{provider}")),
            provider_card_animation(),
            |this, delta| this.opacity(delta).mt(px(-18.0) * (1.0 - delta)),
        )
        .into_any_element()
}

fn auth_verification_progress() -> impl IntoElement {
    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .h(px(3.0))
        .overflow_hidden()
        .bg(theme::complement().opacity(0.18))
        .child(
            div()
                .absolute()
                .top_0()
                .left(px(-96.0))
                .h_full()
                .w(px(96.0))
                .bg(theme::complement())
                .with_animation(
                    "auth-verification-progress",
                    Animation::new(Duration::from_millis(1150)).repeat(),
                    |this, delta| this.left(px(-96.0 + (292.0 + 96.0) * delta)),
                ),
        )
}

fn provider_status_tag(configured: bool) -> impl IntoElement {
    let label = if configured {
        "Configured"
    } else {
        "Not configured"
    };
    let tag = if configured {
        Tag::success()
    } else {
        Tag::secondary()
    };

    tag.outline()
        .rounded(px(0.0))
        .with_size(Size::Small)
        .child(label)
}

pub(crate) fn settings_placeholder(message: impl Into<SharedString>) -> AnyElement {
    div()
        .w_full()
        .border_1()
        .border_color(theme::hairline())
        .bg(theme::app_bg())
        .p_3()
        .text_sm()
        .text_color(theme::text_muted())
        .child(message.into())
        .into_any_element()
}

fn subscription_auth_picker(
    methods: &[OAuthLoginMethod],
    view: Entity<PiDesktop>,
    cx: &mut gpui::App,
) -> AnyElement {
    let browser_view = view.clone();
    let device_view = view.clone();
    let api_key_view = view.clone();

    div()
        .w_full()
        .flex()
        .flex_col()
        .items_center()
        .gap_2()
        .when(methods.contains(&OAuthLoginMethod::Browser), |this| {
            this.child(
                pi_button("browser-auth", "Browser login", PiButtonKind::Secondary, cx)
                    .w_full()
                    .on_click(move |_, _, cx| {
                        browser_view.update(cx, |view, cx| {
                            view.start_oauth_login(OAuthLoginMethod::Browser, cx);
                        });
                    }),
            )
        })
        .when(methods.contains(&OAuthLoginMethod::DeviceCode), |this| {
            this.child(
                pi_button(
                    "device-auth",
                    "Device code login",
                    PiButtonKind::Secondary,
                    cx,
                )
                .w_full()
                .on_click(move |_, _, cx| {
                    device_view.update(cx, |view, cx| {
                        view.start_oauth_login(OAuthLoginMethod::DeviceCode, cx);
                    });
                }),
            )
        })
        .child(
            pi_button("api-key-method", "API key", PiButtonKind::Ghost, cx)
                .w_full()
                .on_click(move |_, _, cx| {
                    api_key_view.update(cx, |view, cx| view.show_api_key_flow(cx));
                }),
        )
        .into_any_element()
}

fn api_key_auth_form(
    api_key_input: Entity<InputState>,
    pending: bool,
    view: Entity<PiDesktop>,
    cx: &mut gpui::App,
) -> AnyElement {
    let save_view = view.clone();

    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(pi_input(&api_key_input).h(px(38.0)))
        .child(
            div().flex().gap_2().child(
                pi_button("submit-key", "Submit", PiButtonKind::Primary, cx)
                    .loading(pending)
                    .on_click(move |_, _, cx| {
                        save_view.update(cx, |view, cx| view.save_api_key(true, cx));
                    }),
            ),
        )
        .into_any_element()
}
