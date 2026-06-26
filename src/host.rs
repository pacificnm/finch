use crate::action::{AppAction, PanelAction, TaskOwner};
use crate::state::AppState;
use crate::types::PanelId;

/// Validate a `PanelAction` from a panel and translate it to an `AppAction`
/// the host can apply to `AppState`. Returns `None` when the action is a
/// no-op or fails validation (e.g. the panel is not registered).
pub fn dispatch_panel_action(
    panel_id: PanelId,
    action: PanelAction,
    state: &AppState,
) -> Option<AppAction> {
    match action {
        PanelAction::None => None,
        PanelAction::Quit => Some(AppAction::Quit),
        PanelAction::Redraw => Some(AppAction::Redraw),
        PanelAction::Close => {
            if state.panels.contains(panel_id) {
                Some(AppAction::ClosePanel(panel_id))
            } else {
                None
            }
        }
        PanelAction::RequestFocus => {
            if state.panels.contains(panel_id) {
                Some(AppAction::FocusPanel(panel_id))
            } else {
                None
            }
        }
        PanelAction::OpenPanel(request) => Some(AppAction::OpenPanel(request)),
        PanelAction::Notify(notification) => Some(AppAction::Notify(notification)),
        PanelAction::StartTask(mut request) => {
            // Attribute the task to the panel that requested it.
            request.owner = TaskOwner::Panel(panel_id);
            Some(AppAction::StartTask(request))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{TaskOwner, TaskRequest};
    use crate::reducer::reduce;
    use crate::state::notification::{Notification, NotificationLevel, NotificationSource};
    use crate::state::panel::{OpenPanelRequest, PanelKind};
    use crate::types::NotificationId;

    fn make_state_with_panel() -> (AppState, PanelId) {
        let mut state = AppState::default();
        let id = state.panels.open(OpenPanelRequest { kind: PanelKind::Terminal, workspace_id: None });
        (state, id)
    }

    #[test]
    fn none_returns_none() {
        let state = AppState::default();
        let result = dispatch_panel_action(PanelId(1), PanelAction::None, &state);
        assert!(result.is_none());
    }

    #[test]
    fn quit_returns_quit_action() {
        let state = AppState::default();
        let result = dispatch_panel_action(PanelId(1), PanelAction::Quit, &state);
        assert!(matches!(result, Some(AppAction::Quit)));
    }

    #[test]
    fn redraw_returns_redraw_action() {
        let state = AppState::default();
        let result = dispatch_panel_action(PanelId(1), PanelAction::Redraw, &state);
        assert!(matches!(result, Some(AppAction::Redraw)));
    }

    #[test]
    fn close_known_panel_returns_close_panel_action() {
        let (state, id) = make_state_with_panel();
        let result = dispatch_panel_action(id, PanelAction::Close, &state);
        assert!(matches!(result, Some(AppAction::ClosePanel(pid)) if pid == id));
    }

    #[test]
    fn close_unknown_panel_returns_none() {
        let state = AppState::default();
        let result = dispatch_panel_action(PanelId(99), PanelAction::Close, &state);
        assert!(result.is_none());
    }

    #[test]
    fn request_focus_known_panel_returns_focus_panel_action() {
        let (state, id) = make_state_with_panel();
        let result = dispatch_panel_action(id, PanelAction::RequestFocus, &state);
        assert!(matches!(result, Some(AppAction::FocusPanel(pid)) if pid == id));
    }

    #[test]
    fn request_focus_unknown_panel_returns_none() {
        let state = AppState::default();
        let result = dispatch_panel_action(PanelId(99), PanelAction::RequestFocus, &state);
        assert!(result.is_none());
    }

    #[test]
    fn open_panel_passes_through_request() {
        let state = AppState::default();
        let req = OpenPanelRequest { kind: PanelKind::FileBrowser, workspace_id: None };
        let result = dispatch_panel_action(PanelId(1), PanelAction::OpenPanel(req), &state);
        assert!(matches!(result, Some(AppAction::OpenPanel(_))));
    }

    #[test]
    fn notify_passes_through_notification() {
        let state = AppState::default();
        let n = Notification {
            id: NotificationId(0),
            level: NotificationLevel::Info,
            title: "hello".to_string(),
            message: None,
            source: NotificationSource::Core,
            read: false,
        };
        let result = dispatch_panel_action(PanelId(1), PanelAction::Notify(n), &state);
        assert!(matches!(result, Some(AppAction::Notify(_))));
    }

    #[test]
    fn start_task_rewrites_owner_to_panel() {
        let state = AppState::default();
        let req = TaskRequest { label: "watch".to_string(), owner: TaskOwner::Core };
        let result = dispatch_panel_action(PanelId(7), PanelAction::StartTask(req), &state);
        match result {
            Some(AppAction::StartTask(r)) => {
                assert!(matches!(r.owner, TaskOwner::Panel(PanelId(7))));
            }
            _ => panic!("expected StartTask action"),
        }
    }

    #[test]
    fn start_task_label_is_preserved() {
        let state = AppState::default();
        let req = TaskRequest { label: "build".to_string(), owner: TaskOwner::Core };
        let result = dispatch_panel_action(PanelId(1), PanelAction::StartTask(req), &state);
        match result {
            Some(AppAction::StartTask(r)) => assert_eq!(r.label, "build"),
            _ => panic!("expected StartTask action"),
        }
    }

    #[test]
    fn dispatched_close_applies_to_state_via_reducer() {
        let (mut state, id) = make_state_with_panel();
        let action = dispatch_panel_action(id, PanelAction::Close, &state).unwrap();
        reduce(&mut state, action);
        assert!(!state.panels.contains(id));
    }

    #[test]
    fn dispatched_focus_applies_to_state_via_reducer() {
        let (mut state, id) = make_state_with_panel();
        reduce(&mut state, AppAction::CreateWorkspace { name: "dev".to_string() });
        let action = dispatch_panel_action(id, PanelAction::RequestFocus, &state).unwrap();
        reduce(&mut state, action);
        let ws = state.workspaces.active_workspace().unwrap();
        assert_eq!(ws.focus.focused_panel, Some(id));
    }

    #[test]
    fn dispatched_notify_adds_notification_via_reducer() {
        let mut state = AppState::default();
        let n = Notification {
            id: NotificationId(0),
            level: NotificationLevel::Warning,
            title: "oops".to_string(),
            message: None,
            source: NotificationSource::Core,
            read: false,
        };
        let action = dispatch_panel_action(PanelId(1), PanelAction::Notify(n), &state).unwrap();
        reduce(&mut state, action);
        assert_eq!(state.notifications.unread_count(), 1);
    }
}
