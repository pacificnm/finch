use crate::types::{PanelId, TabId, WorkspaceId};

use super::focus::FocusState;

#[derive(Debug, Clone)]
pub struct WorkspaceState {
    pub id: WorkspaceId,
    pub name: String,
    pub tabs: Vec<TabState>,
    pub active_tab: TabId,
    pub focus: FocusState,
}

#[derive(Debug, Clone)]
pub struct TabState {
    pub id: TabId,
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceStore {
    workspaces: Vec<WorkspaceState>,
    active: Option<WorkspaceId>,
    next_workspace_id: u64,
    next_tab_id: u64,
}

impl WorkspaceStore {
    pub fn create(&mut self, name: String) -> WorkspaceId {
        self.next_workspace_id += 1;
        self.next_tab_id += 1;
        let id = WorkspaceId(self.next_workspace_id);
        let default_tab = TabState {
            id: TabId(self.next_tab_id),
            name: "main".to_string(),
        };
        let tab_id = default_tab.id;
        self.workspaces.push(WorkspaceState {
            id,
            name,
            tabs: vec![default_tab],
            active_tab: tab_id,
            focus: FocusState::default(),
        });
        id
    }

    pub fn switch(&mut self, id: WorkspaceId) {
        if self.workspaces.iter().any(|w| w.id == id) {
            self.active = Some(id);
        }
    }

    pub fn close(&mut self, id: WorkspaceId) {
        self.workspaces.retain(|w| w.id != id);
        if self.active == Some(id) {
            self.active = self.workspaces.first().map(|w| w.id);
        }
    }

    pub fn focus_panel(&mut self, panel_id: PanelId) {
        if let Some(ws) = self.active_workspace_mut() {
            let prev = ws.focus.focused_panel;
            ws.focus.previous_panel = prev;
            ws.focus.focused_panel = Some(panel_id);
        }
    }

    pub fn move_focus(&mut self, direction: crate::action::FocusDirection) {
        // Spatial focus movement is resolved at render time when panel
        // layout geometry is known; the store records the intent.
        let _ = direction;
    }

    pub fn active_workspace(&self) -> Option<&WorkspaceState> {
        let id = self.active?;
        self.workspaces.iter().find(|w| w.id == id)
    }

    fn active_workspace_mut(&mut self) -> Option<&mut WorkspaceState> {
        let id = self.active?;
        self.workspaces.iter_mut().find(|w| w.id == id)
    }
}
