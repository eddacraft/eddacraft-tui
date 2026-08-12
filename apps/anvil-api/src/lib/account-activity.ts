/**
 * BACT allowlisted account feature keys (OQ1, 2026-08-12).
 *
 * Closed enum only — free-form command strings, argv, paths, and install ids
 * must never enter the identity-bound activity pipe. Login itself is tracked
 * via beta_users login stamps (BACT-002); `auth` records the first/subsequent
 * use of the authenticated product surface after session mint when the CLI
 * emits a feature-touch for the auth command path.
 */
export const ACCOUNT_FEATURE_KEYS = ['watch', 'start', 'check', 'auth'] as const;

export type AccountFeatureKey = (typeof ACCOUNT_FEATURE_KEYS)[number];

export const ACCOUNT_FEATURE_KEY_SET: ReadonlySet<string> = new Set(ACCOUNT_FEATURE_KEYS);

export function isAccountFeatureKey(value: string): value is AccountFeatureKey {
  return ACCOUNT_FEATURE_KEY_SET.has(value);
}

/** Default idle window for CS engagement filters (BACT-006 / OQ3). */
export const DEFAULT_IDLE_DAYS = 30;

/**
 * Closed set of account `plan` values (ADR-121 / BACT-008 `beta_users.plan`
 * CHECK constraint). Widen alongside the CHECK and the catalogue plan-axis
 * audiences (`flags/audiences.json`) when new plans ship — never invent a
 * parallel plan list (ADR-121 decision 3).
 */
export const ACCOUNT_PLANS = ['beta'] as const;

export type AccountPlan = (typeof ACCOUNT_PLANS)[number];
