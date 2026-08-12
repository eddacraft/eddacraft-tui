//! DISTRIB-005 — cross-version config *schema* migration registry.
//!
//! This is deliberately distinct from MLP2-040's `.anvilrc` →
//! `.anvil.<ext>` *filename/format* migration, which lives in the CLI's
//! `commands/migrate.rs` under the `format` subcommand. That migration
//! changes where the config lives and how it is encoded; this module
//! reconciles the *contents* of an already-discovered `.anvil.<ext>`
//! config when a newer anvil minor version changes the config schema.
//!
//! The production registry is intentionally **empty** today:
//! [`production_migrations`] returns no entries because no shipped anvil
//! version has changed the config schema, so every real config resolves
//! to "no migration needed". The registry exists as the seam a future
//! schema change registers its transform into, so `anvil migrate schema`
//! can apply it without operators hand-editing files. Keeping it empty
//! but wired means the seam is exercised end-to-end (CLI plumbing, version
//! delta, dry-run/apply) before the first real migration arrives.

use semver::Version;
use serde_json::Value;

/// A single registered schema migration.
///
/// `apply` mutates a parsed config value in place; `introduced_in` is the
/// anvil version that first requires the transform. A project created by
/// an *earlier* version needs this migration when it is upgraded to
/// `introduced_in` or any later version.
pub struct SchemaMigration {
    /// Operator-facing one-line description rendered in the migration
    /// preview (e.g. `"rename `checks.import-boundaries` to
    /// `checks.imports`"`).
    pub description: String,
    /// The anvil version that introduced this schema change.
    pub introduced_in: Version,
    /// In-place transform applied to the parsed config value. The `fn`
    /// pointer accepts a bare function or a non-capturing closure (the
    /// tests use closures); it cannot capture environment — migrations are
    /// intentionally pure transforms with no captured state and no I/O.
    pub apply: fn(&mut Value),
}

impl SchemaMigration {
    /// Construct a migration whose `introduced_in` is parsed from a version
    /// string. Convenience for registry authors (and callers without a
    /// `semver` dependency, such as the CLI's tests) that hold a string
    /// literal rather than a [`Version`].
    pub fn new(
        description: impl Into<String>,
        introduced_in: &str,
        apply: fn(&mut Value),
    ) -> Result<Self, semver::Error> {
        Ok(Self {
            description: description.into(),
            introduced_in: Version::parse(introduced_in)?,
            apply,
        })
    }
}

impl std::fmt::Debug for SchemaMigration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `apply` is a fn pointer with no useful Debug; omit it.
        f.debug_struct("SchemaMigration")
            .field("description", &self.description)
            .field("introduced_in", &self.introduced_in)
            .finish_non_exhaustive()
    }
}

/// Legacy camelCase → canonical snake_case key pairs at the top level of
/// the project config. ADR-120 pt 3: snake_case is the canonical key space
/// across all formats; these are the keys the pre-UCFG-003 writers emitted
/// in camelCase (`init.rs` YAML/JSON, `start.rs pre_write_anvil_config_format`).
pub const LEGACY_CAMEL_KEYS: [(&str, &str); 2] = [
    ("schemaVersion", "schema_version"),
    ("planningDir", "planning_dir"),
];

/// Rename legacy camelCase keys to their canonical snake_case names, in
/// place, returning the camelCase keys that were present (sorted).
///
/// Rules: a camelCase value moves to the snake_case slot only when the
/// snake_case key is absent; when both exist the canonical snake_case
/// value wins and the shadowed camelCase duplicate is dropped. Either way
/// the camelCase key is removed and reported, so callers can render
/// [`legacy_keys_deprecation_note`]. Non-objects are left untouched.
///
/// This is an explicit opt-in at typed boundaries and the transform body
/// of the ADR-120 schema migration. It is deliberately **never** called by
/// `parse_str` / `parse_file`: raw parses keep raw keys so
/// [`crate::canonical_json_bytes`] consumers (rule-cache config SHA,
/// capsule digests) see unchanged bytes for unchanged files.
pub fn normalize_legacy_keys(config: &mut Value) -> Vec<String> {
    let Some(obj) = config.as_object_mut() else {
        return Vec::new();
    };
    let mut renamed: Vec<String> = Vec::new();
    for (camel, snake) in LEGACY_CAMEL_KEYS {
        if let Some(value) = obj.remove(camel) {
            if !obj.contains_key(snake) {
                obj.insert(snake.to_string(), value);
            }
            renamed.push(camel.to_string());
        }
    }
    renamed.sort();
    renamed
}

/// Render the operator-facing deprecation note for legacy camelCase keys
/// found by [`normalize_legacy_keys`]. `None` when nothing was renamed.
#[must_use]
pub fn legacy_keys_deprecation_note(renamed: &[String]) -> Option<String> {
    if renamed.is_empty() {
        return None;
    }
    Some(format!(
        "deprecated camelCase config keys detected: {} — snake_case is canonical \
         (ADR-120); run `anvil migrate schema` to rewrite the file",
        renamed.join(", ")
    ))
}

/// The production migration registry.
///
/// A future schema change appends its [`SchemaMigration`] here with the
/// version that introduced it (see the module docs).
#[must_use]
pub fn production_migrations() -> Vec<SchemaMigration> {
    vec![SchemaMigration {
        description: "rename legacy camelCase config keys (schemaVersion, planningDir) \
                      to canonical snake_case (ADR-120 / UCFG-003)"
            .to_string(),
        introduced_in: Version::parse("0.10.0-beta").expect("static version parses"),
        apply: |v| {
            normalize_legacy_keys(v);
        },
    }]
}

/// Select the migrations that apply when upgrading a config created by
/// `from` to the currently-running `to`, ordered oldest-first.
///
/// A migration applies when `from < introduced_in <= to`: the project
/// predates the schema change (so it has not been applied yet) and the
/// running version includes it. Equal versions and downgrades
/// (`to <= from`) therefore yield an empty plan — anvil does not author
/// downgrade transforms.
///
/// Pre-release semantics follow semver: a migration `introduced_in =
/// 0.7.0` does **not** apply when `to = 0.7.0-beta`, because
/// `0.7.0-beta < 0.7.0`. Pre-release binaries (e.g. beta testers) only
/// receive a migration once they run the stable tag at or after its
/// `introduced_in`.
#[must_use]
pub fn plan_for<'a>(
    from: &Version,
    to: &Version,
    migrations: &'a [SchemaMigration],
) -> Vec<&'a SchemaMigration> {
    let mut steps: Vec<&'a SchemaMigration> = migrations
        .iter()
        .filter(|m| m.introduced_in > *from && m.introduced_in <= *to)
        .collect();
    steps.sort_by(|a, b| a.introduced_in.cmp(&b.introduced_in));
    steps
}

/// Convenience wrapper over [`plan_for`] that parses the two version
/// strings first.
///
/// Returns [`semver::Error`] if either string is not valid semver — for
/// example a malformed `created_by_version` in `anvil/project-id`. Callers
/// that hold raw version strings (the CLI reads `created_by_version` and
/// `CARGO_PKG_VERSION`) use this so they need not depend on `semver`
/// directly.
pub fn plan_for_versions<'a>(
    from: &str,
    to: &str,
    migrations: &'a [SchemaMigration],
) -> Result<Vec<&'a SchemaMigration>, semver::Error> {
    let from = Version::parse(from)?;
    let to = Version::parse(to)?;
    Ok(plan_for(&from, &to, migrations))
}

/// Apply an ordered migration plan to a parsed config value in place.
///
/// Steps are applied in the order given; pass the output of [`plan_for`],
/// which is already sorted oldest-first.
pub fn apply_steps(steps: &[&SchemaMigration], config: &mut Value) {
    for step in steps {
        (step.apply)(config);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ver(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    /// Two fixture migrations: one that adds a marker key, one that renames
    /// a key. Returned out of version order on purpose so [`plan_for`]'s
    /// sort is exercised.
    fn fixture_registry() -> Vec<SchemaMigration> {
        vec![
            SchemaMigration {
                description: "0.8.0: rename `oldkey` to `newkey`".to_string(),
                introduced_in: ver("0.8.0"),
                apply: |v| {
                    if let Some(obj) = v.as_object_mut()
                        && let Some(val) = obj.remove("oldkey")
                    {
                        obj.insert("newkey".to_string(), val);
                    }
                },
            },
            SchemaMigration {
                description: "0.7.0: add `schema_touched` marker".to_string(),
                introduced_in: ver("0.7.0"),
                apply: |v| {
                    if let Some(obj) = v.as_object_mut() {
                        obj.insert("schema_touched".to_string(), json!(true));
                    }
                },
            },
        ]
    }

    #[test]
    fn schema_migration_new_parses_version_string() {
        let m = SchemaMigration::new("desc", "0.7.0", |_| {}).unwrap();
        assert_eq!(m.introduced_in, ver("0.7.0"));
        assert!(SchemaMigration::new("desc", "nope", |_| {}).is_err());
    }

    // ---- UCFG-003: legacy camelCase key normalisation ----

    #[test]
    fn normalize_renames_camel_keys_and_reports_them() {
        let mut v = json!({
            "schemaVersion": "1.0.0",
            "planningDir": "plans",
            "format": "yaml",
            "checks": ["a"],
        });
        let renamed = normalize_legacy_keys(&mut v);
        assert_eq!(renamed, vec!["planningDir", "schemaVersion"]);
        assert_eq!(v["schema_version"], "1.0.0");
        assert_eq!(v["planning_dir"], "plans");
        assert!(v.get("schemaVersion").is_none());
        assert!(v.get("planningDir").is_none());
        // Untouched keys survive.
        assert_eq!(v["format"], "yaml");
        assert_eq!(v["checks"][0], "a");
    }

    #[test]
    fn normalize_keeps_snake_value_when_both_present() {
        // A mixed-key file keeps the canonical snake_case value; the
        // shadowed camelCase duplicate is removed and still reported so
        // the deprecation note names it.
        let mut v = json!({
            "schemaVersion": "camel",
            "schema_version": "snake",
        });
        let renamed = normalize_legacy_keys(&mut v);
        assert_eq!(renamed, vec!["schemaVersion"]);
        assert_eq!(v["schema_version"], "snake");
        assert!(v.get("schemaVersion").is_none());
    }

    #[test]
    fn normalize_is_noop_on_snake_case_and_non_objects() {
        let mut v = json!({"schema_version": "1.0.0", "planning_dir": "plans"});
        assert!(normalize_legacy_keys(&mut v).is_empty());
        assert_eq!(v["schema_version"], "1.0.0");

        let mut arr = json!([1, 2]);
        assert!(normalize_legacy_keys(&mut arr).is_empty());
        assert_eq!(arr, json!([1, 2]));
    }

    #[test]
    fn deprecation_note_names_keys_and_remedy() {
        let note =
            legacy_keys_deprecation_note(&["schemaVersion".to_string()]).expect("note expected");
        assert!(note.contains("schemaVersion"), "got: {note}");
        assert!(note.contains("anvil migrate schema"), "got: {note}");
        assert!(legacy_keys_deprecation_note(&[]).is_none());
    }

    #[test]
    fn production_registry_carries_the_casing_migration_for_the_beta_window() {
        let registry = production_migrations();
        let steps = plan_for_versions("0.9.4-beta", "0.10.0-beta", &registry).unwrap();
        assert_eq!(steps.len(), 1, "expected the ADR-120 casing migration");

        let mut v = json!({"schemaVersion": "1.0.0", "planningDir": "plans"});
        apply_steps(&steps, &mut v);
        assert_eq!(v["schema_version"], "1.0.0");
        assert_eq!(v["planning_dir"], "plans");
        assert!(v.get("schemaVersion").is_none());
    }

    #[test]
    fn parse_does_not_normalise_keys_so_canonical_hashes_are_stable() {
        // Hash-stability contract (ADR-120 pt 3 constraint): key
        // normalisation is an explicit opt-in at typed boundaries, never
        // inside `parse_*`, so `canonical_json_bytes` consumers (rule
        // cache config SHA, capsule digests) see unchanged bytes for
        // unchanged files.
        let value = crate::parse_str(
            "schemaVersion: \"1.0.0\"\n",
            crate::ConfigFormat::Yaml,
            std::path::Path::new(".anvil.yaml"),
        )
        .expect("fixture parses");
        assert!(value.get("schemaVersion").is_some());
        assert!(value.get("schema_version").is_none());
        let bytes = crate::canonical_json_bytes(&value).expect("canonical bytes");
        assert!(String::from_utf8(bytes).unwrap().contains("schemaVersion"));
    }

    #[test]
    fn production_registry_holds_exactly_the_casing_migration() {
        // The honest current state: one shipped schema change — the
        // ADR-120 / UCFG-003 camelCase → snake_case key rename.
        let reg = production_migrations();
        assert_eq!(reg.len(), 1);
        assert_eq!(reg[0].introduced_in, ver("0.10.0-beta"));
    }

    #[test]
    fn registry_plan_spans_only_the_casing_migration_for_a_full_delta() {
        let reg = production_migrations();
        let plan = plan_for(&ver("0.1.0"), &ver("9.9.9"), &reg);
        assert_eq!(plan.len(), 1);
    }

    #[test]
    fn plan_includes_all_migrations_strictly_after_from_up_to_to() {
        let reg = fixture_registry();
        let plan = plan_for(&ver("0.6.0"), &ver("0.8.0"), &reg);
        assert_eq!(plan.len(), 2);
        // Oldest-first ordering despite the registry being out of order.
        assert_eq!(plan[0].introduced_in, ver("0.7.0"));
        assert_eq!(plan[1].introduced_in, ver("0.8.0"));
    }

    #[test]
    fn migration_at_exactly_from_is_excluded() {
        // A project *created by* 0.7.0 already has 0.7.0's schema.
        let reg = fixture_registry();
        let plan = plan_for(&ver("0.7.0"), &ver("0.8.0"), &reg);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].introduced_in, ver("0.8.0"));
    }

    #[test]
    fn migration_at_exactly_to_is_included() {
        let reg = fixture_registry();
        let plan = plan_for(&ver("0.6.0"), &ver("0.7.0"), &reg);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].introduced_in, ver("0.7.0"));
    }

    #[test]
    fn equal_versions_yield_empty_plan() {
        let reg = fixture_registry();
        assert!(plan_for(&ver("0.8.0"), &ver("0.8.0"), &reg).is_empty());
    }

    #[test]
    fn downgrade_yields_empty_plan() {
        let reg = fixture_registry();
        assert!(plan_for(&ver("0.8.0"), &ver("0.7.0"), &reg).is_empty());
    }

    #[test]
    fn migration_does_not_fire_when_to_is_prerelease_of_introduced_version() {
        // A 0.7.0 migration must NOT apply on a 0.7.0-beta binary
        // (0.7.0-beta < 0.7.0). Beta testers receive it only on the
        // stable tag. Immediately relevant: the workspace is on a -beta.
        let reg = fixture_registry();
        assert!(plan_for(&ver("0.6.0"), &ver("0.7.0-beta"), &reg).is_empty());
    }

    #[test]
    fn prerelease_origin_is_older_than_its_release() {
        // A project created by 0.7.0-beta, now running 0.7.0, picks up a
        // migration introduced in 0.7.0 (release > prerelease in semver).
        let reg = fixture_registry();
        let plan = plan_for(&ver("0.7.0-beta"), &ver("0.7.0"), &reg);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].introduced_in, ver("0.7.0"));
    }

    #[test]
    fn apply_steps_runs_transforms_in_order() {
        let reg = fixture_registry();
        let plan = plan_for(&ver("0.6.0"), &ver("0.8.0"), &reg);
        let mut config = json!({ "oldkey": "value", "keep": 1 });
        apply_steps(&plan, &mut config);

        let obj = config.as_object().unwrap();
        // 0.7.0 added the marker...
        assert_eq!(obj.get("schema_touched"), Some(&json!(true)));
        // ...and 0.8.0 renamed oldkey -> newkey.
        assert!(obj.get("oldkey").is_none());
        assert_eq!(obj.get("newkey"), Some(&json!("value")));
        // Untouched keys survive.
        assert_eq!(obj.get("keep"), Some(&json!(1)));
    }

    #[test]
    fn plan_for_versions_parses_strings_and_resolves() {
        let reg = fixture_registry();
        let plan = plan_for_versions("0.6.0", "0.8.0", &reg).unwrap();
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].introduced_in, ver("0.7.0"));
    }

    #[test]
    fn plan_for_versions_rejects_malformed_version() {
        let reg = fixture_registry();
        assert!(plan_for_versions("not-a-version", "0.8.0", &reg).is_err());
        assert!(plan_for_versions("0.6.0", "", &reg).is_err());
    }

    #[test]
    fn apply_empty_plan_leaves_config_unchanged() {
        let before = json!({ "checks": ["secret-detection"] });
        let mut after = before.clone();
        apply_steps(&[], &mut after);
        assert_eq!(before, after);
    }
}
