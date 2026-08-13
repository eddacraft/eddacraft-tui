use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ArchitectureConfig {
    pub layers: Vec<LayerDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayerDef {
    pub name: String,
    pub paths: Vec<String>,
    #[serde(default)]
    pub allowed_imports: Vec<String>,
}

impl ArchitectureConfig {
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Parse the kernel schema (`layers` list) or the definition schema
    /// (`layers` map with `patterns` / `depends_on`). A main-config
    /// document whose `architecture:` section is inline is accepted so
    /// `anvil watch` can load and reload inline sections from the same
    /// file (UCFG-013).
    pub fn from_yaml_any(yaml: &str) -> Result<Self, serde_yaml::Error> {
        let value: serde_yaml::Value = serde_yaml::from_str(yaml)?;
        Self::from_value_any(value)
    }

    fn from_value_any(value: serde_yaml::Value) -> Result<Self, serde_yaml::Error> {
        let root = match value.get("architecture") {
            Some(inner) if inner.is_mapping() => {
                let source_only = inner.as_mapping().is_some_and(|map| {
                    map.len() == 1 && map.contains_key(serde_yaml::Value::from("source"))
                });
                if source_only { value } else { inner.clone() }
            }
            _ => value,
        };
        match root.get("layers") {
            Some(serde_yaml::Value::Mapping(map)) => {
                serde_yaml::from_value(definition_layers_to_kernel(map))
            }
            _ => serde_yaml::from_value(root),
        }
    }

    /// Find which layer a file belongs to by matching against layer path patterns.
    /// Patterns use simple prefix matching (e.g. "src/domain/" matches "src/domain/foo.ts").
    pub fn layer_for_file(&self, file_path: &str) -> Option<&LayerDef> {
        self.layers.iter().find(|layer| {
            layer
                .paths
                .iter()
                .any(|pat| matches_pattern(pat, file_path))
        })
    }

    /// Check whether a file in `from_layer` is allowed to import from `to_layer`.
    pub fn is_import_allowed(&self, from_layer: &str, to_layer: &str) -> bool {
        self.layers
            .iter()
            .find(|l| l.name == from_layer)
            .is_some_and(|l| l.allowed_imports.iter().any(|a| a == to_layer))
    }
}

fn definition_layers_to_kernel(map: &serde_yaml::Mapping) -> serde_yaml::Value {
    let mut entries: Vec<(&str, &serde_yaml::Value)> = map
        .iter()
        .filter_map(|(key, layer)| key.as_str().map(|name| (name, layer)))
        .collect();
    entries.sort_by(|left, right| left.0.cmp(right.0));
    let layers = entries
        .into_iter()
        .map(|(name, layer)| {
            let mut mapped = serde_yaml::Mapping::new();
            mapped.insert(
                serde_yaml::Value::from("name"),
                serde_yaml::Value::from(name),
            );
            let paths = match layer.get("patterns") {
                Some(serde_yaml::Value::Sequence(seq)) => seq.clone(),
                _ => Vec::new(),
            };
            mapped.insert(
                serde_yaml::Value::from("paths"),
                serde_yaml::Value::Sequence(paths),
            );
            let allowed = match layer.get("depends_on") {
                Some(serde_yaml::Value::Sequence(seq)) => seq.clone(),
                _ => Vec::new(),
            };
            mapped.insert(
                serde_yaml::Value::from("allowed_imports"),
                serde_yaml::Value::Sequence(allowed),
            );
            serde_yaml::Value::Mapping(mapped)
        })
        .collect();
    let mut root = serde_yaml::Mapping::new();
    root.insert(
        serde_yaml::Value::from("layers"),
        serde_yaml::Value::Sequence(layers),
    );
    serde_yaml::Value::Mapping(root)
}

/// Simple glob-like matching: trailing `*` acts as a prefix match,
/// otherwise exact match. Normalises path separators to `/` for
/// cross-platform compatibility.
fn matches_pattern(pattern: &str, path: &str) -> bool {
    let norm_path = path.replace('\\', "/");
    let norm_pattern = pattern.replace('\\', "/");
    if let Some(prefix) = norm_pattern.strip_suffix('*') {
        norm_path.starts_with(prefix)
    } else if norm_pattern.ends_with('/') {
        norm_path.starts_with(&norm_pattern)
    } else {
        norm_path == norm_pattern
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_YAML: &str = r#"
layers:
  - name: domain
    paths:
      - "src/domain/*"
    allowed_imports:
      - domain
  - name: infrastructure
    paths:
      - "src/infra/*"
    allowed_imports:
      - domain
      - infrastructure
  - name: presentation
    paths:
      - "src/ui/*"
    allowed_imports:
      - domain
      - infrastructure
      - presentation
"#;

    #[test]
    fn parse_valid_yaml_config() {
        let config = ArchitectureConfig::from_yaml(SAMPLE_YAML).unwrap();
        assert_eq!(config.layers.len(), 3);
        assert_eq!(config.layers[0].name, "domain");
        assert_eq!(config.layers[1].name, "infrastructure");
        assert_eq!(config.layers[2].name, "presentation");
    }

    #[test]
    fn layer_for_file_matches_patterns() {
        let config = ArchitectureConfig::from_yaml(SAMPLE_YAML).unwrap();

        let layer = config.layer_for_file("src/domain/user.ts").unwrap();
        assert_eq!(layer.name, "domain");

        let layer = config.layer_for_file("src/infra/db.ts").unwrap();
        assert_eq!(layer.name, "infrastructure");

        let layer = config.layer_for_file("src/ui/button.tsx").unwrap();
        assert_eq!(layer.name, "presentation");
    }

    #[test]
    fn layer_for_file_returns_none_for_unmatched() {
        let config = ArchitectureConfig::from_yaml(SAMPLE_YAML).unwrap();
        assert!(config.layer_for_file("test/helpers.ts").is_none());
    }

    #[test]
    fn allowed_imports_restricts_layers() {
        let config = ArchitectureConfig::from_yaml(SAMPLE_YAML).unwrap();

        // domain can only import from domain
        assert!(config.is_import_allowed("domain", "domain"));
        assert!(!config.is_import_allowed("domain", "infrastructure"));
        assert!(!config.is_import_allowed("domain", "presentation"));

        // infrastructure can import from domain and infrastructure
        assert!(config.is_import_allowed("infrastructure", "domain"));
        assert!(config.is_import_allowed("infrastructure", "infrastructure"));
        assert!(!config.is_import_allowed("infrastructure", "presentation"));

        // presentation can import from all
        assert!(config.is_import_allowed("presentation", "domain"));
        assert!(config.is_import_allowed("presentation", "infrastructure"));
        assert!(config.is_import_allowed("presentation", "presentation"));
    }

    #[test]
    fn matches_pattern_prefix_glob() {
        assert!(matches_pattern("src/domain/*", "src/domain/user.ts"));
        assert!(matches_pattern("src/domain/*", "src/domain/nested/deep.ts"));
        assert!(!matches_pattern("src/domain/*", "src/infra/db.ts"));
    }

    #[test]
    fn matches_pattern_windows_separators() {
        assert!(matches_pattern("src/domain/*", "src\\domain\\user.ts"));
        assert!(matches_pattern("src/domain/", "src\\domain\\user.ts"));
        assert!(!matches_pattern("src/domain/*", "src\\infra\\db.ts"));
    }

    #[test]
    fn matches_pattern_trailing_slash() {
        assert!(matches_pattern("src/domain/", "src/domain/user.ts"));
        assert!(!matches_pattern("src/domain/", "src/infra/db.ts"));
    }

    #[test]
    fn matches_pattern_exact() {
        assert!(matches_pattern("src/main.ts", "src/main.ts"));
        assert!(!matches_pattern("src/main.ts", "src/other.ts"));
    }

    #[test]
    fn missing_allowed_imports_defaults_to_empty() {
        let yaml = r#"
layers:
  - name: lib
    paths:
      - "src/lib/*"
"#;
        let config = ArchitectureConfig::from_yaml(yaml).unwrap();
        assert!(config.layers[0].allowed_imports.is_empty());
        assert!(!config.is_import_allowed("lib", "lib"));
    }

    const DEFINITION_YAML: &str = r#"
schema_version: "0.1.0"
layers:
  domain:
    patterns:
      - "src/domain/*"
    depends_on:
      - domain
  infrastructure:
    patterns:
      - "src/infra/*"
    depends_on:
      - domain
      - infrastructure
"#;

    #[test]
    fn from_yaml_any_maps_definition_schema() {
        let config = ArchitectureConfig::from_yaml_any(DEFINITION_YAML).unwrap();
        assert_eq!(config.layers.len(), 2);
        assert_eq!(config.layers[0].name, "domain");
        assert_eq!(config.layers[0].paths, vec!["src/domain/*"]);
        assert_eq!(config.layers[0].allowed_imports, vec!["domain"]);
        assert_eq!(config.layers[1].name, "infrastructure");
        assert!(config.is_import_allowed("infrastructure", "domain"));
        assert!(!config.is_import_allowed("domain", "infrastructure"));
        assert_eq!(
            config.layer_for_file("src/domain/user.ts").unwrap().name,
            "domain"
        );
    }

    #[test]
    fn from_yaml_any_extracts_inline_architecture_section() {
        let yaml = r#"
version: 1
architecture:
  schema_version: "0.1.0"
  layers:
    core:
      patterns: ["src/core/**"]
      depends_on: []
"#;
        let config = ArchitectureConfig::from_yaml_any(yaml).unwrap();
        assert_eq!(config.layers.len(), 1);
        assert_eq!(config.layers[0].name, "core");
        assert_eq!(config.layers[0].paths, vec!["src/core/**"]);
    }

    #[test]
    fn from_yaml_any_keeps_kernel_list_schema() {
        let config = ArchitectureConfig::from_yaml_any(SAMPLE_YAML).unwrap();
        assert_eq!(config.layers.len(), 3);
        assert_eq!(config.layers[0].name, "domain");
    }

    #[test]
    fn from_yaml_any_does_not_follow_source_only_section() {
        let yaml = "architecture:\n  source: \".anvil/architecture.yaml\"\n";
        let err = ArchitectureConfig::from_yaml_any(yaml).unwrap_err();
        assert!(
            err.to_string().contains("layers") || err.to_string().contains("missing"),
            "source-only section must not be treated as a definition: {err}"
        );
    }
}
