//! Shared architecture check used by `gate` and `watch` (#3918).
//!
//! Watch must not invent a second classifier. Both surfaces resolve the
//! same architecture section and run [`anvil_architecture::validate_with_files_and_edges`]
//! so a forbidden dependency produces the configured boundary name
//! (`no-core-to-app`) on either command.

use std::path::Path;

use crate::util::is_ignored_dir_name;

const INCLUDE_EXTENSIONS: &[&str] = &[
    "ts", "tsx", "js", "jsx", "mjs", "cjs", "rs", "py", "pyi", "dart", "go", "java", "kt", "kts",
    "cs", "c", "h", "cpp", "cc", "cxx", "c++", "hpp", "hh", "hxx", "h++",
];

/// One configured boundary violation, using the same names as gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArchitectureFinding {
    pub policy_id: String,
    pub file: String,
    pub message: String,
}

/// Outcome of the shared architecture check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArchitectureCheckOutcome {
    Skipped { message: String },
    Passed { message: String },
    Violations { findings: Vec<ArchitectureFinding> },
    Failed { message: String },
}

/// Run gate's architecture check, optionally scoping import-edge extraction
/// to `changed_files` so watch stays save-time cheap.
pub(crate) fn check_architecture(
    project_root: &Path,
    changed_files: Option<&[String]>,
) -> ArchitectureCheckOutcome {
    let definition = match crate::architecture_source::resolve_architecture(project_root) {
        Ok(None) => {
            return ArchitectureCheckOutcome::Skipped {
                message: "No architecture config found (architecture section or \
                          .anvil/architecture.yaml). Skipping."
                    .to_string(),
            };
        }
        Ok(Some((definition, _origin))) => definition,
        Err(error) => {
            return ArchitectureCheckOutcome::Failed {
                message: format!("Architecture validation failed: {error:#}"),
            };
        }
    };

    let diagnostics = anvil_architecture::diagnose_definition(&definition);
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.is_error())
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        return ArchitectureCheckOutcome::Failed {
            message: format!(
                "Architecture config preflight failed:\n{}",
                errors
                    .iter()
                    .map(|diagnostic| diagnostic.message.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        };
    }

    let source_files = anvil_architecture::collect_source_files(project_root, &definition);
    let edges = extract_import_edges(project_root, Some(changed_files.unwrap_or(&source_files)));
    let result =
        anvil_architecture::validate_with_files_and_edges(&definition, &source_files, &edges);

    if result.valid {
        ArchitectureCheckOutcome::Passed {
            message: "Architecture config is valid".to_string(),
        }
    } else {
        let findings = result
            .violations
            .iter()
            .map(|violation| {
                let policy_id = violation
                    .boundary
                    .as_ref()
                    .map_or("unknown", |boundary| boundary.name.as_str())
                    .to_string();
                let message = violation
                    .boundary
                    .as_ref()
                    .map_or("boundary violation", |boundary| boundary.message.as_str())
                    .to_string();
                ArchitectureFinding {
                    policy_id,
                    file: violation.edge.from.clone(),
                    message,
                }
            })
            .collect();
        ArchitectureCheckOutcome::Violations { findings }
    }
}

/// Extract import edges from source files using the kernel's tree-sitter parser.
///
/// When `source_files` is provided, only those files are parsed (avoids a
/// redundant directory walk). Otherwise falls back to walking `project_root`.
pub(crate) fn extract_import_edges(
    project_root: &Path,
    source_files: Option<&[String]>,
) -> Vec<anvil_architecture::ImportEdge> {
    let mut parser = anvil_kernel::parser::Parser::new();
    let mut edges = Vec::new();

    let owned_paths: Vec<String>;
    let file_paths: &[String] = if let Some(files) = source_files {
        files
    } else {
        owned_paths = walk_source_files(project_root, INCLUDE_EXTENSIONS);
        &owned_paths
    };

    for rel_path in file_paths {
        let ext = Path::new(rel_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        if !INCLUDE_EXTENSIONS.contains(&ext) {
            continue;
        }

        let path = project_root.join(rel_path);
        let Ok(content) = std::fs::read(&path) else {
            continue;
        };
        let Ok(parse_result) = parser.parse_bytes(&path, &content) else {
            continue;
        };
        let file_symbols =
            anvil_kernel::parser::extract::extract_symbols(&parse_result.tree, &content, &path, 0);

        for import in &file_symbols.imports {
            let resolved = if ext == "py" || ext == "pyi" {
                anvil_architecture::resolve_python_import(project_root, rel_path, &import.to_source)
            } else if import.to_source.starts_with('.') {
                resolve_import(rel_path, &import.to_source)
            } else if import.to_source.contains("::") {
                anvil_architecture::resolve_rust_import(project_root, rel_path, &import.to_source)
            } else {
                None
            };

            if let Some(to_file) = resolved {
                edges.push(anvil_architecture::ImportEdge {
                    from_file: rel_path.clone(),
                    to_file,
                    line: import.line,
                });
            }
        }
    }

    edges
}

fn walk_source_files(project_root: &Path, extensions: &[&str]) -> Vec<String> {
    let walker = ignore::WalkBuilder::new(project_root)
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                return !is_ignored_dir_name(&name);
            }
            true
        })
        .build();

    let mut files = Vec::new();
    for entry in walker.filter_map(std::result::Result::ok) {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let path = entry.path();
        if !extensions.is_empty() {
            let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
            if !extensions.contains(&ext) {
                continue;
            }
        }
        let rel_path = path
            .strip_prefix(project_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        files.push(rel_path);
    }
    files
}

/// Resolve a relative import specifier to a workspace-relative path.
pub(crate) fn resolve_import(from_file: &str, specifier: &str) -> Option<String> {
    let from_dir = from_file.rsplit_once('/').map_or("", |(dir, _)| dir);
    let combined = if from_dir.is_empty() {
        specifier.to_string()
    } else {
        format!("{from_dir}/{specifier}")
    };

    let mut parts: Vec<&str> = Vec::new();
    for segment in combined.split('/') {
        match segment {
            "." | "" => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }

    if parts.is_empty() {
        return None;
    }

    Some(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rust_layers() -> &'static str {
        r#"
schema_version: "0.1.0"
layers:
  core:
    patterns: ["src/core/**"]
    depends_on: []
  app:
    patterns: ["src/app/**"]
    depends_on: ["core"]
"#
    }

    fn seed_rust_core_to_app(root: &Path) {
        std::fs::create_dir_all(root.join("src/core")).unwrap();
        std::fs::create_dir_all(root.join("src/app")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("src/core/entity.rs"),
            "use crate::app::service::Service;\npub struct Entity;\n",
        )
        .unwrap();
        std::fs::write(root.join("src/app/service.rs"), "pub struct Service;\n").unwrap();
    }

    #[test]
    fn rust_core_to_app_is_no_core_to_app() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".anvil")).unwrap();
        std::fs::write(tmp.path().join(".anvil/architecture.yaml"), rust_layers()).unwrap();
        seed_rust_core_to_app(tmp.path());

        let outcome = check_architecture(tmp.path(), Some(&["src/core/entity.rs".into()]));
        match outcome {
            ArchitectureCheckOutcome::Violations { findings } => {
                assert!(
                    findings
                        .iter()
                        .any(|finding| finding.policy_id == "no-core-to-app"
                            && finding.file == "src/core/entity.rs"),
                    "expected no-core-to-app, got {findings:?}"
                );
            }
            other => panic!("expected violations, got {other:?}"),
        }
    }

    #[test]
    fn missing_depends_on_is_preflight_failure() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "architecture:\n  schema_version: \"0.1.0\"\n  layers:\n    core:\n      patterns: [\"src/core/**\"]\n      depends_on: []\n    app:\n      patterns: [\"src/app/**\"]\n      depends_on: [missing]\n",
        )
        .unwrap();
        let outcome = check_architecture(tmp.path(), None);
        match outcome {
            ArchitectureCheckOutcome::Failed { message } => {
                assert!(message.contains("preflight"), "{message}");
                assert!(message.contains("unknown layer"), "{message}");
            }
            other => panic!("expected preflight failure, got {other:?}"),
        }
    }
}
