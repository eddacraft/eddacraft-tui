/// A deterministic fix request emitted by an interactive surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixRequest {
    /// Apply a doctor auto-fix to the selected check index.
    DoctorCheck { index: usize },
    /// Apply a deterministic anti-pattern fix at a specific file location.
    AntiPatternWarning {
        file: String,
        line: usize,
        warning_id: String,
    },
    /// Remove a standalone `console.log` / `console.error` statement.
    AuditConsoleStatement { file: String, line: usize },
}
