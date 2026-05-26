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

/// The production migration registry.
///
/// Empty until a shipped anvil version changes the config schema (see the
/// module docs). A future schema change appends its [`SchemaMigration`]
/// here with the version that introduced it.
#[must_use]
pub fn production_migrations() -> Vec<SchemaMigration> {
    Vec::new()
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

    #[test]
    fn production_registry_is_empty() {
        // The honest current state: no shipped schema change.
        assert!(production_migrations().is_empty());
    }

    #[test]
    fn empty_registry_yields_empty_plan_for_any_delta() {
        let reg = production_migrations();
        let plan = plan_for(&ver("0.1.0"), &ver("9.9.9"), &reg);
        assert!(plan.is_empty());
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
