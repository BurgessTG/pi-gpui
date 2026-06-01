use gpui::{
    Context, InteractiveElement as _, IntoElement, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use gpui_component::button::{Toggle, ToggleVariants as _};
use gpui_component::{Icon, IconName, Sizable as _, Size, tooltip::Tooltip};

use crate::app::PiDesktop;
use crate::design::theme;

pub fn bottom_dock(
    settings_selected: bool,
    snap_to_grid: bool,
    drawing_tools_visible: bool,
    cx: &mut Context<PiDesktop>,
) -> impl IntoElement {
    div()
        .bg(theme::app_bg())
        .h(px(28.0))
        .px_2()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .id("bottom-dock-snap-grid-tooltip")
                        .tooltip(|window, cx| Tooltip::new("Snap grid").build(window, cx))
                        .child(
                            Toggle::new("bottom-dock-snap-grid")
                                .ghost()
                                .small()
                                .checked(snap_to_grid)
                                .text_color(theme::text())
                                .icon(
                                    Icon::new(IconName::Frame)
                                        .with_size(Size::Size(px(18.0)))
                                        .text_color(theme::text()),
                                )
                                .on_click(cx.listener(|view, checked: &bool, _, cx| {
                                    view.set_snap_to_grid(*checked, cx);
                                })),
                        ),
                )
                .child(
                    div()
                        .id("bottom-dock-drawing-tools-tooltip")
                        .tooltip(|window, cx| Tooltip::new("Drawing tools").build(window, cx))
                        .child(
                            Toggle::new("bottom-dock-drawing-tools")
                                .ghost()
                                .small()
                                .checked(drawing_tools_visible)
                                .text_color(theme::text())
                                .icon(
                                    Icon::new(IconName::Palette)
                                        .with_size(Size::Size(px(18.0)))
                                        .text_color(theme::text()),
                                )
                                .on_click(cx.listener(|view, checked: &bool, _, cx| {
                                    view.set_drawing_tools_visible(*checked, cx);
                                })),
                        ),
                ),
        )
        .child(
            div()
                .id("bottom-dock-settings-tooltip")
                .tooltip(|window, cx| Tooltip::new("Settings").build(window, cx))
                .child(
                    Toggle::new("bottom-dock-settings")
                        .ghost()
                        .small()
                        .checked(settings_selected)
                        .text_color(theme::text())
                        .icon(
                            Icon::new(IconName::Settings)
                                .with_size(Size::Size(px(18.0)))
                                .text_color(theme::text()),
                        )
                        .on_click(cx.listener(|view, _: &bool, _, cx| {
                            view.select_bottom_dock_item(0, cx);
                        })),
                ),
        )
}
