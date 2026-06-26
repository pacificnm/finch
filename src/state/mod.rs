pub mod command;
pub mod config;
pub mod focus;
pub mod notification;
pub mod panel;
pub mod plugin;
pub mod session;
pub mod task;
pub mod ui;
pub mod workspace;

use command::CommandStore;
use config::ConfigState;
use notification::NotificationStore;
use panel::PanelStore;
use plugin::PluginStore;
use session::SessionState;
use task::TaskStore;
use ui::UiState;
use workspace::WorkspaceStore;

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub config: ConfigState,
    pub ui: UiState,
    pub workspaces: WorkspaceStore,
    pub panels: PanelStore,
    pub commands: CommandStore,
    pub tasks: TaskStore,
    pub notifications: NotificationStore,
    pub plugins: PluginStore,
    pub session: SessionState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_constructs_without_panic() {
        let _ = AppState::default();
    }

    #[test]
    fn default_command_palette_is_closed() {
        let s = AppState::default();
        assert!(!s.ui.command_palette.open);
    }

    #[test]
    fn default_has_no_active_workspace() {
        let s = AppState::default();
        assert!(s.workspaces.active_workspace().is_none());
    }

    #[test]
    fn default_has_no_active_tasks() {
        let s = AppState::default();
        assert_eq!(s.tasks.active_count(), 0);
    }

    #[test]
    fn default_has_no_unread_notifications() {
        let s = AppState::default();
        assert_eq!(s.notifications.unread_count(), 0);
    }

    #[test]
    fn default_session_has_no_active_workspace() {
        let s = AppState::default();
        assert!(s.session.active_workspace.is_none());
    }

    #[test]
    fn clone_produces_independent_copy() {
        let mut original = AppState::default();
        original.workspaces.create("dev".to_string());
        let clone = original.clone();
        assert_eq!(original.workspaces.all().len(), clone.workspaces.all().len());
    }
}
