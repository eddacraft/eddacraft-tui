use anvil_tui::surfaces::init::AvailableCheck;

#[derive(Debug, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct CheckDefinition {
    pub(crate) stable_id: &'static str,
    pub(crate) canonical_name: &'static str,
    pub(crate) internal_name: &'static str,
    pub(crate) aliases: &'static [&'static str],
    pub(crate) description: &'static str,
    pub(crate) init_enabled: bool,
    pub(crate) init_visible: bool,
    pub(crate) gate_supported: bool,
    pub(crate) gate_config_supported: bool,
    /// OPSUP-006 — file-shape patterns this check needs in the workspace.
    /// Empty means "always run" (the current default for every core
    /// check). Future Track 3 surface modules (e.g. `*.sql`, `Dockerfile`)
    /// declare patterns here so absent shapes short-circuit before any
    /// expensive work runs.
    pub(crate) file_shape_globs: &'static [&'static str],
    /// OPSUP-006 — per-check **soft** wall-time budget in whole seconds.
    /// `None` means "no budget declared". The budget is report-only:
    /// overrun is surfaced via
    /// [`super::check_guards::WallTimeGuard::Exceeded`] and appended to
    /// the result message; the check itself is NOT pre-empted because
    /// Rust threads cannot be safely cancelled mid-flight. The `_soft_`
    /// in the field name is deliberate so future Track 3/4 authors do
    /// not mistake this for a hard cancellation deadline.
    pub(crate) wall_time_soft_budget_secs: Option<u64>,
}

pub(crate) const CHECK_DEFINITIONS: &[CheckDefinition] = &[
    CheckDefinition {
        stable_id: "ANV-CORE-001",
        canonical_name: "secret-detection",
        internal_name: "secret",
        aliases: &["secret"],
        description: "Detect leaked secrets and credentials",
        init_enabled: true,
        init_visible: true,
        gate_supported: true,
        gate_config_supported: true,
        file_shape_globs: &[],
        wall_time_soft_budget_secs: None,
    },
    CheckDefinition {
        stable_id: "ANV-CORE-002",
        canonical_name: "import-boundaries",
        internal_name: "architecture",
        aliases: &["architecture"],
        description: "Enforce module import boundaries",
        init_enabled: true,
        init_visible: true,
        gate_supported: true,
        gate_config_supported: true,
        file_shape_globs: &[],
        wall_time_soft_budget_secs: None,
    },
    CheckDefinition {
        stable_id: "ANV-CORE-003",
        canonical_name: "antipattern-scan",
        internal_name: "antipattern-scan",
        aliases: &[],
        description: "Detect common code antipatterns",
        init_enabled: true,
        init_visible: true,
        gate_supported: true,
        gate_config_supported: true,
        file_shape_globs: &[],
        wall_time_soft_budget_secs: None,
    },
    CheckDefinition {
        stable_id: "ANV-CORE-004",
        canonical_name: "policy",
        internal_name: "policy",
        aliases: &[],
        description: "Evaluate OPA policy rules",
        init_enabled: false,
        init_visible: true,
        gate_supported: true,
        gate_config_supported: true,
        file_shape_globs: &[],
        wall_time_soft_budget_secs: None,
    },
    CheckDefinition {
        stable_id: "ANV-CORE-005",
        canonical_name: "lint",
        internal_name: "lint",
        aliases: &[],
        description: "Code quality and style checks",
        init_enabled: false,
        init_visible: false,
        gate_supported: true,
        gate_config_supported: true,
        file_shape_globs: &[],
        wall_time_soft_budget_secs: None,
    },
    CheckDefinition {
        stable_id: "ANV-CORE-006",
        canonical_name: "test",
        internal_name: "test",
        aliases: &[],
        description: "Test suite execution",
        init_enabled: false,
        init_visible: false,
        gate_supported: true,
        gate_config_supported: true,
        file_shape_globs: &[],
        wall_time_soft_budget_secs: None,
    },
    CheckDefinition {
        stable_id: "ANV-CORE-007",
        canonical_name: "coverage",
        internal_name: "coverage",
        aliases: &[],
        description: "Code coverage thresholds",
        init_enabled: false,
        init_visible: false,
        gate_supported: true,
        gate_config_supported: true,
        file_shape_globs: &[],
        wall_time_soft_budget_secs: None,
    },
    CheckDefinition {
        stable_id: "ANV-CORE-008",
        canonical_name: "dependency",
        internal_name: "dependency",
        aliases: &[],
        description: "Dependency vulnerability scanning",
        init_enabled: false,
        init_visible: false,
        gate_supported: true,
        gate_config_supported: true,
        file_shape_globs: &[],
        wall_time_soft_budget_secs: None,
    },
    CheckDefinition {
        stable_id: "ANV-CORE-009",
        canonical_name: "command-safety",
        internal_name: "command-safety",
        aliases: &[],
        description: "Detect dangerous shell commands in plan-described scripts",
        init_enabled: false,
        init_visible: false,
        gate_supported: true,
        gate_config_supported: true,
        file_shape_globs: &[],
        wall_time_soft_budget_secs: None,
    },
    // SURFSQL (Track 3) — the first governance-surface check in the registry.
    // Dispatchable (in GATE_INTERNAL_CHECKS) and on by default behind the
    // `track.surface.sql` flag (SURFSQL-005), which graduated to default-on
    // after the v0.8.1-beta clean release; `ANVIL_TRACK_SURFACE_SQL=0` opts a
    // session out. `gate_config_supported=false`: activation is flag-driven,
    // not via the `.anvil` checks list, so it stays out of the editable config.
    CheckDefinition {
        stable_id: "ANV-SURF-SQL-001",
        canonical_name: "sql-migrations",
        internal_name: "sql-migrations",
        aliases: &["sql"],
        description: "Flag destructive/irreversible operations in SQL migrations",
        init_enabled: false,
        // Not an `anvil init` wizard toggle — activation is flag-driven
        // (track.surface.sql), so it stays out of the init available-checks list.
        init_visible: false,
        gate_supported: true,
        gate_config_supported: false,
        file_shape_globs: &[
            "*.sql",
            "migrations/**",
            "db/migrations/**",
            "supabase/migrations/**",
        ],
        wall_time_soft_budget_secs: Some(30),
    },
    // SURFGHA (Track 3) — same contract as SURFSQL: dispatchable and default-on
    // behind `track.surface.gha` (SURFGHA-006, graduated post-v0.8.1-beta);
    // flag-driven activation, so out of the init wizard and editable config.
    CheckDefinition {
        stable_id: "ANV-SURF-GHA-001",
        canonical_name: "github-actions",
        internal_name: "github-actions",
        aliases: &["gha"],
        description: "Flag supply-chain risks in GitHub Actions workflows",
        init_enabled: false,
        init_visible: false,
        gate_supported: true,
        gate_config_supported: false,
        file_shape_globs: &[".github/workflows/*.yml", ".github/workflows/*.yaml"],
        wall_time_soft_budget_secs: Some(30),
    },
    // SURFDOCK (Track 3) — same opt-in contract. `file_shape_globs` is empty
    // because the file-presence matcher's only any-depth, name-anchored form is
    // `*.ext` (case-sensitive); it has no any-depth name-only match, so a bare
    // `Dockerfile`/`Containerfile` (no extension) at arbitrary depth and the
    // case-insensitive `*.Dockerfile`/suffixed `Dockerfile.<v>` variants can't
    // be expressed. The check self-filters via `is_dockerfile` over the walked
    // files (cheap), and the `track.surface.dock` flag gate skips it when an
    // operator opts the session out (ANVIL_TRACK_SURFACE_DOCK=0); it is
    // default-on (graduated post-v0.8.1-beta).
    CheckDefinition {
        stable_id: "ANV-SURF-DOCK-001",
        canonical_name: "dockerfile",
        internal_name: "dockerfile",
        aliases: &["dock"],
        description: "Flag build-hygiene / supply-chain risks in Dockerfiles",
        init_enabled: false,
        init_visible: false,
        gate_supported: true,
        gate_config_supported: false,
        file_shape_globs: &[],
        wall_time_soft_budget_secs: Some(30),
    },
    // SURFSH (Track 3, T1) — same contract; default-on behind `track.surface.sh`
    // (graduated post-v0.8.1-beta). Reuses the shared command_safety catalogue.
    CheckDefinition {
        stable_id: "ANV-SURF-SH-001",
        canonical_name: "shell-scripts",
        internal_name: "shell-scripts",
        aliases: &["sh", "shell"],
        description: "Flag dangerous commands in checked-in shell scripts",
        init_enabled: false,
        init_visible: false,
        gate_supported: true,
        gate_config_supported: false,
        file_shape_globs: &["*.sh", "*.bash"],
        wall_time_soft_budget_secs: Some(30),
    },
];

pub(crate) const DEFAULT_INIT_CHECKS: &[&str] =
    &["secret-detection", "import-boundaries", "antipattern-scan"];

pub(crate) const GATE_INTERNAL_CHECKS: &[&str] = &[
    "lint",
    "test",
    "coverage",
    "dependency",
    "antipattern-scan",
    "secret",
    "architecture",
    "policy",
    "command-safety",
    // Track 3 governance surfaces: dispatchable + in the default loop, on by
    // default behind their `track.surface.*` flag (graduated post-v0.8.1-beta;
    // ANVIL_TRACK_SURFACE_<X>=0 opts a session out).
    "sql-migrations",
    "github-actions",
    "dockerfile",
    "shell-scripts",
];

pub(crate) fn definition_by_canonical(name: &str) -> Option<&'static CheckDefinition> {
    CHECK_DEFINITIONS
        .iter()
        .find(|def| def.canonical_name == name)
}

pub(crate) fn definition_by_stable_id(id: &str) -> Option<&'static CheckDefinition> {
    CHECK_DEFINITIONS.iter().find(|def| def.stable_id == id)
}

/// Lookup by the internal dispatcher name (`internal_name`). The gate
/// dispatch loop in `gate.rs` keys on internal names; OPSUP-006 guards
/// also key on those so the lookup needs to be O(1)-callable from
/// `run_single_check`.
pub(crate) fn definition_by_internal(name: &str) -> Option<&'static CheckDefinition> {
    CHECK_DEFINITIONS
        .iter()
        .find(|def| def.internal_name == name)
}

pub(crate) fn definition_by_name(name: &str) -> Option<&'static CheckDefinition> {
    definition_by_stable_id(name).or_else(|| {
        CHECK_DEFINITIONS
            .iter()
            .find(|def| def.canonical_name == name || def.aliases.contains(&name))
    })
}

pub(crate) fn canonical_check_name(name: &str) -> Option<&'static str> {
    definition_by_name(name).map(|def| def.canonical_name)
}

/// Suggest the registered identifier closest to an unknown `input`, or
/// `None` when nothing is near enough to be a useful "did you mean…".
///
/// OPSUP-002: an unknown `--skip-checks` / `.anvil.<ext>` `checks:` entry must
/// produce a deterministic error that names the closest registered ID rather
/// than silently skipping nothing.
///
/// Each definition contributes its canonical name, its stable `ANV-*` ID, and
/// its legacy aliases as *match targets*. When the closest target is a name or
/// alias the suggestion is the definition's **canonical** name (steering toward
/// the recommended form, not a legacy alias); when it is the stable ID the
/// suggestion is the ID itself.
///
/// Matching is by Levenshtein distance computed case-insensitively (so a
/// wrong-cased `anv-core-005` or `LINT` is still pointed at the right form),
/// gated by a per-target threshold of `max(len, 3) / 3` — the same
/// length-relative heuristic rustc uses — so a short input cannot be dragged
/// toward an unrelated short check (`dep` must not "correct" to `test`). Ties
/// resolve to the first match in registry order, so the result is
/// deterministic.
pub(crate) fn closest_registered_id(input: &str) -> Option<&'static str> {
    /// No legitimate check identifier is anywhere near this long. Capping the
    /// needle before the O(n·m) Levenshtein DP stops a pathological input — e.g.
    /// a multi-megabyte string in a cloned repo's `.anvilrc#checks` — from
    /// burning CPU on every `anvil gate` run. Anything this far from a ~16-char
    /// identifier is past the suggestion threshold anyway.
    const MAX_NEEDLE_LEN: usize = 64;

    if input.is_empty() || input.len() > MAX_NEEDLE_LEN {
        return None;
    }
    let needle = input.to_ascii_lowercase();

    let mut best: Option<(usize, &'static str)> = None;
    for def in CHECK_DEFINITIONS {
        // (match target, suggestion). Aliases map to the canonical name.
        let targets = [
            (def.canonical_name, def.canonical_name),
            (def.stable_id, def.stable_id),
        ]
        .into_iter()
        .chain(def.aliases.iter().map(|alias| (*alias, def.canonical_name)));

        for (target, suggestion) in targets {
            let distance = levenshtein(&needle, &target.to_ascii_lowercase());
            let max_distance = std::cmp::max(target.chars().count(), 3) / 3;
            if distance <= max_distance && best.is_none_or(|(d, _)| distance < d) {
                best = Some((distance, suggestion));
            }
        }
    }

    best.map(|(_, suggestion)| suggestion)
}

/// Levenshtein edit distance over Unicode scalar values. The inputs here are
/// short ASCII check identifiers, so the simple two-row DP is more than fast
/// enough and avoids pulling in a dependency for a handful of comparisons.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }

    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr: Vec<usize> = vec![0; b.len() + 1];

    for (i, &ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            curr[j + 1] = (prev[j + 1] + 1) // deletion
                .min(curr[j] + 1) // insertion
                .min(prev[j] + cost); // substitution
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b.len()]
}

pub(crate) fn gate_internal_name(name: &str) -> Option<&'static str> {
    definition_by_name(name)
        .filter(|def| def.gate_supported)
        .map(|def| def.internal_name)
}

pub(crate) fn gate_canonical_name_from_internal(name: &str) -> String {
    CHECK_DEFINITIONS
        .iter()
        .find(|def| def.internal_name == name && def.gate_supported)
        .map_or_else(|| name.to_string(), |def| def.canonical_name.to_string())
}

pub(crate) fn gate_canonical_names() -> Vec<&'static str> {
    CHECK_DEFINITIONS
        .iter()
        .filter(|def| def.gate_supported)
        .map(|def| def.canonical_name)
        .collect()
}

pub(crate) fn default_init_check_names() -> Vec<String> {
    DEFAULT_INIT_CHECKS
        .iter()
        .map(|name| (*name).to_string())
        .collect()
}

pub(crate) fn default_init_available_checks() -> Vec<AvailableCheck> {
    CHECK_DEFINITIONS
        .iter()
        .filter(|def| def.init_visible)
        .map(|def| AvailableCheck {
            name: def.canonical_name.to_string(),
            description: def.description.to_string(),
            enabled: def.init_enabled,
        })
        .collect()
}

pub(crate) fn default_gate_config_checks() -> Vec<(&'static str, &'static str, bool)> {
    CHECK_DEFINITIONS
        .iter()
        .filter(|def| def.gate_config_supported)
        .map(|def| {
            (
                def.canonical_name,
                def.description,
                def.canonical_name != "coverage",
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn closest_registered_id_suggests_near_canonical_typo() {
        // A one-character typo of a canonical name resolves to it.
        assert_eq!(closest_registered_id("lnt"), Some("lint"));
        assert_eq!(closest_registered_id("coverage-"), Some("coverage"));
    }

    #[test]
    fn closest_registered_id_suggests_near_stable_id_typo() {
        // A near-miss of a stable ID suggests the stable ID, not a flat list.
        assert_eq!(
            closest_registered_id("ANV-SURF-SQL-01"),
            Some("ANV-SURF-SQL-001")
        );
    }

    #[test]
    fn closest_registered_id_returns_none_for_far_input() {
        // An input nowhere near any registered identifier yields no
        // suggestion rather than a misleading one.
        assert_eq!(closest_registered_id("totally-different-xyz"), None);
    }

    #[test]
    fn closest_registered_id_does_not_suggest_for_empty() {
        assert_eq!(closest_registered_id(""), None);
    }

    #[test]
    fn closest_registered_id_caps_pathological_input_without_running_levenshtein() {
        // Any input beyond the cap must short-circuit to None rather than
        // running the O(n·m) DP over the registry.
        let over_cap = "x".repeat(65);
        assert_eq!(closest_registered_id(&over_cap), None);
    }

    #[test]
    fn closest_registered_id_does_not_drag_short_abbreviations_to_short_checks() {
        // The length-relative threshold must stop a short abbreviation from
        // "correcting" to an unrelated short check at edit distance 2-3
        // (`dep`/`sec` previously dragged to `test`).
        for abbrev in ["dep", "sec"] {
            assert_eq!(
                closest_registered_id(abbrev),
                None,
                "{abbrev} must not be dragged to an unrelated short check"
            );
        }
        // A near-neighbour suggestion is still fine — `sect` is two edits from
        // the `secret` alias, so it steers to that canonical, never to the
        // unrelated `test`. Asserted positively so a future registry change
        // can't silently regress it back toward a wrong short check.
        assert_eq!(closest_registered_id("sect"), Some("secret-detection"));
    }

    #[test]
    fn closest_registered_id_is_case_insensitive() {
        // A wrong-cased identifier is pointed at the correctly-cased form.
        assert_eq!(closest_registered_id("LINT"), Some("lint"));
        assert_eq!(closest_registered_id("anv-core-005"), Some("ANV-CORE-005"));
    }

    #[test]
    fn closest_registered_id_suggests_canonical_for_alias_typo() {
        // A typo of a legacy alias steers toward the canonical name, not the
        // alias. `architecture` is the legacy alias for `import-boundaries`.
        assert_eq!(
            closest_registered_id("architecure"),
            Some("import-boundaries")
        );
    }

    #[test]
    fn registry_assigns_unique_stable_ids() {
        let mut ids = HashSet::new();

        for definition in CHECK_DEFINITIONS {
            let stable_id = definition.stable_id;
            assert!(
                is_valid_stable_id(stable_id),
                "{} has invalid stable ID {}",
                definition.canonical_name,
                stable_id
            );
            assert!(
                ids.insert(stable_id),
                "duplicate stable check ID {stable_id}"
            );
        }
    }

    /// Stable check IDs follow `ANV-CORE-NNN` for core checks and
    /// `ANV-SURF-<SURFACE>-NNN` / `ANV-PACK-<PACK>-NNN` for Track 3/4
    /// surface and pack checks (OPSUP-001 scheme). After the `ANV-` prefix
    /// every leading segment is uppercase letters (the family / surface /
    /// pack name) and the final segment is a zero-padded 3-digit counter.
    fn is_valid_stable_id(id: &str) -> bool {
        let Some(rest) = id.strip_prefix("ANV-") else {
            return false;
        };
        let segments: Vec<&str> = rest.split('-').collect();
        if segments.len() < 2 {
            return false;
        }
        let (last, head) = segments.split_last().expect("len checked >= 2");
        // Final segment is a zero-padded 3-digit counter (e.g. `001`),
        // matching the established `ANV-CORE-009` convention.
        if last.len() != 3 || !last.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        // Leading segments (family / surface / pack) are uppercase letters
        // (CORE / SURF / PACK / SQL / PULUMI …) — no digits.
        head.iter()
            .all(|seg| !seg.is_empty() && seg.chars().all(|c| c.is_ascii_uppercase()))
    }

    #[test]
    fn registry_pins_current_stable_id_assignments() {
        let actual: Vec<(&str, &str)> = CHECK_DEFINITIONS
            .iter()
            .map(|def| (def.canonical_name, def.stable_id))
            .collect();

        assert_eq!(
            actual,
            vec![
                ("secret-detection", "ANV-CORE-001"),
                ("import-boundaries", "ANV-CORE-002"),
                ("antipattern-scan", "ANV-CORE-003"),
                ("policy", "ANV-CORE-004"),
                ("lint", "ANV-CORE-005"),
                ("test", "ANV-CORE-006"),
                ("coverage", "ANV-CORE-007"),
                ("dependency", "ANV-CORE-008"),
                ("command-safety", "ANV-CORE-009"),
                ("sql-migrations", "ANV-SURF-SQL-001"),
                ("github-actions", "ANV-SURF-GHA-001"),
                ("dockerfile", "ANV-SURF-DOCK-001"),
                ("shell-scripts", "ANV-SURF-SH-001"),
            ]
        );
    }

    #[test]
    fn shell_scripts_check_resolves_and_declares_shell_shapes() {
        let def = definition_by_name("shell-scripts").expect("registered");
        assert_eq!(def.stable_id, "ANV-SURF-SH-001");
        assert_eq!(
            definition_by_name("sh").map(|d| d.stable_id),
            Some("ANV-SURF-SH-001")
        );
        assert!(!def.init_enabled, "Track 3 surface ships opt-in");
        assert!(def.gate_supported);
        assert!(def.file_shape_globs.contains(&"*.sh"));
        assert!(def.file_shape_globs.contains(&"*.bash"));
    }

    #[test]
    fn dockerfile_check_resolves_and_is_self_filtering() {
        let def = definition_by_name("dockerfile").expect("registered");
        assert_eq!(def.stable_id, "ANV-SURF-DOCK-001");
        assert_eq!(
            definition_by_name("dock").map(|d| d.stable_id),
            Some("ANV-SURF-DOCK-001")
        );
        assert!(!def.init_enabled, "Track 3 surface ships opt-in");
        assert!(def.gate_supported);
        // Empty globs by design — Dockerfile naming can't be expressed in the
        // file-presence glob vocabulary; the check self-filters.
        assert!(def.file_shape_globs.is_empty());
    }

    #[test]
    fn github_actions_check_resolves_and_declares_workflow_shapes() {
        let def = definition_by_name("github-actions").expect("registered");
        assert_eq!(def.stable_id, "ANV-SURF-GHA-001");
        assert_eq!(
            definition_by_name("gha").map(|d| d.stable_id),
            Some("ANV-SURF-GHA-001")
        );
        assert!(!def.init_enabled, "Track 3 surface ships opt-in");
        assert!(def.gate_supported);
        assert!(
            def.file_shape_globs.contains(&".github/workflows/*.yml"),
            "must declare workflow globs for the file-presence guard"
        );
    }

    #[test]
    fn surface_stable_id_scheme_is_accepted_and_core_unchanged() {
        assert!(is_valid_stable_id("ANV-SURF-SQL-001"));
        assert!(is_valid_stable_id("ANV-PACK-PULUMI-012"));
        assert!(is_valid_stable_id("ANV-CORE-001"));
        // Malformed: lowercase, missing counter, non-numeric counter,
        // non-3-digit counter, digit family segment, missing prefix.
        assert!(!is_valid_stable_id("ANV-surf-sql-001"));
        assert!(!is_valid_stable_id("ANV-SURF-SQL"));
        assert!(!is_valid_stable_id("ANV-SURF-SQL-foo"));
        assert!(!is_valid_stable_id("ANV-CORE-0001"));
        assert!(!is_valid_stable_id("ANV-1-001"));
        assert!(!is_valid_stable_id("SURF-SQL-001"));
    }

    #[test]
    fn sql_migrations_check_resolves_and_declares_sql_file_shapes() {
        let def = definition_by_name("sql-migrations").expect("registered");
        assert_eq!(def.stable_id, "ANV-SURF-SQL-001");
        assert_eq!(
            definition_by_name("sql").map(|d| d.stable_id),
            Some("ANV-SURF-SQL-001")
        );
        assert!(!def.init_enabled, "Track 3 surface ships opt-in");
        assert!(def.gate_supported);
        assert!(
            def.file_shape_globs.contains(&"*.sql"),
            "must declare .sql so the file-presence guard short-circuits clean repos"
        );
    }

    #[test]
    fn registry_makes_internal_renames_explicit_aliases() {
        for definition in CHECK_DEFINITIONS {
            if definition.internal_name != definition.canonical_name {
                assert!(
                    definition.aliases.contains(&definition.internal_name),
                    "{} internal name '{}' must be an explicit alias",
                    definition.canonical_name,
                    definition.internal_name
                );
            }
        }
    }

    #[test]
    fn registry_aliases_do_not_collide() {
        let stable_ids: HashSet<&str> = CHECK_DEFINITIONS.iter().map(|def| def.stable_id).collect();
        let canonical_names: HashSet<&str> = CHECK_DEFINITIONS
            .iter()
            .map(|def| def.canonical_name)
            .collect();
        let mut aliases = HashSet::new();

        for definition in CHECK_DEFINITIONS {
            for alias in definition.aliases {
                assert!(
                    !stable_ids.contains(alias),
                    "{} alias '{}' collides with a stable ID",
                    definition.canonical_name,
                    alias
                );
                assert!(
                    !canonical_names.contains(alias),
                    "{} alias '{}' collides with a canonical name",
                    definition.canonical_name,
                    alias
                );
                assert!(aliases.insert(*alias), "duplicate alias '{alias}'");
            }
        }
    }

    #[test]
    fn gate_supported_registry_entries_match_dispatch_list() {
        for definition in CHECK_DEFINITIONS.iter().filter(|def| def.gate_supported) {
            assert!(
                GATE_INTERNAL_CHECKS.contains(&definition.internal_name),
                "gate-supported check '{}' has no dispatch entry '{}'",
                definition.canonical_name,
                definition.internal_name
            );
        }

        for internal_name in GATE_INTERNAL_CHECKS {
            let matches = CHECK_DEFINITIONS
                .iter()
                .filter(|def| def.gate_supported && def.internal_name == *internal_name)
                .count();
            assert_eq!(
                matches, 1,
                "dispatch entry '{internal_name}' must map to exactly one gate-supported check"
            );
        }
    }

    #[test]
    fn registry_resolves_by_stable_id() {
        let definition = definition_by_stable_id("ANV-CORE-001").unwrap();

        assert_eq!(definition.canonical_name, "secret-detection");
    }

    #[test]
    fn registry_resolves_legacy_aliases() {
        let secret = definition_by_name("secret").unwrap();
        let boundaries = definition_by_name("architecture").unwrap();

        assert_eq!(secret.stable_id, "ANV-CORE-001");
        assert_eq!(secret.canonical_name, "secret-detection");
        assert!(secret.aliases.contains(&"secret"));
        assert_eq!(boundaries.stable_id, "ANV-CORE-002");
        assert_eq!(boundaries.canonical_name, "import-boundaries");
        assert!(boundaries.aliases.contains(&"architecture"));
    }

    #[test]
    fn canonical_name_resolves_from_stable_id() {
        assert_eq!(
            canonical_check_name("ANV-CORE-003"),
            Some("antipattern-scan")
        );
    }

    #[test]
    fn opsup_006_core_checks_default_to_unguarded() {
        // Migration-safety contract for OPSUP-006: every current *core*
        // check ships with no file-shape guard and no wall-time cap so
        // observable gate behaviour is unchanged. Surface/pack checks
        // (ANV-SURF-*/ANV-PACK-*) opt in by declaring file shapes and are
        // exempt — they stay dark behind their track flag until opt-in.
        for definition in CHECK_DEFINITIONS
            .iter()
            .filter(|def| def.stable_id.starts_with("ANV-CORE-"))
        {
            assert!(
                definition.file_shape_globs.is_empty(),
                "{} ({}) regressed OPSUP-006 default — core checks must not declare file_shape_globs",
                definition.canonical_name,
                definition.stable_id
            );
            assert!(
                definition.wall_time_soft_budget_secs.is_none(),
                "{} ({}) regressed OPSUP-006 default — core checks must not declare wall_time_soft_budget_secs",
                definition.canonical_name,
                definition.stable_id
            );
        }
    }

    #[test]
    fn definition_by_internal_resolves_dispatch_names() {
        // The gate dispatch loop keys on internal names; OPSUP-006 guards
        // look up the definition via internal_name to consult the
        // declared file-shape / wall-time fields.
        for internal_name in GATE_INTERNAL_CHECKS {
            let def = definition_by_internal(internal_name).unwrap_or_else(|| {
                panic!("internal dispatch name '{internal_name}' must resolve to a definition")
            });
            assert_eq!(def.internal_name, *internal_name);
            assert!(def.gate_supported);
        }
    }
}
