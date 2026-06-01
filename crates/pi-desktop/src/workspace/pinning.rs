#![allow(dead_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PinnedAxis {
    #[default]
    Horizontal,
    Vertical,
}

impl PinnedAxis {
    pub fn toggled(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PinnedPanel {
    node_id: usize,
}

impl PinnedPanel {
    pub fn new(node_id: usize) -> Self {
        Self { node_id }
    }

    pub fn node_id(&self) -> usize {
        self.node_id
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PinnedLayout {
    axis: PinnedAxis,
    panels: Vec<PinnedPanel>,
    focused_node_id: Option<usize>,
}

impl PinnedLayout {
    pub fn axis(&self) -> PinnedAxis {
        self.axis
    }

    pub fn panels(&self) -> &[PinnedPanel] {
        &self.panels
    }

    pub fn focused_node_id(&self) -> Option<usize> {
        self.focused_node_id
    }

    pub fn is_empty(&self) -> bool {
        self.panels.is_empty()
    }

    pub fn is_pinned(&self, node_id: usize) -> bool {
        self.panels.iter().any(|panel| panel.node_id == node_id)
    }

    pub fn pin(&mut self, node_id: usize) -> bool {
        if self.is_pinned(node_id) {
            self.focused_node_id = Some(node_id);
            return false;
        }
        self.panels.push(PinnedPanel::new(node_id));
        self.focused_node_id = Some(node_id);
        true
    }

    pub fn unpin(&mut self, node_id: usize) -> bool {
        let Some(index) = self.panel_index(node_id) else {
            return false;
        };
        let removing_focused = self.focused_node_id == Some(node_id);
        self.panels.remove(index);
        if removing_focused || self.focused_node_id.is_none() {
            self.focused_node_id = next_focus_after_remove(&self.panels, index);
        }
        true
    }

    pub fn toggle(&mut self, node_id: usize) -> bool {
        if self.is_pinned(node_id) {
            self.unpin(node_id)
        } else {
            self.pin(node_id)
        }
    }

    pub fn focus(&mut self, node_id: usize) -> bool {
        if !self.is_pinned(node_id) {
            return false;
        }
        if self.focused_node_id == Some(node_id) {
            return false;
        }
        self.focused_node_id = Some(node_id);
        true
    }

    pub fn toggle_axis(&mut self) -> bool {
        if self.panels.len() < 2 {
            return false;
        }
        self.axis = self.axis.toggled();
        true
    }

    pub fn swap_focused_with_adjacent(&mut self) -> bool {
        if self.panels.len() < 2 {
            return false;
        }
        let focused = self
            .focused_node_id
            .and_then(|node_id| self.panel_index(node_id))
            .unwrap_or(0);
        let swap_with = if focused + 1 < self.panels.len() {
            focused + 1
        } else {
            focused.saturating_sub(1)
        };
        if focused == swap_with {
            return false;
        }
        self.panels.swap(focused, swap_with);
        self.focused_node_id = Some(self.panels[swap_with].node_id);
        true
    }

    pub fn retain_nodes(&mut self, live_node_ids: impl Fn(usize) -> bool) -> bool {
        let before = self.panels.len();
        self.panels.retain(|panel| live_node_ids(panel.node_id));
        let changed = self.panels.len() != before;
        if self
            .focused_node_id
            .is_some_and(|node_id| !self.is_pinned(node_id))
        {
            self.focused_node_id = self.panels.first().map(PinnedPanel::node_id);
        }
        changed
    }

    fn panel_index(&self, node_id: usize) -> Option<usize> {
        self.panels
            .iter()
            .position(|panel| panel.node_id == node_id)
    }
}

fn next_focus_after_remove(panels: &[PinnedPanel], removed_index: usize) -> Option<usize> {
    if panels.is_empty() {
        return None;
    }
    Some(panels[removed_index.min(panels.len() - 1)].node_id())
}

#[cfg(test)]
mod tests {
    use super::{PinnedAxis, PinnedLayout};

    #[test]
    fn pinning_preserves_order_and_focus() {
        let mut layout = PinnedLayout::default();

        assert!(layout.pin(10));
        assert!(layout.pin(20));
        assert!(!layout.pin(20));

        assert_eq!(layout.focused_node_id(), Some(20));
        assert_eq!(
            layout
                .panels()
                .iter()
                .map(|panel| panel.node_id())
                .collect::<Vec<_>>(),
            vec![10, 20]
        );
    }

    #[test]
    fn swap_prefers_next_panel_then_previous() {
        let mut layout = PinnedLayout::default();
        layout.pin(1);
        layout.pin(2);
        layout.pin(3);
        layout.focus(2);

        assert!(layout.swap_focused_with_adjacent());
        assert_eq!(
            layout
                .panels()
                .iter()
                .map(|panel| panel.node_id())
                .collect::<Vec<_>>(),
            vec![1, 3, 2]
        );
        assert_eq!(layout.focused_node_id(), Some(2));

        assert!(layout.swap_focused_with_adjacent());
        assert_eq!(
            layout
                .panels()
                .iter()
                .map(|panel| panel.node_id())
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn axis_toggle_requires_multiple_panels() {
        let mut layout = PinnedLayout::default();
        layout.pin(1);
        assert!(!layout.toggle_axis());
        assert_eq!(layout.axis(), PinnedAxis::Horizontal);

        layout.pin(2);
        assert!(layout.toggle_axis());
        assert_eq!(layout.axis(), PinnedAxis::Vertical);
    }

    #[test]
    fn unpin_moves_focus_to_neighbor() {
        let mut layout = PinnedLayout::default();
        layout.pin(1);
        layout.pin(2);
        layout.pin(3);
        layout.focus(2);

        assert!(layout.unpin(2));
        assert_eq!(layout.focused_node_id(), Some(3));
        assert!(!layout.is_pinned(2));
    }

    #[test]
    fn unpinning_non_focused_panel_preserves_focus() {
        let mut layout = PinnedLayout::default();
        layout.pin(1);
        layout.pin(2);
        layout.pin(3);
        layout.focus(2);

        assert!(layout.unpin(1));
        assert_eq!(layout.focused_node_id(), Some(2));
        assert!(!layout.is_pinned(1));
    }
}
