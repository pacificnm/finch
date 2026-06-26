use crate::types::PanelId;

#[derive(Debug, Clone, Default)]
pub struct FocusState {
    pub focused_panel: Option<PanelId>,
    pub previous_panel: Option<PanelId>,
    pub mode: InputMode,
}

impl FocusState {
    pub fn focus(&mut self, panel_id: PanelId) {
        self.previous_panel = self.focused_panel;
        self.focused_panel = Some(panel_id);
    }

    pub fn release(&mut self) {
        self.previous_panel = self.focused_panel;
        self.focused_panel = None;
    }

    pub fn restore_previous(&mut self) {
        let prev = self.previous_panel;
        self.previous_panel = self.focused_panel;
        self.focused_panel = prev;
    }

    pub fn is_focused(&self, panel_id: PanelId) -> bool {
        self.focused_panel == Some(panel_id)
    }

    pub fn set_mode(&mut self, mode: InputMode) {
        self.mode = mode;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn pid(n: u64) -> PanelId {
        PanelId(n)
    }

    #[test]
    fn default_has_no_focused_panel() {
        let f = FocusState::default();
        assert!(f.focused_panel.is_none());
    }

    #[test]
    fn default_has_no_previous_panel() {
        let f = FocusState::default();
        assert!(f.previous_panel.is_none());
    }

    #[test]
    fn default_mode_is_normal() {
        let f = FocusState::default();
        assert_eq!(f.mode, InputMode::Normal);
    }

    #[test]
    fn focus_sets_focused_panel() {
        let mut f = FocusState::default();
        f.focus(pid(1));
        assert_eq!(f.focused_panel, Some(pid(1)));
    }

    #[test]
    fn focus_replaces_focused_panel() {
        let mut f = FocusState::default();
        f.focus(pid(1));
        f.focus(pid(2));
        assert_eq!(f.focused_panel, Some(pid(2)));
    }

    #[test]
    fn focus_tracks_previous_panel() {
        let mut f = FocusState::default();
        f.focus(pid(1));
        f.focus(pid(2));
        assert_eq!(f.previous_panel, Some(pid(1)));
    }

    #[test]
    fn focus_first_panel_leaves_no_previous() {
        let mut f = FocusState::default();
        f.focus(pid(1));
        assert!(f.previous_panel.is_none());
    }

    #[test]
    fn focus_enforces_single_panel_at_a_time() {
        let mut f = FocusState::default();
        f.focus(pid(1));
        f.focus(pid(2));
        f.focus(pid(3));
        assert_eq!(f.focused_panel, Some(pid(3)));
        assert_eq!(f.previous_panel, Some(pid(2)));
    }

    #[test]
    fn release_clears_focused_panel() {
        let mut f = FocusState::default();
        f.focus(pid(1));
        f.release();
        assert!(f.focused_panel.is_none());
    }

    #[test]
    fn release_saves_released_panel_as_previous() {
        let mut f = FocusState::default();
        f.focus(pid(1));
        f.release();
        assert_eq!(f.previous_panel, Some(pid(1)));
    }

    #[test]
    fn restore_previous_returns_to_prior_panel() {
        let mut f = FocusState::default();
        f.focus(pid(1));
        f.focus(pid(2));
        f.restore_previous();
        assert_eq!(f.focused_panel, Some(pid(1)));
    }

    #[test]
    fn restore_previous_swaps_both_slots() {
        let mut f = FocusState::default();
        f.focus(pid(1));
        f.focus(pid(2));
        f.restore_previous();
        assert_eq!(f.previous_panel, Some(pid(2)));
    }

    #[test]
    fn restore_previous_with_no_previous_clears_focus() {
        let mut f = FocusState::default();
        f.focus(pid(1));
        f.restore_previous();
        assert!(f.focused_panel.is_none());
    }

    #[test]
    fn is_focused_returns_true_for_focused_panel() {
        let mut f = FocusState::default();
        f.focus(pid(5));
        assert!(f.is_focused(pid(5)));
    }

    #[test]
    fn is_focused_returns_false_for_unfocused_panel() {
        let mut f = FocusState::default();
        f.focus(pid(5));
        assert!(!f.is_focused(pid(6)));
    }

    #[test]
    fn is_focused_returns_false_when_no_panel_focused() {
        let f = FocusState::default();
        assert!(!f.is_focused(pid(1)));
    }

    #[test]
    fn set_mode_changes_input_mode() {
        let mut f = FocusState::default();
        f.set_mode(InputMode::TextInput);
        assert_eq!(f.mode, InputMode::TextInput);
    }

    #[test]
    fn set_mode_can_return_to_normal() {
        let mut f = FocusState::default();
        f.set_mode(InputMode::CommandPalette);
        f.set_mode(InputMode::Normal);
        assert_eq!(f.mode, InputMode::Normal);
    }

    #[test]
    fn all_input_modes_are_distinct() {
        assert_ne!(InputMode::Normal, InputMode::CommandPalette);
        assert_ne!(InputMode::Normal, InputMode::Search);
        assert_ne!(InputMode::Normal, InputMode::TextInput);
        assert_ne!(InputMode::Normal, InputMode::PanelCapture);
    }
}
