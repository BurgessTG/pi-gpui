use gpui::{
    App, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use gpui_component::button::{Toggle, ToggleVariants as _};
use gpui_component::{Icon, IconName, Sizable as _, Size, tooltip::Tooltip};

use crate::app::PiDesktop;
use crate::design::theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BottomDockProps {
    pub settings_selected: bool,
    pub snap_to_grid: bool,
    pub drawing_tools_visible: bool,
}

pub struct BottomDockView {
    app: Entity<PiDesktop>,
    props: BottomDockProps,
}

impl BottomDockView {
    pub fn new(app: Entity<PiDesktop>, props: BottomDockProps) -> Self {
        Self { app, props }
    }

    pub fn sync(&mut self, props: BottomDockProps, cx: &mut Context<Self>) -> bool {
        if self.props == props {
            return false;
        }
        self.props = props;
        cx.notify();
        true
    }
}

impl Render for BottomDockView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        crate::instrumentation::record_render("BottomDockView");
        let app = self.app.clone();
        let props = self.props;

        div()
            .size_full()
            .bg(theme::app_bg())
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
                                    .checked(props.snap_to_grid)
                                    .text_color(theme::text())
                                    .icon(
                                        Icon::new(IconName::Frame)
                                            .with_size(Size::Size(px(18.0)))
                                            .text_color(theme::text()),
                                    )
                                    .on_click(app_listener(
                                        app.clone(),
                                        |view, checked: &bool, _, cx| {
                                            view.set_snap_to_grid(*checked, cx);
                                        },
                                    )),
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
                                    .checked(props.drawing_tools_visible)
                                    .text_color(theme::text())
                                    .icon(
                                        Icon::new(IconName::Palette)
                                            .with_size(Size::Size(px(18.0)))
                                            .text_color(theme::text()),
                                    )
                                    .on_click(app_listener(
                                        app.clone(),
                                        |view, checked: &bool, _, cx| {
                                            view.set_drawing_tools_visible(*checked, cx);
                                        },
                                    )),
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
                            .checked(props.settings_selected)
                            .text_color(theme::text())
                            .icon(
                                Icon::new(IconName::Settings)
                                    .with_size(Size::Size(px(18.0)))
                                    .text_color(theme::text()),
                            )
                            .on_click(app_listener(app, |view, _: &bool, _, cx| {
                                view.select_bottom_dock_item(0, cx);
                            })),
                    ),
            )
    }
}

fn app_listener<E: ?Sized>(
    app: Entity<PiDesktop>,
    f: impl Fn(&mut PiDesktop, &E, &mut Window, &mut Context<PiDesktop>) + 'static,
) -> impl Fn(&E, &mut Window, &mut App) + 'static {
    move |event, window, cx| {
        app.update(cx, |view, cx| f(view, event, window, cx));
    }
}
