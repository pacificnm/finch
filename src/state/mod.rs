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
