use gpui::{KeyBinding, actions};

use super::*;

pub(crate) const WORKSPACE_KEY_CONTEXT: &str = "PiWorkspace";

actions!(pi_pins, [SwapPinnedPanel, TogglePinnedPanelAxis]);

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-j", SwapPinnedPanel, Some(WORKSPACE_KEY_CONTEXT)),
        KeyBinding::new("ctrl-k", TogglePinnedPanelAxis, Some(WORKSPACE_KEY_CONTEXT)),
    ]);
}

impl PiDesktop {
    pub(crate) fn toggle_session_node_pin(
        &mut self,
        workspace_id: usize,
        node_id: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace_index) = self.workspace_index_for_id(workspace_id) else {
            self.status = "Session node is no longer available.".into();
            cx.notify();
            return;
        };

        if !self
            .workspace_state
            .toggle_session_node_pin(workspace_index, node_id)
        {
            self.status = "Session node is no longer available.".into();
            cx.notify();
            return;
        }

        let pinned = self
            .workspace_state
            .tab(workspace_index)
            .is_some_and(|tab| tab.pinned_layout().is_pinned(node_id));
        self.status = if pinned {
            format!("Pinned session node #{node_id}.").into()
        } else {
            format!("Unpinned session node #{node_id}.").into()
        };
        cx.notify();
    }

    pub(crate) fn focus_pinned_node(
        &mut self,
        workspace_id: usize,
        node_id: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace_index) = self.workspace_index_for_id(workspace_id) else {
            return;
        };
        if self
            .workspace_state
            .focus_pinned_node(workspace_index, node_id)
        {
            self.status = format!("Focused pinned session node #{node_id}.").into();
            cx.notify();
        }
    }

    #[allow(dead_code)]
    pub(crate) fn swap_focused_pinned_panel(&mut self, cx: &mut Context<Self>) {
        let Some(workspace_index) = self.workspace_state.active_index() else {
            return;
        };
        if self
            .workspace_state
            .swap_focused_pinned_panel(workspace_index)
        {
            self.status = "Swapped pinned panel with its adjacent panel.".into();
            cx.notify();
        }
    }

    #[allow(dead_code)]
    pub(crate) fn toggle_focused_pinned_panel_axis(&mut self, cx: &mut Context<Self>) {
        let Some(workspace_index) = self.workspace_state.active_index() else {
            return;
        };
        if self
            .workspace_state
            .toggle_pinned_panel_axis(workspace_index)
        {
            self.status = "Toggled pinned panel orientation.".into();
            cx.notify();
        }
    }
}
