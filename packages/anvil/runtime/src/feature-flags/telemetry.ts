import type { ResolutionReason } from './resolver.js';

// =============================================================================
// Session Telemetry (emitted once at session start)
// =============================================================================

export interface FlagSessionTelemetry {
  snapshotVersion: number;
  environment: string;
  runtime: string;
  timestamp: string;
}

export interface SessionTelemetryInput {
  snapshotVersion: number;
  environment: string;
  runtime: string;
}

export function createSessionTelemetry(input: SessionTelemetryInput): FlagSessionTelemetry {
  return {
    snapshotVersion: input.snapshotVersion,
    environment: input.environment,
    runtime: input.runtime,
    timestamp: new Date().toISOString().replace(/\.\d{3}Z$/, 'Z'),
  };
}

// =============================================================================
// Evaluation Event (emitted on first use per flag per session)
// =============================================================================

export interface FlagEvaluationEvent {
  flagKey: string;
  variant: string;
  reason: ResolutionReason;
  timestamp: string;
}

export interface EvaluationEventInput {
  flagKey: string;
  variant: string;
  reason: ResolutionReason;
}

export function createEvaluationEvent(input: EvaluationEventInput): FlagEvaluationEvent {
  return {
    flagKey: input.flagKey,
    variant: input.variant,
    reason: input.reason,
    timestamp: new Date().toISOString().replace(/\.\d{3}Z$/, 'Z'),
  };
}

// =============================================================================
// Override Event (emitted when an override is applied)
// =============================================================================

export type OverrideSource = 'emergency' | 'local';

export interface FlagOverrideEvent {
  flagKey: string;
  variant: string;
  source: OverrideSource;
  timestamp: string;
}

export interface OverrideEventInput {
  flagKey: string;
  variant: string;
  source: OverrideSource;
}

export function createOverrideEvent(input: OverrideEventInput): FlagOverrideEvent {
  return {
    flagKey: input.flagKey,
    variant: input.variant,
    source: input.source,
    timestamp: new Date().toISOString().replace(/\.\d{3}Z$/, 'Z'),
  };
}
