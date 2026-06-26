use crate::state::notification::Notification;
use crate::state::panel::OpenPanelRequest;
use crate::types::{PanelId, PluginId, TaskId, WorkspaceId};

#[derive(Debug, Clone)]
pub enum AppAction {
    Quit,
    Redraw,
    OpenCommandPalette,
    CloseCommandPalette,
    SwitchWorkspace(WorkspaceId),
    CreateWorkspace { name: String },
    CloseWorkspace(WorkspaceId),
    FocusPanel(PanelId),
    OpenPanel(OpenPanelRequest),
    ClosePanel(PanelId),
    SplitPanel { direction: SplitDirection, panel: OpenPanelRequest },
    MoveFocus(FocusDirection),
    StartTask(TaskRequest),
    CancelTask(TaskId),
    TaskCompleted(TaskResult),
    Notify(Notification),
    Plugin(PluginAction),
}

#[derive(Debug, Clone, Copy)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy)]
pub enum FocusDirection {
    Left,
    Right,
    Up,
    Down,
    Next,
    Previous,
}

#[derive(Debug, Clone)]
pub struct TaskRequest {
    pub label: String,
    pub owner: TaskOwner,
}

#[derive(Debug, Clone)]
pub enum TaskOwner {
    Core,
    Panel(PanelId),
    Plugin(PluginId),
}

#[derive(Debug, Clone)]
pub struct TaskResult {
    pub task_id: TaskId,
    pub outcome: TaskOutcome,
}

#[derive(Debug, Clone)]
pub enum TaskOutcome {
    Success,
    Failure(String),
    Cancelled,
}

#[derive(Debug, Clone)]
pub enum PluginAction {
    Enable(PluginId),
    Disable(PluginId),
    Command { plugin_id: PluginId, command: String, args: Vec<String> },
}
