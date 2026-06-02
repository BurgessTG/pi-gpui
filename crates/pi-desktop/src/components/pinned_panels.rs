use std::collections::HashMap;

use gpui::{AnyElement, Entity, IntoElement as _, ParentElement as _, Styled as _, div, px};
use gpui_component::resizable::{ResizableState, h_resizable, resizable_panel, v_resizable};

use crate::components::chat_node;
use crate::design::theme;
use crate::workspace::pinning::PinnedAxis;
use crate::workspace::state::WorkspaceTab;

#[allow(clippy::too_many_arguments)]
pub(crate) fn pinned_panel_region(
    tab: &WorkspaceTab,
    pin_panel_state: Entity<ResizableState>,
    chat_node_views: &HashMap<usize, Entity<chat_node::ChatNodeView>>,
) -> AnyElement {
    let layout = tab.pinned_layout();
    let panels = layout.panels().iter().filter_map(|panel| {
        let node_id = panel.node_id();
        let node_view = chat_node_views.get(&node_id)?.clone();
        Some(resizable_panel().size(px(360.0)).child(node_view))
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
