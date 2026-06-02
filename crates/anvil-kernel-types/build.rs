//! FLAGCAT-004: generate Rust constants + `FeatureFlagDefinition` builders from
//! the canonical `flags/manifest.json` at build time, so the Rust surfaces
//! consume the same source of truth as the TS catalogue.
//!
//! Output goes to `$OUT_DIR/feature_flags_generated.rs`, included from
//! `src/feature_flags_catalogue.rs`. `serde_json` is a `[build-dependencies]`
//! entry — linker-isolated from the consumer crate, so no new *runtime*
//! dependency is added to `eddacraft-anvil-kernel-types`.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use serde_json::Value;

/// Walk upward from the crate manifest dir until a `Cargo.toml` containing
/// `[workspace]` is found; that directory is the workspace root.
fn workspace_root() -> PathBuf {
    let mut dir: PathBuf = env::var_os("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR is set by cargo")
        .into();
    loop {
        let candidate = dir.join("Cargo.toml");
        if candidate.is_file() {
            // An unreadable intermediate Cargo.toml shouldn't abort the upward
            // search — treat it as "not the root" and keep walking.
            let text = fs::read_to_string(&candidate).unwrap_or_default();
            // Match a `[workspace]` table header allowing a trailing comment
            // (`[workspace] # foo`), while excluding commented-out lines
            // (`# [workspace]`) and the distinct `[workspace.metadata]` table.
            if text.lines().any(|l| {
                l.trim_start().starts_with("[workspace]") && !l.trim_start().starts_with('#')
            }) {
                return dir;
            }
        }
        assert!(
            dir.pop(),
            "FLAGCAT-004 build.rs: workspace root (Cargo.toml with [workspace]) not found above CARGO_MANIFEST_DIR"
        );
    }
}

/// JSON `key` → Rust module path: `.`/`-` → `_`.
fn rust_module_name(key: &str) -> String {
    key.replace(['.', '-'], "_")
}

/// Variant key → `SCREAMING_SNAKE` constant name.
fn rust_variant_const(key: &str) -> String {
    key.replace(['.', '-'], "_").to_uppercase()
}

/// `snake_case` enum value → `PascalCase` Rust variant (`ops_kill_switch` →
/// `OpsKillSwitch`, `not_equals` → `NotEquals`).
fn pascal(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect()
}

/// Emit a Rust string literal token (escaped) for a JSON string value.
fn str_lit(s: &str) -> String {
    format!("{s:?}")
}

/// Emit `crate::FlagValue::…` for a variant value.
fn flag_value_lit(v: &Value) -> String {
    match v {
        Value::Bool(b) => format!("crate::FlagValue::Boolean({b})"),
        Value::Number(n) => {
            let f = n.as_f64().expect("variant numeric value");
            assert!(
                f.is_finite(),
                "FLAGCAT-004 build.rs: non-finite numeric variant value {f} cannot be emitted as a Rust literal"
            );
            format!("crate::FlagValue::Number({f:?})")
        }
        Value::String(s) => format!("crate::FlagValue::String({}.to_owned())", str_lit(s)),
        other => panic!(
            "FLAGCAT-004 build.rs: unsupported variant value (only boolean/string/number are emitted): {other}"
        ),
    }
}

/// Emit `crate::ConditionValue::…` for a targeting condition value.
fn condition_value_lit(v: &Value) -> String {
    match v {
        Value::String(s) => format!("crate::ConditionValue::Single({}.to_owned())", str_lit(s)),
        Value::Number(n) => {
            let f = n.as_f64().expect("condition numeric value");
            assert!(
                f.is_finite(),
                "FLAGCAT-004 build.rs: non-finite numeric condition value {f} cannot be emitted as a Rust literal"
            );
            format!("crate::ConditionValue::Numeric({f:?})")
        }
        Value::Array(items) => {
            let parts: Vec<String> = items
                .iter()
                .map(|i| {
                    format!(
                        "{}.to_owned()",
                        str_lit(i.as_str().expect("condition set value must be a string"))
                    )
                })
                .collect();
            format!("crate::ConditionValue::Set(vec![{}])", parts.join(", "))
        }
        other => panic!("FLAGCAT-004 build.rs: unsupported condition value: {other}"),
    }
}

/// Emit `Option<String>` field (`Some("…".to_owned())` / `None`).
fn opt_string_lit(v: Option<&Value>) -> String {
    match v.and_then(|x| x.as_str()) {
        Some(s) => format!("Some({}.to_owned())", str_lit(s)),
        None => "None".to_owned(),
    }
}

/// Emit the body of `pub fn definition() -> crate::FeatureFlagDefinition`.
fn definition_expr(flag: &Value) -> String {
    let get_str = |k: &str| -> &str {
        flag.get(k)
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("FLAGCAT-004 build.rs: flag missing string field {k}"))
    };

    let variants: Vec<String> = flag
        .get("variants")
        .and_then(|v| v.as_array())
        .expect("flag.variants")
        .iter()
        .map(|variant| {
            format!(
                "crate::FlagVariant {{ key: {}.to_owned(), value: {} }}",
                str_lit(
                    variant
                        .get("key")
                        .and_then(|k| k.as_str())
                        .expect("variant.key")
                ),
                flag_value_lit(variant.get("value").expect("variant.value")),
            )
        })
        .collect();

    let targeting = match flag.get("targeting").and_then(|t| t.as_array()) {
        None => "None".to_owned(),
        Some(rules) => {
            let rule_lits: Vec<String> = rules
                .iter()
                .map(|rule| {
                    let conditions: Vec<String> = rule
                        .get("conditions")
                        .and_then(|c| c.as_array())
                        .expect("rule.conditions")
                        .iter()
                        .map(|cond| {
                            format!(
                                "crate::TargetingCondition {{ attribute: {}.to_owned(), operator: crate::TargetingOperator::{}, value: {} }}",
                                str_lit(cond.get("attribute").and_then(|a| a.as_str()).expect("condition.attribute")),
                                pascal(cond.get("operator").and_then(|o| o.as_str()).expect("condition.operator")),
                                condition_value_lit(cond.get("value").expect("condition.value")),
                            )
                        })
                        .collect();
                    format!(
                        "crate::TargetingRule {{ conditions: vec![{}], variant: {}.to_owned() }}",
                        conditions.join(", "),
                        str_lit(rule.get("variant").and_then(|v| v.as_str()).expect("rule.variant")),
                    )
                })
                .collect();
            format!("Some(vec![{}])", rule_lits.join(", "))
        }
    };

    let tags = match flag.get("tags").and_then(|t| t.as_array()) {
        None => "None".to_owned(),
        Some(items) => {
            let parts: Vec<String> = items
                .iter()
                .map(|i| {
                    format!(
                        "{}.to_owned()",
                        str_lit(i.as_str().expect("tag must be a string"))
                    )
                })
                .collect();
            format!("Some(vec![{}])", parts.join(", "))
        }
    };

    format!(
        "crate::FeatureFlagDefinition {{\n        key: {key}.to_owned(),\n        owner: {owner}.to_owned(),\n        intent: {intent}.to_owned(),\n        class: crate::FlagClass::{class},\n        value_type: crate::FlagValueType::{value_type},\n        variants: vec![{variants}],\n        default_variant: {default_variant}.to_owned(),\n        status: crate::FlagStatus::{status},\n        created_for: {created_for}.to_owned(),\n        expiry_or_review_date: {expiry},\n        description: {description},\n        targeting: {targeting},\n        primary_group: {primary_group},\n        tags: {tags},\n    }}",
        key = str_lit(get_str("key")),
        owner = str_lit(get_str("owner")),
        intent = str_lit(get_str("intent")),
        class = pascal(get_str("class")),
        value_type = pascal(get_str("valueType")),
        variants = variants.join(", "),
        default_variant = str_lit(get_str("defaultVariant")),
        status = pascal(get_str("status")),
        created_for = str_lit(get_str("createdFor")),
        expiry = opt_string_lit(flag.get("expiryOrReviewDate")),
        description = opt_string_lit(flag.get("description")),
        targeting = targeting,
        primary_group = opt_string_lit(flag.get("primaryGroup")),
        tags = tags,
    )
}

fn main() {
    // Re-run when the build script itself changes — emitting any
    // `rerun-if-changed` line disables cargo's default build.rs tracking.
    println!("cargo:rerun-if-changed=build.rs");

    let root = workspace_root();
    let manifest_path = root.join("flags").join("manifest.json");
    println!("cargo:rerun-if-changed={}", manifest_path.display());
    // Expose the resolved absolute path so in-crate tests embed the SAME file
    // this script consumed, rather than a separately-hardcoded relative path.
    println!(
        "cargo:rustc-env=FLAGCAT_MANIFEST_PATH={}",
        manifest_path.display()
    );

    let display = manifest_path.display();
    let raw = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("FLAGCAT-004 build.rs: cannot read {display}: {e}"));
    let manifest: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("FLAGCAT-004 build.rs: invalid JSON in {display}: {e}"));

    let flags = manifest
        .get("flags")
        .and_then(|f| f.as_array())
        .expect("FLAGCAT-004 build.rs: manifest.flags must be an array");

    let mut out =
        String::from("// @generated by build.rs from flags/manifest.json — do not hand-edit\n\n");

    // Per-flag modules, sorted by key to match the manifest ordering rule.
    let mut sorted: Vec<&Value> = flags.iter().collect();
    sorted.sort_by_key(|f| {
        f.get("key")
            .and_then(|k| k.as_str())
            .unwrap_or_default()
            .to_owned()
    });

    let mut module_names: Vec<String> = Vec::with_capacity(sorted.len());
    let mut keys: Vec<String> = Vec::with_capacity(sorted.len());

    for flag in &sorted {
        let key = flag.get("key").and_then(|k| k.as_str()).expect("flag.key");
        let default_variant = flag
            .get("defaultVariant")
            .and_then(|d| d.as_str())
            .expect("flag.defaultVariant");
        keys.push(key.to_owned());
        let module = rust_module_name(key);
        // Two keys that normalise to the same module (e.g. `a.b` and `a-b`)
        // would emit duplicate `pub mod` blocks — a cryptic E0428. Fail here
        // with both offending keys instead.
        assert!(
            !module_names.contains(&module),
            "FLAGCAT-004 build.rs: flag key {key:?} normalises to module {module:?}, which collides with another key — rename one"
        );
        module_names.push(module.clone());

        let _ = writeln!(out, "pub mod {module} {{");
        let _ = writeln!(out, "    pub const KEY: &str = {};", str_lit(key));
        let _ = writeln!(
            out,
            "    pub const DEFAULT_VARIANT: &str = {};",
            str_lit(default_variant)
        );
        out.push_str("    pub mod variants {\n");
        if let Some(variants) = flag.get("variants").and_then(|v| v.as_array()) {
            for variant in variants {
                if let Some(vk) = variant.get("key").and_then(|k| k.as_str()) {
                    let _ = writeln!(
                        out,
                        "        pub const {}: &str = {};",
                        rust_variant_const(vk),
                        str_lit(vk)
                    );
                }
            }
        }
        out.push_str("    }\n");
        let _ = writeln!(
            out,
            "    /// Builds the catalogue-sourced `FeatureFlagDefinition`.\n    pub fn definition() -> crate::FeatureFlagDefinition {{\n        {}\n    }}",
            definition_expr(flag)
        );
        out.push_str("}\n\n");
    }

    out.push_str("pub mod all {\n    pub const KEYS: &[&str] = &[\n");
    for key in &keys {
        let _ = writeln!(out, "        {},", str_lit(key));
    }
    out.push_str("    ];\n\n");
    out.push_str("    /// Every catalogue flag definition, in sorted key order.\n");
    out.push_str(
        "    pub fn definitions() -> Vec<crate::FeatureFlagDefinition> {\n        vec![\n",
    );
    for module in &module_names {
        let _ = writeln!(out, "            super::{module}::definition(),");
    }
    out.push_str("        ]\n    }\n}\n");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR is set by cargo");
    let dest = PathBuf::from(out_dir).join("feature_flags_generated.rs");
    fs::write(&dest, out).expect("write feature_flags_generated.rs");
}
