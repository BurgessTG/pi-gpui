use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, Entity, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, div, px,
};
use gpui_component::StyledExt as _;

use crate::app::PiDesktop;
use crate::design::theme::{self, AppFont, AppearanceSettings, ThemePreset};

#[derive(Clone, Copy)]
pub(crate) struct ThemeSettingsState {
    pub(crate) appearance: AppearanceSettings,
}

pub(crate) fn theme_settings_content(
    state: ThemeSettingsState,
    view: Entity<PiDesktop>,
    _cx: &mut gpui::App,
) -> AnyElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_3()
        .child(section_header(
            "Theme preset",
            "Start with Pi, neutral dark, or light palettes.",
        ))
        .child(div().w_full().flex().flex_col().gap_2().children(
            ThemePreset::ALL.map(|preset| {
                preset_card(preset, state.appearance, view.clone()).into_any_element()
            }),
        ))
        .child(section_header(
            "App font",
            "Choose the base font used by Pi controls and text.",
        ))
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap_2()
                .children(AppFont::ALL.map(|font| {
                    font_card(font, state.appearance, view.clone()).into_any_element()
                })),
        )
        .into_any_element()
}

fn section_header(title: &'static str, description: &'static str) -> impl gpui::IntoElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_sm().font_semibold().child(title))
        .child(
            div()
                .text_xs()
                .text_color(theme::text_muted())
                .child(description),
        )
}

fn preset_card(
    preset: ThemePreset,
    appearance: AppearanceSettings,
    view: Entity<PiDesktop>,
) -> impl gpui::IntoElement {
    let selected = appearance.preset == preset;
    let next = appearance.with_preset(preset);
    let palette = preset.palette();
    let status = appearance_status(next);

    div()
        .id(SharedString::from(format!("theme-preset-{}", preset.id())))
        .w_full()
        .border_1()
        .border_color(if selected {
            theme::accent()
        } else {
            theme::hairline()
        })
        .bg(if selected {
            theme::surface_selected()
        } else {
            theme::app_bg()
        })
        .p_3()
        .cursor_pointer()
        .hover(|style| style.bg(theme::surface_hover()))
        .on_click(move |_, window, cx| {
            theme::set_appearance(next);
            theme::apply_component_theme_for_window(window, cx);
            view.update(cx, |view, cx| {
                view.set_appearance(next, status.clone(), cx);
            });
        })
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(58.0))
                .h(px(38.0))
                .border_1()
                .border_color(palette.hairline)
                .bg(palette.app_bg)
                .flex()
                .items_end()
                .p_1()
                .gap_1()
                .child(div().w(px(12.0)).h(px(26.0)).bg(palette.surface))
                .child(div().w(px(12.0)).h(px(18.0)).bg(palette.surface_hover))
                .child(div().w(px(12.0)).h(px(30.0)).bg(palette.accent)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(div().text_sm().font_semibold().child(preset.label()))
                        .when(selected, |this| {
                            this.child(div().text_xs().text_color(theme::accent()).child("Active"))
                        }),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::text_muted())
                        .child(preset.description()),
                ),
        )
}

fn font_card(
    font: AppFont,
    appearance: AppearanceSettings,
    view: Entity<PiDesktop>,
) -> impl gpui::IntoElement {
    let selected = appearance.font == font;
    let next = appearance.with_font(font);
    let status = appearance_status(next);

    div()
        .id(SharedString::from(format!("theme-font-{}", font.id())))
        .w_full()
        .border_1()
        .border_color(if selected {
            theme::accent()
        } else {
            theme::hairline()
        })
        .bg(if selected {
            theme::surface_selected()
        } else {
            theme::app_bg()
        })
        .p_3()
        .cursor_pointer()
        .hover(|style| style.bg(theme::surface_hover()))
        .on_click(move |_, window, cx| {
            theme::set_appearance(next);
            theme::apply_component_theme_for_window(window, cx);
            view.update(cx, |view, cx| {
                view.set_appearance(next, status.clone(), cx);
            });
        })
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(div().text_sm().font_semibold().child(font.label()))
                .when(selected, |this| {
                    this.child(div().text_xs().text_color(theme::accent()).child("Active"))
                }),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme::text_muted())
                .child(font.description()),
        )
        .child(
            div()
                .w_full()
                .border_1()
                .border_color(theme::hairline())
                .bg(theme::surface())
                .p_2()
                .text_sm()
                .font_family(font.family())
                .text_color(theme::text())
                .child(font.preview()),
        )
}

fn appearance_status(settings: AppearanceSettings) -> String {
    format!(
        "Theme set to {} with {} font.",
        settings.preset.label(),
        settings.font.label()
    )
}
