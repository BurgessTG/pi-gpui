use gpui::{
    AnyView, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, StatefulInteractiveElement as _, StyleRefinement, Styled as _,
    Window, div, px, svg,
};

use crate::app::PiDesktop;
use crate::components::workspace_tabs::{self, WorkspaceTabsView};
use crate::design::theme;
use crate::ui;

#[derive(Clone, PartialEq)]
pub struct StatusBarProps {
    pub tabs: Vec<workspace_tabs::WorkspaceTabInfo>,
    pub active_index: Option<usize>,
    pub previous_index: Option<usize>,
}

pub struct StatusBarView {
    app: Entity<PiDesktop>,
    props: StatusBarProps,
    tabs_view: Entity<WorkspaceTabsView>,
}

impl StatusBarView {
    pub fn new(app: Entity<PiDesktop>, props: StatusBarProps, cx: &mut Context<Self>) -> Self {
        let tabs_view = cx.new(|_| {
            WorkspaceTabsView::new(
                app.clone(),
                workspace_tabs::WorkspaceTabsProps {
                    tabs: props.tabs.clone(),
                    active_index: props.active_index,
                    previous_index: props.previous_index,
                },
            )
        });
        Self {
            app,
            props,
            tabs_view,
        }
    }

    pub fn sync(&mut self, props: StatusBarProps, cx: &mut Context<Self>) -> bool {
        if self.props == props {
            return false;
        }
        self.props = props;
        cx.notify();
        true
    }
}

impl Render for StatusBarView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::instrumentation::record_render("StatusBarView");
        let tabs_changed = self.tabs_view.update(cx, |view, cx| {
            view.sync(
                workspace_tabs::WorkspaceTabsProps {
                    tabs: self.props.tabs.clone(),
                    active_index: self.props.active_index,
                    previous_index: self.props.previous_index,
                },
                cx,
            )
        });

        let app = self.app.clone();
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
                    .on_click(cx.listener(move |_view, _, _window, cx| {
                        app.update(cx, |app, cx| app.open_landing(cx));
                    }))
                    .child(
                        svg()
                            .path(ui::logo_path())
                            .size_4()
                            .text_color(theme::text()),
                    ),
            )
            .child(div().flex_1().min_w_0().h_full().child({
                let tabs_view = AnyView::from(self.tabs_view.clone());
                if tabs_changed {
                    tabs_view.into_any_element()
                } else {
                    tabs_view
                        .cached(StyleRefinement::default().size_full())
                        .into_any_element()
                }
            }))
            .child(div().w(px(24.0)))
    }
}
