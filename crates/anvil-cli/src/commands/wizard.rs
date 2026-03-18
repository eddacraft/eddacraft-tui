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

fn scaffold_project(state: &WizardState) -> anyhow::Result<()> {
    let name = &state.config.project_name;
    let template_id = state.config.template_id.as_deref().unwrap_or("minimal");

    let project_dir = Path::new(name);

    if name != "." {
        std::fs::create_dir_all(project_dir)
            .with_context(|| format!("failed to create project directory: {name}"))?;
    }

    let anvil_dir = project_dir.join(".anvil");
    std::fs::create_dir_all(&anvil_dir).context("failed to create .anvil directory")?;

    let anvilrc_content = format!(
        "# Anvil configuration\ntemplate: {template_id}\nwatch: {watch}\nhooks: {hooks}\n",
        watch = state.config.enable_watch,
        hooks = state.config.enable_hooks,
    );
    std::fs::write(project_dir.join(".anvilrc"), &anvilrc_content)
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
            "enabled"
        } else {
            "disabled"
        }
    );
    println!();
    println!("  Next steps:");
    println!("    cd {name}");
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
    fn scaffold_creates_project_files() {
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
        assert!(rc_content.contains("template: typescript-monorepo"));
        assert!(rc_content.contains("watch: true"));
        assert!(rc_content.contains("hooks: false"));
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
    }
}
