use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    Select,
    Toggle,
    Back,
    Quit,
    Character(char),
    Backspace,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    None,
}

/// A user-visible binding between an [`Action`] and the keys that trigger it.
///
/// Used by [`KeyHandler::default_bindings`] and consumed by `HelpBar` to render
/// auto-generated help text driven by a single source of truth.
#[derive(Debug, Clone, Copy)]
pub struct Binding {
    /// Display label for the keys (e.g. `"↑/k"`, `"enter"`).
    pub keys: &'static str,
    pub action: Action,
    /// Short, human-readable description (e.g. `"Move up"`).
    pub label: &'static str,
}

const DEFAULT_BINDINGS: &[Binding] = &[
    Binding {
        keys: "↑/k",
        action: Action::Up,
        label: "Up",
    },
    Binding {
        keys: "↓/j",
        action: Action::Down,
        label: "Down",
    },
    Binding {
        keys: "←/h",
        action: Action::Left,
        label: "Left",
    },
    Binding {
        keys: "→/l",
        action: Action::Right,
        label: "Right",
    },
    Binding {
        keys: "enter",
        action: Action::Select,
        label: "Select",
    },
    Binding {
        keys: "space",
        action: Action::Toggle,
        label: "Toggle",
    },
    Binding {
        keys: "esc",
        action: Action::Back,
        label: "Back",
    },
    Binding {
        keys: "q",
        action: Action::Quit,
        label: "Quit",
    },
];

pub struct KeyHandler;

impl KeyHandler {
    /// Curated subset of bindings handled by [`KeyHandler::map`], suitable for
    /// rendering as help text. Omits keys that are either redundant for users
    /// (e.g. `Ctrl+C` alongside `q` for [`Action::Quit`]) or context-specific
    /// (`Backspace`/`Delete`/`Home`/`End`/`PageUp`/`PageDown`/character input).
    /// Pass to `HelpBar::bindings` to render the active key hints.
    #[must_use]
    pub fn default_bindings() -> &'static [Binding] {
        DEFAULT_BINDINGS
    }

    pub fn map(event: KeyEvent) -> Action {
        if event.modifiers.contains(KeyModifiers::CONTROL) {
            return match event.code {
                KeyCode::Char('c') => Action::Quit,
                _ => Action::None,
            };
        }

        match event.code {
            KeyCode::Up | KeyCode::Char('k') => Action::Up,
            KeyCode::Down | KeyCode::Char('j') => Action::Down,
            KeyCode::Left | KeyCode::Char('h') => Action::Left,
            KeyCode::Right | KeyCode::Char('l') => Action::Right,
            KeyCode::Enter => Action::Select,
            KeyCode::Char(' ') => Action::Toggle,
            KeyCode::Esc => Action::Back,
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Backspace => Action::Backspace,
            KeyCode::Delete => Action::Delete,
            KeyCode::Home => Action::Home,
            KeyCode::End => Action::End,
            KeyCode::PageUp => Action::PageUp,
            KeyCode::PageDown => Action::PageDown,
            KeyCode::Char(c) => Action::Character(c),
            _ => Action::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    #[test]
    fn vim_navigation() {
        assert_eq!(KeyHandler::map(key(KeyCode::Char('j'))), Action::Down);
        assert_eq!(KeyHandler::map(key(KeyCode::Char('k'))), Action::Up);
        assert_eq!(KeyHandler::map(key(KeyCode::Char('h'))), Action::Left);
        assert_eq!(KeyHandler::map(key(KeyCode::Char('l'))), Action::Right);
    }

    #[test]
    fn arrow_navigation() {
        assert_eq!(KeyHandler::map(key(KeyCode::Down)), Action::Down);
        assert_eq!(KeyHandler::map(key(KeyCode::Up)), Action::Up);
        assert_eq!(KeyHandler::map(key(KeyCode::Left)), Action::Left);
        assert_eq!(KeyHandler::map(key(KeyCode::Right)), Action::Right);
    }

    #[test]
    fn selection_keys() {
        assert_eq!(KeyHandler::map(key(KeyCode::Enter)), Action::Select);
        assert_eq!(KeyHandler::map(key(KeyCode::Char(' '))), Action::Toggle);
    }

    #[test]
    fn quit_keys() {
        assert_eq!(KeyHandler::map(key(KeyCode::Char('q'))), Action::Quit);
        assert_eq!(KeyHandler::map(key(KeyCode::Esc)), Action::Back);
        assert_eq!(KeyHandler::map(ctrl(KeyCode::Char('c'))), Action::Quit);
    }

    #[test]
    fn default_bindings_cover_all_navigation_actions() {
        let actions: Vec<Action> = KeyHandler::default_bindings()
            .iter()
            .map(|b| b.action)
            .collect();
        for required in [
            Action::Up,
            Action::Down,
            Action::Left,
            Action::Right,
            Action::Select,
            Action::Toggle,
            Action::Back,
            Action::Quit,
        ] {
            assert!(
                actions.contains(&required),
                "default bindings missing {required:?}",
            );
        }
    }

    #[test]
    fn default_binding_labels_are_non_empty() {
        for binding in KeyHandler::default_bindings() {
            assert!(!binding.keys.is_empty(), "keys label is empty");
            assert!(!binding.label.is_empty(), "human label is empty");
        }
    }

    fn with_mods(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    #[test]
    fn editing_keys_map_to_their_actions() {
        assert_eq!(KeyHandler::map(key(KeyCode::Backspace)), Action::Backspace);
        assert_eq!(KeyHandler::map(key(KeyCode::Delete)), Action::Delete);
        assert_eq!(KeyHandler::map(key(KeyCode::Home)), Action::Home);
        assert_eq!(KeyHandler::map(key(KeyCode::End)), Action::End);
        assert_eq!(KeyHandler::map(key(KeyCode::PageUp)), Action::PageUp);
        assert_eq!(KeyHandler::map(key(KeyCode::PageDown)), Action::PageDown);
    }

    #[test]
    fn printable_chars_fall_through_to_character() {
        // Any non-binding character is returned verbatim as Action::Character.
        for c in ['a', 'Z', '1', '?'] {
            assert_eq!(
                KeyHandler::map(key(KeyCode::Char(c))),
                Action::Character(c),
                "char {c:?}",
            );
        }
        // Uppercase variants of binding chars are NOT bindings — the match arms
        // are case-sensitive, so they fall through to Character.
        assert_eq!(
            KeyHandler::map(key(KeyCode::Char('J'))),
            Action::Character('J'),
        );
        assert_eq!(
            KeyHandler::map(key(KeyCode::Char('Q'))),
            Action::Character('Q'),
        );
    }

    #[test]
    fn binding_chars_take_precedence_over_character() {
        // The vim/quit/toggle chars must resolve to their action, never to
        // Action::Character — the specific arms win over the catch-all.
        for (c, expected) in [
            ('k', Action::Up),
            ('j', Action::Down),
            ('h', Action::Left),
            ('l', Action::Right),
            ('q', Action::Quit),
            (' ', Action::Toggle),
        ] {
            assert_eq!(
                KeyHandler::map(key(KeyCode::Char(c))),
                expected,
                "binding char {c:?}",
            );
        }
    }

    #[test]
    fn unmapped_keys_return_none() {
        for code in [
            KeyCode::Tab,
            KeyCode::BackTab,
            KeyCode::Insert,
            KeyCode::F(1),
            KeyCode::Null,
            KeyCode::CapsLock,
        ] {
            assert_eq!(
                KeyHandler::map(key(code)),
                Action::None,
                "unmapped {code:?}"
            );
        }
    }

    #[test]
    fn control_modifier_only_maps_ctrl_c() {
        // Ctrl+C is the sole Control binding.
        assert_eq!(KeyHandler::map(ctrl(KeyCode::Char('c'))), Action::Quit);
        // Every other Control combination short-circuits to None — including
        // keys that WOULD map to an action without the Control modifier.
        for code in [
            KeyCode::Char('a'),
            KeyCode::Char('j'), // 'j' alone is Down; with Ctrl it is None
            KeyCode::Char('q'), // 'q' alone is Quit; with Ctrl it is None
            KeyCode::Char('C'), // uppercase 'C' is not the bound 'c'
            KeyCode::Up,
            KeyCode::Enter,
            KeyCode::Backspace,
        ] {
            assert_eq!(KeyHandler::map(ctrl(code)), Action::None, "ctrl+{code:?}");
        }
    }

    #[test]
    fn non_control_modifiers_pass_through_to_base_mapping() {
        // Only the Control modifier is inspected; Alt/Shift are ignored and the
        // base key mapping applies.
        assert_eq!(
            KeyHandler::map(with_mods(KeyCode::Char('j'), KeyModifiers::ALT)),
            Action::Down,
        );
        assert_eq!(
            KeyHandler::map(with_mods(KeyCode::Down, KeyModifiers::SHIFT)),
            Action::Down,
        );
        assert_eq!(
            KeyHandler::map(with_mods(KeyCode::Enter, KeyModifiers::ALT)),
            Action::Select,
        );
    }
}
