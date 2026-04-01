//! Terminal detection and minimum size validation.

const MIN_COLS: u16 = 80;
const MIN_ROWS: u16 = 24;

/// Information about the current terminal.
#[derive(Debug, Clone)]
pub struct TerminalInfo {
    pub cols: u16,
    pub rows: u16,
    pub term: String,
}

/// Detect the current terminal dimensions and type.
///
/// Uses `crossterm::terminal::size()` for dimensions and the `$TERM`
/// environment variable for terminal identification.
pub fn detect_terminal() -> TerminalInfo {
    let (cols, rows) = crossterm::terminal::size().unwrap_or((MIN_COLS, MIN_ROWS));
    let term = std::env::var("TERM").unwrap_or_else(|_| "unknown".to_string());

    TerminalInfo { cols, rows, term }
}

/// Validate that the terminal meets minimum size requirements (80x24).
pub fn validate_minimum_size(info: &TerminalInfo) -> Result<(), String> {
    if info.cols < MIN_COLS && info.rows < MIN_ROWS {
        return Err(format!(
            "terminal too small: {}x{} — minimum is {MIN_COLS}x{MIN_ROWS}",
            info.cols, info.rows
        ));
    }

    if info.cols < MIN_COLS {
        return Err(format!(
            "terminal too narrow: {} columns — minimum is {MIN_COLS}",
            info.cols
        ));
    }

    if info.rows < MIN_ROWS {
        return Err(format!(
            "terminal too short: {} rows — minimum is {MIN_ROWS}",
            info.rows
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(cols: u16, rows: u16) -> TerminalInfo {
        TerminalInfo {
            cols,
            rows,
            term: "xterm-256color".to_string(),
        }
    }

    #[test]
    fn validates_80x24_passes() {
        assert!(validate_minimum_size(&info(80, 24)).is_ok());
    }

    #[test]
    fn validates_larger_passes() {
        assert!(validate_minimum_size(&info(120, 40)).is_ok());
    }

    #[test]
    fn validates_79x24_fails() {
        let result = validate_minimum_size(&info(79, 24));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("narrow"));
    }

    #[test]
    fn validates_80x23_fails() {
        let result = validate_minimum_size(&info(80, 23));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("short"));
    }

    #[test]
    fn validates_both_too_small() {
        let result = validate_minimum_size(&info(40, 10));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too small"));
    }

    #[test]
    fn detect_terminal_returns_info() {
        // Smoke test — just ensure it does not panic
        let info = detect_terminal();
        assert!(!info.term.is_empty());
    }
}
