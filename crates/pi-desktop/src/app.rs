use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    Animation, AnimationExt as _, AnyElement, AnyView, App, AppContext as _, Bounds, Context,
    Entity, FocusHandle, Focusable, InteractiveElement as _, IntoElement, MouseDownEvent,
    ParentElement as _, Pixels, Render, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled as _, Subscription, Timer, Window, canvas, div, fill, point, px, size,
};
use gpui_component::animation::cubic_bezier;
use gpui_component::input::{InputEvent, InputState};
use gpui_component::resizable::{ResizableState, h_resizable, resizable_panel};
use gpui_component::setting::{SettingGroup, SettingItem, SettingPage, Settings};
use gpui_component::slider::{SliderEvent, SliderState, SliderValue};
use gpui_component::table::TableState;
use gpui_component::tree::TreeState;
use gpui_component::{Sizable as _, Size};
use pi_bridge_types::{
    AuthFlowUpdate, BridgeEvent, BridgeEventEnvelope, CoreStateSnapshot, OAuthLoginMethod,
    PackageSearchResult, ProviderAuthStatus,
};

use crate::backend::{BackendData, BackendSession, BackendSnapshot};
use crate::chat::transcript::ChatTranscript;
use crate::components::auth_settings::{
    AuthSettingsState, auth_settings_content, settings_placeholder,
};
use crate::components::package_settings::InstalledPackagesTableDelegate;
use crate::components::{
    bottom_dock, chat_node, file_picker, pinned_panels, status_bar, workspace_canvas,
    workspace_canvas_view, workspace_launcher, workspace_tabs,
};
use crate::design::theme;
use crate::ui;
use crate::workspace::canvas::{
    CanvasDrawing, CanvasDrawingTool, SessionNodeMetadata, SessionNodePrimitive, WorldPoint,
    WorldSize,
};
use crate::workspace::picker::{self, DEFAULT_DIRECTORY_DEPTH};
use crate::workspace::state::WorkspaceState;

mod appearance_actions;
mod backend_flow;
mod canvas_actions;
mod chat_actions;
mod package_actions;
mod pinned_actions;
mod render;
mod workspace_actions;

pub(crate) fn init(cx: &mut App) {
    pinned_actions::init(cx);
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthFlow {
    Choose,
    ApiKey,
}

const DRAWER_ANIMATION_DURATION: Duration = Duration::from_millis(220);
const DRAWER_WIDTH: f32 = 520.0;
const PROVIDER_HOVER_DELAY: Duration = Duration::from_secs(1);
const STATUS_BAR_HEIGHT: f32 = 28.0;
const BOTTOM_DOCK_HEIGHT: f32 = 28.0;
const MINIMAP_WIDTH: f32 = 148.0;
const MINIMAP_HEIGHT: f32 = 108.0;
const NEW_FOLDER_ROW_ANIMATION: Duration = Duration::from_millis(180);
const FRAME_RENDER_INTERVAL: Duration = Duration::from_millis(8);
const BACKEND_EVENT_BATCH_INTERVAL: Duration = Duration::from_millis(16);

fn drawer_animation() -> Animation {
    Animation::new(DRAWER_ANIMATION_DURATION).with_easing(cubic_bezier(0.32, 0.72, 0.0, 1.0))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WorkspaceDialog {
    OpenWorkspace,
}

#[derive(Clone)]
struct QueuedChatPrompt {
    workspace_id: usize,
    node_id: usize,
    session_path: String,
    text: String,
}

pub struct PiDesktop {
    focus_handle: FocusHandle,
    settings_drawer_open: bool,
    settings_drawer_visible: bool,
    backend: Option<Arc<BackendSession>>,
    data: Option<BackendData>,
    selected_provider: Option<String>,
    hovered_provider: Option<String>,
    hover_card_provider: Option<String>,
    auth_flow: AuthFlow,
    appearance: theme::AppearanceSettings,
    api_key_input: Entity<InputState>,
    package_search_input: Entity<InputState>,
    package_results: Vec<PackageSearchResult>,
    package_pending: bool,
    installing_package: Option<String>,
    removing_package: Option<String>,
    new_installed_package: Option<String>,
    installed_packages_table: Entity<TableState<InstalledPackagesTableDelegate>>,
    workspace_name_input: Entity<InputState>,
    new_folder_name_input: Entity<InputState>,
    workspace_tree: Entity<TreeState>,
    workspace_state: WorkspaceState,
    snap_to_grid: bool,
    drawing_tools_visible: bool,
    active_drawing_tool: CanvasDrawingTool,
    drawing_stroke_width: f32,
    drawing_stroke_slider: Entity<SliderState>,
    pin_shell_state: Entity<ResizableState>,
    pin_panel_state: Entity<ResizableState>,
    bottom_dock_view: Entity<bottom_dock::BottomDockView>,
    status_bar_view: Entity<status_bar::StatusBarView>,
    workspace_canvas_views: HashMap<usize, Entity<workspace_canvas_view::WorkspaceCanvasView>>,
    chat_inputs: HashMap<(usize, usize), Entity<InputState>>,
    chat_input_subscriptions: HashMap<(usize, usize), Subscription>,
    title_inputs: HashMap<(usize, usize), Entity<InputState>>,
    title_input_subscriptions: HashMap<(usize, usize), Subscription>,
    text_box_inputs: HashMap<(usize, usize), Entity<InputState>>,
    text_box_input_subscriptions: HashMap<(usize, usize), Subscription>,
    chat_transcripts: HashMap<(usize, usize), Entity<ChatTranscript>>,
    chat_body_views: HashMap<(usize, usize), Entity<chat_node::ChatBodyView>>,
    chat_node_views: HashMap<(usize, usize), Entity<chat_node::ChatNodeView>>,
    chat_node_render_revision: u64,
    queued_chat_prompts: VecDeque<QueuedChatPrompt>,
    streaming_nodes: HashSet<(usize, usize)>,
    editing_title: Option<(usize, usize)>,
    editing_text_box: Option<(usize, usize)>,
    previous_workspace_index: Option<usize>,
    showing_landing: bool,
    workspace_dialog: Option<WorkspaceDialog>,
    new_folder_input_visible: bool,
    showing_new_folder_input: bool,
    pending_delete_folder: Option<PathBuf>,
    showing_delete_folder_confirmation: bool,
    workspace_picker_root: PathBuf,
    status: SharedString,
    pending: bool,
    event_render_scheduled: bool,
    canvas_render_scheduled: bool,
    cwd: PathBuf,
    _subscriptions: Vec<Subscription>,
}

impl PiDesktop {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let api_key_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Paste API key")
                .masked(true)
        });
        let package_search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search Pi packages"));
        let package_search_subscription = cx.subscribe_in(
            &package_search_input,
            window,
            |view, _input, event, _window, cx| {
                if matches!(event, InputEvent::PressEnter { secondary: false }) {
                    view.search_packages(cx);
                }
            },
        );
        let package_view = cx.entity().clone();
        let installed_packages_table = cx.new(|cx| {
            TableState::new(
                InstalledPackagesTableDelegate::new(package_view.clone()),
                window,
                cx,
            )
        });
        let workspace_name_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Workspace name"));
        let workspace_name_subscription = cx.subscribe_in(
            &workspace_name_input,
            window,
            |view, input, event, window, cx| {
                if matches!(event, InputEvent::PressEnter { secondary: false })
                    && !input.read(cx).value().trim().is_empty()
                {
                    view.create_workspace_from_name(window, cx);
                }
            },
        );
        let new_folder_name_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Folder name"));
        let new_folder_name_subscription = cx.subscribe_in(
            &new_folder_name_input,
            window,
            |view, input, event, window, cx| {
                if matches!(event, InputEvent::PressEnter { secondary: false })
                    && !input.read(cx).value().trim().is_empty()
                {
                    view.create_folder_in_selected_workspace_path(window, cx);
                }
            },
        );
        let drawing_stroke_slider = cx.new(|_cx| {
            SliderState::new()
                .min(1.0)
                .max(16.0)
                .step(1.0)
                .default_value(5.0)
        });
        let pin_shell_state = cx.new(|_| ResizableState::default());
        let pin_panel_state = cx.new(|_| ResizableState::default());
        let bottom_dock_app = cx.entity().clone();
        let bottom_dock_view = cx.new(|_| {
            bottom_dock::BottomDockView::new(
                bottom_dock_app,
                bottom_dock::BottomDockProps {
                    settings_selected: false,
                    snap_to_grid: true,
                    drawing_tools_visible: false,
                },
            )
        });
        let status_bar_app = cx.entity().clone();
        let status_bar_view = cx.new(|cx| {
            status_bar::StatusBarView::new(
                status_bar_app,
                status_bar::StatusBarProps {
                    tabs: Vec::new(),
                    active_index: None,
                    previous_index: None,
                },
                cx,
            )
        });
        let drawing_stroke_subscription = cx.subscribe_in(
            &drawing_stroke_slider,
            window,
            |view, _slider, event, _window, cx| {
                let SliderEvent::Change(SliderValue::Single(value)) = event else {
                    return;
                };
                view.set_drawing_stroke_width(*value, cx);
            },
        );
        let cwd = std::env::current_dir().unwrap_or_else(|_error| PathBuf::from("."));
        let workspace_picker_root = cwd.clone();
        let workspace_tree = cx.new(|cx| {
            TreeState::new(cx).items(picker::build_directory_tree(
                &workspace_picker_root,
                DEFAULT_DIRECTORY_DEPTH,
            ))
        });
        let mut this = Self {
            focus_handle: cx.focus_handle(),
            settings_drawer_open: false,
            settings_drawer_visible: false,
            backend: None,
            data: None,
            selected_provider: None,
            hovered_provider: None,
            hover_card_provider: None,
            auth_flow: AuthFlow::Choose,
            appearance: theme::current_appearance(),
            api_key_input,
            package_search_input,
            package_results: Vec::new(),
            package_pending: false,
            installing_package: None,
            removing_package: None,
            new_installed_package: None,
            installed_packages_table,
            workspace_name_input,
            new_folder_name_input,
            workspace_tree,
            workspace_state: WorkspaceState::new(),
            snap_to_grid: true,
            drawing_tools_visible: false,
            active_drawing_tool: CanvasDrawingTool::Select,
            drawing_stroke_width: 5.0,
            drawing_stroke_slider,
            pin_shell_state,
            pin_panel_state,
            bottom_dock_view,
            status_bar_view,
            workspace_canvas_views: HashMap::new(),
            chat_inputs: HashMap::new(),
            chat_input_subscriptions: HashMap::new(),
            title_inputs: HashMap::new(),
            title_input_subscriptions: HashMap::new(),
            text_box_inputs: HashMap::new(),
            text_box_input_subscriptions: HashMap::new(),
            chat_transcripts: HashMap::new(),
            chat_body_views: HashMap::new(),
            chat_node_views: HashMap::new(),
            chat_node_render_revision: 0,
            queued_chat_prompts: VecDeque::new(),
            streaming_nodes: HashSet::new(),
            editing_title: None,
            editing_text_box: None,
            previous_workspace_index: None,
            showing_landing: true,
            workspace_dialog: None,
            new_folder_input_visible: false,
            showing_new_folder_input: false,
            pending_delete_folder: None,
            showing_delete_folder_confirmation: false,
            workspace_picker_root,
            status: "Starting Pi worker backend…".into(),
            pending: true,
            event_render_scheduled: false,
            canvas_render_scheduled: false,
            cwd,
            _subscriptions: vec![
                package_search_subscription,
                workspace_name_subscription,
                new_folder_name_subscription,
                drawing_stroke_subscription,
            ],
        };
        this.start_backend(cx);
        this
    }

    fn workspace_index_for_id(&self, workspace_id: usize) -> Option<usize> {
        self.workspace_state.index_for_id(workspace_id)
    }

    fn workspace_id_for_index(&self, workspace_index: usize) -> Option<usize> {
        self.workspace_state.tab_id(workspace_index)
    }

    fn ensure_chat_transcript(
        &mut self,
        key: (usize, usize),
        cx: &mut Context<Self>,
    ) -> Entity<ChatTranscript> {
        self.chat_transcripts
            .entry(key)
            .or_insert_with(|| cx.new(|_| ChatTranscript::default()))
            .clone()
    }

    fn update_chat_transcript(
        &mut self,
        key: (usize, usize),
        cx: &mut Context<Self>,
        update: impl FnOnce(&mut ChatTranscript),
    ) -> bool {
        let transcript = self.ensure_chat_transcript(key, cx);
        transcript.update(cx, |transcript, cx| {
            let before = transcript.revision();
            update(transcript);
            let changed = transcript.revision() != before;
            if changed {
                cx.notify();
            }
            changed
        })
    }

    fn hydrate_chat_transcripts_from_state(
        &mut self,
        metadata: &SessionNodeMetadata,
        messages: &[serde_json::Value],
        cx: &mut Context<Self>,
    ) {
        if messages.is_empty() {
            return;
        }
        let keys = self
            .workspace_state
            .tabs()
            .iter()
            .flat_map(|tab| {
                tab.canvas()
                    .nodes()
                    .iter()
                    .filter(|node| session_metadata_matches(node.metadata(), metadata))
                    .map(|node| (tab.id(), node.id()))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        for key in keys {
            self.update_chat_transcript(key, cx, |transcript| {
                transcript.replace_from_snapshot_messages(messages);
            });
        }
    }

    fn remove_session_node_ui_state(&mut self, workspace_id: usize, node_id: usize) {
        let key = (workspace_id, node_id);
        self.chat_inputs.remove(&key);
        self.chat_input_subscriptions.remove(&key);
        self.title_inputs.remove(&key);
        self.title_input_subscriptions.remove(&key);
        self.chat_transcripts.remove(&key);
        self.chat_body_views.remove(&key);
        self.chat_node_views.remove(&key);
        self.streaming_nodes.remove(&key);
        self.queued_chat_prompts
            .retain(|prompt| (prompt.workspace_id, prompt.node_id) != key);
        if self.editing_title == Some(key) {
            self.editing_title = None;
        }
    }

    fn remove_workspace_ui_state(&mut self, workspace_id: usize) {
        self.workspace_canvas_views.remove(&workspace_id);
        let node_keys = self
            .chat_inputs
            .keys()
            .chain(self.title_inputs.keys())
            .chain(self.chat_transcripts.keys())
            .chain(self.chat_body_views.keys())
            .chain(self.chat_node_views.keys())
            .copied()
            .filter(|key| key.0 == workspace_id)
            .collect::<HashSet<_>>();
        for (_, node_id) in node_keys {
            self.remove_session_node_ui_state(workspace_id, node_id);
        }

        let text_keys = self
            .text_box_inputs
            .keys()
            .copied()
            .filter(|key| key.0 == workspace_id)
            .collect::<Vec<_>>();
        for key in text_keys {
            self.text_box_inputs.remove(&key);
            self.text_box_input_subscriptions.remove(&key);
            if self.editing_text_box == Some(key) {
                self.editing_text_box = None;
            }
        }
    }

    fn retain_workspace_node_ui_state(
        &mut self,
        workspace_id: usize,
        live_node_ids: &HashSet<usize>,
    ) {
        let stale_keys = self
            .chat_inputs
            .keys()
            .chain(self.title_inputs.keys())
            .chain(self.chat_transcripts.keys())
            .chain(self.chat_body_views.keys())
            .chain(self.chat_node_views.keys())
            .copied()
            .filter(|key| key.0 == workspace_id && !live_node_ids.contains(&key.1))
            .collect::<HashSet<_>>();
        for (_, node_id) in stale_keys {
            self.remove_session_node_ui_state(workspace_id, node_id);
        }
    }

    fn retain_workspace_text_box_ui_state(
        &mut self,
        workspace_id: usize,
        live_drawing_indices: &HashSet<usize>,
    ) {
        let stale_keys = self
            .text_box_inputs
            .keys()
            .copied()
            .filter(|key| key.0 == workspace_id && !live_drawing_indices.contains(&key.1))
            .collect::<Vec<_>>();
        for key in stale_keys {
            self.text_box_inputs.remove(&key);
            self.text_box_input_subscriptions.remove(&key);
            if self.editing_text_box == Some(key) {
                self.editing_text_box = None;
            }
        }
    }
}

impl Focusable for PiDesktop {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn paint_background_grid_axis(bounds: Bounds<Pixels>, vertical: bool, window: &mut Window) {
    let spacing = 16.0;
    let extent = if vertical {
        f32::from(bounds.size.width)
    } else {
        f32::from(bounds.size.height)
    };
    let line_count = (extent / spacing).ceil() as i32 + 1;

    for index in 0..line_count {
        let position = index as f32 * spacing;
        let color = if index % 4 == 0 {
            theme::grid_major()
        } else {
            theme::grid_minor()
        };
        let line_bounds = if vertical {
            Bounds::new(
                point(bounds.origin.x + px(position), bounds.origin.y),
                size(px(1.0), bounds.size.height),
            )
        } else {
            Bounds::new(
                point(bounds.origin.x, bounds.origin.y + px(position)),
                size(bounds.size.width, px(1.0)),
            )
        };
        window.paint_quad(fill(line_bounds, color));
    }
}

fn next_new_folder_name(parent: &Path) -> String {
    let first = "New Folder".to_owned();
    if !parent.join(&first).exists() {
        return first;
    }

    (2..)
        .map(|index| format!("New Folder {index}"))
        .find(|name| !parent.join(name).exists())
        .unwrap_or(first)
}

fn valid_new_folder_name(name: &str) -> bool {
    !name.is_empty()
        && !Path::new(name).is_absolute()
        && Path::new(name)
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}

fn remove_auto_inserted_enter_newline(
    input: &Entity<InputState>,
    window: &mut Window,
    cx: &mut Context<PiDesktop>,
) {
    let (value, cursor) = {
        let input = input.read(cx);
        (input.value().to_string(), input.cursor())
    };
    let Some(remove_at) = enter_newline_before_cursor(&value, cursor) else {
        return;
    };

    let mut cleaned = value;
    cleaned.remove(remove_at);
    input.update(cx, |input, cx| input.set_value(cleaned, window, cx));
}

fn enter_newline_before_cursor(value: &str, cursor: usize) -> Option<usize> {
    if cursor == 0 || cursor > value.len() {
        return None;
    }
    let remove_at = cursor - 1;
    value
        .as_bytes()
        .get(remove_at)
        .is_some_and(|byte| *byte == b'\n')
        .then_some(remove_at)
}

fn canvas_local_point(screen_position: WorldPoint) -> WorldPoint {
    WorldPoint::new(screen_position.x, screen_position.y - STATUS_BAR_HEIGHT)
}

fn minimap_size() -> WorldSize {
    WorldSize::new(MINIMAP_WIDTH, MINIMAP_HEIGHT)
}

fn workspace_canvas_size(window: &Window) -> WorldSize {
    let size = window.bounds().size;
    WorldSize::new(
        f32::from(size.width),
        (f32::from(size.height) - STATUS_BAR_HEIGHT - BOTTOM_DOCK_HEIGHT).max(1.0),
    )
}

fn session_node_metadata(state: &CoreStateSnapshot) -> SessionNodeMetadata {
    SessionNodeMetadata {
        session_id: state.session_id.clone(),
        session_name: state.session_name.clone(),
        session_file: state.session_file.clone(),
        cwd: state.cwd.clone(),
        message_count: state.messages.len(),
    }
}

fn session_metadata_matches(
    node_metadata: &SessionNodeMetadata,
    metadata: &SessionNodeMetadata,
) -> bool {
    let matches_session_id = metadata.session_id.is_some()
        && node_metadata.session_id.as_ref() == metadata.session_id.as_ref();
    let matches_session_file = metadata.session_file.is_some()
        && node_metadata.session_file.as_ref() == metadata.session_file.as_ref();
    matches_session_id || matches_session_file
}

fn pending_session_node_metadata(primitive: SessionNodePrimitive) -> SessionNodeMetadata {
    SessionNodeMetadata {
        session_id: None,
        session_name: Some(format!("{}…", primitive.label())),
        session_file: None,
        cwd: None,
        message_count: 0,
    }
}

fn empty_session_node_metadata() -> SessionNodeMetadata {
    SessionNodeMetadata {
        session_id: None,
        session_name: None,
        session_file: None,
        cwd: None,
        message_count: 0,
    }
}

fn latest_json_id(messages: &[serde_json::Value]) -> Option<String> {
    messages.iter().rev().find_map(latest_json_id_in_value)
}

fn latest_json_id_in_value(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(object) => object
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| object.values().rev().find_map(latest_json_id_in_value)),
        serde_json::Value::Array(values) => values.iter().rev().find_map(latest_json_id_in_value),
        _other => None,
    }
}

fn auth_update_status(update: &AuthFlowUpdate) -> String {
    let mut message = update.message.clone();
    if let Some(user_code) = &update.user_code {
        message.push_str(" Code: ");
        message.push_str(user_code);
    }
    if let Some(url) = &update.url {
        message.push_str(" URL: ");
        message.push_str(url);
    }
    message
}

fn is_terminal_chat_event(event: &serde_json::Value) -> bool {
    matches!(
        event.get("type").and_then(serde_json::Value::as_str),
        Some("agent_end" | "agent_error")
    )
}

fn chat_event_status(event: &serde_json::Value) -> String {
    let event_type = event
        .get("type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("event");
    match event_type {
        "agent_start" => "Pi is working…".to_owned(),
        "agent_end" => "Pi idle.".to_owned(),
        "agent_error" => format!(
            "Pi chat failed: {}",
            event
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown error")
        ),
        "message_start" | "message_update" | "assistant_text_delta" => {
            "Pi is responding…".to_owned()
        }
        "message_end" => "Pi response complete.".to_owned(),
        "tool_execution_start" | "tool_execution_update" => format!(
            "Running {}…",
            event
                .get("toolName")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("tool")
        ),
        "tool_execution_end" => format!(
            "Finished {}.",
            event
                .get("toolName")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("tool")
        ),
        other => format!("Pi {other}"),
    }
}

const BROWSER_AUTH: [OAuthLoginMethod; 1] = [OAuthLoginMethod::Browser];
const DEVICE_AUTH: [OAuthLoginMethod; 1] = [OAuthLoginMethod::DeviceCode];
const BROWSER_AND_DEVICE_AUTH: [OAuthLoginMethod; 2] =
    [OAuthLoginMethod::Browser, OAuthLoginMethod::DeviceCode];
const NO_OAUTH_METHODS: [OAuthLoginMethod; 0] = [];

pub(crate) fn oauth_methods_for(provider: &str) -> &'static [OAuthLoginMethod] {
    match provider {
        "anthropic" => &BROWSER_AUTH,
        "github-copilot" => &DEVICE_AUTH,
        "openai-codex" => &BROWSER_AND_DEVICE_AUTH,
        _ => &NO_OAUTH_METHODS,
    }
}

#[cfg(test)]
mod tests {
    use super::enter_newline_before_cursor;

    #[test]
    fn detects_auto_inserted_enter_newline_before_cursor() {
        assert_eq!(enter_newline_before_cursor("hello\n", 6), Some(5));
        assert_eq!(enter_newline_before_cursor("hello", 5), None);
        assert_eq!(enter_newline_before_cursor("hello\n", 0), None);
    }
}
