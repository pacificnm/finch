use crate::action::TaskRequest;
use crate::types::{PluginId, TaskId};

#[derive(Debug, Clone)]
pub enum Effect {
    Quit,
    RequestRedraw,
    PersistSession,
    SpawnTask { task_id: TaskId, request: TaskRequest },
    CancelTask(TaskId),
    OpenExternalUrl(String),
    WriteClipboard(String),
    PluginCall(PluginCall),
}

#[derive(Debug, Clone)]
pub struct PluginCall {
    pub plugin_id: PluginId,
    pub command: String,
    pub args: Vec<String>,
}
