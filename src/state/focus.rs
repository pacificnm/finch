use crate::types::PanelId;

#[derive(Debug, Clone, Default)]
pub struct FocusState {
    pub focused_panel: Option<PanelId>,
    pub previous_panel: Option<PanelId>,
    pub mode: InputMode,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    Normal,
    CommandPalette,
    Search,
    TextInput,
    PanelCapture,
}
