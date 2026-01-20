/**
 * Confidence Scale Definitions (STACK-003)
 *
 * Defines confidence levels used across Ember and Edda layers.
 *
 * Key distinction:
 * - Ember: Numeric confidence (0.0-1.0) — heuristic, computed, probabilistic
 * - Edda: Categorical confidence (low/medium/high) — human-asserted, judgemental
 *
 * This reflects the fundamental difference:
 * - Ember proposes (algorithmic evaluation)
 * - Edda asserts (human judgement)
 *
 * @module @eddacraft/anvil-edda-stack/contracts/confidence
 */

import { z } from 'zod';

// =============================================================================
// Ember Confidence (Heuristic, Numeric)
// =============================================================================

/**
 * Ember confidence score: 0.0 to 1.0
 *
 * This is a heuristic score computed by evaluation rules.
 * It represents "likelihood this is worth remembering" not "likelihood this is true".
 *
 * Interpretation:
 * - 0.0-0.3: Low signal, likely noise
 * - 0.3-0.6: Moderate signal, worth reviewing
 * - 0.6-0.8: Strong signal, likely candidate for promotion
 * - 0.8-1.0: Very strong signal, high-priority candidate
 */
export const EmberConfidenceSchema = z
  .number()
  .min(0)
  .max(1)
  .describe('Heuristic confidence score (0.0-1.0)');

export type EmberConfidence = z.infer<typeof EmberConfidenceSchema>;

/**
 * Confidence thresholds for Ember evaluation
 */
export const EmberConfidenceThresholdsSchema = z.object({
  /** Minimum confidence to create a proposal (filter noise) */
  min_to_propose: EmberConfidenceSchema.default(0.3),
  /** Minimum confidence to suggest for promotion */
  min_to_suggest: EmberConfidenceSchema.default(0.6),
  /** Confidence level for high-priority flagging */
  high_priority: EmberConfidenceSchema.default(0.8),
});

export type EmberConfidenceThresholds = z.infer<typeof EmberConfidenceThresholdsSchema>;

// =============================================================================
// Edda Confidence (Human-Asserted, Categorical)
// =============================================================================

/**
 * Edda confidence level: human-asserted judgement
 *
 * This is NOT computed — it's a human decision about how confident
 * we are that this memory is accurate and applicable.
 *
 * - low: "We think this might be true, but aren't certain"
 * - medium: "We're reasonably confident this is accurate"
 * - high: "We're very confident this is accurate and stable"
 */
export const EddaConfidenceLevelSchema = z.enum(['low', 'medium', 'high']);

export type EddaConfidenceLevel = z.infer<typeof EddaConfidenceLevelSchema>;

/**
 * Edda confidence with optional rationale
 */
export const EddaConfidenceSchema = z.object({
  level: EddaConfidenceLevelSchema,
  rationale: z.string().optional().describe('Why this confidence level was chosen'),
});

export type EddaConfidence = z.infer<typeof EddaConfidenceSchema>;

// =============================================================================
// Confidence Mapping (Ember → Edda)
// =============================================================================

/**
 * Suggested mapping from Ember numeric confidence to Edda categorical
 *
 * This is advisory only — humans make the final decision.
 * The mapping provides a starting suggestion during promotion.
 */
export const confidenceMappingDefaults = {
  low: { min: 0.0, max: 0.5 },
  medium: { min: 0.5, max: 0.75 },
  high: { min: 0.75, max: 1.0 },
} as const;

/**
 * Suggest an Edda confidence level from an Ember score
 * This is a suggestion, not a determination
 */
export function suggestEddaConfidence(emberScore: EmberConfidence): EddaConfidenceLevel {
  if (emberScore >= confidenceMappingDefaults.high.min) {
    return 'high';
  }
  if (emberScore >= confidenceMappingDefaults.medium.min) {
    return 'medium';
  }
  return 'low';
}

// =============================================================================
// Confidence Utilities
// =============================================================================

/**
 * Check if an Ember confidence meets a threshold
 */
export function meetsThreshold(score: EmberConfidence, threshold: EmberConfidence): boolean {
  return score >= threshold;
}

/**
 * Clamp a value to valid Ember confidence range
 */
export function clampConfidence(value: number): EmberConfidence {
  return Math.max(0, Math.min(1, value)) as EmberConfidence;
}

/**
 * Combine multiple confidence scores (average)
 */
export function averageConfidence(scores: EmberConfidence[]): EmberConfidence {
  if (scores.length === 0) return 0 as EmberConfidence;
  const sum = scores.reduce((acc, score) => acc + score, 0);
  return (sum / scores.length) as EmberConfidence;
}

/**
 * Combine multiple confidence scores (max)
 */
export function maxConfidence(scores: EmberConfidence[]): EmberConfidence {
  if (scores.length === 0) return 0 as EmberConfidence;
  return Math.max(...scores) as EmberConfidence;
}

/**
 * Combine multiple confidence scores (weighted average)
 */
export function weightedConfidence(
  scores: Array<{ score: EmberConfidence; weight: number }>
): EmberConfidence {
  if (scores.length === 0) return 0 as EmberConfidence;
  const totalWeight = scores.reduce((acc, { weight }) => acc + weight, 0);
  if (totalWeight === 0) return 0 as EmberConfidence;
  const weightedSum = scores.reduce((acc, { score, weight }) => acc + score * weight, 0);
  return (weightedSum / totalWeight) as EmberConfidence;
}

/**
 * Format confidence for display
 */
export function formatEmberConfidence(score: EmberConfidence): string {
  return `${(score * 100).toFixed(0)}%`;
}

/**
 * Format Edda confidence for display
 */
export function formatEddaConfidence(level: EddaConfidenceLevel): string {
  const labels: Record<EddaConfidenceLevel, string> = {
    low: 'Low confidence',
    medium: 'Medium confidence',
    high: 'High confidence',
  };
  return labels[level];
}
