use std::collections::BTreeMap;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anvil_tui::surfaces::browser::{
    BrowserState, TemplateCategory, TemplateEntry, TemplateVariable,
};
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct NewArgs {
    /// Template ID to scaffold from (omit for interactive browser).
    template_id: Option<String>,

    /// Output file path (default: <template-id>.md).
    #[arg(short, long)]
    output: Option<String>,

    /// Overwrite an existing output file.
    #[arg(short, long)]
    force: bool,

    /// Set a template variable (repeatable: --var key=value).
    #[arg(long = "var", value_name = "KEY=VALUE")]
    vars: Vec<String>,

    /// List all available templates without launching the browser.
    #[arg(short, long)]
    list: bool,

    /// Filter templates by category.
    #[arg(short, long)]
    category: Option<String>,
}

pub fn run(args: &NewArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let (categories, templates, bodies) = load_templates()?;

    if templates.is_empty() {
        anyhow::bail!("no templates found");
    }

    if args.list {
        if global.json {
            print_json(&categories, &templates)?;
        } else {
            print_plain(&categories, &templates);
        }
        return Ok(());
    }

    let chosen_id = if let Some(ref id) = args.template_id {
        id.clone()
    } else if global.json {
        print_json(&categories, &templates)?;
        return Ok(());
    } else if !global.no_tui && std::io::stdout().is_terminal() {
        let state = BrowserState::new(categories.clone(), templates.clone());
        let state = crate::tui::run_surface(state)?;
        match state.chosen {
            Some(id) => id,
            None => return Ok(()),
        }
    } else {
        print_plain(&categories, &templates);
        return Ok(());
    };

    let template = templates
        .iter()
        .find(|t| t.id == chosen_id)
        .ok_or_else(|| anyhow::anyhow!("unknown template: {chosen_id}"))?;

    let body = bodies
        .get(&chosen_id)
        .ok_or_else(|| anyhow::anyhow!("template body not found: {chosen_id}"))?;

    let variables = parse_variables(&args.vars)?;
    let rendered = render_template(body, template, &variables)?;

    let output_path = resolve_output_path(args.output.as_deref(), &chosen_id)?;

    if output_path.exists() && !args.force {
        anyhow::bail!(
            "file already exists: {}  (use --force to overwrite)",
            output_path.display()
        );
    }

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&output_path, rendered)?;
    // Issue #3947: the scaffold path used to ignore an accepted `--json`
    // and print the human lines; the next-step hint rides as a field.
    if global.json {
        crate::output::json::print(&serde_json::json!({
            "template": template.id,
            "template_name": template.name,
            "output": output_path.display().to_string(),
            "next_steps": ["anvil doctor"],
        }))?;
    } else {
        println!(
            "Created {} from template \"{}\"\n",
            output_path.display(),
            template.name
        );
        println!("Next steps:");
        println!("  anvil doctor");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Template loading from disk
// ---------------------------------------------------------------------------

/// Locate the templates directory by checking:
/// 1. `ANVIL_TEMPLATES_DIR` env var
/// 2. `<binary-dir>/templates/` (adjacent to installed binary)
/// 3. `<binary-dir>/../../crates/anvil-cli/templates/` (cargo workspace layout)
/// 4. `<binary-dir>/../../apps/anvil-cli/templates/` (workspace development)
/// 5. `<workspace-root>/apps/anvil-cli/templates/` (workspace-relative fallback)
fn find_templates_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("ANVIL_TEMPLATES_DIR") {
        let p = PathBuf::from(dir);
        if p.is_dir() {
            return Some(p);
        }
    }

    if let Ok(exe) = std::env::current_exe()
        && let Some(bin_dir) = exe.parent()
    {
        // Check adjacent to binary (for installed crates).
        let candidate = bin_dir.join("templates");
        if candidate.is_dir() {
            return Some(candidate);
        }

        // In a cargo workspace: target/debug/anvil → ../../crates/anvil-cli/templates
        if let Some(parent) = bin_dir.parent()
            && let Some(grandparent) = parent.parent()
        {
            // Check crate-bundled templates first (works for cargo install).
            let crate_templates = grandparent.join("crates/anvil-cli/templates");
            if crate_templates.is_dir() {
                return Some(crate_templates);
            }
            // Fall back to apps/ path (workspace development).
            let apps_templates = grandparent.join("apps/anvil-cli/templates");
            if apps_templates.is_dir() {
                return Some(apps_templates);
            }
        }
    }

    // Fall back to workspace-relative path (useful during development)
    let workspace = crate::util::workspace_root().ok()?;
    let candidate = workspace.join("apps/anvil-cli/templates");
    if candidate.is_dir() {
        return Some(candidate);
    }

    None
}

#[derive(serde::Deserialize)]
struct TemplateFrontmatter {
    id: Option<String>,
    name: Option<String>,
    description: Option<String>,
    category: Option<String>,
    tags: Option<Vec<String>>,
    variables: Option<Vec<FrontmatterVariable>>,
}

#[derive(serde::Deserialize)]
struct FrontmatterVariable {
    name: String,
    description: Option<String>,
    #[serde(default)]
    required: bool,
    default: Option<serde_yaml::Value>,
}

type TemplateCatalogue = (
    Vec<TemplateCategory>,
    Vec<TemplateEntry>,
    BTreeMap<String, String>,
);

const EMBEDDED_TEMPLATES: &[(&str, &str)] = &[
    (
        "api-integration",
        include_str!("../../templates/api-integration.md"),
    ),
    (
        "authentication-jwt",
        include_str!("../../templates/authentication-jwt.md"),
    ),
    (
        "caching-layer",
        include_str!("../../templates/caching-layer.md"),
    ),
    (
        "ci-cd-pipeline",
        include_str!("../../templates/ci-cd-pipeline.md"),
    ),
    (
        "database-migration",
        include_str!("../../templates/database-migration.md"),
    ),
    (
        "error-handling",
        include_str!("../../templates/error-handling.md"),
    ),
    (
        "file-upload",
        include_str!("../../templates/file-upload.md"),
    ),
    (
        "frontend-component",
        include_str!("../../templates/frontend-component.md"),
    ),
    (
        "rest-api-crud",
        include_str!("../../templates/rest-api-crud.md"),
    ),
    (
        "testing-suite",
        include_str!("../../templates/testing-suite.md"),
    ),
    (
        "websocket-realtime",
        include_str!("../../templates/websocket-realtime.md"),
    ),
];

fn load_templates() -> anyhow::Result<TemplateCatalogue> {
    if let Some(dir) = find_templates_dir() {
        return load_templates_from_dir(&dir);
    }
    // Fall back to embedded templates for `cargo install` environments.
    Ok(load_embedded_templates())
}

fn load_embedded_templates() -> TemplateCatalogue {
    let mut entries: Vec<TemplateEntry> = Vec::new();
    let mut bodies: BTreeMap<String, String> = BTreeMap::new();

    for &(name, content) in EMBEDDED_TEMPLATES {
        match load_template_from_content(name, content) {
            Ok((te, body)) => {
                bodies.insert(te.id.clone(), body);
                entries.push(te);
            }
            Err(e) => {
                eprintln!("warning: skipping {name}: {e}");
            }
        }
    }

    entries.sort_by(|a, b| a.category.cmp(&b.category).then(a.id.cmp(&b.id)));
    let categories = derive_categories(&entries);
    (categories, entries, bodies)
}

fn load_template_from_content(
    name: &str,
    content: &str,
) -> anyhow::Result<(TemplateEntry, String)> {
    let (fm_str, body) = split_frontmatter(content)
        .ok_or_else(|| anyhow::anyhow!("missing frontmatter delimiters"))?;

    let fm: TemplateFrontmatter = serde_yaml::from_str(fm_str)?;

    let variables: Vec<TemplateVariable> = fm
        .variables
        .unwrap_or_default()
        .into_iter()
        .map(|v| {
            let default_value = v.default.map(|d| match d {
                serde_yaml::Value::String(s) => s,
                other => format!("{other:?}"),
            });
            TemplateVariable {
                name: v.name,
                description: v.description.unwrap_or_default(),
                default_value,
                required: v.required,
            }
        })
        .collect();

    Ok((
        TemplateEntry {
            id: fm.id.unwrap_or_else(|| name.to_string()),
            name: fm.name.unwrap_or_else(|| name.replace('-', " ")),
            description: fm.description.unwrap_or_default(),
            category: fm.category.unwrap_or_else(|| "general".to_string()),
            tags: fm.tags.unwrap_or_default(),
            variables,
        },
        body.to_string(),
    ))
}

fn load_templates_from_dir(dir: &Path) -> anyhow::Result<TemplateCatalogue> {
    let mut entries: Vec<TemplateEntry> = Vec::new();
    let mut bodies: BTreeMap<String, String> = BTreeMap::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        match load_single_template(&path) {
            Ok((te, body)) => {
                bodies.insert(te.id.clone(), body);
                entries.push(te);
            }
            Err(e) => {
                eprintln!("warning: skipping {}: {e}", path.display());
            }
        }
    }

    entries.sort_by(|a, b| a.category.cmp(&b.category).then(a.id.cmp(&b.id)));

    let categories = derive_categories(&entries);

    Ok((categories, entries, bodies))
}

fn load_single_template(path: &Path) -> anyhow::Result<(TemplateEntry, String)> {
    let content = std::fs::read_to_string(path)?;

    let (fm_str, body) = split_frontmatter(&content)
        .ok_or_else(|| anyhow::anyhow!("missing frontmatter delimiters"))?;

    let fm: TemplateFrontmatter = serde_yaml::from_str(fm_str)?;

    let file_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    let id = fm.id.unwrap_or_else(|| file_stem.to_string());

    let variables: Vec<TemplateVariable> = fm
        .variables
        .unwrap_or_default()
        .into_iter()
        .map(|v| {
            let default_value = v.default.map(|d| match d {
                serde_yaml::Value::String(s) => s,
                other => format!("{other:?}"),
            });
            TemplateVariable {
                name: v.name,
                description: v.description.unwrap_or_default(),
                default_value,
                required: v.required,
            }
        })
        .collect();

    let entry = TemplateEntry {
        id: id.clone(),
        name: fm.name.unwrap_or_else(|| file_stem.replace('-', " ")),
        description: fm.description.unwrap_or_default(),
        category: fm.category.unwrap_or_else(|| "general".into()),
        tags: fm.tags.unwrap_or_default(),
        variables,
    };

    Ok((entry, body.to_string()))
}

fn split_frontmatter(content: &str) -> Option<(&str, &str)> {
    // Handle both Unix (\n) and Windows (\r\n) line endings so templates
    // authored on Windows with CRLF parse correctly.
    let content = content
        .strip_prefix("---\r\n")
        .or_else(|| content.strip_prefix("---\n"))?;

    if let Some(end) = content.find("\r\n---\r\n") {
        let fm = &content[..end];
        let body = &content[end + 7..]; // skip "\r\n---\r\n"
        Some((fm, body))
    } else {
        let end = content.find("\n---\n")?;
        let fm = &content[..end];
        let body = &content[end + 5..]; // skip "\n---\n"
        Some((fm, body))
    }
}

fn derive_categories(templates: &[TemplateEntry]) -> Vec<TemplateCategory> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for t in templates {
        *counts.entry(t.category.as_str()).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(name, count)| TemplateCategory {
            name: name.to_string(),
            description: format!("{name} templates"),
            template_count: count,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Template rendering
// ---------------------------------------------------------------------------

fn render_template(
    body: &str,
    template: &TemplateEntry,
    variables: &BTreeMap<String, String>,
) -> anyhow::Result<String> {
    // Check for missing required variables
    let missing: Vec<&str> = template
        .variables
        .iter()
        .filter(|v| v.required && variables.get(&v.name).is_none() && v.default_value.is_none())
        .map(|v| v.name.as_str())
        .collect();

    if !missing.is_empty() {
        anyhow::bail!("missing required variables: {}", missing.join(", "));
    }

    let mut rendered = body.to_string();

    // Substitute provided variables
    for (key, value) in variables {
        rendered = apply_template_variable(rendered, key, value);
    }

    // Substitute defaults for any remaining placeholders
    for var in &template.variables {
        if let Some(ref default) = var.default_value {
            rendered = apply_template_variable(rendered, &var.name, default);
        }
    }

    Ok(rendered)
}

fn apply_template_variable(mut rendered: String, key: &str, value: &str) -> String {
    let patterns = [
        (format!("{{{{ {key} }}}}"), value.to_string()),
        (format!("{{{{{key}}}}}"), value.to_string()),
        (format!("{{{{ {key} | uppercase }}}}"), value.to_uppercase()),
        (format!("{{{{{key}|uppercase}}}}"), value.to_uppercase()),
    ];

    for (pattern, replacement) in patterns {
        rendered = rendered.replace(&pattern, &replacement);
    }

    rendered
}

fn parse_variables(vars: &[String]) -> anyhow::Result<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    for v in vars {
        let eq = v
            .find('=')
            .ok_or_else(|| anyhow::anyhow!("invalid variable format: {v} (expected key=value)"))?;
        let key = v[..eq].to_string();
        let value = v[eq + 1..].to_string();
        map.insert(key, value);
    }
    Ok(map)
}

// ---------------------------------------------------------------------------
// Path safety
// ---------------------------------------------------------------------------

fn resolve_output_path(output: Option<&str>, template_id: &str) -> anyhow::Result<PathBuf> {
    let default_name = format!("{template_id}.md");
    let raw = output.unwrap_or(&default_name);
    let raw_path = Path::new(raw);

    let root = crate::util::workspace_root()?;

    let resolved = if raw_path.is_absolute() {
        raw_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(raw_path)
    };

    let canonical_root = std::fs::canonicalize(&root).unwrap_or_else(|_| root.clone());

    // For new files the path itself won't exist yet, so canonicalise the parent.
    let canonical_resolved = if resolved.exists() {
        std::fs::canonicalize(&resolved)?
    } else {
        let parent = resolved
            .parent()
            .ok_or_else(|| anyhow::anyhow!("cannot determine parent directory"))?;

        // Canonicalise parent if it exists; otherwise normalise path segments
        // to prevent traversal attacks via ".." components.
        let canonical_parent = if parent.exists() {
            std::fs::canonicalize(parent)?
        } else {
            normalize_path(parent)
        };
        canonical_parent.join(resolved.file_name().unwrap_or_default())
    };

    if !canonical_resolved.starts_with(&canonical_root) {
        anyhow::bail!(
            "output path escapes workspace root: {} is outside {}",
            canonical_resolved.display(),
            canonical_root.display()
        );
    }

    Ok(resolved)
}

/// Normalise a path by resolving `.` and `..` segments without requiring
/// the path to exist on disk. Prevents traversal attacks on non-existent paths.
fn normalize_path(path: &std::path::Path) -> std::path::PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                // Never pop past the root/prefix — prevents turning absolute paths relative.
                if components.last().is_some_and(|c| {
                    !matches!(
                        c,
                        std::path::Component::RootDir | std::path::Component::Prefix(_)
                    )
                }) {
                    components.pop();
                }
            }
            other => components.push(other),
        }
    }
    components.iter().collect()
}

// ---------------------------------------------------------------------------
// Output: plain text
// ---------------------------------------------------------------------------

fn print_plain(categories: &[TemplateCategory], templates: &[TemplateEntry]) {
    println!("ANVIL TEMPLATES\n");

    for cat in categories {
        println!("  {} \u{2014} {}", cat.name, cat.description);
        let cat_templates: Vec<&TemplateEntry> = templates
            .iter()
            .filter(|t| t.category == cat.name)
            .collect();
        for t in cat_templates {
            println!("    {:<25}{}", t.id, t.name);
        }
        println!();
    }

    println!("Use `anvil new` in a terminal for the interactive browser.");
}

// ---------------------------------------------------------------------------
// Output: JSON
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct CatalogueOutput {
    categories: Vec<CategoryOutput>,
    templates: Vec<TemplateOutput>,
}

#[derive(Serialize)]
struct CategoryOutput {
    name: String,
    description: String,
    template_count: usize,
}

#[derive(Serialize)]
struct TemplateOutput {
    id: String,
    name: String,
    description: String,
    category: String,
    tags: Vec<String>,
    variables: Vec<VariableOutput>,
}

#[derive(Serialize)]
struct VariableOutput {
    name: String,
    description: String,
    default_value: Option<String>,
    required: bool,
}

fn print_json(categories: &[TemplateCategory], templates: &[TemplateEntry]) -> anyhow::Result<()> {
    let output = CatalogueOutput {
        categories: categories
            .iter()
            .map(|c| CategoryOutput {
                name: c.name.clone(),
                description: c.description.clone(),
                template_count: c.template_count,
            })
            .collect(),
        templates: templates
            .iter()
            .map(|t| TemplateOutput {
                id: t.id.clone(),
                name: t.name.clone(),
                description: t.description.clone(),
                category: t.category.clone(),
                tags: t.tags.clone(),
                variables: t
                    .variables
                    .iter()
                    .map(|v| VariableOutput {
                        name: v.name.clone(),
                        description: v.description.clone(),
                        default_value: v.default_value.clone(),
                        required: v.required,
                    })
                    .collect(),
            })
            .collect(),
    };

    let json = serde_json::to_string_pretty(&output)?;
    println!("{json}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn write_template(dir: &Path, name: &str, content: &str) {
        std::fs::write(dir.join(name), content).unwrap();
    }

    fn sample_frontmatter() -> &'static str {
        "---\nid: test-tmpl\nname: Test Template\ndescription: A test template\ncategory: testing\ntags: [test, sample]\nvariables:\n- name: project_name\n  description: Project name\n  default: my-project\n  required: true\n---\n# Hello {{ project_name }}\n"
    }

    #[test]
    fn split_frontmatter_parses() {
        let (fm, body) = split_frontmatter(sample_frontmatter()).unwrap();
        assert!(fm.contains("id: test-tmpl"));
        assert!(body.contains("Hello"));
    }

    #[test]
    fn split_frontmatter_parses_crlf() {
        let crlf = "---\r\nid: test-tmpl\r\nname: CRLF Template\r\n---\r\n# Body\r\n";
        let (fm, body) = split_frontmatter(crlf).unwrap();
        assert!(fm.contains("id: test-tmpl"));
        assert!(body.contains("Body"));
    }

    #[test]
    fn load_single_template_from_file() {
        let dir = tempfile::tempdir().unwrap();
        write_template(dir.path(), "example.md", sample_frontmatter());

        let (entry, body) = load_single_template(&dir.path().join("example.md")).unwrap();
        assert_eq!(entry.id, "test-tmpl");
        assert_eq!(entry.name, "Test Template");
        assert_eq!(entry.category, "testing");
        assert_eq!(entry.variables.len(), 1);
        assert!(body.contains("Hello"));
    }

    #[test]
    fn derive_categories_groups() {
        let entries = vec![
            TemplateEntry {
                id: "a".into(),
                name: "A".into(),
                description: String::new(),
                category: "cat1".into(),
                tags: vec![],
                variables: vec![],
            },
            TemplateEntry {
                id: "b".into(),
                name: "B".into(),
                description: String::new(),
                category: "cat1".into(),
                tags: vec![],
                variables: vec![],
            },
            TemplateEntry {
                id: "c".into(),
                name: "C".into(),
                description: String::new(),
                category: "cat2".into(),
                tags: vec![],
                variables: vec![],
            },
        ];
        let cats = derive_categories(&entries);
        assert_eq!(cats.len(), 2);
        assert_eq!(cats[0].template_count, 2);
        assert_eq!(cats[1].template_count, 1);
    }

    #[test]
    fn render_substitutes_variables() {
        let body = "Hello {{ name }}, version {{version}}, env {{ name | uppercase }}";
        let template = TemplateEntry {
            id: "t".into(),
            name: "T".into(),
            description: String::new(),
            category: "c".into(),
            tags: vec![],
            variables: vec![
                TemplateVariable {
                    name: "name".into(),
                    description: String::new(),
                    default_value: None,
                    required: true,
                },
                TemplateVariable {
                    name: "version".into(),
                    description: String::new(),
                    default_value: Some("1.0".into()),
                    required: false,
                },
            ],
        };
        let mut vars = BTreeMap::new();
        vars.insert("name".into(), "Anvil".into());

        let result = render_template(body, &template, &vars).unwrap();
        assert_eq!(result, "Hello Anvil, version 1.0, env ANVIL");
    }

    #[test]
    fn render_rejects_missing_required() {
        let body = "{{ name }}";
        let template = TemplateEntry {
            id: "t".into(),
            name: "T".into(),
            description: String::new(),
            category: "c".into(),
            tags: vec![],
            variables: vec![TemplateVariable {
                name: "name".into(),
                description: String::new(),
                default_value: None,
                required: true,
            }],
        };
        let vars = BTreeMap::new();
        assert!(render_template(body, &template, &vars).is_err());
    }

    #[test]
    fn parse_variables_ok() {
        let vars = vec!["key=value".into(), "foo=bar=baz".into()];
        let map = parse_variables(&vars).unwrap();
        assert_eq!(map["key"], "value");
        assert_eq!(map["foo"], "bar=baz");
    }

    #[test]
    fn parse_variables_rejects_bad_format() {
        let vars = vec!["noequals".into()];
        assert!(parse_variables(&vars).is_err());
    }

    #[test]
    fn resolve_output_rejects_escape() {
        let result = resolve_output_path(Some("/tmp/evil.md"), "test");
        assert!(result.is_err());
    }

    #[test]
    fn json_output_serialises() {
        let cats = [TemplateCategory {
            name: "test".into(),
            description: "desc".into(),
            template_count: 1,
        }];
        let tmpls = [TemplateEntry {
            id: "t1".into(),
            name: "T1".into(),
            description: "d".into(),
            category: "test".into(),
            tags: vec![],
            variables: vec![],
        }];

        let output = CatalogueOutput {
            categories: cats
                .iter()
                .map(|c| CategoryOutput {
                    name: c.name.clone(),
                    description: c.description.clone(),
                    template_count: c.template_count,
                })
                .collect(),
            templates: tmpls
                .iter()
                .map(|t| TemplateOutput {
                    id: t.id.clone(),
                    name: t.name.clone(),
                    description: t.description.clone(),
                    category: t.category.clone(),
                    tags: t.tags.clone(),
                    variables: vec![],
                })
                .collect(),
        };

        let json = serde_json::to_string(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["categories"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["templates"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn load_templates_from_disk_dir() {
        let dir = tempfile::tempdir().unwrap();
        write_template(
            dir.path(),
            "alpha.md",
            "---\nid: alpha\nname: Alpha\ncategory: cat-a\n---\nBody A\n",
        );
        write_template(
            dir.path(),
            "beta.md",
            "---\nid: beta\nname: Beta\ncategory: cat-b\n---\nBody B\n",
        );

        let (cats, tmpls, bodies) = load_templates_from_dir(dir.path()).unwrap();

        assert_eq!(tmpls.len(), 2);
        assert_eq!(cats.len(), 2);
        assert!(bodies.contains_key("alpha"));
        assert!(bodies.contains_key("beta"));
    }
}
