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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::notification::{Notification, NotificationLevel, NotificationSource};
    use crate::state::panel::{OpenPanelRequest, PanelKind};
    use crate::types::{NotificationId, PanelId, PluginId, TaskId, WorkspaceId};

    #[test]
    fn quit_is_constructable() {
        let action = AppAction::Quit;
        assert!(matches!(action, AppAction::Quit));
    }

    #[test]
    fn redraw_is_constructable() {
        let action = AppAction::Redraw;
        assert!(matches!(action, AppAction::Redraw));
    }

    #[test]
    fn open_command_palette_is_constructable() {
        assert!(matches!(AppAction::OpenCommandPalette, AppAction::OpenCommandPalette));
    }

    #[test]
    fn close_command_palette_is_constructable() {
        assert!(matches!(AppAction::CloseCommandPalette, AppAction::CloseCommandPalette));
    }

    #[test]
    fn switch_workspace_carries_id() {
        let id = WorkspaceId(42);
        let action = AppAction::SwitchWorkspace(id);
        assert!(matches!(action, AppAction::SwitchWorkspace(WorkspaceId(42))));
    }

    #[test]
    fn create_workspace_carries_name() {
        let action = AppAction::CreateWorkspace { name: "dev".to_string() };
        assert!(matches!(action, AppAction::CreateWorkspace { .. }));
    }

    #[test]
    fn close_workspace_carries_id() {
        let action = AppAction::CloseWorkspace(WorkspaceId(1));
        assert!(matches!(action, AppAction::CloseWorkspace(WorkspaceId(1))));
    }

    #[test]
    fn focus_panel_carries_id() {
        let action = AppAction::FocusPanel(PanelId(5));
        assert!(matches!(action, AppAction::FocusPanel(PanelId(5))));
    }

    #[test]
    fn open_panel_carries_request() {
        let req = OpenPanelRequest { kind: PanelKind::Terminal, workspace_id: None };
        let action = AppAction::OpenPanel(req);
        assert!(matches!(action, AppAction::OpenPanel(_)));
    }

    #[test]
    fn close_panel_carries_id() {
        let action = AppAction::ClosePanel(PanelId(3));
        assert!(matches!(action, AppAction::ClosePanel(PanelId(3))));
    }

    #[test]
    fn split_panel_carries_direction_and_request() {
        let req = OpenPanelRequest { kind: PanelKind::FileBrowser, workspace_id: None };
        let action = AppAction::SplitPanel { direction: SplitDirection::Vertical, panel: req };
        assert!(matches!(action, AppAction::SplitPanel { direction: SplitDirection::Vertical, .. }));
    }

    #[test]
    fn move_focus_carries_direction() {
        let action = AppAction::MoveFocus(FocusDirection::Right);
        assert!(matches!(action, AppAction::MoveFocus(FocusDirection::Right)));
    }

    #[test]
    fn start_task_carries_request() {
        let req = TaskRequest { label: "build".to_string(), owner: TaskOwner::Core };
        let action = AppAction::StartTask(req);
        assert!(matches!(action, AppAction::StartTask(_)));
    }

    #[test]
    fn cancel_task_carries_id() {
        let action = AppAction::CancelTask(TaskId(7));
        assert!(matches!(action, AppAction::CancelTask(TaskId(7))));
    }

    #[test]
    fn task_completed_carries_result() {
        let result = TaskResult { task_id: TaskId(1), outcome: TaskOutcome::Success };
        let action = AppAction::TaskCompleted(result);
        assert!(matches!(action, AppAction::TaskCompleted(_)));
    }

    #[test]
    fn notify_carries_notification() {
        let n = Notification {
            id: NotificationId(0),
            level: NotificationLevel::Info,
            title: "ok".to_string(),
            message: None,
            source: NotificationSource::Core,
            read: false,
        };
        let action = AppAction::Notify(n);
        assert!(matches!(action, AppAction::Notify(_)));
    }

    #[test]
    fn plugin_action_enable_carries_id() {
        let action = AppAction::Plugin(PluginAction::Enable(PluginId(1)));
        assert!(matches!(action, AppAction::Plugin(PluginAction::Enable(PluginId(1)))));
    }

    #[test]
    fn plugin_action_disable_carries_id() {
        let action = AppAction::Plugin(PluginAction::Disable(PluginId(2)));
        assert!(matches!(action, AppAction::Plugin(PluginAction::Disable(PluginId(2)))));
    }

    #[test]
    fn plugin_action_command_carries_fields() {
        let action = AppAction::Plugin(PluginAction::Command {
            plugin_id: PluginId(3),
            command: "refresh".to_string(),
            args: vec!["--force".to_string()],
        });
        assert!(matches!(action, AppAction::Plugin(PluginAction::Command { .. })));
    }

    #[test]
    fn split_direction_variants_are_distinct() {
        assert_ne!(SplitDirection::Horizontal, SplitDirection::Vertical);
    }

    #[test]
    fn focus_direction_all_variants_are_distinct() {
        let dirs = [
            FocusDirection::Left,
            FocusDirection::Right,
            FocusDirection::Up,
            FocusDirection::Down,
            FocusDirection::Next,
            FocusDirection::Previous,
        ];
        for (i, a) in dirs.iter().enumerate() {
            for (j, b) in dirs.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn task_owner_core_variant() {
        let owner = TaskOwner::Core;
        assert!(matches!(owner, TaskOwner::Core));
    }

    #[test]
    fn task_owner_panel_carries_id() {
        let owner = TaskOwner::Panel(PanelId(9));
        assert!(matches!(owner, TaskOwner::Panel(PanelId(9))));
    }

    #[test]
    fn task_owner_plugin_carries_id() {
        let owner = TaskOwner::Plugin(PluginId(2));
        assert!(matches!(owner, TaskOwner::Plugin(PluginId(2))));
    }

    #[test]
    fn task_outcome_success_variant() {
        assert_eq!(TaskOutcome::Success, TaskOutcome::Success);
    }

    #[test]
    fn task_outcome_failure_carries_message() {
        let outcome = TaskOutcome::Failure("timeout".to_string());
        assert_eq!(outcome, TaskOutcome::Failure("timeout".to_string()));
    }

    #[test]
    fn task_outcome_cancelled_variant() {
        assert_eq!(TaskOutcome::Cancelled, TaskOutcome::Cancelled);
    }

    #[test]
    fn task_request_stores_label_and_owner() {
        let req = TaskRequest { label: "watch".to_string(), owner: TaskOwner::Core };
        assert_eq!(req.label, "watch");
        assert!(matches!(req.owner, TaskOwner::Core));
    }

    #[test]
    fn task_result_stores_id_and_outcome() {
        let res = TaskResult { task_id: TaskId(5), outcome: TaskOutcome::Cancelled };
        assert_eq!(res.task_id, TaskId(5));
        assert_eq!(res.outcome, TaskOutcome::Cancelled);
    }
}
