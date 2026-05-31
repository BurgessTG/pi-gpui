use gpui::{App, ElementId, SharedString, px};
use gpui_component::IconName;
use gpui_component::button::{Button, ButtonCustomVariant, ButtonVariants as _};

use crate::design::theme;

#[derive(Clone, Copy)]
pub enum PiButtonKind {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

pub fn pi_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    kind: PiButtonKind,
    cx: &App,
) -> Button {
    base_button(id, kind, cx).label(label)
}

pub fn pi_icon_button(
    id: impl Into<ElementId>,
    icon: IconName,
    kind: PiButtonKind,
    cx: &App,
) -> Button {
    base_button(id, kind, cx).icon(icon).compact()
}

fn base_button(id: impl Into<ElementId>, kind: PiButtonKind, cx: &App) -> Button {
    let variant = match kind {
        PiButtonKind::Primary => ButtonCustomVariant::new(cx)
            .color(theme::accent())
            .foreground(theme::app_bg())
            .border(theme::accent())
            .hover(theme::text_muted())
            .active(theme::text()),
        PiButtonKind::Secondary => ButtonCustomVariant::new(cx)
            .color(theme::surface())
            .foreground(theme::text())
            .border(theme::hairline())
            .hover(theme::surface_hover())
            .active(theme::surface_selected()),
        PiButtonKind::Ghost => ButtonCustomVariant::new(cx)
            .color(gpui::transparent_black())
            .foreground(theme::text_muted())
            .border(gpui::transparent_black())
            .hover(theme::surface_hover())
            .active(theme::surface_selected()),
        PiButtonKind::Danger => ButtonCustomVariant::new(cx)
            .color(theme::danger_soft())
            .foreground(theme::danger())
            .border(theme::danger_soft())
            .hover(theme::surface_hover())
            .active(theme::surface_selected()),
    };

    Button::new(id).custom(variant).rounded(px(0.0))
}
