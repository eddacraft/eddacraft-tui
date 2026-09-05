//! Shared gate-time source scan for CLI and MCP (CONV-002 / ADR-140).
//!
//! Callers own admission, path selection and presentation. This internal
//! handler is not a wire contract or a daemon-owned operation.

use anvil_checks::antipattern::{
    AntipatternCheckConfig, AntipatternCheckResult, run_antipattern_check,
};
use anvil_checks_ast::{AstScanOptions, AstScanOutput};

/// Keep both typed tier results intact: CLI needs AST diagnostics and SARIF
/// provenance, while MCP projects the combined findings into its legacy shape.
pub(crate) struct SourceScanResult {
    pub(crate) regex: AntipatternCheckResult,
    pub(crate) ast: AstScanOutput,
}

/// Scan the caller-selected files without changing its admission or selection
/// policy. Both tiers receive the same opt-in setting and workspace root.
/// This stays in the application crate: ADR-064/071 forbid the AST tier in
/// the resident intercept daemon.
pub(crate) fn scan_source_files(
    files: &[&str],
    config: &AntipatternCheckConfig,
    workspace_root: Option<&str>,
) -> SourceScanResult {
    SourceScanResult {
        regex: run_antipattern_check(files, config, workspace_root),
        ast: anvil_checks_ast::scan_paths(
            files,
            workspace_root,
            &AstScanOptions {
                registry_path: None,
                include_opt_in: config.include_opt_in,
            },
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_scan_retains_ast_findings_and_relative_locations() {
        let workspace = tempfile::tempdir().expect("workspace");
        let path = workspace.path().join("source.rs");
        std::fs::write(
            &path,
            "pub fn value(input: Option<u8>) -> u8 { input.unwrap() }\n",
        )
        .expect("source");
        let path = path.to_str().expect("UTF-8 path");
        let root = workspace.path().to_str().expect("UTF-8 root");
        let result = scan_source_files(&[path], &AntipatternCheckConfig::default(), Some(root));

        assert_eq!(result.regex.files_scanned, 1);
        assert_eq!(result.ast.files_scanned, 1);
        assert!(
            result.ast.init_errors.is_empty(),
            "{:?}",
            result.ast.init_errors
        );
        assert!(
            result
                .ast
                .warnings
                .iter()
                .any(|warning| { warning.id == "RS-001" && warning.location.file == "source.rs" })
        );
    }

    #[test]
    fn source_scan_preserves_regex_exclusion_configuration() {
        let workspace = tempfile::tempdir().expect("workspace");
        let path = workspace.path().join("excluded.ts");
        std::fs::write(&path, "export const value = 1;\n").expect("source");
        let path = path.to_str().expect("UTF-8 path");
        let root = workspace.path().to_str().expect("UTF-8 root");
        let config = AntipatternCheckConfig {
            exclude_globs: vec!["excluded.ts".into()],
            ..AntipatternCheckConfig::default()
        };

        let excluded = scan_source_files(&[path], &config, Some(root));
        assert_eq!(excluded.regex.files_scanned, 0);
        let included = scan_source_files(&[path], &AntipatternCheckConfig::default(), Some(root));
        assert_eq!(included.regex.files_scanned, 1);
    }
}
