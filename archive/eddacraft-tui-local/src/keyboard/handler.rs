use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

pub struct KeyHandler;

impl KeyHandler {
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
}
