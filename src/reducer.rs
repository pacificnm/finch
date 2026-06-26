use crate::action::{AppAction, PluginAction};
use crate::effect::{Effect, PluginCall};
use crate::state::AppState;

pub fn reduce(state: &mut AppState, action: AppAction) -> Vec<Effect> {
    match action {
        AppAction::Quit => {
            vec![Effect::PersistSession, Effect::Quit]
        }

        AppAction::Redraw => {
            vec![Effect::RequestRedraw]
        }

        AppAction::OpenCommandPalette => {
            state.ui.command_palette.open = true;
            state.ui.command_palette.query.clear();
            state.ui.command_palette.selected_index = 0;
            vec![Effect::RequestRedraw]
        }

        AppAction::CloseCommandPalette => {
            state.ui.command_palette.open = false;
            state.ui.command_palette.query.clear();
            state.ui.command_palette.results.clear();
            vec![Effect::RequestRedraw]
        }

        AppAction::SwitchWorkspace(id) => {
            state.workspaces.switch(id);
            state.session.active_workspace = Some(id);
            vec![Effect::RequestRedraw]
        }

        AppAction::CreateWorkspace { name } => {
            let id = state.workspaces.create(name);
            state.workspaces.switch(id);
            state.session.active_workspace = Some(id);
            vec![Effect::RequestRedraw]
        }

        AppAction::CloseWorkspace(id) => {
            state.workspaces.close(id);
            vec![Effect::RequestRedraw]
        }

        AppAction::FocusPanel(panel_id) => {
            state.workspaces.focus_panel(panel_id);
            vec![Effect::RequestRedraw]
        }

        AppAction::OpenPanel(request) => {
            state.panels.open(request);
            vec![Effect::RequestRedraw]
        }

        AppAction::ClosePanel(panel_id) => {
            state.panels.close(panel_id);
            vec![Effect::RequestRedraw]
        }

        AppAction::SplitPanel { direction, panel } => {
            state.panels.open(panel);
            // Spatial layout update is deferred to the layout engine with the
            // direction hint; state records the new panel only.
            let _ = direction;
            vec![Effect::RequestRedraw]
        }

        AppAction::MoveFocus(direction) => {
            state.workspaces.move_focus(direction);
            vec![Effect::RequestRedraw]
        }

        AppAction::StartTask(request) => {
            let task_id = state.tasks.register(request.clone());
            vec![Effect::SpawnTask { task_id, request }]
        }

        AppAction::CancelTask(task_id) => {
            state.tasks.cancel(task_id);
            vec![Effect::CancelTask(task_id)]
        }

        AppAction::TaskCompleted(result) => {
            state.tasks.complete(result);
            vec![Effect::RequestRedraw]
        }

        AppAction::Notify(notification) => {
            state.notifications.push(notification);
            vec![Effect::RequestRedraw]
        }

        AppAction::Plugin(plugin_action) => match plugin_action {
            PluginAction::Enable(plugin_id) => {
                state.plugins.set_enabled(plugin_id, true);
                vec![Effect::RequestRedraw]
            }
            PluginAction::Disable(plugin_id) => {
                state.plugins.set_enabled(plugin_id, false);
                vec![Effect::RequestRedraw]
            }
            PluginAction::Command { plugin_id, command, args } => {
                vec![Effect::PluginCall(PluginCall { plugin_id, command, args })]
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{FocusDirection, TaskOutcome, TaskOwner, TaskRequest, TaskResult};
    use crate::state::notification::{Notification, NotificationLevel, NotificationSource};
    use crate::state::panel::{OpenPanelRequest, PanelKind};
    use crate::types::{NotificationId, PanelId, PluginId, TaskId};

    fn state() -> AppState {
        AppState::default()
    }

    #[test]
    fn quit_returns_persist_and_quit_effects() {
        let mut s = state();
        let effects = reduce(&mut s, AppAction::Quit);
        assert!(effects.iter().any(|e| matches!(e, Effect::PersistSession)));
        assert!(effects.iter().any(|e| matches!(e, Effect::Quit)));
    }

    #[test]
    fn redraw_returns_request_redraw() {
        let mut s = state();
        let effects = reduce(&mut s, AppAction::Redraw);
        assert!(matches!(effects[0], Effect::RequestRedraw));
    }

    #[test]
    fn open_command_palette_sets_open_flag() {
        let mut s = state();
        assert!(!s.ui.command_palette.open);
        reduce(&mut s, AppAction::OpenCommandPalette);
        assert!(s.ui.command_palette.open);
    }

    #[test]
    fn close_command_palette_clears_state() {
        let mut s = state();
        s.ui.command_palette.open = true;
        s.ui.command_palette.query = "foo".to_string();
        reduce(&mut s, AppAction::CloseCommandPalette);
        assert!(!s.ui.command_palette.open);
        assert!(s.ui.command_palette.query.is_empty());
    }

    #[test]
    fn create_workspace_sets_active_session() {
        let mut s = state();
        reduce(&mut s, AppAction::CreateWorkspace { name: "dev".to_string() });
        assert!(s.session.active_workspace.is_some());
    }

    #[test]
    fn switch_workspace_updates_session() {
        let mut s = state();
        reduce(&mut s, AppAction::CreateWorkspace { name: "dev".to_string() });
        let id = s.session.active_workspace.unwrap();
        reduce(&mut s, AppAction::SwitchWorkspace(id));
        assert_eq!(s.session.active_workspace, Some(id));
    }

    #[test]
    fn focus_panel_updates_workspace_focus() {
        let mut s = state();
        reduce(&mut s, AppAction::CreateWorkspace { name: "dev".to_string() });
        let panel_id = PanelId(42);
        reduce(&mut s, AppAction::FocusPanel(panel_id));
        let ws = s.workspaces.active_workspace().unwrap();
        assert_eq!(ws.focus.focused_panel, Some(panel_id));
    }

    #[test]
    fn open_panel_registers_panel() {
        let mut s = state();
        reduce(
            &mut s,
            AppAction::OpenPanel(OpenPanelRequest {
                kind: PanelKind::Terminal,
                workspace_id: None,
            }),
        );
        assert!(s.panels.contains(PanelId(1)));
    }

    #[test]
    fn close_panel_removes_panel() {
        let mut s = state();
        reduce(
            &mut s,
            AppAction::OpenPanel(OpenPanelRequest {
                kind: PanelKind::Terminal,
                workspace_id: None,
            }),
        );
        let id = PanelId(1);
        assert!(s.panels.contains(id));
        reduce(&mut s, AppAction::ClosePanel(id));
        assert!(!s.panels.contains(id));
    }

    #[test]
    fn move_focus_returns_redraw() {
        let mut s = state();
        let effects = reduce(&mut s, AppAction::MoveFocus(FocusDirection::Right));
        assert!(effects.iter().any(|e| matches!(e, Effect::RequestRedraw)));
    }

    #[test]
    fn start_task_spawns_task_effect() {
        let mut s = state();
        let request = TaskRequest {
            label: "build".to_string(),
            owner: TaskOwner::Core,
        };
        let effects = reduce(&mut s, AppAction::StartTask(request));
        assert!(effects.iter().any(|e| matches!(e, Effect::SpawnTask { .. })));
        assert_eq!(s.tasks.active_count(), 1);
    }

    #[test]
    fn cancel_task_emits_cancel_effect() {
        let mut s = state();
        let request = TaskRequest {
            label: "watch".to_string(),
            owner: TaskOwner::Core,
        };
        reduce(&mut s, AppAction::StartTask(request));
        let effects = reduce(&mut s, AppAction::CancelTask(TaskId(1)));
        assert!(effects.iter().any(|e| matches!(e, Effect::CancelTask(_))));
    }

    #[test]
    fn task_completed_updates_task_status() {
        let mut s = state();
        let request = TaskRequest {
            label: "fetch".to_string(),
            owner: TaskOwner::Core,
        };
        reduce(&mut s, AppAction::StartTask(request));
        reduce(
            &mut s,
            AppAction::TaskCompleted(TaskResult {
                task_id: TaskId(1),
                outcome: TaskOutcome::Success,
            }),
        );
        use crate::state::task::TaskStatus;
        assert_eq!(
            s.tasks.get(TaskId(1)).unwrap().status,
            TaskStatus::Completed
        );
    }

    #[test]
    fn notify_adds_to_notification_store() {
        let mut s = state();
        let notification = Notification {
            id: NotificationId(0),
            level: NotificationLevel::Info,
            title: "Done".to_string(),
            message: None,
            source: NotificationSource::Core,
            read: false,
        };
        reduce(&mut s, AppAction::Notify(notification));
        assert_eq!(s.notifications.unread_count(), 1);
    }

    #[test]
    fn plugin_enable_sets_enabled() {
        let mut s = state();
        use crate::action::PluginAction;
        use crate::state::plugin::PluginState;
        s.plugins.register(PluginState {
            id: PluginId(1),
            name: "git".to_string(),
            enabled: false,
            registered_panels: vec![],
            registered_commands: vec![],
        });
        reduce(&mut s, AppAction::Plugin(PluginAction::Enable(PluginId(1))));
        assert!(s.plugins.get(PluginId(1)).unwrap().enabled);
    }

    #[test]
    fn plugin_command_emits_plugin_call_effect() {
        let mut s = state();
        use crate::action::PluginAction;
        let effects = reduce(
            &mut s,
            AppAction::Plugin(PluginAction::Command {
                plugin_id: PluginId(1),
                command: "refresh".to_string(),
                args: vec![],
            }),
        );
        assert!(effects.iter().any(|e| matches!(e, Effect::PluginCall(_))));
    }
}
