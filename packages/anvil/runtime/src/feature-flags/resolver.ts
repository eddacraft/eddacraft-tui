import type {
  EvaluationContext,
  FeatureFlagDefinition,
  FlagClass,
  TargetingCondition,
  TargetingRule,
} from '@eddacraft/anvil-contracts';

import { failClosedClasses } from '@eddacraft/anvil-contracts';

// =============================================================================
// Resolution Details (OpenFeature-aligned)
// =============================================================================

export type ResolutionReason =
  | 'emergency_override'
  | 'local_override'
  | 'targeting_match'
  | 'default'
  | 'error'
  | 'disabled'
  | 'invalid_override_fallthrough';

export interface ResolutionDetails<T = unknown> {
  value: T;
  variant: string;
  reason: ResolutionReason;
  flagKey: string;
  errorCode?: string;
  errorMessage?: string;
}

// =============================================================================
// Override Sources
// =============================================================================

export interface FlagOverrides {
  emergency?: Record<string, string>;
  local?: Record<string, string>;
}

// =============================================================================
// Resolver
// =============================================================================

export function resolveFlag(
  flag: FeatureFlagDefinition,
  context: EvaluationContext,
  overrides?: FlagOverrides
): ResolutionDetails {
  // Retired flags always resolve to default
  if (flag.status === 'retired' || flag.status === 'draft') {
    return resolveDefault(flag, 'disabled');
  }

  // 1. Emergency override / kill switch
  const emergencyVariant = overrides?.emergency?.[flag.key];
  if (emergencyVariant !== undefined) {
    const variant = flag.variants.find((v) => v.key === emergencyVariant);
    if (variant) {
      return {
        value: variant.value,
        variant: variant.key,
        reason: 'emergency_override',
        flagKey: flag.key,
      };
    }
    // C-006: invalid override on fail-closed class is an error
    if (isFailClosed(flag.class)) {
      return {
        value: failClosedValue(flag.class),
        variant: '__fail_closed',
        reason: 'error',
        flagKey: flag.key,
        errorCode: 'INVALID_OVERRIDE_VARIANT',
        errorMessage: `Emergency override variant "${emergencyVariant}" not found in flag "${flag.key}"`,
      };
    }
    // C-019: non-fail-closed class — fall through but with distinguishable reason
    return resolveDefault(flag, 'invalid_override_fallthrough');
  }

  // 2. Local operator override
  const localVariant = overrides?.local?.[flag.key];
  if (localVariant !== undefined) {
    const variant = flag.variants.find((v) => v.key === localVariant);
    if (variant) {
      return {
        value: variant.value,
        variant: variant.key,
        reason: 'local_override',
        flagKey: flag.key,
      };
    }
    // C-006: invalid override on fail-closed class is an error
    if (isFailClosed(flag.class)) {
      return {
        value: failClosedValue(flag.class),
        variant: '__fail_closed',
        reason: 'error',
        flagKey: flag.key,
        errorCode: 'INVALID_OVERRIDE_VARIANT',
        errorMessage: `Local override variant "${localVariant}" not found in flag "${flag.key}"`,
      };
    }
    // C-019: non-fail-closed class — fall through but with distinguishable reason
    return resolveDefault(flag, 'invalid_override_fallthrough');
  }

  // 3. Targeting rules
  if (flag.targeting) {
    for (const rule of flag.targeting) {
      if (evaluateRule(rule, context)) {
        const variant = flag.variants.find((v) => v.key === rule.variant);
        if (variant) {
          return {
            value: variant.value,
            variant: variant.key,
            reason: 'targeting_match',
            flagKey: flag.key,
          };
        }
      }
    }
  }

  // 4. Manifest default
  return resolveDefault(flag, 'default');
}

function resolveDefault(flag: FeatureFlagDefinition, reason: ResolutionReason): ResolutionDetails {
  const variant = flag.variants.find((v) => v.key === flag.defaultVariant);
  if (variant) {
    return {
      value: variant.value,
      variant: variant.key,
      reason,
      flagKey: flag.key,
    };
  }

  // Fallback: class-based failure policy
  return {
    value: failClosedValue(flag.class),
    variant: '__fail_closed',
    reason: 'error',
    flagKey: flag.key,
    errorCode: 'MISSING_DEFAULT_VARIANT',
    errorMessage: `Default variant "${flag.defaultVariant}" not found in flag "${flag.key}"`,
  };
}

// C-002: class-based failure policy
function isFailClosed(flagClass: FlagClass): boolean {
  return failClosedClasses().includes(flagClass);
}

function failClosedValue(_flagClass: FlagClass): boolean {
  // All classes currently resolve to false on failure:
  // entitlement/kill-switch: deny access; rollout: feature disabled
  return false;
}

// =============================================================================
// Rule Evaluation
// =============================================================================

function evaluateRule(rule: TargetingRule, context: EvaluationContext): boolean {
  return rule.conditions.every((c) => evaluateCondition(c, context));
}

function evaluateCondition(condition: TargetingCondition, context: EvaluationContext): boolean {
  const actual = resolveAttribute(condition.attribute, context);

  switch (condition.operator) {
    case 'equals':
      return actual === String(condition.value);
    case 'not_equals':
      // C-004: missing attribute should not match not_equals
      if (actual === undefined) return false;
      return actual !== String(condition.value);
    case 'in_set':
      // C-005: missing attribute should not match in_set
      if (actual === undefined) return false;
      return Array.isArray(condition.value) && condition.value.includes(actual);
    case 'not_in_set':
      // C-004: missing attribute should not match not_in_set
      if (actual === undefined) return false;
      return Array.isArray(condition.value) && !condition.value.includes(actual);
    case 'percentage':
      return evaluatePercentage(context.targetingKey, Number(condition.value));
    case 'segment':
      // CIB-117: segment acts as string equality, reconciled with the Rust
      // kernel resolver (C-011) and docs/guides/feature-flag-reference.md;
      // reserved for a future segment lookup. Only single string values
      // match — no coercion — mirroring the Rust ConditionValue::Single arm.
      if (actual === undefined) return false;
      return typeof condition.value === 'string' && actual === condition.value;
    default:
      return false;
  }
}

function resolveAttribute(attribute: string, context: EvaluationContext): string | undefined {
  // Environment dimensions
  if (attribute === 'environment') return context.environment.environment;
  if (attribute === 'channel') return context.environment.channel ?? undefined;
  if (attribute === 'deploymentRing') return context.environment.deploymentRing ?? undefined;

  // Audience dimensions
  const audience = context.audience;
  if (!audience) return undefined;
  if (attribute === 'accountTier') return audience.accountTier ?? undefined;
  if (attribute === 'licencePlan') return audience.licencePlan ?? undefined;
  if (attribute === 'organisationId') return audience.organisationId ?? undefined;
  if (attribute === 'userRole') return audience.userRole ?? undefined;
  if (attribute === 'cohort') return audience.cohort ?? undefined;

  return undefined;
}

// =============================================================================
// Percentage Rollout
// =============================================================================

/**
 * Deterministic percentage evaluation using a simple hash of the targeting key.
 * Returns true if the hash falls within the given percentage (0–100).
 */
export function evaluatePercentage(targetingKey: string, percentage: number): boolean {
  if (percentage <= 0) return false;
  if (percentage >= 100) return true;
  const hash = simpleHash(targetingKey);
  return hash % 100 < percentage;
}

// C-001: use TextEncoder for consistent UTF-8 byte handling
function simpleHash(input: string): number {
  const bytes = new TextEncoder().encode(input);
  let hash = 0;
  for (const byte of bytes) {
    hash = ((hash << 5) - hash + byte) | 0;
  }
  return Math.abs(hash);
}
