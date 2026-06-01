use std::path::{Path, PathBuf};

use crate::workspace::{
    canvas::{
        CanvasDrawingTool, CanvasState, SessionNodeMetadata, SessionNodePrimitive, WorldPoint,
        WorldSize,
    },
    pinning::PinnedLayout,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkspaceKind {
    NamedBlank,
    Folder,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorkspaceTab {
    id: usize,
    title: String,
    root: Option<PathBuf>,
    kind: WorkspaceKind,
    canvas: CanvasState,
    pinned_layout: PinnedLayout,
}

impl WorkspaceTab {
    pub fn id(&self) -> usize {
        self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn canvas(&self) -> &CanvasState {
        &self.canvas
    }

    pub fn pinned_layout(&self) -> &PinnedLayout {
        &self.pinned_layout
    }
}

#[derive(Debug, Default)]
pub struct WorkspaceState {
    tabs: Vec<WorkspaceTab>,
    active_index: Option<usize>,
    next_workspace_id: usize,
}

impl WorkspaceState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tabs(&self) -> &[WorkspaceTab] {
        &self.tabs
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub fn active_index(&self) -> Option<usize> {
        self.active_index
    }

    pub fn active_tab(&self) -> Option<&WorkspaceTab> {
        self.active_index.and_then(|index| self.tabs.get(index))
    }

    pub fn tab(&self, index: usize) -> Option<&WorkspaceTab> {
        self.tabs.get(index)
    }

    pub fn tab_id(&self, index: usize) -> Option<usize> {
        self.tab(index).map(WorkspaceTab::id)
    }

    pub fn index_for_id(&self, workspace_id: usize) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.id == workspace_id)
    }

    pub fn active_canvas_mut(&mut self) -> Option<&mut CanvasState> {
        self.active_index
            .and_then(|index| self.tabs.get_mut(index))
            .map(|tab| &mut tab.canvas)
    }

    pub fn note_context_position(&mut self, screen_position: WorldPoint) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.note_context_position(screen_position);
        true
    }

    pub fn zoom_active_canvas_in(&mut self) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.zoom_in();
        true
    }

    pub fn zoom_active_canvas_out(&mut self) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.zoom_out();
        true
    }

    pub fn zoom_active_canvas_by_at(&mut self, factor: f32, screen_position: WorldPoint) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.zoom_by_at(factor, screen_position);
        true
    }

    pub fn start_active_canvas_pan(&mut self, screen_position: WorldPoint) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.start_pan(screen_position);
        true
    }

    pub fn update_active_canvas_pan(&mut self, screen_position: WorldPoint) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.update_pan(screen_position)
    }

    pub fn end_active_canvas_pan(&mut self) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.end_pan()
    }

    pub fn start_active_minimap_pan(
        &mut self,
        local_position: WorldPoint,
        minimap_size: WorldSize,
        viewport_size: WorldSize,
    ) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.start_minimap_pan(local_position, minimap_size, viewport_size)
    }

    pub fn update_active_minimap_pan(
        &mut self,
        local_position: WorldPoint,
        minimap_size: WorldSize,
    ) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.update_minimap_pan(local_position, minimap_size)
    }

    pub fn end_active_minimap_pan(&mut self) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.end_minimap_pan()
    }

    pub fn start_active_node_drag(&mut self, node_id: usize, screen_position: WorldPoint) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.start_node_drag(node_id, screen_position)
    }

    pub fn update_active_node_drag(
        &mut self,
        screen_position: WorldPoint,
        snap_to_grid: bool,
    ) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.update_node_drag(screen_position, snap_to_grid)
    }

    pub fn end_active_node_drag(&mut self) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.end_node_drag()
    }

    pub fn start_active_node_resize(
        &mut self,
        node_id: usize,
        screen_position: WorldPoint,
    ) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.start_node_resize(node_id, screen_position)
    }

    pub fn update_active_node_resize(&mut self, screen_position: WorldPoint) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.update_node_resize(screen_position)
    }

    pub fn end_active_node_resize(&mut self) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.end_node_resize()
    }

    pub fn select_drawing(&mut self, workspace_index: usize, drawing_index: usize) -> bool {
        let Some(tab) = self.tabs.get_mut(workspace_index) else {
            return false;
        };
        tab.canvas.select_drawing(drawing_index)
    }

    pub fn clear_active_drawing_selection(&mut self) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.clear_drawing_selection()
    }

    pub fn update_text_box_text(
        &mut self,
        workspace_index: usize,
        drawing_index: usize,
        text: String,
    ) -> bool {
        let Some(tab) = self.tabs.get_mut(workspace_index) else {
            return false;
        };
        tab.canvas.update_text_box_text(drawing_index, text)
    }

    pub fn start_active_drawing_drag(
        &mut self,
        drawing_index: usize,
        screen_position: WorldPoint,
    ) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.start_drawing_drag(drawing_index, screen_position)
    }

    pub fn start_active_drawing_drag_at(&mut self, screen_position: WorldPoint) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.start_drawing_drag_at(screen_position)
    }

    pub fn update_active_drawing_drag(
        &mut self,
        screen_position: WorldPoint,
        snap_to_grid: bool,
    ) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.update_drawing_drag(screen_position, snap_to_grid)
    }

    pub fn end_active_drawing_drag(&mut self) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.end_drawing_drag()
    }

    pub fn start_active_drawing(
        &mut self,
        tool: CanvasDrawingTool,
        screen_position: WorldPoint,
        snap_to_grid: bool,
    ) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.start_drawing(tool, screen_position, snap_to_grid)
    }

    pub fn update_active_drawing(
        &mut self,
        screen_position: WorldPoint,
        snap_to_grid: bool,
    ) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.update_drawing(screen_position, snap_to_grid)
    }

    pub fn end_active_drawing(&mut self) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.end_drawing()
    }

    pub fn undo_active_drawing(&mut self) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.undo_drawing()
    }

    pub fn redo_active_drawing(&mut self) -> bool {
        let Some(canvas) = self.active_canvas_mut() else {
            return false;
        };
        canvas.redo_drawing()
    }

    pub fn active_canvas_can_undo_drawing(&self) -> bool {
        self.active_tab()
            .is_some_and(|tab| tab.canvas().can_undo_drawing())
    }

    pub fn active_canvas_can_redo_drawing(&self) -> bool {
        self.active_tab()
            .is_some_and(|tab| tab.canvas().can_redo_drawing())
    }

    pub fn add_session_node_to_active_canvas(
        &mut self,
        primitive: SessionNodePrimitive,
        metadata: SessionNodeMetadata,
        snap_to_grid: bool,
    ) -> Option<usize> {
        self.active_canvas_mut()
            .map(|canvas| canvas.add_session_node(primitive, metadata, snap_to_grid))
    }

    pub fn update_session_node_metadata(
        &mut self,
        workspace_index: usize,
        node_id: usize,
        metadata: SessionNodeMetadata,
    ) -> bool {
        let Some(tab) = self.tabs.get_mut(workspace_index) else {
            return false;
        };
        tab.canvas.update_session_node_metadata(node_id, metadata)
    }

    pub fn remove_session_node(&mut self, workspace_index: usize, node_id: usize) -> bool {
        let Some(tab) = self.tabs.get_mut(workspace_index) else {
            return false;
        };
        let removed = tab.canvas.remove_session_node(node_id);
        if removed {
            tab.pinned_layout.unpin(node_id);
        }
        removed
    }

    pub fn toggle_session_node_pin(&mut self, workspace_index: usize, node_id: usize) -> bool {
        let Some(tab) = self.tabs.get_mut(workspace_index) else {
            return false;
        };
        if !tab.canvas.nodes().iter().any(|node| node.id() == node_id) {
            return false;
        }
        tab.pinned_layout.toggle(node_id)
    }

    pub fn focus_pinned_node(&mut self, workspace_index: usize, node_id: usize) -> bool {
        let Some(tab) = self.tabs.get_mut(workspace_index) else {
            return false;
        };
        tab.pinned_layout.focus(node_id)
    }

    pub fn swap_focused_pinned_panel(&mut self, workspace_index: usize) -> bool {
        let Some(tab) = self.tabs.get_mut(workspace_index) else {
            return false;
        };
        tab.pinned_layout.swap_focused_with_adjacent()
    }

    pub fn toggle_pinned_panel_axis(&mut self, workspace_index: usize) -> bool {
        let Some(tab) = self.tabs.get_mut(workspace_index) else {
            return false;
        };
        tab.pinned_layout.toggle_axis()
    }

    pub fn sync_session_metadata(&mut self, metadata: &SessionNodeMetadata) -> bool {
        let mut changed = false;
        for tab in &mut self.tabs {
            changed |= tab.canvas.sync_session_metadata(metadata);
        }
        changed
    }

    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.tabs.len() {
            return false;
        }
        self.active_index = Some(index);
        true
    }

    pub fn close(&mut self, index: usize) -> Option<WorkspaceTab> {
        if index >= self.tabs.len() {
            return None;
        }

        let removed = self.tabs.remove(index);
        self.active_index = match (self.tabs.is_empty(), self.active_index) {
            (true, _) => None,
            (false, Some(active_index)) if active_index == index => {
                Some(index.min(self.tabs.len() - 1))
            }
            (false, Some(active_index)) if active_index > index => Some(active_index - 1),
            (false, Some(active_index)) => Some(active_index),
            (false, None) => Some(0),
        };

        Some(removed)
    }

    pub fn add_named_blank(&mut self, title: impl Into<String>) -> Option<usize> {
        let title = title.into();
        let title = title.trim();
        if title.is_empty() {
            return None;
        }

        let index = self.push_tab(WorkspaceTab {
            id: 0,
            title: title.to_owned(),
            root: None,
            kind: WorkspaceKind::NamedBlank,
            canvas: CanvasState::new(),
            pinned_layout: PinnedLayout::default(),
        });
        Some(index)
    }

    pub fn add_folder(&mut self, root: PathBuf) -> usize {
        if let Some(index) = self
            .tabs
            .iter()
            .position(|workspace| workspace.root.as_deref() == Some(root.as_path()))
        {
            self.active_index = Some(index);
            return index;
        }

        self.push_tab(WorkspaceTab {
            id: 0,
            title: folder_title(&root),
            root: Some(root),
            kind: WorkspaceKind::Folder,
            canvas: CanvasState::new(),
            pinned_layout: PinnedLayout::default(),
        })
    }

    fn push_tab(&mut self, mut tab: WorkspaceTab) -> usize {
        tab.id = self.next_workspace_id;
        self.next_workspace_id = self.next_workspace_id.saturating_add(1);
        self.tabs.push(tab);
        let index = self.tabs.len() - 1;
        self.active_index = Some(index);
        index
    }
}

fn folder_title(root: &Path) -> String {
    root.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| root.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::{WorkspaceKind, WorkspaceState};
    use std::path::PathBuf;

    #[test]
    fn named_blank_workspaces_trim_titles_and_become_active() {
        let mut state = WorkspaceState::new();

        assert_eq!(state.add_named_blank("   "), None);
        let index = state.add_named_blank("  Design Lab  ");

        assert_eq!(index, Some(0));
        assert_eq!(state.active_index(), Some(0));
        assert_eq!(
            state.active_tab().map(|tab| tab.title()),
            Some("Design Lab")
        );
        assert_eq!(
            state.active_tab().map(|tab| &tab.kind),
            Some(&WorkspaceKind::NamedBlank)
        );
    }

    #[test]
    fn folder_workspaces_deduplicate_by_root() {
        let mut state = WorkspaceState::new();
        let root = PathBuf::from("/tmp/pi-workspace");

        assert_eq!(state.add_folder(root.clone()), 0);
        assert_eq!(state.add_folder(root), 0);

        assert_eq!(state.tabs().len(), 1);
        assert_eq!(state.active_index(), Some(0));
        assert_eq!(
            state.active_tab().map(|tab| tab.title()),
            Some("pi-workspace")
        );
        assert_eq!(
            state.active_tab().map(|tab| &tab.kind),
            Some(&WorkspaceKind::Folder)
        );
    }

    #[test]
    fn closing_active_workspace_selects_the_next_available_tab() {
        let mut state = WorkspaceState::new();
        state.add_named_blank("First");
        state.add_named_blank("Second");
        state.add_named_blank("Third");
        assert!(state.select(1));

        let removed = state.close(1);

        assert_eq!(removed.as_ref().map(|tab| tab.title()), Some("Second"));
        assert_eq!(state.tabs().len(), 2);
        assert_eq!(state.active_index(), Some(1));
        assert_eq!(state.active_tab().map(|tab| tab.title()), Some("Third"));
    }

    #[test]
    fn closing_workspace_before_active_tab_keeps_same_workspace_active() {
        let mut state = WorkspaceState::new();
        state.add_named_blank("First");
        state.add_named_blank("Second");
        state.add_named_blank("Third");

        let removed = state.close(0);

        assert_eq!(removed.as_ref().map(|tab| tab.title()), Some("First"));
        assert_eq!(state.active_index(), Some(1));
        assert_eq!(state.active_tab().map(|tab| tab.title()), Some("Third"));
    }

    #[test]
    fn closing_last_workspace_clears_active_tab() {
        let mut state = WorkspaceState::new();
        state.add_named_blank("Only");

        assert_eq!(state.close(0).as_ref().map(|tab| tab.title()), Some("Only"));

        assert!(state.tabs().is_empty());
        assert_eq!(state.active_index(), None);
        assert!(state.close(0).is_none());
    }

    #[test]
    fn workspace_ids_stay_stable_after_tab_close() {
        let mut state = WorkspaceState::new();
        state.add_named_blank("First");
        state.add_named_blank("Second");
        state.add_named_blank("Third");
        let ids = state.tabs().iter().map(|tab| tab.id()).collect::<Vec<_>>();
        assert_eq!(ids.len(), 3);
        let second_id = ids[1];
        let third_id = ids[2];

        state.close(0);

        assert_eq!(state.tab_id(0), Some(second_id));
        assert_eq!(state.tab_id(1), Some(third_id));
        assert_eq!(state.index_for_id(second_id), Some(0));
        assert_eq!(state.index_for_id(third_id), Some(1));
    }
}
