use crate::types::{PanelId, PluginId, WorkspaceId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanelKind {
    Terminal,
    FileBrowser,
    GitStatus,
    LogViewer,
    Plugin { plugin_id: PluginId, panel_type: String },
}

#[derive(Debug, Clone)]
pub struct OpenPanelRequest {
    pub kind: PanelKind,
    pub workspace_id: Option<WorkspaceId>,
}

#[derive(Debug, Clone)]
pub struct PanelEntry {
    pub id: PanelId,
    pub kind: PanelKind,
    pub workspace_id: Option<WorkspaceId>,
}

#[derive(Debug, Clone, Default)]
pub struct PanelStore {
    panels: Vec<PanelEntry>,
    next_id: u64,
}

impl PanelStore {
    pub fn open(&mut self, request: OpenPanelRequest) -> PanelId {
        self.next_id += 1;
        let id = PanelId(self.next_id);
        self.panels.push(PanelEntry { id, kind: request.kind, workspace_id: request.workspace_id });
        id
    }

    pub fn close(&mut self, id: PanelId) {
        self.panels.retain(|p| p.id != id);
    }

    pub fn contains(&self, id: PanelId) -> bool {
        self.panels.iter().any(|p| p.id == id)
    }

    pub fn get(&self, id: PanelId) -> Option<&PanelEntry> {
        self.panels.iter().find(|p| p.id == id)
    }

    pub fn panels_for_workspace(&self, workspace_id: WorkspaceId) -> Vec<&PanelEntry> {
        self.panels.iter().filter(|p| p.workspace_id == Some(workspace_id)).collect()
    }

    pub fn all(&self) -> &[PanelEntry] {
        &self.panels
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::WorkspaceId;

    fn ws(n: u64) -> WorkspaceId {
        WorkspaceId(n)
    }

    fn open(store: &mut PanelStore, kind: PanelKind, workspace_id: Option<WorkspaceId>) -> PanelId {
        store.open(OpenPanelRequest { kind, workspace_id })
    }

    #[test]
    fn open_returns_unique_ids() {
        let mut s = PanelStore::default();
        let a = open(&mut s, PanelKind::Terminal, None);
        let b = open(&mut s, PanelKind::Terminal, None);
        assert_ne!(a, b);
    }

    #[test]
    fn open_panel_is_retrievable_by_id() {
        let mut s = PanelStore::default();
        let id = open(&mut s, PanelKind::FileBrowser, None);
        let entry = s.get(id).unwrap();
        assert_eq!(entry.id, id);
        assert_eq!(entry.kind, PanelKind::FileBrowser);
    }

    #[test]
    fn open_stores_workspace_id() {
        let mut s = PanelStore::default();
        let wid = ws(1);
        let id = open(&mut s, PanelKind::Terminal, Some(wid));
        assert_eq!(s.get(id).unwrap().workspace_id, Some(wid));
    }

    #[test]
    fn open_without_workspace_stores_none() {
        let mut s = PanelStore::default();
        let id = open(&mut s, PanelKind::Terminal, None);
        assert_eq!(s.get(id).unwrap().workspace_id, None);
    }

    #[test]
    fn contains_returns_true_for_open_panel() {
        let mut s = PanelStore::default();
        let id = open(&mut s, PanelKind::Terminal, None);
        assert!(s.contains(id));
    }

    #[test]
    fn contains_returns_false_for_unknown_panel() {
        let s = PanelStore::default();
        assert!(!s.contains(PanelId(999)));
    }

    #[test]
    fn get_returns_none_for_unknown_id() {
        let s = PanelStore::default();
        assert!(s.get(PanelId(999)).is_none());
    }

    #[test]
    fn close_removes_panel() {
        let mut s = PanelStore::default();
        let id = open(&mut s, PanelKind::Terminal, None);
        s.close(id);
        assert!(!s.contains(id));
        assert!(s.get(id).is_none());
    }

    #[test]
    fn close_nonexistent_panel_is_noop() {
        let mut s = PanelStore::default();
        let id = open(&mut s, PanelKind::Terminal, None);
        s.close(PanelId(999));
        assert!(s.contains(id));
    }

    #[test]
    fn close_only_removes_targeted_panel() {
        let mut s = PanelStore::default();
        let a = open(&mut s, PanelKind::Terminal, None);
        let b = open(&mut s, PanelKind::FileBrowser, None);
        s.close(a);
        assert!(!s.contains(a));
        assert!(s.contains(b));
    }

    #[test]
    fn panels_for_workspace_returns_matching_panels() {
        let mut s = PanelStore::default();
        let w1 = ws(1);
        let w2 = ws(2);
        let a = open(&mut s, PanelKind::Terminal, Some(w1));
        let b = open(&mut s, PanelKind::GitStatus, Some(w1));
        let _c = open(&mut s, PanelKind::LogViewer, Some(w2));
        let result = s.panels_for_workspace(w1);
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|p| p.id == a));
        assert!(result.iter().any(|p| p.id == b));
    }

    #[test]
    fn panels_for_workspace_excludes_other_workspaces() {
        let mut s = PanelStore::default();
        let w1 = ws(1);
        let w2 = ws(2);
        open(&mut s, PanelKind::Terminal, Some(w1));
        assert!(s.panels_for_workspace(w2).is_empty());
    }

    #[test]
    fn panels_for_workspace_excludes_unassigned_panels() {
        let mut s = PanelStore::default();
        let w1 = ws(1);
        open(&mut s, PanelKind::Terminal, None);
        assert!(s.panels_for_workspace(w1).is_empty());
    }

    #[test]
    fn panels_for_workspace_empty_after_close() {
        let mut s = PanelStore::default();
        let w1 = ws(1);
        let id = open(&mut s, PanelKind::Terminal, Some(w1));
        s.close(id);
        assert!(s.panels_for_workspace(w1).is_empty());
    }

    #[test]
    fn all_returns_every_panel() {
        let mut s = PanelStore::default();
        open(&mut s, PanelKind::Terminal, None);
        open(&mut s, PanelKind::FileBrowser, None);
        assert_eq!(s.all().len(), 2);
    }

    #[test]
    fn all_empty_by_default() {
        let s = PanelStore::default();
        assert!(s.all().is_empty());
    }
}
