use crate::types::{PanelId, PluginId};

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
    pub workspace_id: Option<crate::types::WorkspaceId>,
}

#[derive(Debug, Clone, Default)]
pub struct PanelStore {
    panels: Vec<PanelEntry>,
    next_id: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PanelEntry {
    pub id: PanelId,
    pub kind: PanelKind,
}

impl PanelStore {
    pub fn open(&mut self, request: OpenPanelRequest) -> PanelId {
        self.next_id += 1;
        let id = PanelId(self.next_id);
        self.panels.push(PanelEntry { id, kind: request.kind });
        id
    }

    pub fn close(&mut self, id: PanelId) {
        self.panels.retain(|p| p.id != id);
    }

    pub fn contains(&self, id: PanelId) -> bool {
        self.panels.iter().any(|p| p.id == id)
    }
}
