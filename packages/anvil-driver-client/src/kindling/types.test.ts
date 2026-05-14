import { describe, expect, it } from 'vitest';

import type { Diagnostic } from '../diagnostics/index.js';
import {
  fromMidEditResponse,
  type GateEvaluatedObservation,
  KIND_GATE_EVALUATED,
  MIDEDIT_GATE_ID,
  type MidEditResponseLike,
  type ObservationContext,
} from './types.js';

/**
 * MLP2-030: cross-language parity tests for the Kindling
 * `gate_evaluated` observation builder.
 *
 * The reference JSON below is the byte-exact output of the Rust
 * `serde_json::to_string(&from_midedit_response(&ctx, &response))`
 * captured by a one-shot test in
 * `crates/anvil-intercept/src/kindling_observation.rs`. Pinning
 * the fixture string verbatim means any drift between the Rust
 * `from_midedit_response` and the TS `fromMidEditResponse` —
 * field rename, optional-skip policy change, severity-mapping
 * tweak — fails this test loudly.
 */
const RUST_EMITTED_GATE_EVALUATED_JSON =
  '{"kind":"gate_evaluated","session_id":"11111111-1111-4111-8111-111111111111","timestamp":"2026-05-14T12:00:00Z","gate_eval_id":"gate-eval-abc","gate_id":"midEdit","inputs":{"file_count":1,"changed_files":["src/a.ts"]},"outcome":"fail","rules_evaluated":["anvil.secret.aws","anvil.lint.foo"],"rules_violated":["anvil.secret.aws","anvil.lint.foo"],"enforcement":"blocking","duration_ms":42,"violation_count":1,"warning_count":1}';

const FIXTURE_CTX: ObservationContext = {
  session_id: '11111111-1111-4111-8111-111111111111',
  timestamp: '2026-05-14T12:00:00Z',
  gate_eval_id: 'gate-eval-abc',
  file_path: 'src/a.ts',
  duration_ms: 42,
};

function makeDiag(ruleId: string, severity: Diagnostic['severity']): Diagnostic {
  return {
    schema: 'anvil.diagnostic.v1',
    id: `diag-${ruleId}`,
    severity,
    message: 'test diagnostic',
    location: { file: 'src/a.ts' },
    category: 'other',
    source: { rule_id: ruleId, source_module: 'x' },
    mode: 'mid-edit',
  };
}

describe('Kindling observation constants', () => {
  it('mirrors the Rust KIND_GATE_EVALUATED constant', () => {
    expect(KIND_GATE_EVALUATED).toBe('gate_evaluated');
  });

  it('mirrors the Rust MIDEDIT_GATE_ID constant', () => {
    expect(MIDEDIT_GATE_ID).toBe('midEdit');
  });
});

describe('fromMidEditResponse — Rust parity', () => {
  it('emits the byte-exact Rust-serde-emitted JSON for a mixed-severity batch', () => {
    const response: MidEditResponseLike = {
      diagnostics: [makeDiag('anvil.secret.aws', 'error'), makeDiag('anvil.lint.foo', 'warning')],
    };
    const obs = fromMidEditResponse(FIXTURE_CTX, response);
    expect(obs).not.toBeNull();
    expect(JSON.stringify(obs)).toBe(RUST_EMITTED_GATE_EVALUATED_JSON);
  });

  it('round-trips through JSON.parse with field equality', () => {
    const response: MidEditResponseLike = {
      diagnostics: [makeDiag('anvil.secret.aws', 'error'), makeDiag('anvil.lint.foo', 'warning')],
    };
    const obs = fromMidEditResponse(FIXTURE_CTX, response);
    expect(obs).not.toBeNull();
    const wire = JSON.stringify(obs);
    const decoded = JSON.parse(wire) as GateEvaluatedObservation;
    expect(decoded).toEqual(obs);
  });
});

describe('fromMidEditResponse — volume-control contract', () => {
  it('returns null when diagnostics are empty (pass-no-finding silence)', () => {
    // Mirrors Rust `from_midedit_response` returning `None` for an
    // empty diagnostics vec. The caller is free to invoke this on
    // every scan; the helper filters out the no-finding case so the
    // call site does not have to track that policy independently.
    const obs = fromMidEditResponse(FIXTURE_CTX, { diagnostics: [] });
    expect(obs).toBeNull();
  });
});

describe('fromMidEditResponse — severity → enforcement mapping', () => {
  it('maps error-only batch to enforcement: blocking', () => {
    const obs = fromMidEditResponse(FIXTURE_CTX, {
      diagnostics: [makeDiag('r1', 'error')],
    });
    expect(obs?.enforcement).toBe('blocking');
    expect(obs?.violation_count).toBe(1);
    expect(obs?.warning_count).toBe(0);
  });

  it('maps warning-only batch to enforcement: warning', () => {
    const obs = fromMidEditResponse(FIXTURE_CTX, {
      diagnostics: [makeDiag('r1', 'warning')],
    });
    expect(obs?.enforcement).toBe('warning');
    expect(obs?.violation_count).toBe(0);
    expect(obs?.warning_count).toBe(1);
  });

  it('maps info-only batch to enforcement: informational', () => {
    const obs = fromMidEditResponse(FIXTURE_CTX, {
      diagnostics: [makeDiag('r1', 'info')],
    });
    expect(obs?.enforcement).toBe('informational');
    expect(obs?.violation_count).toBe(0);
    expect(obs?.warning_count).toBe(0);
    // Info-only batches still produce an observation (diagnostics
    // is non-empty, so the volume filter does NOT short-circuit).
    // `rules_violated` is omitted because info doesn't qualify.
    expect(obs?.rules_violated).toBeUndefined();
  });

  it('picks the worst severity in a mixed batch', () => {
    // info + warning + error → blocking (error wins).
    const obs = fromMidEditResponse(FIXTURE_CTX, {
      diagnostics: [makeDiag('r1', 'info'), makeDiag('r2', 'warning'), makeDiag('r3', 'error')],
    });
    expect(obs?.enforcement).toBe('blocking');
    expect(obs?.violation_count).toBe(1);
    expect(obs?.warning_count).toBe(1);
  });
});

describe('fromMidEditResponse — rules_violated optionality', () => {
  it('omits rules_violated key entirely when no error or warning diagnostics exist', () => {
    // Info-only batch produces a row, but rules_violated is absent
    // on the wire (matches Rust `skip_serializing_if =
    // "Option::is_none"`).
    const obs = fromMidEditResponse(FIXTURE_CTX, {
      diagnostics: [makeDiag('r1', 'info'), makeDiag('r2', 'info')],
    });
    expect(obs).not.toBeNull();
    const wire = JSON.parse(JSON.stringify(obs)) as Record<string, unknown>;
    expect('rules_violated' in wire).toBe(false);
    // rules_evaluated still carries both rules.
    expect(obs?.rules_evaluated).toEqual(['r1', 'r2']);
  });

  it('includes rules_violated when at least one diagnostic qualifies', () => {
    const obs = fromMidEditResponse(FIXTURE_CTX, {
      diagnostics: [makeDiag('r1', 'info'), makeDiag('r2', 'warning')],
    });
    expect(obs?.rules_violated).toEqual(['r2']);
    // Wire form has the key.
    const wire = JSON.parse(JSON.stringify(obs)) as Record<string, unknown>;
    expect('rules_violated' in wire).toBe(true);
  });
});

describe('fromMidEditResponse — observation context plumbing', () => {
  it('echoes the caller-supplied context into the row', () => {
    const ctx: ObservationContext = {
      session_id: 'sess-x',
      timestamp: '2026-01-01T00:00:00Z',
      gate_eval_id: 'gate-x',
      file_path: 'src/path/with/dirs/file.rs',
      duration_ms: 999,
    };
    const obs = fromMidEditResponse(ctx, {
      diagnostics: [makeDiag('r1', 'error')],
    });
    expect(obs?.session_id).toBe('sess-x');
    expect(obs?.timestamp).toBe('2026-01-01T00:00:00Z');
    expect(obs?.gate_eval_id).toBe('gate-x');
    expect(obs?.duration_ms).toBe(999);
    expect(obs?.inputs.changed_files).toEqual(['src/path/with/dirs/file.rs']);
    expect(obs?.inputs.file_count).toBe(1);
    // Mid-edit rows never carry a baseline_hash.
    expect(obs?.inputs.baseline_hash).toBeUndefined();
  });

  it('pins kind + gate_id even when the caller would have set them differently', () => {
    // The builder MUST emit the canonical constants regardless of
    // anything the caller could mistakenly try to override.
    const obs = fromMidEditResponse(FIXTURE_CTX, {
      diagnostics: [makeDiag('r1', 'error')],
    });
    expect(obs?.kind).toBe('gate_evaluated');
    expect(obs?.gate_id).toBe('midEdit');
  });
});
