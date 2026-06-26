use crate::types::{CommandId, PluginId};

use super::panel::PanelKind;

#[derive(Debug, Clone)]
pub struct PluginState {
    pub id: PluginId,
    pub name: String,
    pub enabled: bool,
    pub registered_panels: Vec<PanelKind>,
    pub registered_commands: Vec<CommandId>,
}

#[derive(Debug, Clone, Default)]
pub struct PluginStore {
    plugins: Vec<PluginState>,
}

impl PluginStore {
    pub fn register(&mut self, plugin: PluginState) {
        self.plugins.push(plugin);
    }

    pub fn set_enabled(&mut self, id: PluginId, enabled: bool) {
        if let Some(p) = self.plugins.iter_mut().find(|p| p.id == id) {
            p.enabled = enabled;
        }
    }

    pub fn get(&self, id: PluginId) -> Option<&PluginState> {
        self.plugins.iter().find(|p| p.id == id)
    }
}
