use std::io::IsTerminal;

use anvil_tui::surfaces::browser::{
    BrowserState, TemplateCategory, TemplateEntry, TemplateVariable,
};
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct NewArgs {}

pub fn run(_args: &NewArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let categories = builtin_categories();
    let templates = builtin_templates();

    if global.json {
        print_json(&categories, &templates)?;
    } else if !global.no_tui && std::io::stdout().is_terminal() {
        let state = BrowserState::new(categories, templates);
        let state = crate::tui::run_surface(state)?;
        if let Some(ref chosen) = state.chosen {
            println!("\nSelected template: {chosen}");
            println!("\nNext steps:");
            println!("  1. anvil init        — initialise Anvil in your project");
            println!("  2. anvil gate        — run your first gate check");
            println!("  3. anvil status      — view project health");
        }
    } else {
        print_plain(&categories, &templates);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Template catalogue
// ---------------------------------------------------------------------------

fn builtin_categories() -> Vec<TemplateCategory> {
    vec![
        TemplateCategory {
            name: "Governance".into(),
            description: "Policy and compliance templates".into(),
            template_count: 2,
        },
        TemplateCategory {
            name: "Quality".into(),
            description: "Code quality and testing templates".into(),
            template_count: 2,
        },
    ]
}

fn builtin_templates() -> Vec<TemplateEntry> {
    vec![
        TemplateEntry {
            id: "soc2-compliance".into(),
            name: "SOC 2 Compliance".into(),
            description: "Pre-built gates for SOC 2 Type II requirements".into(),
            category: "Governance".into(),
            tags: vec!["compliance".into(), "soc2".into()],
            variables: vec![TemplateVariable {
                name: "org_name".into(),
                description: "Organisation name".into(),
                default_value: None,
                required: true,
            }],
        },
        TemplateEntry {
            id: "gdpr-data-handling".into(),
            name: "GDPR Data Handling".into(),
            description: "Gates for personal data protection".into(),
            category: "Governance".into(),
            tags: vec!["compliance".into(), "gdpr".into()],
            variables: vec![],
        },
        TemplateEntry {
            id: "test-coverage-gates".into(),
            name: "Test Coverage Gates".into(),
            description: "Enforce minimum test coverage".into(),
            category: "Quality".into(),
            tags: vec!["testing".into(), "coverage".into()],
            variables: vec![TemplateVariable {
                name: "min_coverage".into(),
                description: "Minimum coverage percentage".into(),
                default_value: Some("80".into()),
                required: false,
            }],
        },
        TemplateEntry {
            id: "lint-standard".into(),
            name: "Lint Standard".into(),
            description: "Standardised linting rules".into(),
            category: "Quality".into(),
            tags: vec!["linting".into()],
            variables: vec![],
        },
    ]
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
            println!("    {:<20}{}", t.id, t.name);
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

    #[test]
    fn catalogue_has_two_categories() {
        let cats = builtin_categories();
        assert_eq!(cats.len(), 2);
    }

    #[test]
    fn catalogue_has_four_templates() {
        let tmpls = builtin_templates();
        assert_eq!(tmpls.len(), 4);
    }

    #[test]
    fn json_output_is_valid() {
        let cats = builtin_categories();
        let tmpls = builtin_templates();

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

        let json = serde_json::to_string_pretty(&output).expect("serialise");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed["categories"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["templates"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn template_categories_match() {
        let cats = builtin_categories();
        let tmpls = builtin_templates();
        let cat_names: Vec<&str> = cats.iter().map(|c| c.name.as_str()).collect();
        for t in &tmpls {
            assert!(
                cat_names.contains(&t.category.as_str()),
                "template {} has unknown category {}",
                t.id,
                t.category,
            );
        }
    }

    #[test]
    fn template_counts_match() {
        let cats = builtin_categories();
        let tmpls = builtin_templates();
        for cat in &cats {
            let count = tmpls.iter().filter(|t| t.category == cat.name).count();
            assert_eq!(
                count, cat.template_count,
                "category {} declares {} templates but has {}",
                cat.name, cat.template_count, count,
            );
        }
    }
}
