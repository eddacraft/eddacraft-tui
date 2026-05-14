use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleMode {
    Off,
    Warn,
    Enforce,
}

impl RuleMode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Warn => "warn",
            Self::Enforce => "enforce",
        }
    }

    fn parse(raw: &str, path: &str) -> Result<Self, RuleModeError> {
        match raw.to_ascii_lowercase().as_str() {
            "off" | "disabled" | "none" => Ok(Self::Off),
            "warn" | "warning" => Ok(Self::Warn),
            "enforce" | "block" | "error" => Ok(Self::Enforce),
            _ => Err(RuleModeError::UnknownMode {
                path: path.to_string(),
                mode: raw.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuleModeError {
    #[error("unknown rule mode `{mode}` at `{path}`; expected off, warn, or enforce")]
    UnknownMode { path: String, mode: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleModes {
    pub public_api_expansion: RuleMode,
    pub new_dependency_introduction: RuleMode,
    pub cross_layer_violation: RuleMode,
    pub privilege_expansion: RuleMode,
}

impl Default for RuleModes {
    fn default() -> Self {
        Self {
            public_api_expansion: RuleMode::Warn,
            new_dependency_introduction: RuleMode::Warn,
            cross_layer_violation: RuleMode::Warn,
            privilege_expansion: RuleMode::Warn,
        }
    }
}

impl RuleModes {
    pub fn from_value(value: &Value) -> Result<Self, RuleModeError> {
        let mut modes = Self::default();
        let Some(rules) = rules_object(value) else {
            return Ok(modes);
        };

        apply_rule(
            rules,
            "public-api-expansion",
            "enforcement.rules.public-api-expansion.mode",
            &mut modes.public_api_expansion,
        )?;
        apply_rule(
            rules,
            "new-dependency-introduction",
            "enforcement.rules.new-dependency-introduction.mode",
            &mut modes.new_dependency_introduction,
        )?;
        apply_rule(
            rules,
            "cross-layer-violation",
            "enforcement.rules.cross-layer-violation.mode",
            &mut modes.cross_layer_violation,
        )?;
        apply_rule(
            rules,
            "privilege-expansion",
            "enforcement.rules.privilege-expansion.mode",
            &mut modes.privilege_expansion,
        )?;

        Ok(modes)
    }

    #[must_use]
    pub fn summary(self) -> String {
        format!(
            "public-api-expansion={}, new-dependency-introduction={}, cross-layer-violation={}, privilege-expansion={}",
            self.public_api_expansion.as_str(),
            self.new_dependency_introduction.as_str(),
            self.cross_layer_violation.as_str(),
            self.privilege_expansion.as_str()
        )
    }
}

fn rules_object(value: &Value) -> Option<&serde_json::Map<String, Value>> {
    value
        .get("enforcement")
        .and_then(Value::as_object)
        .and_then(|obj| obj.get("rules"))
        .and_then(Value::as_object)
        .or_else(|| value.get("rules").and_then(Value::as_object))
}

fn apply_rule(
    rules: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
    target: &mut RuleMode,
) -> Result<(), RuleModeError> {
    let Some(entry) = rules.get(key) else {
        return Ok(());
    };
    if let Some(raw) = entry.as_str() {
        *target = RuleMode::parse(raw, path)?;
    } else if let Some(raw) = entry
        .as_object()
        .and_then(|obj| obj.get("mode"))
        .and_then(Value::as_str)
    {
        *target = RuleMode::parse(raw, path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{RuleMode, RuleModes};

    #[test]
    fn defaults_all_watchux_rules_to_warn() {
        let modes = RuleModes::from_value(&json!({})).unwrap();

        assert_eq!(modes.public_api_expansion, RuleMode::Warn);
        assert_eq!(modes.new_dependency_introduction, RuleMode::Warn);
        assert_eq!(modes.cross_layer_violation, RuleMode::Warn);
        assert_eq!(modes.privilege_expansion, RuleMode::Warn);
    }

    #[test]
    fn parses_explicit_rule_modes() {
        let modes = RuleModes::from_value(&json!({
            "enforcement": {
                "rules": {
                    "public-api-expansion": { "mode": "off" },
                    "new-dependency-introduction": { "mode": "warn" },
                    "cross-layer-violation": { "mode": "enforce" },
                    "privilege-expansion": { "mode": "block" }
                }
            }
        }))
        .unwrap();

        assert_eq!(modes.public_api_expansion, RuleMode::Off);
        assert_eq!(modes.new_dependency_introduction, RuleMode::Warn);
        assert_eq!(modes.cross_layer_violation, RuleMode::Enforce);
        assert_eq!(modes.privilege_expansion, RuleMode::Enforce);
    }
}
