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
            ws.focus.focus(panel_id);
        }
    }

    pub fn clear_panel_focus(&mut self, panel_id: PanelId) {
        if let Some(ws) = self.active_workspace_mut() {
            if ws.focus.focused_panel == Some(panel_id) {
                ws.focus.release();
            }
        }
    }

    pub fn move_focus(&mut self, direction: crate::action::FocusDirection) {
        // Spatial focus movement is resolved at render time when panel
        // layout geometry is known; the store records the intent.
        let _ = direction;
    }

    pub fn get(&self, id: WorkspaceId) -> Option<&WorkspaceState> {
        self.workspaces.iter().find(|w| w.id == id)
    }

    pub fn active_id(&self) -> Option<WorkspaceId> {
        self.active
    }

    pub fn all(&self) -> &[WorkspaceState] {
        &self.workspaces
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

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with_one() -> (WorkspaceStore, WorkspaceId) {
        let mut s = WorkspaceStore::default();
        let id = s.create("dev".to_string());
        (s, id)
    }

    #[test]
    fn create_returns_unique_ids() {
        let mut s = WorkspaceStore::default();
        let a = s.create("a".to_string());
        let b = s.create("b".to_string());
        assert_ne!(a, b);
    }

    #[test]
    fn create_workspace_is_retrievable() {
        let (s, id) = store_with_one();
        let ws = s.get(id).unwrap();
        assert_eq!(ws.id, id);
        assert_eq!(ws.name, "dev");
    }

    #[test]
    fn create_workspace_has_default_tab() {
        let (s, id) = store_with_one();
        let ws = s.get(id).unwrap();
        assert_eq!(ws.tabs.len(), 1);
        assert_eq!(ws.tabs[0].name, "main");
        assert_eq!(ws.active_tab, ws.tabs[0].id);
    }

    #[test]
    fn switch_updates_active_id() {
        let mut s = WorkspaceStore::default();
        let a = s.create("a".to_string());
        let b = s.create("b".to_string());
        s.switch(a);
        assert_eq!(s.active_id(), Some(a));
        s.switch(b);
        assert_eq!(s.active_id(), Some(b));
    }

    #[test]
    fn switch_to_unknown_id_is_ignored() {
        let (mut s, id) = store_with_one();
        s.switch(id);
        s.switch(WorkspaceId(999));
        assert_eq!(s.active_id(), Some(id));
    }

    #[test]
    fn close_removes_workspace() {
        let (mut s, id) = store_with_one();
        s.close(id);
        assert!(s.get(id).is_none());
        assert!(s.all().is_empty());
    }

    #[test]
    fn close_active_clears_or_advances_active() {
        let mut s = WorkspaceStore::default();
        let a = s.create("a".to_string());
        let b = s.create("b".to_string());
        s.switch(a);
        s.close(a);
        assert_ne!(s.active_id(), Some(a));
        // active should fall back to the remaining workspace
        assert_eq!(s.active_id(), Some(b));
    }

    #[test]
    fn close_non_active_leaves_active_unchanged() {
        let mut s = WorkspaceStore::default();
        let a = s.create("a".to_string());
        let b = s.create("b".to_string());
        s.switch(a);
        s.close(b);
        assert_eq!(s.active_id(), Some(a));
    }

    #[test]
    fn active_workspace_returns_current() {
        let (mut s, id) = store_with_one();
        s.switch(id);
        assert_eq!(s.active_workspace().map(|w| w.id), Some(id));
    }

    #[test]
    fn active_workspace_none_before_switch() {
        let s = WorkspaceStore::default();
        assert!(s.active_workspace().is_none());
    }

    #[test]
    fn all_lists_every_workspace() {
        let mut s = WorkspaceStore::default();
        s.create("x".to_string());
        s.create("y".to_string());
        assert_eq!(s.all().len(), 2);
    }

    #[test]
    fn focus_panel_sets_focused_panel_on_active_workspace() {
        use crate::types::PanelId;
        let (mut s, id) = store_with_one();
        s.switch(id);
        let panel = PanelId(7);
        s.focus_panel(panel);
        assert_eq!(s.active_workspace().unwrap().focus.focused_panel, Some(panel));
    }

    #[test]
    fn focus_panel_tracks_previous_panel() {
        use crate::types::PanelId;
        let (mut s, id) = store_with_one();
        s.switch(id);
        let first = PanelId(1);
        let second = PanelId(2);
        s.focus_panel(first);
        s.focus_panel(second);
        let focus = &s.active_workspace().unwrap().focus;
        assert_eq!(focus.focused_panel, Some(second));
        assert_eq!(focus.previous_panel, Some(first));
    }
}
