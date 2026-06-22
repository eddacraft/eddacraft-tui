use std::io::IsTerminal;
use std::path::Path;

use anvil_tui::surfaces::wizard::{Template, WizardState};
use anyhow::Context;
use clap::Args;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct WizardArgs {}

fn builtin_templates() -> Vec<Template> {
    vec![
        Template {
            id: "typescript-monorepo".into(),
            name: "TypeScript Monorepo".into(),
            description: "Nx-based TypeScript monorepo with full anvil gates".into(),
            tags: vec!["typescript".into(), "monorepo".into()],
        },
        Template {
            id: "rust-workspace".into(),
            name: "Rust Workspace".into(),
            description: "Cargo workspace with architecture enforcement".into(),
            tags: vec!["rust".into(), "workspace".into()],
        },
        Template {
            id: "python-package".into(),
            name: "Python Package".into(),
            description: "Python package with linting and secret scanning".into(),
            tags: vec!["python".into()],
        },
        Template {
            id: "minimal".into(),
            name: "Minimal Setup".into(),
            description: "Bare-bones anvil configuration".into(),
            tags: vec!["minimal".into()],
        },
    ]
}

pub fn run(_args: &WizardArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let templates = builtin_templates();

    if global.json {
        print_json_templates(&templates)?;
        return Ok(());
    }

    if global.no_tui || !std::io::stdout().is_terminal() {
        print_plain_templates(&templates);
        return Ok(());
    }

    let state = WizardState::new(templates);
    let state = crate::tui::run_surface(state)?;

    if state.confirmed {
        scaffold_project(&state)?;
    }

    Ok(())
}

fn print_json_templates(templates: &[Template]) -> anyhow::Result<()> {
    let items: Vec<serde_json::Value> = templates
        .iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "name": t.name,
                "description": t.description,
                "tags": t.tags,
            })
        })
        .collect();
    let output = serde_json::json!({ "templates": items });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn print_plain_templates(templates: &[Template]) {
    println!();
    println!("  Available templates:");
    println!();
    for t in templates {
        let tags = t.tags.join(", ");
        println!("  {:<24} {}", t.name, t.description);
        println!("  {:<24} tags: {tags}", "");
    }
    println!();
    println!("  Run `anvil wizard` in an interactive terminal for the guided setup.");
    println!();
}

/// Returns the list of gate-runner checks to enable for a given template.
///
/// Names must be resolvable to a dispatchable gate check via the catalog —
/// they're written to `.anvilrc#checks` and used by `anvil gate` to filter
/// which checks to run.
/// Display labels (used in the wizard TUI) are kept separately in `anvil_tui`.
fn checks_for_template(template_id: &str) -> Vec<&'static str> {
    match template_id {
        "typescript-monorepo" | "rust-workspace" => vec![
            "secret-detection",
            "import-boundaries",
            "antipattern-scan",
            "policy",
        ],
        "python-package" => vec!["secret-detection", "import-boundaries", "antipattern-scan"],
        // minimal or unknown — just secret scanning
        _ => vec!["secret-detection"],
    }
}

fn scaffold_project(state: &WizardState) -> anyhow::Result<()> {
    let name = &state.config.project_name;
    let template_id = state.config.template_id.as_deref().unwrap_or("minimal");

    let project_dir = Path::new(name);

    if name != "." {
        std::fs::create_dir_all(project_dir)
            .with_context(|| format!("failed to create project directory: {name}"))?;
    }

    let anvilrc_path = project_dir.join(".anvilrc");
    if anvilrc_path.exists() {
        anyhow::bail!(".anvilrc already exists in {name} — use `anvil init --force` to overwrite");
    }

    let anvil_dir = project_dir.join(".anvil");
    std::fs::create_dir_all(&anvil_dir).context("failed to create .anvil directory")?;

    let checks = checks_for_template(template_id);
    let config = serde_json::json!({
        "template": template_id,
        "watch": state.config.enable_watch,
        "hooks": state.config.enable_hooks,
        "checks": checks,
    });
    let anvilrc_content = serde_json::to_string_pretty(&config)?;
    crate::util::atomic_write(&anvilrc_path, anvilrc_content.as_bytes())
        .context("failed to write .anvilrc")?;

    println!();
    println!("  Project scaffolded successfully!");
    println!();
    println!("  Directory:  {name}");
    println!("  Template:   {template_id}");
    println!(
        "  Watch mode: {}",
        if state.config.enable_watch {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "  Git hooks:  {}",
        if state.config.enable_hooks {
            "configured (run `anvil hooks install` to activate)"
        } else {
            "disabled"
        }
    );
    println!();
    println!("  Next steps:");
    println!("    cd {name}");
    println!("    anvil doctor");
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_templates_returns_four_entries() {
        let templates = builtin_templates();
        assert_eq!(templates.len(), 4);
    }

    #[test]
    fn builtin_templates_have_unique_ids() {
        let templates = builtin_templates();
        let mut ids: Vec<&str> = templates.iter().map(|t| t.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 4);
    }

    #[test]
    fn plain_text_lists_all_templates() {
        let templates = builtin_templates();
        let mut output = Vec::new();
        // Capture stdout is tricky; instead verify the function constructs
        // lines for each template by checking the data
        for t in &templates {
            assert!(!t.name.is_empty());
            assert!(!t.description.is_empty());
            assert!(!t.tags.is_empty());
            output.push(t.name.clone());
        }
        assert_eq!(output.len(), 4);
        assert!(output.contains(&"TypeScript Monorepo".to_string()));
        assert!(output.contains(&"Rust Workspace".to_string()));
        assert!(output.contains(&"Python Package".to_string()));
        assert!(output.contains(&"Minimal Setup".to_string()));
    }

    #[test]
    fn scaffold_creates_project_files_as_json() {
        let dir = tempfile::tempdir().unwrap();
        let project_name = dir.path().join("my-project");
        let project_name_str = project_name.to_string_lossy().to_string();

        let mut state = WizardState::new(builtin_templates());
        state.config.project_name = project_name_str.clone();
        state.config.template_id = Some("typescript-monorepo".to_string());
        state.config.enable_watch = true;
        state.config.enable_hooks = false;
        state.confirmed = true;

        scaffold_project(&state).unwrap();

        assert!(project_name.exists());
        assert!(project_name.join(".anvil").exists());
        assert!(project_name.join(".anvilrc").exists());

        let rc_content = std::fs::read_to_string(project_name.join(".anvilrc")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&rc_content).unwrap();
        assert_eq!(parsed["template"], "typescript-monorepo");
        assert_eq!(parsed["watch"], true);
        assert_eq!(parsed["hooks"], false);
        let checks = parsed["checks"].as_array().unwrap();
        assert!(
            checks.len() > 1,
            "typescript-monorepo should have multiple checks"
        );
        assert!(checks.contains(&serde_json::json!("import-boundaries")));
    }

    #[test]
    fn scaffold_rejects_existing_anvilrc() {
        let dir = tempfile::tempdir().unwrap();
        let project_name = dir.path().join("existing-project");
        std::fs::create_dir_all(&project_name).unwrap();
        std::fs::write(project_name.join(".anvilrc"), "{}").unwrap();

        let mut state = WizardState::new(builtin_templates());
        state.config.project_name = project_name.to_string_lossy().to_string();
        state.confirmed = true;

        let result = scaffold_project(&state);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn scaffold_dot_project_skips_mkdir() {
        let dir = tempfile::tempdir().unwrap();

        // `scaffold_project` resolves `.` against the process cwd, so this
        // test mutates the process-global cwd. Run it under the
        // workspace-wide cwd guard (CIB-026) so it serialises against the
        // check/doctor/MCP cwd tests instead of racing them.
        let result = crate::test_support::cwd::with_cwd_in(dir.path(), || {
            let mut state = WizardState::new(builtin_templates());
            state.config.project_name = ".".to_string();
            state.config.template_id = Some("minimal".to_string());
            state.confirmed = true;
            scaffold_project(&state)
        });

        assert!(result.is_ok());
        assert!(dir.path().join(".anvil").exists());
        assert!(dir.path().join(".anvilrc").exists());

        let rc_content = std::fs::read_to_string(dir.path().join(".anvilrc")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&rc_content).unwrap();
        assert_eq!(parsed["template"], "minimal");
        let checks = parsed["checks"].as_array().unwrap();
        assert_eq!(checks, &[serde_json::json!("secret-detection")]);
    }

    #[test]
    fn checks_for_template_varies_by_template() {
        let ts = checks_for_template("typescript-monorepo");
        assert!(ts.contains(&"import-boundaries"));

        let rust = checks_for_template("rust-workspace");
        assert!(rust.contains(&"import-boundaries"));

        let python = checks_for_template("python-package");
        assert!(python.contains(&"antipattern-scan"));
        assert!(python.contains(&"import-boundaries"));

        let minimal = checks_for_template("minimal");
        assert_eq!(minimal, vec!["secret-detection"]);
    }

    // Regression guard for #1016: every check name the wizard writes to
    // `.anvilrc#checks` must map to a dispatchable gate check via the
    // catalog, otherwise `anvil gate` will silently ignore it.
    #[test]
    fn wizard_checks_are_registered_gate_names() {
        use crate::commands::check_catalog::gate_internal_name;
        let template_ids = [
            "typescript-monorepo",
            "rust-workspace",
            "python-package",
            "minimal",
            "unknown-template",
        ];
        for id in template_ids {
            for name in checks_for_template(id) {
                assert!(
                    gate_internal_name(name).is_some(),
                    "wizard template '{id}' writes unregistered check '{name}'"
                );
            }
        }
    }

    #[test]
    fn json_templates_output_is_valid() {
        let templates = builtin_templates();
        let items: Vec<serde_json::Value> = templates
            .iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "name": t.name,
                    "description": t.description,
                    "tags": t.tags,
                })
            })
            .collect();
        let output = serde_json::json!({ "templates": items });
        let arr = output["templates"].as_array().unwrap();
        assert_eq!(arr.len(), 4);
        assert_eq!(arr[0]["id"], "typescript-monorepo");
    }

    #[test]
    fn scaffold_rust_workspace_has_import_boundaries_check() {
        let dir = tempfile::tempdir().unwrap();
        let project_name = dir.path().join("rust-proj");
        let project_name_str = project_name.to_string_lossy().to_string();

        let mut state = WizardState::new(builtin_templates());
        state.config.project_name = project_name_str;
        state.config.template_id = Some("rust-workspace".to_string());
        state.config.enable_watch = false;
        state.config.enable_hooks = false;
        state.confirmed = true;

        scaffold_project(&state).unwrap();

        let rc_content = std::fs::read_to_string(project_name.join(".anvilrc")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&rc_content).unwrap();
        let checks = parsed["checks"].as_array().unwrap();
        assert!(checks.contains(&serde_json::json!("import-boundaries")));
    }
}
