use crate::action::{AppAction, FocusDirection};
use crate::state::focus::InputMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    pub key: Key,
    pub modifiers: KeyModifiers,
}

impl KeyEvent {
    pub fn new(key: Key, modifiers: KeyModifiers) -> Self {
        Self { key, modifiers }
    }

    pub fn plain(key: Key) -> Self {
        Self { key, modifiers: KeyModifiers::NONE }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Char(char),
    F(u8),
    Left,
    Right,
    Up,
    Down,
    Enter,
    Backspace,
    Delete,
    Escape,
    Tab,
    BackTab,
    Home,
    End,
    PageUp,
    PageDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyModifiers(pub u8);

impl KeyModifiers {
    pub const NONE: Self = Self(0);
    pub const SHIFT: Self = Self(0b001);
    pub const CTRL: Self = Self(0b010);
    pub const ALT: Self = Self(0b100);

    pub fn ctrl(self) -> bool {
        self.0 & Self::CTRL.0 != 0
    }

    pub fn shift(self) -> bool {
        self.0 & Self::SHIFT.0 != 0
    }

    pub fn alt(self) -> bool {
        self.0 & Self::ALT.0 != 0
    }
}

impl std::ops::BitOr for KeyModifiers {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Map a raw keyboard event to an `AppAction` based on the current input mode.
/// Returns `None` when the key has no global binding in the current mode
/// (e.g. PanelCapture passes all input directly to the focused panel).
pub fn map_input(event: KeyEvent, mode: &InputMode) -> Option<AppAction> {
    match mode {
        InputMode::Normal => map_normal(event),
        InputMode::CommandPalette => map_command_palette(event),
        InputMode::TextInput => map_text_input(event),
        InputMode::Search => map_search(event),
        InputMode::PanelCapture => None,
    }
}

fn map_normal(event: KeyEvent) -> Option<AppAction> {
    match event.key {
        Key::Char('q') if event.modifiers == KeyModifiers::NONE => Some(AppAction::Quit),
        Key::Char(':') if event.modifiers == KeyModifiers::NONE => {
            Some(AppAction::OpenCommandPalette)
        }
        Key::Char('p') if event.modifiers == KeyModifiers::CTRL => {
            Some(AppAction::OpenCommandPalette)
        }
        Key::Char('h') | Key::Left if event.modifiers == KeyModifiers::NONE => {
            Some(AppAction::MoveFocus(FocusDirection::Left))
        }
        Key::Char('l') | Key::Right if event.modifiers == KeyModifiers::NONE => {
            Some(AppAction::MoveFocus(FocusDirection::Right))
        }
        Key::Char('k') | Key::Up if event.modifiers == KeyModifiers::NONE => {
            Some(AppAction::MoveFocus(FocusDirection::Up))
        }
        Key::Char('j') | Key::Down if event.modifiers == KeyModifiers::NONE => {
            Some(AppAction::MoveFocus(FocusDirection::Down))
        }
        Key::Tab if event.modifiers == KeyModifiers::NONE => {
            Some(AppAction::MoveFocus(FocusDirection::Next))
        }
        Key::BackTab => Some(AppAction::MoveFocus(FocusDirection::Previous)),
        _ => None,
    }
}

fn map_command_palette(event: KeyEvent) -> Option<AppAction> {
    match event.key {
        Key::Escape => Some(AppAction::CloseCommandPalette),
        Key::Backspace => Some(AppAction::CommandPaletteBackspace),
        // Accept plain chars and Shift-modified chars (uppercase letters, symbols).
        // Crossterm reports Shift+A as Char('A') with KeyModifiers::SHIFT.
        Key::Char(c)
            if event.modifiers == KeyModifiers::NONE
                || event.modifiers == KeyModifiers::SHIFT =>
        {
            Some(AppAction::CommandPaletteInput(c))
        }
        _ => None,
    }
}

fn map_text_input(event: KeyEvent) -> Option<AppAction> {
    match event.key {
        Key::Char(c)
            if event.modifiers == KeyModifiers::NONE
                || event.modifiers == KeyModifiers::SHIFT =>
        {
            Some(AppAction::TextInput(c))
        }
        _ => None,
    }
}

fn map_search(event: KeyEvent) -> Option<AppAction> {
    match event.key {
        Key::Char(c)
            if event.modifiers == KeyModifiers::NONE
                || event.modifiers == KeyModifiers::SHIFT =>
        {
            Some(AppAction::TextInput(c))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(key: Key) -> KeyEvent {
        KeyEvent::plain(key)
    }

    fn with_mod(key: Key, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(key, modifiers)
    }

    // --- Normal mode ---

    #[test]
    fn normal_q_quits() {
        assert!(matches!(
            map_input(plain(Key::Char('q')), &InputMode::Normal),
            Some(AppAction::Quit)
        ));
    }

    #[test]
    fn normal_colon_opens_command_palette() {
        assert!(matches!(
            map_input(plain(Key::Char(':')), &InputMode::Normal),
            Some(AppAction::OpenCommandPalette)
        ));
    }

    #[test]
    fn normal_ctrl_p_opens_command_palette() {
        assert!(matches!(
            map_input(with_mod(Key::Char('p'), KeyModifiers::CTRL), &InputMode::Normal),
            Some(AppAction::OpenCommandPalette)
        ));
    }

    #[test]
    fn normal_h_moves_focus_left() {
        assert!(matches!(
            map_input(plain(Key::Char('h')), &InputMode::Normal),
            Some(AppAction::MoveFocus(FocusDirection::Left))
        ));
    }

    #[test]
    fn normal_left_arrow_moves_focus_left() {
        assert!(matches!(
            map_input(plain(Key::Left), &InputMode::Normal),
            Some(AppAction::MoveFocus(FocusDirection::Left))
        ));
    }

    #[test]
    fn normal_l_moves_focus_right() {
        assert!(matches!(
            map_input(plain(Key::Char('l')), &InputMode::Normal),
            Some(AppAction::MoveFocus(FocusDirection::Right))
        ));
    }

    #[test]
    fn normal_right_arrow_moves_focus_right() {
        assert!(matches!(
            map_input(plain(Key::Right), &InputMode::Normal),
            Some(AppAction::MoveFocus(FocusDirection::Right))
        ));
    }

    #[test]
    fn normal_k_moves_focus_up() {
        assert!(matches!(
            map_input(plain(Key::Char('k')), &InputMode::Normal),
            Some(AppAction::MoveFocus(FocusDirection::Up))
        ));
    }

    #[test]
    fn normal_up_arrow_moves_focus_up() {
        assert!(matches!(
            map_input(plain(Key::Up), &InputMode::Normal),
            Some(AppAction::MoveFocus(FocusDirection::Up))
        ));
    }

    #[test]
    fn normal_j_moves_focus_down() {
        assert!(matches!(
            map_input(plain(Key::Char('j')), &InputMode::Normal),
            Some(AppAction::MoveFocus(FocusDirection::Down))
        ));
    }

    #[test]
    fn normal_down_arrow_moves_focus_down() {
        assert!(matches!(
            map_input(plain(Key::Down), &InputMode::Normal),
            Some(AppAction::MoveFocus(FocusDirection::Down))
        ));
    }

    #[test]
    fn normal_tab_moves_focus_next() {
        assert!(matches!(
            map_input(plain(Key::Tab), &InputMode::Normal),
            Some(AppAction::MoveFocus(FocusDirection::Next))
        ));
    }

    #[test]
    fn normal_backtab_moves_focus_previous() {
        assert!(matches!(
            map_input(plain(Key::BackTab), &InputMode::Normal),
            Some(AppAction::MoveFocus(FocusDirection::Previous))
        ));
    }

    #[test]
    fn normal_unbound_key_returns_none() {
        assert!(map_input(plain(Key::Char('z')), &InputMode::Normal).is_none());
    }

    #[test]
    fn normal_q_with_modifier_returns_none() {
        assert!(map_input(with_mod(Key::Char('q'), KeyModifiers::CTRL), &InputMode::Normal).is_none());
    }

    #[test]
    fn normal_escape_returns_none() {
        assert!(map_input(plain(Key::Escape), &InputMode::Normal).is_none());
    }

    // --- Command palette mode ---

    #[test]
    fn command_palette_escape_closes_palette() {
        assert!(matches!(
            map_input(plain(Key::Escape), &InputMode::CommandPalette),
            Some(AppAction::CloseCommandPalette)
        ));
    }

    #[test]
    fn command_palette_char_appends_to_query() {
        assert!(matches!(
            map_input(plain(Key::Char('g')), &InputMode::CommandPalette),
            Some(AppAction::CommandPaletteInput('g'))
        ));
    }

    #[test]
    fn command_palette_char_preserves_case() {
        assert!(matches!(
            map_input(plain(Key::Char('G')), &InputMode::CommandPalette),
            Some(AppAction::CommandPaletteInput('G'))
        ));
    }

    #[test]
    fn command_palette_backspace_deletes_last_char() {
        assert!(matches!(
            map_input(plain(Key::Backspace), &InputMode::CommandPalette),
            Some(AppAction::CommandPaletteBackspace)
        ));
    }

    #[test]
    fn command_palette_uppercase_char_with_shift_is_accepted() {
        assert!(matches!(
            map_input(with_mod(Key::Char('G'), KeyModifiers::SHIFT), &InputMode::CommandPalette),
            Some(AppAction::CommandPaletteInput('G'))
        ));
    }

    #[test]
    fn command_palette_char_with_ctrl_returns_none() {
        assert!(
            map_input(with_mod(Key::Char('c'), KeyModifiers::CTRL), &InputMode::CommandPalette)
                .is_none()
        );
    }

    #[test]
    fn command_palette_arrow_returns_none() {
        assert!(map_input(plain(Key::Down), &InputMode::CommandPalette).is_none());
    }

    // --- TextInput mode ---

    #[test]
    fn text_input_char_produces_text_input_action() {
        assert!(matches!(
            map_input(plain(Key::Char('a')), &InputMode::TextInput),
            Some(AppAction::TextInput('a'))
        ));
    }

    #[test]
    fn text_input_space_produces_text_input_action() {
        assert!(matches!(
            map_input(plain(Key::Char(' ')), &InputMode::TextInput),
            Some(AppAction::TextInput(' '))
        ));
    }

    #[test]
    fn text_input_escape_returns_none() {
        assert!(map_input(plain(Key::Escape), &InputMode::TextInput).is_none());
    }

    #[test]
    fn text_input_uppercase_char_with_shift_is_accepted() {
        assert!(matches!(
            map_input(with_mod(Key::Char('S'), KeyModifiers::SHIFT), &InputMode::TextInput),
            Some(AppAction::TextInput('S'))
        ));
    }

    #[test]
    fn text_input_char_with_modifier_returns_none() {
        assert!(
            map_input(with_mod(Key::Char('s'), KeyModifiers::CTRL), &InputMode::TextInput).is_none()
        );
    }

    // --- Search mode ---

    #[test]
    fn search_char_produces_text_input_action() {
        assert!(matches!(
            map_input(plain(Key::Char('f')), &InputMode::Search),
            Some(AppAction::TextInput('f'))
        ));
    }

    #[test]
    fn search_escape_returns_none() {
        assert!(map_input(plain(Key::Escape), &InputMode::Search).is_none());
    }

    // --- PanelCapture mode ---

    #[test]
    fn panel_capture_any_key_returns_none() {
        assert!(map_input(plain(Key::Char('x')), &InputMode::PanelCapture).is_none());
    }

    #[test]
    fn panel_capture_quit_returns_none() {
        assert!(map_input(plain(Key::Char('q')), &InputMode::PanelCapture).is_none());
    }

    #[test]
    fn panel_capture_escape_returns_none() {
        assert!(map_input(plain(Key::Escape), &InputMode::PanelCapture).is_none());
    }

    // --- KeyModifiers ---

    #[test]
    fn key_modifiers_none_has_no_flags() {
        let m = KeyModifiers::NONE;
        assert!(!m.ctrl());
        assert!(!m.shift());
        assert!(!m.alt());
    }

    #[test]
    fn key_modifiers_ctrl_flag_is_set() {
        assert!(KeyModifiers::CTRL.ctrl());
        assert!(!KeyModifiers::CTRL.shift());
    }

    #[test]
    fn key_modifiers_shift_flag_is_set() {
        assert!(KeyModifiers::SHIFT.shift());
        assert!(!KeyModifiers::SHIFT.ctrl());
    }

    #[test]
    fn key_modifiers_alt_flag_is_set() {
        assert!(KeyModifiers::ALT.alt());
        assert!(!KeyModifiers::ALT.ctrl());
    }

    #[test]
    fn key_modifiers_bitor_combines_flags() {
        let m = KeyModifiers::CTRL | KeyModifiers::SHIFT;
        assert!(m.ctrl());
        assert!(m.shift());
        assert!(!m.alt());
    }

    #[test]
    fn key_modifiers_none_equals_default() {
        assert_eq!(KeyModifiers::NONE, KeyModifiers::default());
    }

    // --- KeyEvent constructors ---

    #[test]
    fn key_event_plain_has_no_modifiers() {
        let e = KeyEvent::plain(Key::Char('a'));
        assert_eq!(e.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn key_event_new_stores_key_and_modifiers() {
        let e = KeyEvent::new(Key::Char('a'), KeyModifiers::CTRL);
        assert_eq!(e.key, Key::Char('a'));
        assert_eq!(e.modifiers, KeyModifiers::CTRL);
    }
}
