use std::collections::HashMap;

use gpui::{
    div, px, AnyElement, Context, Entity, IntoElement as _, ParentElement as _, Styled as _, Window,
};
use gpui_component::input::InputState;
use gpui_component::resizable::{h_resizable, resizable_panel, v_resizable, ResizableState};

use crate::app::PiDesktop;

use crate::components::chat_node;
use crate::design::theme;
use crate::workspace::pinning::PinnedAxis;
use crate::workspace::state::WorkspaceTab;

#[allow(clippy::too_many_arguments)]
pub(crate) fn pinned_panel_region(
    tab: &WorkspaceTab,
    workspace_id: usize,
    working_node: Option<(usize, usize)>,
    pin_panel_state: Entity<ResizableState>,
    chat_inputs: &HashMap<(usize, usize), Entity<InputState>>,
    title_inputs: &HashMap<(usize, usize), Entity<InputState>>,
    chat_body_views: &HashMap<(usize, usize), Entity<chat_node::ChatBodyView>>,
    editing_title: Option<(usize, usize)>,
    window: &mut Window,
    cx: &mut Context<PiDesktop>,
) -> AnyElement {
    let layout = tab.pinned_layout();
    let focused_node_id = layout.focused_node_id();
    let panels = layout.panels().iter().filter_map(|panel| {
        let node_id = panel.node_id();
        let node = tab
            .canvas()
            .nodes()
            .iter()
            .find(|node| node.id() == node_id)?;
        let key = (workspace_id, node_id);
        let input = chat_inputs.get(&key)?.clone();
        let title_input = title_inputs.get(&key)?.clone();
        let body_view = chat_body_views.get(&key)?.clone();
        Some(
            resizable_panel()
                .size(px(360.0))
                .child(chat_node::pinned_chat_node_panel(
                    workspace_id,
                    node,
                    working_node == Some(key),
                    input,
                    title_input,
                    body_view,
                    editing_title == Some(key),
                    focused_node_id == Some(node_id),
                    window,
                    cx,
                )),
        )
    });

    let group = match layout.axis() {
        PinnedAxis::Horizontal => h_resizable("pinned-panel-region")
            .with_state(&pin_panel_state)
            .children(panels),
        PinnedAxis::Vertical => v_resizable("pinned-panel-region")
            .with_state(&pin_panel_state)
            .children(panels),
    };

    div()
        .size_full()
        .bg(theme::app_bg())
        .child(group)
        .into_any_element()
}
