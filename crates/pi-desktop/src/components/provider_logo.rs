use gpui::{
    AnyElement, Entity, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, div, px, svg,
};
use pi_bridge_types::ProviderAuthStatus;

use crate::app::PiDesktop;
use crate::design::theme;
use crate::ui;

pub fn provider_icon_cell(
    status: ProviderAuthStatus,
    selected_provider: Option<&str>,
    view: Entity<PiDesktop>,
) -> impl IntoElement {
    let selected = selected_provider == Some(status.provider.as_str());
    let click_provider = status.provider.clone();
    let hover_provider = status.provider.clone();
    let hover_view = view.clone();

    div()
        .id(SharedString::from(format!("provider-{}", status.provider)))
        .relative()
        .size(px(42.0))
        .border_1()
        .border_color(if status.configured {
            theme::success()
        } else if selected {
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
            let provider = click_provider.clone();
            view.update(cx, |view, cx| view.select_provider(provider, cx));
        })
        .on_hover(move |hovered, _, cx| {
            let provider = hover_provider.clone();
            if *hovered {
                hover_view.update(cx, |view, cx| view.start_provider_hover(provider, cx));
            } else {
                hover_view.update(cx, |view, cx| view.end_provider_hover(&provider, cx));
            }
        })
        .child(provider_logo(
            &status.provider,
            &status.display_name,
            px(24.0),
        ))
}

pub fn provider_logo(provider: &str, display_name: &str, size: gpui::Pixels) -> AnyElement {
    if matches!(provider, "opencode" | "opencode-go") {
        return opencode_logo(size);
    }

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

pub fn provider_hover_card(display_name: &str, index: usize) -> impl IntoElement {
    let col = (index % 5) as f32;
    let row = (index / 5) as f32;
    let card_width = (display_name.chars().count() as f32 * 7.0 + 22.0).clamp(48.0, 206.0);
    let cell_center_x = 25.0 + col * 50.0 + 21.0;
    let left = (cell_center_x - card_width / 2.0).clamp(8.0, 292.0 - card_width - 8.0);
    let cell_top = px(8.0 + row * 50.0);
    let top = if row == 0.0 {
        cell_top + px(46.0)
    } else {
        cell_top - px(36.0)
    };

    div()
        .absolute()
        .top(top)
        .left(px(left))
        .w(px(card_width))
        .occlude()
        .border_1()
        .border_color(theme::hairline())
        .bg(theme::app_bg())
        .p_2()
        .text_xs()
        .line_height(px(16.0))
        .text_center()
        .text_color(theme::text())
        .child(display_name.to_owned())
}

fn opencode_logo(size: gpui::Pixels) -> AnyElement {
    div()
        .relative()
        .size(size)
        .child(
            div()
                .absolute()
                .left_0()
                .top_0()
                .size_full()
                .bg(gpui::rgb(0x656363)),
        )
        .child(
            div()
                .absolute()
                .left(size * 0.25)
                .top(size * 0.20)
                .w(size * 0.50)
                .h(size * 0.40)
                .bg(theme::surface()),
        )
        .child(
            div()
                .absolute()
                .left(size * 0.25)
                .top(size * 0.40)
                .w(size * 0.50)
                .h(size * 0.40)
                .bg(gpui::rgb(0xcfcecd)),
        )
        .into_any_element()
}
