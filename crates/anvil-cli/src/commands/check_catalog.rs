use anvil_tui::surfaces::init::AvailableCheck;

#[derive(Debug, Clone, Copy)]
pub(crate) struct CheckDefinition {
    pub(crate) canonical_name: &'static str,
    pub(crate) internal_name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) init_enabled: bool,
    pub(crate) init_visible: bool,
    pub(crate) gate_supported: bool,
    pub(crate) gate_config_supported: bool,
}

pub(crate) const CHECK_DEFINITIONS: &[CheckDefinition] = &[
    CheckDefinition {
        canonical_name: "secret-detection",
        internal_name: "secret",
        description: "Detect leaked secrets and credentials",
        init_enabled: true,
        init_visible: true,
        gate_supported: true,
        gate_config_supported: true,
    },
    CheckDefinition {
        canonical_name: "import-boundaries",
        internal_name: "architecture",
        description: "Enforce module import boundaries",
        init_enabled: true,
        init_visible: true,
        gate_supported: true,
        gate_config_supported: true,
    },
    CheckDefinition {
        canonical_name: "antipattern-scan",
        internal_name: "antipattern-scan",
        description: "Detect common code antipatterns",
        init_enabled: true,
        init_visible: true,
        gate_supported: true,
        gate_config_supported: true,
    },
    CheckDefinition {
        canonical_name: "policy",
        internal_name: "policy",
        description: "Evaluate OPA policy rules",
        init_enabled: false,
        init_visible: true,
        gate_supported: true,
        gate_config_supported: true,
    },
    CheckDefinition {
        canonical_name: "lint",
        internal_name: "lint",
        description: "Code quality and style checks",
        init_enabled: false,
        init_visible: false,
        gate_supported: true,
        gate_config_supported: true,
    },
    CheckDefinition {
        canonical_name: "test",
        internal_name: "test",
        description: "Test suite execution",
        init_enabled: false,
        init_visible: false,
        gate_supported: true,
        gate_config_supported: true,
    },
    CheckDefinition {
        canonical_name: "coverage",
        internal_name: "coverage",
        description: "Code coverage thresholds",
        init_enabled: false,
        init_visible: false,
        gate_supported: true,
        gate_config_supported: true,
    },
    CheckDefinition {
        canonical_name: "dependency",
        internal_name: "dependency",
        description: "Dependency vulnerability scanning",
        init_enabled: false,
        init_visible: false,
        gate_supported: true,
        gate_config_supported: true,
    },
];

pub(crate) const DEFAULT_INIT_CHECKS: &[&str] = &[
    "secret-detection",
    "import-boundaries",
    "antipattern-scan",
];

pub(crate) const GATE_INTERNAL_CHECKS: &[&str] = &[
    "lint",
    "test",
    "coverage",
    "dependency",
    "antipattern-scan",
    "secret",
    "architecture",
    "policy",
];

pub(crate) fn definition_by_canonical(name: &str) -> Option<&'static CheckDefinition> {
    CHECK_DEFINITIONS.iter().find(|def| def.canonical_name == name)
}

pub(crate) fn canonical_check_name(name: &str) -> Option<&'static str> {
    definition_by_canonical(name).map(|def| def.canonical_name)
}

pub(crate) fn gate_internal_name(name: &str) -> Option<&'static str> {
    definition_by_canonical(name)
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
    DEFAULT_INIT_CHECKS.iter().map(|name| (*name).to_string()).collect()
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
        .map(|def| (def.canonical_name, def.description, def.canonical_name != "coverage"))
        .collect()
}
