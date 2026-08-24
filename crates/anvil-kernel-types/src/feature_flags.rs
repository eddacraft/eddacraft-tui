use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagClass {
    Rollout,
    Entitlement,
    OpsKillSwitch,
}

impl FlagClass {
    pub fn fail_closed(self) -> bool {
        matches!(self, Self::OpsKillSwitch | Self::Entitlement)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagStatus {
    #[default]
    Draft,
    Active,
    Retiring,
    Retired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagValueType {
    Boolean,
    String,
    Number,
    Object,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FlagValue {
    Boolean(bool),
    Number(f64),
    String(String),
    Object(std::collections::BTreeMap<String, FlagValue>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlagVariant {
    pub key: String,
    pub value: FlagValue,
}

// -------------------------------------------------------------------------
// Environment targeting
// -------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentName {
    Local,
    Development,
    Preview,
    Demo,
    Production,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Development,
    Beta,
    Rc,
    Stable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentContext {
    pub environment: EnvironmentName,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<Channel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_ring: Option<String>,
}

// -------------------------------------------------------------------------
// Audience targeting
// -------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudienceContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub licence_plan: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organisation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cohort: Option<String>,
}

// -------------------------------------------------------------------------
// Evaluation context (OpenFeature-aligned)
// -------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationContext {
    pub targeting_key: String,
    pub environment: EnvironmentContext,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<AudienceContext>,
}

// -------------------------------------------------------------------------
// Targeting operators and rules
// -------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetingOperator {
    Equals,
    NotEquals,
    InSet,
    NotInSet,
    Percentage,
    Segment,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConditionValue {
    Single(String),
    Numeric(f64),
    Set(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetingCondition {
    pub attribute: String,
    pub operator: TargetingOperator,
    pub value: ConditionValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetingRule {
    pub conditions: Vec<TargetingCondition>,
    pub variant: String,
}

// -------------------------------------------------------------------------
// Feature flag definition
// -------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureFlagDefinition {
    pub key: String,
    pub owner: String,
    pub intent: String,
    pub class: FlagClass,
    pub value_type: FlagValueType,
    pub variants: Vec<FlagVariant>,
    pub default_variant: String,
    pub status: FlagStatus,
    pub created_for: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_or_review_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targeting: Option<Vec<TargetingRule>>,
    // FLAGCAT-004 / gating model (ADR-048): the feature group this flag belongs
    // to (matches a `groups.json` id) and an open-set tag list. Optional on the
    // Rust side to match the TS base schema and to tolerate manifests that
    // predate the gating-model fields — deserialising never drops them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// FLAGCAT-013: catalogue product-feature keys this operational flag controls.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub controls_product_features: Vec<String>,
}

// OPSUP-005: per-track feature-flag taxonomy (Track 3 governance surfaces and
// Track 4 semantic packs). Flags are hierarchical under a `track.surface.*` /
// `track.pack.*` namespace and resolve through the matching umbrella group so
// the flag count does not explode one-per-surface. See
// `docs/guides/feature-flag-governance.md` (Per-Track Flags).
pub const TRACK_SURFACE_FLAG: &str = "track.surface";
pub const TRACK_PACK_FLAG: &str = "track.pack";
pub const TRACK_SURFACE_GROUP: &str = "track-surface";
pub const TRACK_PACK_GROUP: &str = "track-pack";

impl FeatureFlagDefinition {
    pub fn default_variant_exists(&self) -> bool {
        self.variants.iter().any(|v| v.key == self.default_variant)
    }

    /// C-013: validate key format matches `^[a-z][a-z0-9]*([._-][a-z0-9]+)*$`
    pub fn has_valid_key(&self) -> bool {
        is_valid_flag_key(&self.key)
    }

    /// True when this flag belongs to the OPSUP-005 per-track taxonomy:
    /// the `track.surface`/`track.pack` umbrellas or a `track.surface.*` /
    /// `track.pack.*` per-leaf override.
    pub fn is_track_flag(&self) -> bool {
        self.expected_track_group().is_some()
    }

    /// The `groups.json` id a per-track flag must resolve to, derived from its
    /// key namespace. `None` when the flag is not a per-track flag.
    pub fn expected_track_group(&self) -> Option<&'static str> {
        if self.key == TRACK_PACK_FLAG || self.key.starts_with("track.pack.") {
            Some(TRACK_PACK_GROUP)
        } else if self.key == TRACK_SURFACE_FLAG || self.key.starts_with("track.surface.") {
            Some(TRACK_SURFACE_GROUP)
        } else {
            None
        }
    }

    /// OPSUP-005 *permanent* invariant for per-track flags: `rollout` class with
    /// a sunset/review date, `createdFor` provenance, and a `primaryGroup` that
    /// resolves through the hierarchical `track-surface` / `track-pack` umbrella.
    /// Returns the list of violations — empty means conformant. Non-track flags
    /// yield no violations (the invariant does not apply).
    ///
    /// Deliberately does NOT pin the default variant: per-track flags ship
    /// opt-in (`defaultVariant: disabled`) at first release, then the default
    /// flips to `enabled` after a clean release (see
    /// `docs/guides/feature-flag-governance.md`). The opt-in-at-first-ship
    /// expectation is a review-time check, not a manifest-wide invariant —
    /// enforcing it here would break CI the moment a surface legitimately
    /// graduates.
    pub fn track_flag_violations(&self) -> Vec<String> {
        let mut violations = Vec::new();
        let Some(expected_group) = self.expected_track_group() else {
            return violations;
        };
        if self.class != FlagClass::Rollout {
            violations.push(format!(
                "{}: per-track flags must be class rollout, got {:?}",
                self.key, self.class
            ));
        }
        if self.created_for.trim().is_empty() {
            violations.push(format!("{}: missing createdFor provenance", self.key));
        }
        if self.expiry_or_review_date.is_none() {
            violations.push(format!(
                "{}: missing expiryOrReviewDate (sunset/review date)",
                self.key
            ));
        }
        match self.primary_group.as_deref() {
            Some(g) if g == expected_group => {}
            Some(g) => violations.push(format!(
                "{}: primaryGroup `{g}` does not resolve to the `{expected_group}` umbrella",
                self.key
            )),
            None => violations.push(format!(
                "{}: missing primaryGroup (must be `{expected_group}`)",
                self.key
            )),
        }
        violations
    }
}

/// Validates a flag key matches the canonical pattern without regex dependency.
/// Pattern: starts with lowercase letter, followed by lowercase alphanumeric,
/// with segments separated by exactly one dot, hyphen, or underscore.
fn is_valid_flag_key(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }
    let mut chars = key.chars().peekable();
    // Must start with a lowercase letter
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    // Rest of first segment: lowercase alphanumeric
    while let Some(&c) = chars.peek() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            chars.next();
        } else if c == '.' || c == '_' || c == '-' {
            break;
        } else {
            return false;
        }
    }
    // Subsequent segments: separator + at least one lowercase alphanumeric
    while let Some(&sep) = chars.peek() {
        if sep != '.' && sep != '_' && sep != '-' {
            return false;
        }
        chars.next(); // consume separator
        // Must have at least one lowercase alphanumeric after separator
        match chars.next() {
            Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
            _ => return false,
        }
        // Rest of segment
        while let Some(&c) = chars.peek() {
            if c.is_ascii_lowercase() || c.is_ascii_digit() {
                chars.next();
            } else if c == '.' || c == '_' || c == '-' {
                break;
            } else {
                return false;
            }
        }
    }
    true
}

pub const FEATURE_FLAG_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureFlagManifest {
    pub schema_version: u32,
    pub flags: Vec<FeatureFlagDefinition>,
}

impl FeatureFlagManifest {
    /// C-007: validate `schema_version` matches the expected version
    pub fn has_valid_schema_version(&self) -> bool {
        self.schema_version == FEATURE_FLAG_SCHEMA_VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_flag() -> FeatureFlagDefinition {
        FeatureFlagDefinition {
            key: "cli.licence-gate".into(),
            owner: "BAUTH".into(),
            intent: "Gate CLI features behind licence validation".into(),
            class: FlagClass::Entitlement,
            value_type: FlagValueType::Boolean,
            variants: vec![
                FlagVariant {
                    key: "enabled".into(),
                    value: FlagValue::Boolean(true),
                },
                FlagVariant {
                    key: "disabled".into(),
                    value: FlagValue::Boolean(false),
                },
            ],
            default_variant: "disabled".into(),
            status: FlagStatus::Active,
            created_for: "FLAGS-008".into(),
            expiry_or_review_date: None,
            description: None,
            targeting: None,
            primary_group: None,
            tags: None,
            controls_product_features: Vec::new(),
        }
    }

    // OPSUP-005: a conformant per-track surface flag — opt-in (default)
    // disabled), sunset date set, resolves through the `track-surface` umbrella.
    fn valid_track_flag() -> FeatureFlagDefinition {
        FeatureFlagDefinition {
            key: "track.surface.sql".into(),
            owner: "SURFSQL".into(),
            intent: "Gate the SQL migrations governance surface".into(),
            class: FlagClass::Rollout,
            value_type: FlagValueType::Boolean,
            variants: vec![
                FlagVariant {
                    key: "enabled".into(),
                    value: FlagValue::Boolean(true),
                },
                FlagVariant {
                    key: "disabled".into(),
                    value: FlagValue::Boolean(false),
                },
            ],
            default_variant: "disabled".into(),
            status: FlagStatus::Active,
            created_for: "SURFSQL-006".into(),
            expiry_or_review_date: Some("2026-12-31T00:00:00Z".into()),
            description: None,
            targeting: None,
            primary_group: Some(TRACK_SURFACE_GROUP.into()),
            tags: None,
            controls_product_features: Vec::new(),
        }
    }

    #[test]
    fn track_flag_detection_by_namespace() {
        assert!(valid_track_flag().is_track_flag());
        let mut umbrella = valid_track_flag();
        umbrella.key = TRACK_PACK_FLAG.into();
        assert_eq!(umbrella.expected_track_group(), Some(TRACK_PACK_GROUP));
        // A non-track flag is exempt from the invariant.
        assert!(!valid_flag().is_track_flag());
        assert!(valid_flag().track_flag_violations().is_empty());
    }

    #[test]
    fn conformant_track_flag_has_no_violations() {
        assert!(valid_track_flag().track_flag_violations().is_empty());
    }

    #[test]
    fn track_flag_missing_sunset_date_is_flagged() {
        let mut flag = valid_track_flag();
        flag.expiry_or_review_date = None;
        assert!(
            flag.track_flag_violations()
                .iter()
                .any(|v| v.contains("expiryOrReviewDate"))
        );
    }

    #[test]
    fn track_flag_wrong_umbrella_group_is_flagged() {
        // Both directions: surface key + pack group, and pack key + surface group.
        let mut surface = valid_track_flag();
        surface.primary_group = Some(TRACK_PACK_GROUP.into());
        assert!(
            surface
                .track_flag_violations()
                .iter()
                .any(|v| v.contains("umbrella"))
        );
        let mut pack = valid_track_flag();
        pack.key = TRACK_PACK_FLAG.into(); // pack key, but surface group from builder
        assert!(
            pack.track_flag_violations()
                .iter()
                .any(|v| v.contains("umbrella"))
        );
    }

    #[test]
    fn track_flag_non_rollout_class_is_flagged() {
        // A per-track flag must be rollout class (the doc's "every per-track
        // flag is rollout class" contract). The default state is NOT pinned —
        // the opt-in default may flip to enabled after one release.
        let mut flag = valid_track_flag();
        flag.class = FlagClass::Entitlement;
        assert!(
            flag.track_flag_violations()
                .iter()
                .any(|v| v.contains("rollout"))
        );
        // The flip path: a rollout track flag with default enabled is conformant.
        let mut flipped = valid_track_flag();
        flipped.default_variant = "enabled".into();
        assert!(flipped.track_flag_violations().is_empty());
    }

    #[test]
    fn track_flag_missing_created_for_is_flagged() {
        let mut flag = valid_track_flag();
        flag.created_for = "  ".into();
        assert!(
            flag.track_flag_violations()
                .iter()
                .any(|v| v.contains("createdFor"))
        );
    }

    #[test]
    fn flag_class_variants_distinct() {
        let variants = [
            FlagClass::Rollout,
            FlagClass::Entitlement,
            FlagClass::OpsKillSwitch,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn flag_class_serde_round_trip() {
        for class in [
            FlagClass::Rollout,
            FlagClass::Entitlement,
            FlagClass::OpsKillSwitch,
        ] {
            let json = serde_json::to_string(&class).unwrap();
            let back: FlagClass = serde_json::from_str(&json).unwrap();
            assert_eq!(class, back);
        }
    }

    #[test]
    fn flag_class_serialises_snake_case() {
        assert_eq!(
            serde_json::to_string(&FlagClass::OpsKillSwitch).unwrap(),
            "\"ops_kill_switch\""
        );
        assert_eq!(
            serde_json::to_string(&FlagClass::Entitlement).unwrap(),
            "\"entitlement\""
        );
    }

    #[test]
    fn fail_closed_classes() {
        assert!(FlagClass::OpsKillSwitch.fail_closed());
        assert!(FlagClass::Entitlement.fail_closed());
        assert!(!FlagClass::Rollout.fail_closed());
    }

    #[test]
    fn flag_status_default() {
        assert_eq!(FlagStatus::default(), FlagStatus::Draft);
    }

    #[test]
    fn flag_status_serde_round_trip() {
        for status in [
            FlagStatus::Draft,
            FlagStatus::Active,
            FlagStatus::Retiring,
            FlagStatus::Retired,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let back: FlagStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, back);
        }
    }

    #[test]
    fn flag_value_type_serde_round_trip() {
        for vt in [
            FlagValueType::Boolean,
            FlagValueType::String,
            FlagValueType::Number,
            FlagValueType::Object,
        ] {
            let json = serde_json::to_string(&vt).unwrap();
            let back: FlagValueType = serde_json::from_str(&json).unwrap();
            assert_eq!(vt, back);
        }
    }

    #[test]
    fn definition_serde_round_trip() {
        let flag = valid_flag();
        let json = serde_json::to_string(&flag).unwrap();
        let back: FeatureFlagDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(flag, back);
    }

    #[test]
    fn definition_with_optional_fields() {
        let flag = FeatureFlagDefinition {
            expiry_or_review_date: Some("2026-07-01T00:00:00Z".into()),
            description: Some("Controls CLI licence gating".into()),
            ..valid_flag()
        };
        let json = serde_json::to_string(&flag).unwrap();
        let back: FeatureFlagDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(flag, back);
    }

    #[test]
    fn default_variant_exists_true() {
        assert!(valid_flag().default_variant_exists());
    }

    #[test]
    fn default_variant_exists_false() {
        let flag = FeatureFlagDefinition {
            default_variant: "nonexistent".into(),
            ..valid_flag()
        };
        assert!(!flag.default_variant_exists());
    }

    #[test]
    fn manifest_serde_round_trip() {
        let manifest = FeatureFlagManifest {
            schema_version: 1,
            flags: vec![valid_flag()],
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let back: FeatureFlagManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(manifest, back);
    }

    #[test]
    fn manifest_empty_flags() {
        let manifest = FeatureFlagManifest {
            schema_version: 1,
            flags: vec![],
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let back: FeatureFlagManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(manifest, back);
    }

    #[test]
    fn deserialise_invalid_class_fails() {
        let result = serde_json::from_str::<FlagClass>("\"experiment\"");
        assert!(result.is_err());
    }

    #[test]
    fn deserialise_invalid_status_fails() {
        let result = serde_json::from_str::<FlagStatus>("\"archived\"");
        assert!(result.is_err());
    }

    // --- C-013: key format validation ---

    #[test]
    fn valid_flag_keys() {
        for key in [
            "cli.licence-gate",
            "docs_access",
            "opa-rollout",
            "simple",
            "a1.b2",
        ] {
            let flag = FeatureFlagDefinition {
                key: key.into(),
                ..valid_flag()
            };
            assert!(flag.has_valid_key(), "expected valid: {key}");
        }
    }

    #[test]
    fn invalid_flag_keys() {
        for key in [
            "",
            "CLI.gate",
            "123-start",
            "has spaces",
            "UPPER",
            "a..b",
            "a-",
            "-a",
        ] {
            let flag = FeatureFlagDefinition {
                key: key.into(),
                ..valid_flag()
            };
            assert!(!flag.has_valid_key(), "expected invalid: {key}");
        }
    }

    // --- C-007: schema version validation ---

    #[test]
    fn valid_schema_version() {
        let manifest = FeatureFlagManifest {
            schema_version: FEATURE_FLAG_SCHEMA_VERSION,
            flags: vec![],
        };
        assert!(manifest.has_valid_schema_version());
    }

    #[test]
    fn invalid_schema_version() {
        let manifest = FeatureFlagManifest {
            schema_version: 99,
            flags: vec![],
        };
        assert!(!manifest.has_valid_schema_version());
    }

    // --- Targeting tests (FLAGS-002) ---

    #[test]
    fn environment_name_serde_round_trip() {
        for env in [
            EnvironmentName::Local,
            EnvironmentName::Development,
            EnvironmentName::Preview,
            EnvironmentName::Demo,
            EnvironmentName::Production,
        ] {
            let json = serde_json::to_string(&env).unwrap();
            let back: EnvironmentName = serde_json::from_str(&json).unwrap();
            assert_eq!(env, back);
        }
    }

    #[test]
    fn environment_name_serializes_to_renamed_values() {
        // FLAGCAT-002: prod->production, dev->development, +demo, -staging.
        // The wire values match the catalogue manifest / NODE_ENV native names.
        assert_eq!(
            serde_json::to_string(&EnvironmentName::Production).unwrap(),
            "\"production\""
        );
        assert_eq!(
            serde_json::to_string(&EnvironmentName::Development).unwrap(),
            "\"development\""
        );
        assert_eq!(
            serde_json::to_string(&EnvironmentName::Demo).unwrap(),
            "\"demo\""
        );
        // Unrenamed variants keep their wire values.
        assert_eq!(
            serde_json::to_string(&EnvironmentName::Local).unwrap(),
            "\"local\""
        );
        assert_eq!(
            serde_json::to_string(&EnvironmentName::Preview).unwrap(),
            "\"preview\""
        );
        // The pre-rename wire values no longer deserialize.
        assert!(serde_json::from_str::<EnvironmentName>("\"prod\"").is_err());
        assert!(serde_json::from_str::<EnvironmentName>("\"dev\"").is_err());
        assert!(serde_json::from_str::<EnvironmentName>("\"staging\"").is_err());
    }

    #[test]
    fn channel_serde_round_trip() {
        for ch in [
            Channel::Development,
            Channel::Beta,
            Channel::Rc,
            Channel::Stable,
        ] {
            let json = serde_json::to_string(&ch).unwrap();
            let back: Channel = serde_json::from_str(&json).unwrap();
            assert_eq!(ch, back);
        }
    }

    #[test]
    fn environment_context_minimal() {
        let ctx = EnvironmentContext {
            environment: EnvironmentName::Production,
            channel: None,
            deployment_ring: None,
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let back: EnvironmentContext = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx, back);
        assert!(!json.contains("channel"));
    }

    #[test]
    fn environment_context_full() {
        let ctx = EnvironmentContext {
            environment: EnvironmentName::Demo,
            channel: Some(Channel::Beta),
            deployment_ring: Some("canary".into()),
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let back: EnvironmentContext = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx, back);
    }

    #[test]
    fn audience_context_default_is_empty() {
        let ctx = AudienceContext::default();
        assert_eq!(ctx.account_tier, None);
        assert_eq!(ctx.licence_plan, None);
        assert_eq!(ctx.organisation_id, None);
        assert_eq!(ctx.user_role, None);
        assert_eq!(ctx.cohort, None);
    }

    #[test]
    fn audience_context_serde_round_trip() {
        let ctx = AudienceContext {
            account_tier: Some("pro".into()),
            licence_plan: Some("team".into()),
            organisation_id: Some("org-123".into()),
            user_role: Some("admin".into()),
            cohort: Some("early-adopter".into()),
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let back: AudienceContext = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx, back);
    }

    #[test]
    fn evaluation_context_minimal() {
        let ctx = EvaluationContext {
            targeting_key: "session-abc".into(),
            environment: EnvironmentContext {
                environment: EnvironmentName::Development,
                channel: None,
                deployment_ring: None,
            },
            audience: None,
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let back: EvaluationContext = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx, back);
    }

    #[test]
    fn evaluation_context_full() {
        let ctx = EvaluationContext {
            targeting_key: "session-xyz".into(),
            environment: EnvironmentContext {
                environment: EnvironmentName::Production,
                channel: Some(Channel::Stable),
                deployment_ring: None,
            },
            audience: Some(AudienceContext {
                account_tier: Some("enterprise".into()),
                ..AudienceContext::default()
            }),
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let back: EvaluationContext = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx, back);
    }

    #[test]
    fn targeting_operator_serde_round_trip() {
        for op in [
            TargetingOperator::Equals,
            TargetingOperator::NotEquals,
            TargetingOperator::InSet,
            TargetingOperator::NotInSet,
            TargetingOperator::Percentage,
            TargetingOperator::Segment,
        ] {
            let json = serde_json::to_string(&op).unwrap();
            let back: TargetingOperator = serde_json::from_str(&json).unwrap();
            assert_eq!(op, back);
        }
    }

    #[test]
    fn targeting_operator_serialises_snake_case() {
        assert_eq!(
            serde_json::to_string(&TargetingOperator::NotEquals).unwrap(),
            "\"not_equals\""
        );
        assert_eq!(
            serde_json::to_string(&TargetingOperator::InSet).unwrap(),
            "\"in_set\""
        );
    }

    #[test]
    fn condition_value_single_string() {
        let v = ConditionValue::Single("prod".into());
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"prod\"");
        let back: ConditionValue = serde_json::from_str(&json).unwrap();
        assert_eq!(v, back);
    }

    #[test]
    fn condition_value_numeric() {
        let v = ConditionValue::Numeric(25.0);
        let json = serde_json::to_string(&v).unwrap();
        let back: ConditionValue = serde_json::from_str(&json).unwrap();
        assert_eq!(v, back);
    }

    #[test]
    fn condition_value_set() {
        let v = ConditionValue::Set(vec!["beta".into(), "rc".into()]);
        let json = serde_json::to_string(&v).unwrap();
        let back: ConditionValue = serde_json::from_str(&json).unwrap();
        assert_eq!(v, back);
    }

    #[test]
    fn targeting_rule_serde_round_trip() {
        let rule = TargetingRule {
            conditions: vec![
                TargetingCondition {
                    attribute: "environment".into(),
                    operator: TargetingOperator::Equals,
                    value: ConditionValue::Single("production".into()),
                },
                TargetingCondition {
                    attribute: "accountTier".into(),
                    operator: TargetingOperator::InSet,
                    value: ConditionValue::Set(vec!["pro".into(), "enterprise".into()]),
                },
            ],
            variant: "enabled".into(),
        };
        let json = serde_json::to_string(&rule).unwrap();
        let back: TargetingRule = serde_json::from_str(&json).unwrap();
        assert_eq!(rule, back);
    }

    #[test]
    fn definition_with_targeting_rules() {
        let flag = FeatureFlagDefinition {
            targeting: Some(vec![TargetingRule {
                conditions: vec![TargetingCondition {
                    attribute: "environment".into(),
                    operator: TargetingOperator::Equals,
                    value: ConditionValue::Single("production".into()),
                }],
                variant: "enabled".into(),
            }]),
            ..valid_flag()
        };
        let json = serde_json::to_string(&flag).unwrap();
        let back: FeatureFlagDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(flag, back);
    }
}
