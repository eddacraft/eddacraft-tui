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
];

pub(crate) fn definition_by_canonical(name: &str) -> Option<&'static CheckDefinition> {
    CHECK_DEFINITIONS
        .iter()
        .find(|def| def.canonical_name == name)
}

pub(crate) fn definition_by_stable_id(id: &str) -> Option<&'static CheckDefinition> {
    CHECK_DEFINITIONS.iter().find(|def| def.stable_id == id)
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
                stable_id.len() == "ANV-CORE-001".len()
                    && stable_id.starts_with("ANV-CORE-")
                    && stable_id[9..].chars().all(|ch| ch.is_ascii_digit()),
                "{} has invalid stable ID {}",
                definition.canonical_name,
                stable_id
            );
            assert!(
                ids.insert(stable_id),
                "duplicate stable check ID {}",
                stable_id
            );
        }
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
            ]
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
                assert!(aliases.insert(*alias), "duplicate alias '{}'", alias);
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
}
