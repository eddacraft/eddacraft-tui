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
    // Dispatchable (in GATE_INTERNAL_CHECKS) but gated dark behind the
    // `track.surface.sql` flag (SURFSQL-005), so the default gate run is
    // behaviourally unchanged until an operator opts in
    // (ANVIL_TRACK_SURFACE_SQL=1) or the flag default flips post-release.
    // `gate_config_supported=false`: activation is flag-driven, not via the
    // `.anvil` checks list, so it stays out of the default editable config.
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
    // Track 3 governance surface: dispatchable + in the default loop, but
    // gated dark behind `track.surface.sql` (SURFSQL-005) until opt-in.
    "sql-migrations",
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
            ]
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
