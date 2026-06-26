#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub command_palette: CommandPaletteState,
}

#[derive(Debug, Clone, Default)]
pub struct CommandPaletteState {
    pub open: bool,
    pub query: String,
    pub selected_index: usize,
    pub results: Vec<CommandSearchResult>,
}

#[derive(Debug, Clone)]
pub struct CommandSearchResult {
    pub label: String,
    pub description: Option<String>,
}
