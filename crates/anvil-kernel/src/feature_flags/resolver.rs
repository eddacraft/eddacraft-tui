use std::cell::RefCell;
use std::collections::HashMap;

use anvil_kernel_types::{
    AudienceContext, ConditionValue, EvaluationContext, FeatureFlagDefinition, FlagStatus,
    TargetingCondition, TargetingOperator, TargetingRule,
};

// -------------------------------------------------------------------------
// Resolution details
// -------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionReason {
    EmergencyOverride,
    LocalOverride,
    TargetingMatch,
    Default,
    Error,
    Disabled,
}

#[derive(Debug, Clone)]
pub struct ResolutionDetails {
    pub value: serde_json::Value,
    pub variant: String,
    pub reason: ResolutionReason,
    pub flag_key: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

// -------------------------------------------------------------------------
// Override sources
// -------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct FlagOverrides {
    pub emergency: HashMap<String, String>,
    pub local: HashMap<String, String>,
}

// -------------------------------------------------------------------------
// Flag capture sink (USAGE-002)
// -------------------------------------------------------------------------

/// A single flag resolution captured during an opt-in capture window.
///
/// Recorded by [`resolve_flag`] only while [`begin_flag_capture`] is
/// active on the current thread. The CLI installs the sink for the
/// auth/routing phase so usage rows can carry the flags resolved while
/// authorising the command (USAGE-002); the daemon never installs it, so
/// off that path capture is a no-op — a single thread-local check with no
/// allocation and no accumulation. Carries the canonical flag `key`, the
/// resolved `variant`, the resolution `reason`, and whether the flag is
/// gate-affecting (its class fails closed — entitlement / ops-kill-switch
/// — per ADR-019).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedResolution {
    pub key: String,
    pub variant: String,
    pub reason: ResolutionReason,
    pub gate_affecting: bool,
}

thread_local! {
    /// Opt-in per-thread capture sink. `None` = not capturing (default,
    /// e.g. the daemon); `Some` = recording resolutions for the current
    /// invocation.
    static FLAG_CAPTURE: RefCell<Option<Vec<CapturedResolution>>> =
        const { RefCell::new(None) };
}

/// Defensive cap so a pathological resolve loop can't grow the sink
/// without bound (real invocations resolve a handful of flags).
const FLAG_CAPTURE_CAP: usize = 256;

/// Begin capturing flag resolutions on the current thread, discarding any
/// prior window. The CLI calls this before the auth/routing phase;
/// [`take_captured_flags`] drains it afterwards.
pub fn begin_flag_capture() {
    FLAG_CAPTURE.with(|sink| *sink.borrow_mut() = Some(Vec::new()));
}

/// Drain and return the flags captured since [`begin_flag_capture`],
/// ending the capture window. Returns empty when no window is active.
#[must_use]
pub fn take_captured_flags() -> Vec<CapturedResolution> {
    FLAG_CAPTURE.with(|sink| sink.borrow_mut().take().unwrap_or_default())
}

fn capture_resolution(flag: &FeatureFlagDefinition, details: &ResolutionDetails) {
    // Invariant: this closure must not transitively call `resolve_flag`
    // while the `borrow_mut` is held — doing so would re-enter the
    // `RefCell` and panic. It only clones owned data and pushes, so the
    // non-re-entrancy guarantee holds.
    FLAG_CAPTURE.with(|sink| {
        if let Some(captured) = sink.borrow_mut().as_mut()
            && captured.len() < FLAG_CAPTURE_CAP
        {
            // Past the cap, resolutions are silently dropped. The cap
            // (256) is far above any real invocation's flag count, so a
            // hit only happens under a pathological resolve loop — bound
            // the sink rather than grow it without limit. `anvil-kernel`
            // has no logging dependency, so the drop is intentionally
            // quiet.
            captured.push(CapturedResolution {
                key: details.flag_key.clone(),
                variant: details.variant.clone(),
                reason: details.reason.clone(),
                gate_affecting: flag.class.fail_closed(),
            });
        }
    });
}

// -------------------------------------------------------------------------
// Resolver
// -------------------------------------------------------------------------

/// Resolve a feature flag. When a capture window is active on the current
/// thread (see [`begin_flag_capture`]), the resolution is also recorded
/// into the thread-local sink for USAGE-002.
pub fn resolve_flag(
    flag: &FeatureFlagDefinition,
    context: &EvaluationContext,
    overrides: Option<&FlagOverrides>,
) -> ResolutionDetails {
    let details = resolve_flag_inner(flag, context, overrides);
    capture_resolution(flag, &details);
    details
}

fn resolve_flag_inner(
    flag: &FeatureFlagDefinition,
    context: &EvaluationContext,
    overrides: Option<&FlagOverrides>,
) -> ResolutionDetails {
    // Retired/draft flags always resolve to default
    if flag.status == FlagStatus::Retired || flag.status == FlagStatus::Draft {
        return resolve_default(flag, ResolutionReason::Disabled);
    }

    // 1. Emergency override
    if let Some(ovr) = overrides {
        if let Some(variant_key) = ovr.emergency.get(&flag.key) {
            if let Some(variant) = flag.variants.iter().find(|v| v.key == *variant_key) {
                return ResolutionDetails {
                    value: flag_value_to_json(&variant.value),
                    variant: variant.key.clone(),
                    reason: ResolutionReason::EmergencyOverride,
                    flag_key: flag.key.clone(),
                    error_code: None,
                    error_message: None,
                };
            }
            // C-006: invalid override on fail-closed class is an error
            if flag.class.fail_closed() {
                return ResolutionDetails {
                    value: serde_json::Value::Bool(false),
                    variant: "__fail_closed".into(),
                    reason: ResolutionReason::Error,
                    flag_key: flag.key.clone(),
                    error_code: Some("INVALID_OVERRIDE_VARIANT".into()),
                    error_message: Some(format!(
                        "Emergency override variant \"{variant_key}\" not found in flag \"{}\"",
                        flag.key
                    )),
                };
            }
        }

        // 2. Local override
        if let Some(variant_key) = ovr.local.get(&flag.key) {
            if let Some(variant) = flag.variants.iter().find(|v| v.key == *variant_key) {
                return ResolutionDetails {
                    value: flag_value_to_json(&variant.value),
                    variant: variant.key.clone(),
                    reason: ResolutionReason::LocalOverride,
                    flag_key: flag.key.clone(),
                    error_code: None,
                    error_message: None,
                };
            }
            // C-006: invalid override on fail-closed class is an error
            if flag.class.fail_closed() {
                return ResolutionDetails {
                    value: serde_json::Value::Bool(false),
                    variant: "__fail_closed".into(),
                    reason: ResolutionReason::Error,
                    flag_key: flag.key.clone(),
                    error_code: Some("INVALID_OVERRIDE_VARIANT".into()),
                    error_message: Some(format!(
                        "Local override variant \"{variant_key}\" not found in flag \"{}\"",
                        flag.key
                    )),
                };
            }
        }
    }

    // 3. Targeting rules
    if let Some(ref targeting) = flag.targeting {
        for rule in targeting {
            if evaluate_rule(rule, context)
                && let Some(variant) = flag.variants.iter().find(|v| v.key == rule.variant)
            {
                return ResolutionDetails {
                    value: flag_value_to_json(&variant.value),
                    variant: variant.key.clone(),
                    reason: ResolutionReason::TargetingMatch,
                    flag_key: flag.key.clone(),
                    error_code: None,
                    error_message: None,
                };
            }
        }
    }

    // 4. Manifest default
    resolve_default(flag, ResolutionReason::Default)
}

fn resolve_default(flag: &FeatureFlagDefinition, reason: ResolutionReason) -> ResolutionDetails {
    if let Some(variant) = flag.variants.iter().find(|v| v.key == flag.default_variant) {
        return ResolutionDetails {
            value: flag_value_to_json(&variant.value),
            variant: variant.key.clone(),
            reason,
            flag_key: flag.key.clone(),
            error_code: None,
            error_message: None,
        };
    }

    // C-002: class-based failure policy
    ResolutionDetails {
        value: serde_json::Value::Bool(false),
        variant: "__fail_closed".into(),
        reason: ResolutionReason::Error,
        flag_key: flag.key.clone(),
        error_code: Some("MISSING_DEFAULT_VARIANT".into()),
        error_message: Some(format!(
            "Default variant \"{}\" not found in flag \"{}\"",
            flag.default_variant, flag.key
        )),
    }
}

// C-014: explicit serialisation instead of unwrap_or_default
fn flag_value_to_json(value: &anvil_kernel_types::FlagValue) -> serde_json::Value {
    serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
}

// -------------------------------------------------------------------------
// Rule evaluation
// -------------------------------------------------------------------------

fn evaluate_rule(rule: &TargetingRule, context: &EvaluationContext) -> bool {
    rule.conditions
        .iter()
        .all(|c| evaluate_condition(c, context))
}

fn evaluate_condition(condition: &TargetingCondition, context: &EvaluationContext) -> bool {
    let actual = resolve_attribute(&condition.attribute, context);

    match condition.operator {
        TargetingOperator::Equals => {
            if let ConditionValue::Single(ref expected) = condition.value {
                actual.as_deref() == Some(expected.as_str())
            } else {
                false
            }
        }
        TargetingOperator::NotEquals => {
            // C-004: missing attribute must not match not_equals
            if let ConditionValue::Single(ref expected) = condition.value {
                match actual.as_deref() {
                    None => false,
                    Some(a) => a != expected.as_str(),
                }
            } else {
                false
            }
        }
        TargetingOperator::InSet => {
            if let ConditionValue::Set(ref set) = condition.value {
                actual.as_ref().is_some_and(|a| set.contains(a))
            } else {
                false
            }
        }
        TargetingOperator::NotInSet => {
            // C-004: missing attribute must not match not_in_set
            if let ConditionValue::Set(ref set) = condition.value {
                match actual.as_ref() {
                    None => false,
                    Some(a) => !set.contains(a),
                }
            } else {
                false
            }
        }
        TargetingOperator::Percentage => {
            if let ConditionValue::Numeric(pct) = condition.value {
                evaluate_percentage(&context.targeting_key, pct)
            } else {
                false
            }
        }
        TargetingOperator::Segment => {
            // C-011: segment acts as string equality; reserved for future segment lookup
            if let ConditionValue::Single(ref expected) = condition.value {
                actual.as_deref() == Some(expected.as_str())
            } else {
                false
            }
        }
    }
}

fn resolve_attribute(attribute: &str, context: &EvaluationContext) -> Option<String> {
    match attribute {
        "environment" => Some(
            serde_json::to_value(context.environment.environment)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
        ),
        "channel" => context.environment.channel.as_ref().map(|c| {
            serde_json::to_value(c)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default()
        }),
        "deploymentRing" => context.environment.deployment_ring.clone(),
        _ => resolve_audience_attribute(attribute, context.audience.as_ref()),
    }
}

fn resolve_audience_attribute(
    attribute: &str,
    audience: Option<&AudienceContext>,
) -> Option<String> {
    let aud = audience?;
    match attribute {
        "accountTier" => aud.account_tier.clone(),
        "licencePlan" => aud.licence_plan.clone(),
        "organisationId" => aud.organisation_id.clone(),
        "userRole" => aud.user_role.clone(),
        "cohort" => aud.cohort.clone(),
        _ => None,
    }
}

// -------------------------------------------------------------------------
// Percentage rollout
// -------------------------------------------------------------------------

// C-009: use f64 comparison instead of truncating cast
pub fn evaluate_percentage(targeting_key: &str, percentage: f64) -> bool {
    if percentage <= 0.0 {
        return false;
    }
    if percentage >= 100.0 {
        return true;
    }
    let hash = simple_hash(targeting_key);
    // C-003: use strict less-than to match TS parity
    f64::from(hash % 100) < percentage
}

fn simple_hash(input: &str) -> u32 {
    let mut hash: i32 = 0;
    for byte in input.bytes() {
        hash = hash
            .wrapping_shl(5)
            .wrapping_sub(hash)
            .wrapping_add(i32::from(byte));
    }
    hash.unsigned_abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::*;

    fn boolean_flag() -> FeatureFlagDefinition {
        FeatureFlagDefinition {
            key: "test.flag".into(),
            owner: "TEST".into(),
            intent: "Test flag".into(),
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
            created_for: "FLAGS-003".into(),
            expiry_or_review_date: None,
            description: None,
            targeting: None,
            primary_group: None,
            tags: None,
        }
    }

    fn rollout_flag() -> FeatureFlagDefinition {
        FeatureFlagDefinition {
            class: FlagClass::Rollout,
            ..boolean_flag()
        }
    }

    // --- USAGE-002 flag capture sink ---

    #[test]
    fn capture_off_by_default_records_nothing() {
        // No begin_flag_capture: resolution must not be recorded.
        let _ = resolve_flag(&boolean_flag(), &dev_context(), None);
        assert!(take_captured_flags().is_empty());
    }

    #[test]
    fn capture_records_key_variant_reason_and_gate_affecting() {
        begin_flag_capture();
        let _ = resolve_flag(&boolean_flag(), &dev_context(), None);
        let captured = take_captured_flags();
        assert_eq!(captured.len(), 1);
        let entry = &captured[0];
        assert_eq!(entry.key, "test.flag");
        assert_eq!(entry.variant, "disabled"); // default_variant
        assert_eq!(entry.reason, ResolutionReason::Default);
        // Entitlement class fails closed → gate-affecting.
        assert!(entry.gate_affecting);
    }

    #[test]
    fn capture_marks_rollout_class_not_gate_affecting() {
        begin_flag_capture();
        let _ = resolve_flag(&rollout_flag(), &dev_context(), None);
        let captured = take_captured_flags();
        assert_eq!(captured.len(), 1);
        assert!(
            !captured[0].gate_affecting,
            "rollout flags are not gate-affecting"
        );
    }

    #[test]
    fn capture_records_local_override_reason() {
        let mut overrides = FlagOverrides::default();
        overrides.local.insert("test.flag".into(), "enabled".into());
        begin_flag_capture();
        let _ = resolve_flag(&boolean_flag(), &dev_context(), Some(&overrides));
        let captured = take_captured_flags();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].variant, "enabled");
        assert_eq!(captured[0].reason, ResolutionReason::LocalOverride);
    }

    #[test]
    fn capture_is_bounded_by_the_cap() {
        begin_flag_capture();
        for _ in 0..(FLAG_CAPTURE_CAP + 10) {
            let _ = resolve_flag(&boolean_flag(), &dev_context(), None);
        }
        assert_eq!(take_captured_flags().len(), FLAG_CAPTURE_CAP);
    }

    #[test]
    fn take_ends_the_capture_window() {
        begin_flag_capture();
        let _ = resolve_flag(&boolean_flag(), &dev_context(), None);
        assert_eq!(
            take_captured_flags().len(),
            1,
            "first drain returns the row"
        );
        // Window ended: a later resolution is not captured until a new begin.
        let _ = resolve_flag(&boolean_flag(), &dev_context(), None);
        assert!(
            take_captured_flags().is_empty(),
            "no window active after drain"
        );
    }

    fn dev_context() -> EvaluationContext {
        EvaluationContext {
            targeting_key: "session-abc".into(),
            environment: EnvironmentContext {
                environment: EnvironmentName::Development,
                channel: None,
                deployment_ring: None,
            },
            audience: None,
        }
    }

    fn prod_context() -> EvaluationContext {
        EvaluationContext {
            targeting_key: "session-abc".into(),
            environment: EnvironmentContext {
                environment: EnvironmentName::Production,
                channel: Some(Channel::Stable),
                deployment_ring: None,
            },
            audience: Some(AudienceContext {
                account_tier: Some("pro".into()),
                ..AudienceContext::default()
            }),
        }
    }

    fn flag_with_prod_targeting() -> FeatureFlagDefinition {
        FeatureFlagDefinition {
            targeting: Some(vec![TargetingRule {
                conditions: vec![TargetingCondition {
                    attribute: "environment".into(),
                    operator: TargetingOperator::Equals,
                    value: ConditionValue::Single("production".into()),
                }],
                variant: "enabled".into(),
            }]),
            ..boolean_flag()
        }
    }

    // --- Precedence ---

    #[test]
    fn default_when_no_overrides_or_targeting() {
        let result = resolve_flag(&boolean_flag(), &dev_context(), None);
        assert_eq!(result.variant, "disabled");
        assert_eq!(result.reason, ResolutionReason::Default);
    }

    #[test]
    fn targeting_overrides_default() {
        let result = resolve_flag(&flag_with_prod_targeting(), &prod_context(), None);
        assert_eq!(result.variant, "enabled");
        assert_eq!(result.reason, ResolutionReason::TargetingMatch);
    }

    #[test]
    fn local_override_overrides_targeting() {
        let overrides = FlagOverrides {
            local: HashMap::from([("test.flag".into(), "disabled".into())]),
            ..Default::default()
        };
        let result = resolve_flag(
            &flag_with_prod_targeting(),
            &prod_context(),
            Some(&overrides),
        );
        assert_eq!(result.variant, "disabled");
        assert_eq!(result.reason, ResolutionReason::LocalOverride);
    }

    #[test]
    fn emergency_override_overrides_all() {
        let overrides = FlagOverrides {
            emergency: HashMap::from([("test.flag".into(), "disabled".into())]),
            local: HashMap::from([("test.flag".into(), "enabled".into())]),
        };
        let result = resolve_flag(
            &flag_with_prod_targeting(),
            &prod_context(),
            Some(&overrides),
        );
        assert_eq!(result.variant, "disabled");
        assert_eq!(result.reason, ResolutionReason::EmergencyOverride);
    }

    // --- Status ---

    #[test]
    fn retired_flag_resolves_disabled() {
        let flag = FeatureFlagDefinition {
            status: FlagStatus::Retired,
            ..flag_with_prod_targeting()
        };
        let result = resolve_flag(&flag, &prod_context(), None);
        assert_eq!(result.variant, "disabled");
        assert_eq!(result.reason, ResolutionReason::Disabled);
    }

    #[test]
    fn draft_flag_resolves_disabled() {
        let flag = FeatureFlagDefinition {
            status: FlagStatus::Draft,
            ..boolean_flag()
        };
        let result = resolve_flag(&flag, &dev_context(), None);
        assert_eq!(result.reason, ResolutionReason::Disabled);
    }

    #[test]
    fn retiring_flag_still_evaluates() {
        let flag = FeatureFlagDefinition {
            status: FlagStatus::Retiring,
            ..flag_with_prod_targeting()
        };
        let result = resolve_flag(&flag, &prod_context(), None);
        assert_eq!(result.reason, ResolutionReason::TargetingMatch);
    }

    // --- Missing default variant ---

    #[test]
    fn missing_default_variant_returns_error() {
        let flag = FeatureFlagDefinition {
            default_variant: "nonexistent".into(),
            ..boolean_flag()
        };
        let result = resolve_flag(&flag, &dev_context(), None);
        assert_eq!(result.reason, ResolutionReason::Error);
        assert_eq!(
            result.error_code.as_deref(),
            Some("MISSING_DEFAULT_VARIANT")
        );
    }

    // --- Targeting operators ---

    #[test]
    fn not_equals_operator() {
        let flag = FeatureFlagDefinition {
            targeting: Some(vec![TargetingRule {
                conditions: vec![TargetingCondition {
                    attribute: "environment".into(),
                    operator: TargetingOperator::NotEquals,
                    value: ConditionValue::Single("production".into()),
                }],
                variant: "enabled".into(),
            }]),
            ..boolean_flag()
        };
        assert_eq!(resolve_flag(&flag, &dev_context(), None).variant, "enabled");
    }

    #[test]
    fn in_set_operator() {
        let flag = FeatureFlagDefinition {
            targeting: Some(vec![TargetingRule {
                conditions: vec![TargetingCondition {
                    attribute: "accountTier".into(),
                    operator: TargetingOperator::InSet,
                    value: ConditionValue::Set(vec!["pro".into(), "enterprise".into()]),
                }],
                variant: "enabled".into(),
            }]),
            ..boolean_flag()
        };
        assert_eq!(
            resolve_flag(&flag, &prod_context(), None).variant,
            "enabled"
        );
    }

    #[test]
    fn not_in_set_operator() {
        let flag = FeatureFlagDefinition {
            targeting: Some(vec![TargetingRule {
                conditions: vec![TargetingCondition {
                    attribute: "accountTier".into(),
                    operator: TargetingOperator::NotInSet,
                    value: ConditionValue::Set(vec!["free".into()]),
                }],
                variant: "enabled".into(),
            }]),
            ..boolean_flag()
        };
        assert_eq!(
            resolve_flag(&flag, &prod_context(), None).variant,
            "enabled"
        );
    }

    #[test]
    fn and_semantics_on_multiple_conditions() {
        let flag = FeatureFlagDefinition {
            targeting: Some(vec![TargetingRule {
                conditions: vec![
                    TargetingCondition {
                        attribute: "environment".into(),
                        operator: TargetingOperator::Equals,
                        value: ConditionValue::Single("production".into()),
                    },
                    TargetingCondition {
                        attribute: "accountTier".into(),
                        operator: TargetingOperator::Equals,
                        value: ConditionValue::Single("pro".into()),
                    },
                ],
                variant: "enabled".into(),
            }]),
            ..boolean_flag()
        };
        assert_eq!(
            resolve_flag(&flag, &prod_context(), None).variant,
            "enabled"
        );
        assert_eq!(
            resolve_flag(&flag, &dev_context(), None).variant,
            "disabled"
        );
    }

    #[test]
    fn missing_audience_attribute_no_match() {
        let flag = FeatureFlagDefinition {
            targeting: Some(vec![TargetingRule {
                conditions: vec![TargetingCondition {
                    attribute: "cohort".into(),
                    operator: TargetingOperator::Equals,
                    value: ConditionValue::Single("beta".into()),
                }],
                variant: "enabled".into(),
            }]),
            ..boolean_flag()
        };
        assert_eq!(
            resolve_flag(&flag, &dev_context(), None).variant,
            "disabled"
        );
    }

    // --- C-004: not_equals/not_in_set with missing attribute ---

    #[test]
    fn not_equals_missing_attribute_returns_false() {
        let flag = FeatureFlagDefinition {
            targeting: Some(vec![TargetingRule {
                conditions: vec![TargetingCondition {
                    attribute: "cohort".into(),
                    operator: TargetingOperator::NotEquals,
                    value: ConditionValue::Single("beta".into()),
                }],
                variant: "enabled".into(),
            }]),
            ..boolean_flag()
        };
        // No audience → cohort is None → should NOT match not_equals
        assert_eq!(
            resolve_flag(&flag, &dev_context(), None).variant,
            "disabled"
        );
    }

    #[test]
    fn not_in_set_missing_attribute_returns_false() {
        let flag = FeatureFlagDefinition {
            targeting: Some(vec![TargetingRule {
                conditions: vec![TargetingCondition {
                    attribute: "cohort".into(),
                    operator: TargetingOperator::NotInSet,
                    value: ConditionValue::Set(vec!["beta".into()]),
                }],
                variant: "enabled".into(),
            }]),
            ..boolean_flag()
        };
        // No audience → cohort is None → should NOT match not_in_set
        assert_eq!(
            resolve_flag(&flag, &dev_context(), None).variant,
            "disabled"
        );
    }

    // --- C-006: invalid override variant on fail-closed class ---

    #[test]
    fn invalid_override_fail_closed_class_returns_error() {
        let overrides = FlagOverrides {
            local: HashMap::from([("test.flag".into(), "nonexistent".into())]),
            ..Default::default()
        };
        // Entitlement is fail-closed
        let result = resolve_flag(&boolean_flag(), &dev_context(), Some(&overrides));
        assert_eq!(result.reason, ResolutionReason::Error);
        assert_eq!(
            result.error_code.as_deref(),
            Some("INVALID_OVERRIDE_VARIANT")
        );
    }

    #[test]
    fn invalid_override_rollout_class_falls_through() {
        let overrides = FlagOverrides {
            local: HashMap::from([("test.flag".into(), "nonexistent".into())]),
            ..Default::default()
        };
        // Rollout is NOT fail-closed, so invalid override falls through to default
        let result = resolve_flag(&rollout_flag(), &dev_context(), Some(&overrides));
        assert_eq!(result.reason, ResolutionReason::Default);
    }

    // --- Percentage rollout ---

    #[test]
    fn percentage_zero_always_false() {
        assert!(!evaluate_percentage("any", 0.0));
    }

    #[test]
    fn percentage_hundred_always_true() {
        assert!(evaluate_percentage("any", 100.0));
    }

    #[test]
    fn percentage_is_deterministic() {
        let a = evaluate_percentage("stable-key", 50.0);
        let b = evaluate_percentage("stable-key", 50.0);
        assert_eq!(a, b);
    }

    #[test]
    fn percentage_produces_distribution() {
        let mut trues = 0;
        let mut falses = 0;
        for i in 0..100 {
            if evaluate_percentage(&format!("key-{i}"), 50.0) {
                trues += 1;
            } else {
                falses += 1;
            }
        }
        assert!(trues > 0 && falses > 0);
    }

    // --- Override edge cases ---

    #[test]
    fn unknown_override_variant_ignored_for_rollout() {
        let overrides = FlagOverrides {
            local: HashMap::from([("test.flag".into(), "nonexistent".into())]),
            ..Default::default()
        };
        let result = resolve_flag(&rollout_flag(), &dev_context(), Some(&overrides));
        assert_eq!(result.reason, ResolutionReason::Default);
    }

    #[test]
    fn override_for_different_flag_ignored() {
        let overrides = FlagOverrides {
            emergency: HashMap::from([("other.flag".into(), "enabled".into())]),
            ..Default::default()
        };
        let result = resolve_flag(&boolean_flag(), &dev_context(), Some(&overrides));
        assert_eq!(result.reason, ResolutionReason::Default);
    }
}
