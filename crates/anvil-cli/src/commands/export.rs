use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;
use crate::services::suppressions::{SuppressionEntry, load_suppressions};

#[derive(Debug, Args)]
pub struct ExportArgs {
    /// Source file path (for plan conversion)
    source: Option<String>,

    /// Target format for plan conversion (aps, json, yaml)
    #[arg(long)]
    to: Option<String>,

    /// Output format for constraint export (llms.txt, mcp-resource, prompt-fragment)
    #[arg(long)]
    format: Option<String>,

    /// Output file path
    #[arg(long, short)]
    output: Option<String>,

    /// Source format override (auto-detected if omitted; not yet implemented)
    #[arg(long, hide = true)]
    from: Option<String>,

    /// Compact JSON output
    #[arg(long)]
    compact: bool,
}

#[derive(Debug, Serialize)]
struct ExportResult {
    output_path: String,
    format: String,
    size_bytes: usize,
}

fn normalize_target_format(format: &str) -> String {
    let lower = format.to_lowercase();
    match lower.as_str() {
        "yml" => "yaml".to_string(),
        _ => lower,
    }
}

fn normalize_constraint_format(format: &str) -> String {
    match format.to_lowercase().as_str() {
        "llms.txt" | "llmstxt" | "llms" => "llms.txt".to_string(),
        "mcp-resource" | "mcp" => "mcp-resource".to_string(),
        "prompt-fragment" | "prompt" => "prompt-fragment".to_string(),
        other => other.to_string(),
    }
}

fn generate_default_output(source: &str, ext: &str) -> String {
    let path = std::path::PathBuf::from(source);
    let mut stem_path = path.clone();
    if stem_path
        .extension()
        .is_some_and(|extname| extname.eq_ignore_ascii_case("md"))
    {
        stem_path = stem_path.with_extension("");
    }
    let stem = stem_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    dir.join(format!("{stem}.aps.{ext}"))
        .to_string_lossy()
        .to_string()
}

/// Collapse `.` / `..` components without requiring intermediate directories.
fn lexical_normalize(path: &std::path::Path) -> std::path::PathBuf {
    use std::path::Component;
    let mut components: Vec<Component<'_>> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(components.last(), Some(Component::Normal(_))) {
                    components.pop();
                } else if matches!(
                    components.last(),
                    Some(Component::RootDir | Component::Prefix(_))
                ) {
                    // Parent of root/prefix is a no-op; drop the `..`.
                } else {
                    // Relative path still climbing (`../x`) — keep the component.
                    components.push(component);
                }
            }
            other => components.push(other),
        }
    }
    components.iter().collect()
}

/// Resolve a path for identity comparison.
/// Prefer filesystem canonicalisation when the path exists; otherwise build an
/// absolute lexically-normalised path so a not-yet-existing output (or a path
/// with `..` through a missing intermediate) can still be compared.
fn resolve_path_for_identity(path: &std::path::Path) -> std::path::PathBuf {
    if let Ok(canon) = dunce::canonicalize(path) {
        return canon;
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(path),
            Err(_) => path.to_path_buf(),
        }
    };
    let normalized = lexical_normalize(&absolute);
    if let Some(parent) = normalized.parent()
        && let Ok(parent_canon) = dunce::canonicalize(parent)
        && let Some(name) = normalized.file_name()
    {
        return parent_canon.join(name);
    }
    normalized
}

/// True when both existing paths refer to the same underlying file.
fn files_share_identity(source: &std::path::Path, output: &std::path::Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let (Ok(src_meta), Ok(out_meta)) = (std::fs::metadata(source), std::fs::metadata(output))
        {
            return src_meta.dev() == out_meta.dev() && src_meta.ino() == out_meta.ino();
        }
        return false;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if let (Ok(src_meta), Ok(out_meta)) = (std::fs::metadata(source), std::fs::metadata(output))
        {
            return src_meta.volume_serial_number() == out_meta.volume_serial_number()
                && src_meta.file_index() == out_meta.file_index();
        }
        return false;
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (source, output);
        false
    }
}

/// True when both paths refer to the same file (file identity when both exist;
/// otherwise normalised absolute paths).
fn output_targets_source(source: &str, output: &str) -> bool {
    let source_path = std::path::Path::new(source);
    let output_path = std::path::Path::new(output);

    if files_share_identity(source_path, output_path) {
        return true;
    }

    resolve_path_for_identity(source_path) == resolve_path_for_identity(output_path)
}

fn export_plan(
    source: &str,
    target_format: &str,
    output: Option<&str>,
    from: Option<&str>,
    compact: bool,
    global: &GlobalArgs,
) -> Result<()> {
    if !std::path::Path::new(source).exists() {
        bail!("Source file not found: {source}");
    }
    if from.is_some() {
        bail!("--from override is not yet supported in the Rust CLI export command");
    }
    let content = std::fs::read_to_string(source).with_context(|| format!("reading {source}"))?;

    let is_markdown = std::path::Path::new(source)
        .extension()
        .is_some_and(|extname| extname.eq_ignore_ascii_case("md"));

    let target_ext = if target_format == "yaml" {
        "yaml"
    } else {
        "json"
    };

    let output_path =
        output.map_or_else(|| generate_default_output(source, target_ext), String::from);

    // Reject explicit (or derived) destinations that would overwrite the source
    // plan — conversion must never clobber the only input file.
    if output_targets_source(source, &output_path) {
        bail!(
            "refusing to overwrite source: output path '{output_path}' resolves to the same file as source '{source}'"
        );
    }

    let output_dir = std::path::Path::new(&output_path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("creating {}", output_dir.display()))?;

    let result_content = if is_markdown {
        let parsed = parse_aps_markdown(&content);
        match target_format {
            "yaml" => {
                let yaml_val =
                    serde_yaml::to_value(&parsed).context("converting APS to YAML value")?;
                serde_yaml::to_string(&yaml_val).context("serialising to YAML")?
            }
            "json" | "aps" => {
                if compact {
                    serde_json::to_string(&parsed)?
                } else {
                    serde_json::to_string_pretty(&parsed)?
                }
            }
            other => bail!("Unsupported target format: {other}"),
        }
    } else {
        match target_format {
            "yaml" => {
                let value: serde_yaml::Value = if let Ok(v) = serde_yaml::from_str(&content) {
                    v
                } else {
                    let json: serde_json::Value = serde_json::from_str(&content)
                        .with_context(|| format!("parsing {source} as JSON or YAML"))?;
                    serde_yaml::to_value(json).context("converting JSON to YAML value")?
                };
                serde_yaml::to_string(&value).context("serialising to YAML")?
            }
            "json" | "aps" => {
                let value: serde_json::Value = serde_yaml::from_str(&content)
                    .or_else(|_| serde_json::from_str(&content))
                    .with_context(|| format!("parsing {source} as YAML or JSON"))?;
                if compact {
                    serde_json::to_string(&value)?
                } else {
                    serde_json::to_string_pretty(&value)?
                }
            }
            other => bail!("Unsupported target format: {other}"),
        }
    };

    std::fs::write(&output_path, &result_content)
        .with_context(|| format!("writing {output_path}"))?;

    let result = ExportResult {
        output_path: output_path.clone(),
        format: target_format.to_string(),
        size_bytes: result_content.len(),
    };

    if global.json {
        crate::output::json::print(&result)?;
    } else {
        println!("Exported to {}", target_format.to_uppercase());
        println!("  Output: {output_path}");
        println!("  Size:   {} bytes", result_content.len());
    }

    Ok(())
}

// =============================================================================
// APS Markdown Parser
// =============================================================================

/// Parsed APS plan structure.
#[derive(Debug, Serialize)]
struct ApsPlan {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    frontmatter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    purpose: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    phases: Vec<ApsPhase>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    metadata: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
struct ApsPhase {
    name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    items: Vec<ApsWorkItem>,
}

#[derive(Debug, Serialize)]
struct ApsWorkItem {
    id: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    intent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    confidence: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    dependencies: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    files: Vec<String>,
}

fn extract_frontmatter(content: &str) -> Option<String> {
    if !content.starts_with("<!--") {
        return None;
    }
    content
        .find("-->")
        .and_then(|end_pos| content.get(4..end_pos).map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
}

fn extract_metadata(lines: &[&str]) -> BTreeMap<String, String> {
    let mut metadata = BTreeMap::new();
    let Some(start) = lines
        .iter()
        .position(|l| l.starts_with("# ") && !l.starts_with("## "))
    else {
        return metadata;
    };

    let mut found_header = false;
    for line in &lines[start + 1..] {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
            if found_header {
                break;
            }
            continue;
        }
        let cells: Vec<&str> = trimmed.split('|').map(str::trim).collect();
        if cells.len() < 4 {
            continue;
        }
        if !found_header {
            if cells[1].to_lowercase() == "id" {
                found_header = true;
            }
            continue;
        }
        if cells[1].contains("---") {
            continue;
        }
        if !cells[1].is_empty() {
            metadata.insert("id".to_string(), cells[1].to_string());
            if cells.len() > 3 && !cells[3].is_empty() && cells[3] != "\u{2014}" {
                metadata.insert("status".to_string(), cells[3].to_string());
            }
            if cells.len() > 4 && !cells[4].is_empty() {
                metadata.insert("progress".to_string(), cells[4].to_string());
            }
            break;
        }
    }
    metadata
}

fn extract_purpose(lines: &[&str]) -> Option<String> {
    let mut in_purpose = false;
    let mut buf: Vec<String> = Vec::new();
    for line in lines {
        if line.starts_with("## Purpose") {
            in_purpose = true;
            continue;
        }
        if in_purpose {
            if line.starts_with("## ") {
                break;
            }
            buf.push(line.to_string());
        }
    }
    let text = buf.join("\n").trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

fn parse_aps_markdown(content: &str) -> ApsPlan {
    let lines: Vec<&str> = content.lines().collect();
    ApsPlan {
        frontmatter: extract_frontmatter(content),
        title: lines
            .iter()
            .find(|l| l.starts_with("# ") && !l.starts_with("## "))
            .map(|l| l.trim_start_matches("# ").trim().to_string())
            .unwrap_or_default(),
        metadata: extract_metadata(&lines),
        purpose: extract_purpose(&lines),
        phases: extract_phases(&lines),
    }
}

#[allow(clippy::too_many_lines)]
fn extract_phases(lines: &[&str]) -> Vec<ApsPhase> {
    let mut phases: Vec<ApsPhase> = Vec::new();
    let mut current_phase: Option<ApsPhase> = None;
    let mut current_item: Option<ApsWorkItem> = None;
    let mut in_work_item = false;
    let mut last_field_is_intent = false;
    let mut in_non_phase_section = false; // True after a non-phase ## header

    // Helper closures to flush accumulated state
    let flush_item = |item: &mut Option<ApsWorkItem>, phase: &mut Option<ApsPhase>| {
        if let Some(it) = item.take() {
            match phase.as_mut() {
                Some(ph) => ph.items.push(it),
                None => {
                    *phase = Some(ApsPhase {
                        name: "Tasks".to_string(),
                        items: vec![it],
                    });
                }
            }
        }
    };
    let flush_phase = |phase: &mut Option<ApsPhase>, phases: &mut Vec<ApsPhase>| {
        if let Some(ph) = phase.take()
            && !ph.items.is_empty()
        {
            phases.push(ph);
        }
    };

    for line in lines {
        // Phase header at ##/###/#### heading levels
        if let Some(rest) = line
            .strip_prefix("## ")
            .or_else(|| line.strip_prefix("### "))
            .or_else(|| line.strip_prefix("#### "))
            && rest.starts_with("Phase ")
        {
            flush_item(&mut current_item, &mut current_phase);
            flush_phase(&mut current_phase, &mut phases);
            current_phase = Some(ApsPhase {
                name: rest.trim().to_string(),
                items: Vec::new(),
            });
            in_work_item = false;
            in_non_phase_section = false;
            continue;
        }

        // Non-phase ## headers discard any in-progress item and mark section as non-phase
        if line.starts_with("## ") {
            current_item.take();
            in_work_item = false;
            in_non_phase_section = true;
            continue;
        }

        // Work item header: ### RCLI-NNN: title (skip if inside a non-phase section)
        if line.starts_with("### ") && !in_non_phase_section {
            flush_item(&mut current_item, &mut current_phase);
            let header = line.trim_start_matches("### ").trim();
            let (id, title_part) = if let Some(colon_pos) = header.find(':') {
                (
                    header[..colon_pos].trim().to_string(),
                    header[colon_pos + 1..].trim().to_string(),
                )
            } else {
                (header.to_string(), String::new())
            };

            current_item = Some(ApsWorkItem {
                id,
                title: title_part,
                status: None,
                intent: None,
                priority: None,
                confidence: None,
                dependencies: Vec::new(),
                files: Vec::new(),
            });
            in_work_item = true;
            last_field_is_intent = false;
            continue;
        }

        // Parse work item fields (supports multi-line continuation for Intent)
        if in_work_item && let Some(ref mut item) = current_item {
            let trimmed = line.trim().trim_start_matches("- ");
            if let Some(rest) = trimmed.strip_prefix("**Status:**") {
                item.status = Some(rest.trim().to_string());
                last_field_is_intent = false;
            } else if let Some(rest) = trimmed.strip_prefix("**Intent:**") {
                item.intent = Some(rest.trim().to_string());
                last_field_is_intent = true;
            } else if let Some(rest) = trimmed.strip_prefix("**Priority:**") {
                item.priority = Some(rest.trim().to_string());
                last_field_is_intent = false;
            } else if let Some(rest) = trimmed.strip_prefix("**Confidence:**") {
                item.confidence = Some(rest.trim().to_string());
                last_field_is_intent = false;
            } else if let Some(rest) = trimmed.strip_prefix("**Dependencies:**") {
                let deps: Vec<String> = rest
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty() && s != "None")
                    .collect();
                item.dependencies = deps;
                last_field_is_intent = false;
            } else if let Some(rest) = trimmed.strip_prefix("**Files:**") {
                let files: Vec<String> = rest
                    .split(',')
                    .map(|s| s.trim().trim_matches('`').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                item.files = files;
                last_field_is_intent = false;
            } else if last_field_is_intent
                && !trimmed.is_empty()
                && !trimmed.starts_with("**")
                && !trimmed.starts_with("---")
            {
                // Continuation line — append to intent only when it was the last parsed field
                if let Some(ref mut intent) = item.intent {
                    intent.push(' ');
                    intent.push_str(trimmed);
                }
            }
        }
    }

    // Flush final item and phase
    flush_item(&mut current_item, &mut current_phase);
    flush_phase(&mut current_phase, &mut phases);

    phases
}

// =============================================================================
// Constraint Collection & Formatting
// =============================================================================

#[derive(Debug, Serialize)]
pub(crate) struct ConstraintData {
    boundaries: Vec<BoundaryEntry>,
    layers: Vec<LayerEntry>,
    anti_patterns: Vec<AntiPatternEntry>,
    conventions: Vec<ConventionEntry>,
    suppressions: Vec<SuppressionEntry>,
    metadata: ConstraintMetadata,
}

#[derive(Debug, Serialize)]
struct BoundaryEntry {
    name: String,
    from: String,
    to: String,
    message: String,
    severity: String,
}

#[derive(Debug, Serialize)]
struct LayerEntry {
    name: String,
    patterns: Vec<String>,
    depends_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Debug, Serialize)]
struct AntiPatternEntry {
    id: String,
    name: String,
    category: String,
    explanation: String,
    suggestion: String,
    severity: String,
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct ConventionEntry {
    category: String,
    description: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    examples: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ConstraintMetadata {
    collected_at: String,
    workspace_root: String,
    has_baseline: bool,
}

/// Project conventions matching the TypeScript `ConstraintCollector`.
fn default_conventions() -> Vec<ConventionEntry> {
    vec![
        ConventionEntry {
            category: "spelling".to_string(),
            description: "Use UK English spelling".to_string(),
            examples: vec![
                "organised (not organized)".to_string(),
                "behaviour (not behavior)".to_string(),
                "colour (not color)".to_string(),
            ],
        },
        ConventionEntry {
            category: "imports".to_string(),
            description: "ESM imports require .js extensions".to_string(),
            examples: vec![
                "import { foo } from './bar.js'".to_string(),
                "NOT: import { foo } from './bar'".to_string(),
            ],
        },
        ConventionEntry {
            category: "schemas".to_string(),
            description: "Zod schemas as source of truth for types".to_string(),
            examples: vec![
                "export const FooSchema = z.object({ ... })".to_string(),
                "export type Foo = z.infer<typeof FooSchema>".to_string(),
            ],
        },
        ConventionEntry {
            category: "naming".to_string(),
            description: "Kebab-case for file names".to_string(),
            examples: vec![
                "gate-runner.ts".to_string(),
                "format-detection.ts".to_string(),
            ],
        },
        ConventionEntry {
            category: "type-safety".to_string(),
            description: "No type assertions without runtime validation".to_string(),
            examples: vec![
                "Use Zod parse, not \"as\" casts".to_string(),
                "Avoid @ts-ignore and @ts-expect-error".to_string(),
            ],
        },
    ]
}

pub(crate) fn collect_constraints(workspace_root: &std::path::Path) -> ConstraintData {
    let now = chrono::Utc::now().to_rfc3339();
    let root_str = workspace_root.to_string_lossy().to_string();

    // Load architecture baseline (warn on errors rather than silently ignoring)
    let file_exists = anvil_architecture::baseline::baseline_exists(workspace_root);
    let baseline = match anvil_architecture::baseline::load_baseline(workspace_root) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("warning: could not load architecture baseline: {e}");
            None
        }
    };
    let has_baseline = file_exists || baseline.is_some();

    let boundaries: Vec<BoundaryEntry> = baseline
        .as_ref()
        .map(|b| {
            b.boundaries
                .iter()
                .map(|boundary| BoundaryEntry {
                    name: boundary.name.clone(),
                    from: boundary.from.clone(),
                    to: boundary.to.clone(),
                    message: boundary.message.clone(),
                    severity: serde_json::to_value(&boundary.severity)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_else(|| format!("{:?}", boundary.severity).to_lowercase()),
                })
                .collect()
        })
        .unwrap_or_default();

    let layers: Vec<LayerEntry> = baseline
        .as_ref()
        .map(|b| {
            b.layers
                .iter()
                .map(|(name, layer)| LayerEntry {
                    name: name.clone(),
                    patterns: layer.patterns.clone(),
                    depends_on: layer.depends_on.clone(),
                    description: layer.description.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    // Load anti-patterns
    let patterns = anvil_checks::antipattern::patterns::get_default_patterns();
    let anti_patterns: Vec<AntiPatternEntry> = patterns
        .into_iter()
        .map(|p| AntiPatternEntry {
            id: p.id,
            name: p.name,
            category: serde_json::to_value(p.category)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| format!("{:?}", p.category).to_lowercase()),
            explanation: p.explanation,
            suggestion: p.suggestion,
            severity: serde_json::to_value(p.severity)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| format!("{:?}", p.severity).to_lowercase()),
            enabled: p.enabled,
        })
        .collect();

    let conventions = default_conventions();

    // Load suppressions from .anvil/suppressions.json
    let suppressions = load_suppressions(workspace_root);

    ConstraintData {
        boundaries,
        layers,
        anti_patterns,
        conventions,
        suppressions,
        metadata: ConstraintMetadata {
            collected_at: now,
            workspace_root: root_str,
            has_baseline,
        },
    }
}

// =============================================================================
// Constraint Formatters
// =============================================================================

fn llms_boundaries(data: &ConstraintData) -> String {
    let mut lines = vec![
        "## Boundary Rules\n".to_string(),
        "These architectural boundaries must be respected. Violations will be flagged as warnings or errors.\n".to_string(),
    ];
    for b in &data.boundaries {
        let emoji = severity_emoji(&b.severity);
        lines.push(format!("- {emoji} **{}**", b.name));
        lines.push(format!("  - From: `{}`", b.from));
        lines.push(format!("  - To: `{}`", b.to));
        lines.push(format!("  - Rule: {}", b.message));
        lines.push(String::new());
    }
    lines.join("\n")
}

fn llms_layers(data: &ConstraintData) -> String {
    let mut lines = vec![
        "## Layer Definitions\n".to_string(),
        "The codebase is organised into architectural layers. Each layer has specific responsibilities and dependencies.\n".to_string(),
    ];
    for l in &data.layers {
        lines.push(format!("### {}\n", l.name));
        if let Some(ref desc) = l.description {
            lines.push(format!("{desc}\n"));
        }
        lines.push("**Patterns:**".to_string());
        for p in &l.patterns {
            lines.push(format!("- `{p}`"));
        }
        lines.push(String::new());
        if l.depends_on.is_empty() {
            lines.push("**Dependencies:** None (leaf layer)".to_string());
        } else {
            lines.push("**Can depend on:**".to_string());
            for d in &l.depends_on {
                lines.push(format!("- {d}"));
            }
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

fn llms_anti_patterns(data: &ConstraintData) -> String {
    let mut lines = vec![
        "## Anti-patterns (Blocked)\n".to_string(),
        "These code patterns are considered anti-patterns and should be avoided. anvil will flag them during code review.\n".to_string(),
    ];
    let mut by_category: BTreeMap<String, Vec<&AntiPatternEntry>> = BTreeMap::new();
    for p in &data.anti_patterns {
        by_category.entry(p.category.clone()).or_default().push(p);
    }
    for (category, patterns) in &by_category {
        lines.push(format!("### {}\n", format_category_name(category)));
        for p in patterns {
            let emoji = severity_emoji(&p.severity);
            lines.push(format!("#### {emoji} {} (`{}`)\n", p.name, p.id));
            lines.push(format!("**Why it's problematic:** {}\n", p.explanation));
            lines.push(format!("**What to do instead:** {}\n", p.suggestion));
        }
    }
    lines.join("\n")
}

fn llms_conventions(data: &ConstraintData) -> String {
    let mut lines = vec![
        "## Conventions\n".to_string(),
        "These conventions should be followed throughout the codebase for consistency and maintainability.\n".to_string(),
    ];
    for c in &data.conventions {
        lines.push(format!("### {}\n", format_category_name(&c.category)));
        lines.push(format!("{}\n", c.description));
        if !c.examples.is_empty() {
            lines.push("**Examples:**".to_string());
            for ex in &c.examples {
                lines.push(format!("- {ex}"));
            }
            lines.push(String::new());
        }
    }
    lines.join("\n")
}

fn llms_suppressions(data: &ConstraintData) -> String {
    let mut lines = vec![
        "## Active Suppressions\n".to_string(),
        "These patterns are intentionally suppressed in specific locations. Do not flag or attempt to fix these.\n".to_string(),
    ];
    let mut by_pattern: BTreeMap<String, Vec<&SuppressionEntry>> = BTreeMap::new();
    for s in &data.suppressions {
        by_pattern.entry(s.pattern_id.clone()).or_default().push(s);
    }
    for (pattern_id, entries) in &by_pattern {
        lines.push(format!("### `{pattern_id}`\n"));
        for s in entries {
            lines.push(format!("- **`{}`** ({})", s.file, s.scope));
            lines.push(format!("  - Reason: {}", s.reason));
            if let Some(ref exp) = s.expires_at {
                lines.push(format!("  - Expires: {}", &exp[..10.min(exp.len())]));
            }
            lines.push(String::new());
        }
    }
    lines.join("\n")
}

fn format_llms_txt(data: &ConstraintData) -> String {
    let mut sections: Vec<String> = Vec::new();

    sections.push("# anvil Architecture Constraints\n".to_string());

    // Metadata
    sections.push(format!(
        "> **Generated:** {}\n> **Workspace:** {}\n> **Has Baseline:** {}\n",
        data.metadata.collected_at,
        data.metadata.workspace_root,
        if data.metadata.has_baseline {
            "Yes"
        } else {
            "No"
        },
    ));

    if !data.boundaries.is_empty() {
        sections.push(llms_boundaries(data));
    }
    if !data.layers.is_empty() {
        sections.push(llms_layers(data));
    }
    if !data.anti_patterns.is_empty() {
        sections.push(llms_anti_patterns(data));
    }
    if !data.conventions.is_empty() {
        sections.push(llms_conventions(data));
    }
    if !data.suppressions.is_empty() {
        sections.push(llms_suppressions(data));
    }

    sections.join("\n")
}

#[derive(Serialize)]
struct McpResource {
    uri: String,
    name: String,
    description: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
    contents: serde_json::Value,
}

fn build_mcp_contents(data: &ConstraintData) -> serde_json::Map<String, serde_json::Value> {
    let mut contents = serde_json::Map::new();

    contents.insert(
        "metadata".to_string(),
        serde_json::json!({
            "generatedAt": data.metadata.collected_at,
            "workspaceRoot": data.metadata.workspace_root,
            "hasBaseline": data.metadata.has_baseline,
            "version": "1.0.0",
        }),
    );

    if !data.boundaries.is_empty() {
        let boundaries: Vec<serde_json::Value> = data
            .boundaries
            .iter()
            .enumerate()
            .map(|(i, b)| {
                serde_json::json!({
                    "id": format!("boundary-{}", i + 1),
                    "name": b.name, "from": b.from, "to": b.to,
                    "message": b.message, "severity": b.severity,
                })
            })
            .collect();
        contents.insert("boundaries".to_string(), serde_json::json!(boundaries));
    }

    if !data.layers.is_empty() {
        let layers: Vec<serde_json::Value> = data
            .layers
            .iter()
            .map(|l| {
                serde_json::json!({
                    "name": l.name, "patterns": l.patterns,
                    "dependsOn": l.depends_on, "description": l.description,
                })
            })
            .collect();
        contents.insert("layers".to_string(), serde_json::json!(layers));
    }

    if !data.anti_patterns.is_empty() {
        let patterns: Vec<serde_json::Value> = data
            .anti_patterns
            .iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id, "name": p.name, "category": p.category,
                    "explanation": p.explanation, "suggestion": p.suggestion,
                    "severity": p.severity, "enabled": p.enabled,
                })
            })
            .collect();
        contents.insert("antiPatterns".to_string(), serde_json::json!(patterns));
    }

    if !data.conventions.is_empty() {
        let conventions: Vec<serde_json::Value> = data
            .conventions
            .iter()
            .map(|c| {
                serde_json::json!({
                    "category": c.category, "description": c.description,
                    "examples": c.examples,
                })
            })
            .collect();
        contents.insert("conventions".to_string(), serde_json::json!(conventions));
    }

    if !data.suppressions.is_empty() {
        let suppressions: Vec<serde_json::Value> = data
            .suppressions
            .iter()
            .map(|s| {
                serde_json::json!({
                    "patternId": s.pattern_id, "file": s.file,
                    "scope": s.scope, "reason": s.reason, "expiresAt": s.expires_at,
                })
            })
            .collect();
        contents.insert("suppressions".to_string(), serde_json::json!(suppressions));
    }

    contents
}

fn format_mcp_resource(data: &ConstraintData) -> Result<String> {
    let resource = McpResource {
        uri: "anvil://constraints".to_string(),
        name: "anvil Architecture Constraints".to_string(),
        description: "Architecture rules, anti-patterns, and conventions for this codebase"
            .to_string(),
        mime_type: "application/json".to_string(),
        contents: serde_json::Value::Object(build_mcp_contents(data)),
    };
    serde_json::to_string_pretty(&resource).context("serialising MCP resource")
}

fn prompt_boundaries(data: &ConstraintData) -> String {
    let mut lines = vec![
        "**Architecture Boundaries**".to_string(),
        "These boundaries define which layers can depend on each other:".to_string(),
        String::new(),
    ];
    for b in &data.boundaries {
        lines.push(format!(
            "- **{}**: Layer \"{}\" must not depend on \"{}\"",
            b.name, b.from, b.to
        ));
        lines.push(format!("  {}", b.message));
        lines.push(format!("  Severity: {}", b.severity));
    }
    lines.join("\n")
}

fn prompt_layers(data: &ConstraintData) -> String {
    let mut lines = vec![
        "**Layer Definitions**".to_string(),
        "The codebase is organised into these architectural layers:".to_string(),
        String::new(),
    ];
    for l in &data.layers {
        lines.push(format!("- **{}**", l.name));
        if let Some(ref desc) = l.description {
            lines.push(format!("  {desc}"));
        }
        lines.push(format!("  Files: {}", l.patterns.join(", ")));
        if l.depends_on.is_empty() {
            lines.push("  No dependencies (leaf layer)".to_string());
        } else {
            lines.push(format!(
                "  Allowed dependencies: {}",
                l.depends_on.join(", ")
            ));
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

fn prompt_anti_patterns(data: &ConstraintData) -> String {
    let mut lines = vec![
        "**Forbidden Anti-patterns**".to_string(),
        "Never introduce these code patterns:".to_string(),
        String::new(),
    ];
    let mut by_category: BTreeMap<String, Vec<&AntiPatternEntry>> = BTreeMap::new();
    for p in &data.anti_patterns {
        by_category.entry(p.category.clone()).or_default().push(p);
    }
    for (category, patterns) in &by_category {
        lines.push(format!("{}:", format_category_name(category)));
        for p in patterns {
            lines.push(format!("- **{}** ({})", p.name, p.id));
            lines.push(format!("  Problem: {}", p.explanation));
            lines.push(format!("  Instead: {}", p.suggestion));
            lines.push(String::new());
        }
    }
    lines.join("\n")
}

fn prompt_suppressions(data: &ConstraintData) -> String {
    let mut lines = vec![
        "**Active Suppressions**".to_string(),
        "These violations are intentionally suppressed. Do not flag or fix them:".to_string(),
        String::new(),
    ];
    let mut by_pattern: BTreeMap<String, Vec<&SuppressionEntry>> = BTreeMap::new();
    for s in &data.suppressions {
        by_pattern.entry(s.pattern_id.clone()).or_default().push(s);
    }
    for (pattern_id, entries) in &by_pattern {
        lines.push(format!("{pattern_id}:"));
        for s in entries {
            lines.push(format!("- **{}** ({}): {}", s.file, s.scope, s.reason));
            if let Some(ref exp) = s.expires_at {
                lines.push(format!("  Expires: {}", &exp[..10.min(exp.len())]));
            }
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

fn format_prompt_fragment(data: &ConstraintData) -> String {
    let mut sections: Vec<String> = Vec::new();

    sections.push(
        "This codebase has specific architecture boundaries, anti-patterns, and conventions that must be followed. \
         When generating or modifying code:\n\n\
         1. Respect all architectural boundaries and layer dependencies\n\
         2. Avoid all listed anti-patterns\n\
         3. Follow project conventions consistently\n\
         4. If a boundary violation or anti-pattern is necessary, explain why and suggest alternatives first"
            .to_string(),
    );

    if !data.boundaries.is_empty() {
        sections.push(prompt_boundaries(data));
    }
    if !data.layers.is_empty() {
        sections.push(prompt_layers(data));
    }
    if !data.anti_patterns.is_empty() {
        sections.push(prompt_anti_patterns(data));
    }
    if !data.conventions.is_empty() {
        let mut lines = vec![
            "**Project Conventions**".to_string(),
            "Follow these conventions for consistency:".to_string(),
            String::new(),
        ];
        for c in &data.conventions {
            lines.push(format!(
                "- **{}**: {}",
                format_category_name(&c.category),
                c.description
            ));
            for ex in &c.examples {
                lines.push(format!("  \u{2022} {ex}"));
            }
        }
        sections.push(lines.join("\n"));
    }
    if !data.suppressions.is_empty() {
        sections.push(prompt_suppressions(data));
    }

    sections.join("\n\n")
}

fn severity_emoji(severity: &str) -> &'static str {
    match severity {
        "error" => "\u{1f6ab}",
        "warning" => "\u{26a0}\u{fe0f}",
        "info" => "\u{2139}\u{fe0f}",
        _ => "\u{2022}",
    }
}

fn format_category_name(category: &str) -> String {
    category
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => format!("{}{}", c.to_uppercase(), chars.collect::<String>()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn export_constraints(format: &str, output: Option<&str>, global: &GlobalArgs) -> Result<()> {
    let normalized = normalize_constraint_format(format);
    let workspace_root = if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        && output.status.success()
    {
        std::path::PathBuf::from(String::from_utf8_lossy(&output.stdout).trim())
    } else {
        std::env::current_dir().context("getting current directory")?
    };

    let default_filename = match normalized.as_str() {
        "llms.txt" => ".llms.txt",
        "mcp-resource" => "anvil-constraints.mcp.json",
        "prompt-fragment" => "anvil-constraints-prompt.txt",
        other => {
            bail!("Unsupported format: {other}. Supported: llms.txt, mcp-resource, prompt-fragment")
        }
    };

    let data = collect_constraints(&workspace_root);

    let formatted = match normalized.as_str() {
        "llms.txt" => format_llms_txt(&data),
        "mcp-resource" => format_mcp_resource(&data)?,
        "prompt-fragment" => format_prompt_fragment(&data),
        _ => unreachable!(),
    };

    let output_path = output.map_or_else(
        || {
            workspace_root
                .join(default_filename)
                .to_string_lossy()
                .to_string()
        },
        String::from,
    );

    // Validate output path stays within workspace for relative paths
    let out_path = std::path::Path::new(&output_path);
    if !out_path.is_absolute() {
        let resolved = workspace_root.join(out_path);
        let canon = resolved.canonicalize().unwrap_or(resolved);
        let canon_ws = workspace_root
            .canonicalize()
            .unwrap_or(workspace_root.clone());
        if !canon.starts_with(&canon_ws) {
            bail!(
                "Output path '{}' escapes workspace root '{}'",
                output_path,
                canon_ws.display()
            );
        }
    }

    if let Some(dir) = out_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }

    std::fs::write(&output_path, &formatted).with_context(|| format!("writing {output_path}"))?;

    let result = ExportResult {
        output_path: output_path.clone(),
        format: normalized.clone(),
        size_bytes: formatted.len(),
    };

    if global.json {
        crate::output::json::print(&result)?;
    } else {
        println!("Exported constraints as {normalized}");
        println!("  Output: {output_path}");
        println!("  Size:   {} bytes", formatted.len());
    }

    Ok(())
}

pub fn run(args: &ExportArgs, global: &GlobalArgs) -> Result<()> {
    if let Some(format) = &args.format {
        export_constraints(format, args.output.as_deref(), global)
    } else if let Some(to) = &args.to {
        let source = args
            .source
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Source file path is required for plan conversion"))?;
        let target = normalize_target_format(to);
        export_plan(
            source,
            &target,
            args.output.as_deref(),
            args.from.as_deref(),
            args.compact,
            global,
        )
    } else {
        bail!(
            "Either --format or --to must be specified\n\nExamples:\n  Constraint export: anvil export --format llms.txt\n  Data conversion:   anvil export source.yaml --to json"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Wrapper {
        #[command(flatten)]
        inner: ExportArgs,
    }

    #[test]
    fn args_parses_empty() {
        let w = Wrapper::try_parse_from(["test"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_with_to() {
        let w = Wrapper::try_parse_from(["test", "file.md", "--to", "json"]).unwrap();
        assert_eq!(w.inner.to.as_deref(), Some("json"));
    }

    #[test]
    fn args_parses_with_format() {
        let w = Wrapper::try_parse_from(["test", "--format", "llms.txt"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn normalize_target_canonicalises_yml() {
        assert_eq!(normalize_target_format("yml"), "yaml");
        assert_eq!(normalize_target_format("YML"), "yaml");
        assert_eq!(normalize_target_format("yaml"), "yaml");
        assert_eq!(normalize_target_format("json"), "json");
    }

    #[test]
    fn generate_output_handles_dotted_stems() {
        let result = generate_default_output("plans/my.plan.md", "json");
        // Normalise path separators for cross-platform comparison
        assert_eq!(result.replace('\\', "/"), "plans/my.aps.json");
    }

    #[test]
    fn generate_output_handles_bare_filename() {
        let result = generate_default_output("plan.md", "yaml");
        assert_eq!(result.replace('\\', "/"), "./plan.aps.yaml");
    }

    #[test]
    fn args_parses_with_from() {
        let w = Wrapper::try_parse_from(["test", "file.yaml", "--to", "json", "--from", "yaml"])
            .unwrap();
        assert_eq!(w.inner.from.as_deref(), Some("yaml"));
    }

    #[test]
    fn parse_aps_markdown_extracts_title() {
        let md = "# My Plan\n\n## Purpose\n\nDo things.\n";
        let plan = parse_aps_markdown(md);
        assert_eq!(plan.title, "My Plan");
        assert_eq!(plan.purpose.as_deref(), Some("Do things."));
    }

    #[test]
    fn parse_aps_markdown_extracts_phases_and_items() {
        let md = "\
# Test Plan

## Phase 1 — Setup

### TEST-001: scaffold project

- **Status:** Complete
- **Intent:** Create the project
- **Priority:** High
- **Confidence:** high
- **Dependencies:** None
- **Files:** `src/main.rs`, `Cargo.toml`

---

### TEST-002: add tests

- **Status:** Proposed
- **Intent:** Write unit tests

---

## Phase 2 — Polish

### TEST-003: docs

- **Status:** Draft
";
        let plan = parse_aps_markdown(md);
        assert_eq!(plan.phases.len(), 2);
        assert_eq!(plan.phases[0].name, "Phase 1 — Setup");
        assert_eq!(plan.phases[0].items.len(), 2);

        let item = &plan.phases[0].items[0];
        assert_eq!(item.id, "TEST-001");
        assert_eq!(item.title, "scaffold project");
        assert_eq!(item.status.as_deref(), Some("Complete"));
        assert_eq!(item.priority.as_deref(), Some("High"));
        assert_eq!(item.files, vec!["src/main.rs", "Cargo.toml"]);

        assert_eq!(plan.phases[1].items.len(), 1);
        assert_eq!(plan.phases[1].items[0].id, "TEST-003");
    }

    #[test]
    fn parse_aps_markdown_extracts_frontmatter() {
        let md = "<!--\nAPS Module: Test\nScopes: TEST\n-->\n\n# Test\n";
        let plan = parse_aps_markdown(md);
        assert!(plan.frontmatter.is_some());
        assert!(plan.frontmatter.unwrap().contains("APS Module: Test"));
    }

    #[test]
    fn format_category_name_capitalises_words() {
        assert_eq!(format_category_name("escape-hatch"), "Escape Hatch");
        assert_eq!(format_category_name("type-safety"), "Type Safety");
        assert_eq!(format_category_name("spelling"), "Spelling");
    }

    #[test]
    fn normalize_constraint_format_handles_aliases() {
        assert_eq!(normalize_constraint_format("llms.txt"), "llms.txt");
        assert_eq!(normalize_constraint_format("llmstxt"), "llms.txt");
        assert_eq!(normalize_constraint_format("mcp"), "mcp-resource");
        assert_eq!(normalize_constraint_format("prompt"), "prompt-fragment");
    }

    fn sample_constraint_data() -> ConstraintData {
        ConstraintData {
            boundaries: vec![BoundaryEntry {
                name: "no-domain-to-presentation".to_string(),
                from: "domain".to_string(),
                to: "presentation".to_string(),
                message: "domain must not depend on presentation".to_string(),
                severity: "error".to_string(),
            }],
            layers: vec![LayerEntry {
                name: "domain".to_string(),
                patterns: vec!["**/domain/**".to_string()],
                depends_on: vec!["shared".to_string()],
                description: Some("Domain layer".to_string()),
            }],
            anti_patterns: vec![AntiPatternEntry {
                id: "AP-001".to_string(),
                name: "Test Pattern".to_string(),
                category: "escape-hatch".to_string(),
                explanation: "Bad because reasons".to_string(),
                suggestion: "Do this instead".to_string(),
                severity: "warning".to_string(),
                enabled: true,
            }],
            conventions: vec![ConventionEntry {
                category: "spelling".to_string(),
                description: "Use UK English".to_string(),
                examples: vec!["organised".to_string()],
            }],
            suppressions: vec![SuppressionEntry {
                pattern_id: "AP-001".to_string(),
                file: "src/legacy.ts".to_string(),
                scope: "file".to_string(),
                reason: "Legacy code".to_string(),
                expires_at: Some("2026-12-31T00:00:00Z".to_string()),
            }],
            metadata: ConstraintMetadata {
                collected_at: "2026-03-31T00:00:00Z".to_string(),
                workspace_root: "/project".to_string(),
                has_baseline: true,
            },
        }
    }

    #[test]
    fn format_llms_txt_contains_all_sections() {
        let data = sample_constraint_data();
        let output = format_llms_txt(&data);
        assert!(output.contains("# anvil Architecture Constraints"));
        assert!(output.contains("## Boundary Rules"));
        assert!(output.contains("## Layer Definitions"));
        assert!(output.contains("## Anti-patterns (Blocked)"));
        assert!(output.contains("## Conventions"));
        assert!(output.contains("## Active Suppressions"));
        assert!(output.contains("no-domain-to-presentation"));
        assert!(output.contains("AP-001"));
        assert!(output.contains("src/legacy.ts"));
    }

    #[test]
    fn format_mcp_resource_is_valid_json_with_uri() {
        let data = sample_constraint_data();
        let output = format_mcp_resource(&data).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["uri"], "anvil://constraints");
        assert_eq!(parsed["mimeType"], "application/json");
        assert!(parsed["contents"]["boundaries"].is_array());
        assert!(parsed["contents"]["antiPatterns"].is_array());
    }

    #[test]
    fn format_prompt_fragment_contains_key_sections() {
        let data = sample_constraint_data();
        let output = format_prompt_fragment(&data);
        assert!(output.contains("**Architecture Boundaries**"));
        assert!(output.contains("**Forbidden Anti-patterns**"));
        assert!(output.contains("**Project Conventions**"));
        assert!(output.contains("**Active Suppressions**"));
    }

    #[test]
    fn parse_aps_markdown_non_phase_h2_does_not_create_phase() {
        let md = "\
# Plan

## In Scope — MVP

### NOT-001: this should be ignored

- **Status:** Draft

## Phase 1 — Real

### REAL-001: real item

- **Status:** Complete
";
        let plan = parse_aps_markdown(md);
        assert_eq!(plan.phases.len(), 1);
        assert_eq!(plan.phases[0].items[0].id, "REAL-001");
    }

    #[test]
    fn parse_aps_markdown_frontmatter_safe_on_empty_comment() {
        let md = "<!---->\n# Title\n";
        let plan = parse_aps_markdown(md);
        assert!(plan.frontmatter.is_none());
    }

    #[test]
    fn parse_aps_markdown_multi_line_intent() {
        let md = "\
# Plan

## Phase 1 — Work

### WI-001: multi-line intent

- **Status:** Proposed
- **Intent:** Create the project scaffold
  including the CLI binary and library crates
- **Priority:** High
";
        let plan = parse_aps_markdown(md);
        let item = &plan.phases[0].items[0];
        assert_eq!(
            item.intent.as_deref(),
            Some("Create the project scaffold including the CLI binary and library crates")
        );
        // Priority should still be parsed correctly after the continuation
        assert_eq!(item.priority.as_deref(), Some("High"));
    }

    #[test]
    fn parse_aps_markdown_continuation_only_appends_to_intent() {
        let md = "\
# Plan

## Phase 1 — Work

### WI-002: continuation after non-intent field

- **Intent:** Some intent
- **Priority:** High
  this line should NOT be appended to intent
";
        let plan = parse_aps_markdown(md);
        let item = &plan.phases[0].items[0];
        // Intent should not have the continuation since Priority was parsed last
        assert_eq!(item.intent.as_deref(), Some("Some intent"));
        assert_eq!(item.priority.as_deref(), Some("High"));
    }

    // =========================================================================
    // export_plan — end-to-end file I/O tests
    // =========================================================================

    fn default_global() -> GlobalArgs {
        GlobalArgs {
            json: false,
            no_tui: true,
            verbose: false,
            ..Default::default()
        }
    }

    fn sample_aps_markdown() -> &'static str {
        "\
<!--
APS Module: Test
Scopes: TST
-->

# Test Plan

| ID  | Name | Status      | Progress |
| --- | ---- | ----------- | -------- |
| TST | Test | In Progress | 1/3      |

## Purpose

Validate the export command.

## Phase 1 — Foundation

### TST-001: scaffold project

- **Status:** Complete
- **Intent:** Create the project scaffold
- **Priority:** High
- **Files:** `src/main.rs`, `Cargo.toml`

---

### TST-002: add tests

- **Status:** Proposed
- **Intent:** Write unit tests
"
    }

    #[test]
    fn export_plan_to_json() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("plan.aps.md");
        std::fs::write(&src, sample_aps_markdown()).unwrap();

        let out = tmp.path().join("plan.aps.json");
        export_plan(
            src.to_str().unwrap(),
            "json",
            Some(out.to_str().unwrap()),
            None,
            false,
            &default_global(),
        )
        .unwrap();

        let content = std::fs::read_to_string(&out).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["title"], "Test Plan");
        assert!(parsed["phases"].is_array());
        assert_eq!(parsed["phases"][0]["items"][0]["id"], "TST-001");
        // Pretty-printed JSON should contain newlines
        assert!(content.contains('\n'));
    }

    #[test]
    fn export_plan_to_json_compact() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("plan.aps.md");
        std::fs::write(&src, sample_aps_markdown()).unwrap();

        let out = tmp.path().join("plan.compact.json");
        export_plan(
            src.to_str().unwrap(),
            "json",
            Some(out.to_str().unwrap()),
            None,
            true,
            &default_global(),
        )
        .unwrap();

        let content = std::fs::read_to_string(&out).unwrap();
        // Compact JSON should be a single line (no embedded newlines)
        assert!(!content.contains('\n'));
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["title"], "Test Plan");
    }

    #[test]
    fn export_plan_to_yaml() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("plan.aps.md");
        std::fs::write(&src, sample_aps_markdown()).unwrap();

        let out = tmp.path().join("plan.aps.yaml");
        export_plan(
            src.to_str().unwrap(),
            "yaml",
            Some(out.to_str().unwrap()),
            None,
            false,
            &default_global(),
        )
        .unwrap();

        let content = std::fs::read_to_string(&out).unwrap();
        let parsed: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
        assert_eq!(
            parsed["title"].as_str(),
            Some("Test Plan"),
            "YAML output should contain title"
        );
    }

    #[test]
    fn export_plan_to_aps_format() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("plan.aps.md");
        std::fs::write(&src, sample_aps_markdown()).unwrap();

        let out = tmp.path().join("plan.aps.json");
        export_plan(
            src.to_str().unwrap(),
            "aps",
            Some(out.to_str().unwrap()),
            None,
            false,
            &default_global(),
        )
        .unwrap();

        let content = std::fs::read_to_string(&out).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["title"], "Test Plan");
        assert!(parsed["phases"][0]["items"].is_array());
        assert_eq!(parsed["phases"][0]["items"][0]["id"], "TST-001");
    }

    #[test]
    fn export_plan_default_output_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("mymod.aps.md");
        std::fs::write(&src, sample_aps_markdown()).unwrap();

        // No explicit output path — should derive from source
        export_plan(
            src.to_str().unwrap(),
            "json",
            None,
            None,
            false,
            &default_global(),
        )
        .unwrap();

        let expected = tmp.path().join("mymod.aps.json");
        assert!(expected.exists(), "default output path should be created");
    }

    #[test]
    fn export_plan_errors_on_missing_source() {
        let tmp = tempfile::TempDir::new().unwrap();
        let missing = tmp.path().join("nonexistent.md");
        assert!(
            !missing.exists(),
            "test setup error: expected missing source file to not exist"
        );

        let result = export_plan(
            missing.to_str().unwrap(),
            "json",
            None,
            None,
            false,
            &default_global(),
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Source file not found"),
            "should report missing source, got: {err}"
        );
    }

    #[test]
    fn export_plan_rejects_output_equal_to_source() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("plan.aps.md");
        std::fs::write(&src, sample_aps_markdown()).unwrap();
        let original_bytes = std::fs::read(&src).unwrap();

        let path = src.to_str().unwrap();
        let result = export_plan(path, "json", Some(path), None, false, &default_global());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("refusing to overwrite source"),
            "should reject output equal to source, got: {err}"
        );
        let after = std::fs::read(&src).unwrap();
        assert_eq!(after, original_bytes, "source plan bytes must be unchanged");
    }

    #[test]
    fn export_plan_rejects_output_resolving_to_source() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("plan.aps.md");
        std::fs::write(&src, sample_aps_markdown()).unwrap();
        let original_bytes = std::fs::read(&src).unwrap();

        // Equivalent path via parent `..` component — must not overwrite source.
        let out = tmp.path().join("nested").join("..").join("plan.aps.md");
        let result = export_plan(
            src.to_str().unwrap(),
            "json",
            Some(out.to_str().unwrap()),
            None,
            false,
            &default_global(),
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("refusing to overwrite source"),
            "should reject output that resolves to source, got: {err}"
        );
        let after = std::fs::read(&src).unwrap();
        assert_eq!(after, original_bytes, "source plan bytes must be unchanged");
    }

    #[test]
    fn export_plan_errors_on_from_override() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("plan.aps.md");
        std::fs::write(&src, sample_aps_markdown()).unwrap();

        let result = export_plan(
            src.to_str().unwrap(),
            "json",
            None,
            Some("yaml"),
            false,
            &default_global(),
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("--from override is not yet supported"),
            "should reject --from, got: {err}"
        );
    }

    #[test]
    fn export_plan_errors_on_unsupported_format() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("plan.aps.md");
        std::fs::write(&src, sample_aps_markdown()).unwrap();

        let result = export_plan(
            src.to_str().unwrap(),
            "xml",
            None,
            None,
            false,
            &default_global(),
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Unsupported target format"),
            "should reject unsupported format, got: {err}"
        );
    }

    #[test]
    fn export_plan_non_markdown_json_to_yaml() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("data.json");
        std::fs::write(&src, r#"{"key": "value", "count": 42}"#).unwrap();

        let out = tmp.path().join("data.aps.yaml");
        export_plan(
            src.to_str().unwrap(),
            "yaml",
            Some(out.to_str().unwrap()),
            None,
            false,
            &default_global(),
        )
        .unwrap();

        let content = std::fs::read_to_string(&out).unwrap();
        let parsed: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
        assert_eq!(parsed["key"].as_str(), Some("value"));
    }

    #[test]
    fn export_plan_non_markdown_yaml_to_json() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("data.yaml");
        std::fs::write(&src, "key: value\ncount: 42\n").unwrap();

        let out = tmp.path().join("data.aps.json");
        export_plan(
            src.to_str().unwrap(),
            "json",
            Some(out.to_str().unwrap()),
            None,
            false,
            &default_global(),
        )
        .unwrap();

        let content = std::fs::read_to_string(&out).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["key"], "value");
    }

    #[test]
    fn export_plan_json_output_mode() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("plan.aps.md");
        std::fs::write(&src, sample_aps_markdown()).unwrap();

        let out = tmp.path().join("plan.aps.json");
        let global = GlobalArgs {
            json: true,
            no_tui: true,
            verbose: false,
            ..Default::default()
        };
        // Should not panic — the JSON output goes to stdout
        export_plan(
            src.to_str().unwrap(),
            "json",
            Some(out.to_str().unwrap()),
            None,
            false,
            &global,
        )
        .unwrap();
        assert!(out.exists());
        let content = std::fs::read_to_string(&out).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["title"], "Test Plan");
    }

    // =========================================================================
    // run — dispatch logic tests
    // =========================================================================

    #[test]
    fn run_errors_when_neither_format_nor_to_specified() {
        let args = ExportArgs {
            source: None,
            to: None,
            format: None,
            output: None,
            from: None,
            compact: false,
        };
        let err = run(&args, &default_global()).unwrap_err().to_string();
        assert!(
            err.contains("--format or --to must be specified"),
            "should require --format or --to, got: {err}"
        );
    }

    #[test]
    fn run_errors_when_to_without_source() {
        let args = ExportArgs {
            source: None,
            to: Some("json".to_string()),
            format: None,
            output: None,
            from: None,
            compact: false,
        };
        let err = run(&args, &default_global()).unwrap_err().to_string();
        assert!(
            err.contains("Source file path is required"),
            "should require source for --to, got: {err}"
        );
    }

    #[test]
    fn run_routes_to_plan_export() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("plan.aps.md");
        std::fs::write(&src, sample_aps_markdown()).unwrap();

        let out = tmp.path().join("plan.aps.json");
        let args = ExportArgs {
            source: Some(src.to_str().unwrap().to_string()),
            to: Some("json".to_string()),
            format: None,
            output: Some(out.to_str().unwrap().to_string()),
            from: None,
            compact: false,
        };
        run(&args, &default_global()).unwrap();

        let content = std::fs::read_to_string(&out).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["title"], "Test Plan");
    }

    #[test]
    fn run_normalises_yml_target() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("plan.aps.md");
        std::fs::write(&src, sample_aps_markdown()).unwrap();

        let out = tmp.path().join("plan.aps.yaml");
        let args = ExportArgs {
            source: Some(src.to_str().unwrap().to_string()),
            to: Some("yml".to_string()),
            format: None,
            output: Some(out.to_str().unwrap().to_string()),
            from: None,
            compact: false,
        };
        run(&args, &default_global()).unwrap();
        assert!(out.exists(), "yml should normalise to yaml");
    }

    // =========================================================================
    // export_constraints — error path tests
    // =========================================================================

    #[test]
    fn export_constraints_errors_on_unsupported_format() {
        let err = export_constraints("csv", None, &default_global())
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("Unsupported format: csv"),
            "should reject unsupported constraint format, got: {err}"
        );
    }

    #[test]
    fn export_constraints_normalises_format_alias() {
        // "llms" normalises to "llms.txt" — exercise normalize_constraint_format
        // directly so we don't write files into the workspace root.
        let normalised = normalize_constraint_format("llms");
        assert_eq!(
            normalised, "llms.txt",
            "llms should normalise to llms.txt, got: {normalised}"
        );
    }
}
