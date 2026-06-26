use crate::types::WorkspaceId;

#[derive(Debug, Clone, Default)]
pub struct SessionState {
    pub active_workspace: Option<WorkspaceId>,
    pub version: u32,
}
