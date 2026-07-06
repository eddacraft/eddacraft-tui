use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchitectureConfigDiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArchitectureConfigDiagnostic {
    pub severity: ArchitectureConfigDiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub section: String,
    pub key: String,
}

impl ArchitectureConfigDiagnostic {
    pub fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        section: impl Into<String>,
        key: impl Into<String>,
    ) -> Self {
        Self {
            severity: ArchitectureConfigDiagnosticSeverity::Error,
            code: code.into(),
            message: message.into(),
            section: section.into(),
            key: key.into(),
        }
    }

    pub fn warning(
        code: impl Into<String>,
        message: impl Into<String>,
        section: impl Into<String>,
        key: impl Into<String>,
    ) -> Self {
        Self {
            severity: ArchitectureConfigDiagnosticSeverity::Warning,
            code: code.into(),
            message: message.into(),
            section: section.into(),
            key: key.into(),
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == ArchitectureConfigDiagnosticSeverity::Error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn architecture_config_diagnostics_include_section_and_key() {
        let diagnostic = ArchitectureConfigDiagnostic::error(
            "unknown-layer",
            "layer depends on unknown layer",
            "layers.api.allowed_imports",
            "domain",
        );

        assert!(diagnostic.is_error());
        assert_eq!(diagnostic.section, "layers.api.allowed_imports");
        assert_eq!(diagnostic.key, "domain");
    }
}
