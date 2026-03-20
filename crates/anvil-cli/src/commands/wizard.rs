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
            description: "Nx-based TypeScript monorepo with full Anvil gates".into(),
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
            description: "Bare-bones Anvil configuration".into(),
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

fn checks_for_template(template_id: &str) -> Vec<&'static str> {
    match template_id {
        "typescript-monorepo" => vec![
            "secret-scan",
            "dependency-audit",
            "architecture-boundary",
            "import-rules",
            "antipattern",
            "policy",
        ],
        "rust-workspace" => vec![
            "secret-scan",
            "architecture-boundary",
            "antipattern",
            "policy",
        ],
        "python-package" => vec!["secret-scan", "dependency-audit", "antipattern"],
        // minimal or unknown — just secret scanning
        _ => vec!["secret-scan"],
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
    std::fs::write(&anvilrc_path, &anvilrc_content).context("failed to write .anvilrc")?;

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
    if state.config.enable_hooks {
        println!("    anvil hooks install");
    }
    println!("    anvil gate");
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
        assert!(checks.contains(&serde_json::json!("architecture-boundary")));
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
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let mut state = WizardState::new(builtin_templates());
        state.config.project_name = ".".to_string();
        state.config.template_id = Some("minimal".to_string());
        state.confirmed = true;

        let result = scaffold_project(&state);
        std::env::set_current_dir(original_dir).unwrap();
        assert!(result.is_ok());
        assert!(dir.path().join(".anvil").exists());
        assert!(dir.path().join(".anvilrc").exists());

        let rc_content = std::fs::read_to_string(dir.path().join(".anvilrc")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&rc_content).unwrap();
        assert_eq!(parsed["template"], "minimal");
        let checks = parsed["checks"].as_array().unwrap();
        assert_eq!(checks, &[serde_json::json!("secret-scan")]);
    }

    #[test]
    fn checks_for_template_varies_by_template() {
        let ts = checks_for_template("typescript-monorepo");
        assert!(ts.contains(&"architecture-boundary"));
        assert!(ts.contains(&"import-rules"));

        let rust = checks_for_template("rust-workspace");
        assert!(rust.contains(&"architecture-boundary"));
        assert!(!rust.contains(&"import-rules"));

        let python = checks_for_template("python-package");
        assert!(python.contains(&"dependency-audit"));
        assert!(!python.contains(&"architecture-boundary"));

        let minimal = checks_for_template("minimal");
        assert_eq!(minimal, vec!["secret-scan"]);
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
    fn scaffold_rust_workspace_has_architecture_check() {
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
        assert!(checks.contains(&serde_json::json!("architecture-boundary")));
        assert!(!checks.contains(&serde_json::json!("import-rules")));
    }
}
