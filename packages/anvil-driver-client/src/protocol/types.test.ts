import { describe, expect, it } from 'vitest';

import type { Diagnostic } from '../diagnostics/types.js';
import {
  ALL_ANVIL_METHODS,
  ALL_CAPABILITIES,
  ANVIL_ENFORCEMENT_ACK,
  ANVIL_GATE_REQUEST,
  ANVIL_PUBLISH_DIAGNOSTICS,
  ANVIL_SCAN_BUFFER,
  ANVIL_STATUS_QUERY,
  ANVIL_SUPPRESSION_APPLY,
  type AnvilPublishDiagnosticsParams,
  type AnvilScanBufferResult,
  type Capability,
  type CapabilityDowngrade,
  type DriverManifestSlice,
} from './types.js';

describe('DRVR-002 protocol method names', () => {
  // These tests pin the wire strings byte-for-byte against the Rust
  // authoritative constants in
  // `crates/anvil-intercept-proto/src/protocol.rs`. If the Rust side
  // changes a name, this test fails and the TS side must be re-pinned
  // before merging — that is the intent of the contract.
  it('publishDiagnostics is anvil/publishDiagnostics', () => {
    expect(ANVIL_PUBLISH_DIAGNOSTICS).toBe('anvil/publishDiagnostics');
  });

  it('scan_buffer is anvil/scan_buffer', () => {
    expect(ANVIL_SCAN_BUFFER).toBe('anvil/scan_buffer');
  });

  it('enforcement/ack is anvil/enforcement/ack', () => {
    // DRVR-008 hinges on this string. A typo here lets a stock-LSP
    // driver claim enforcement support it does not have.
    expect(ANVIL_ENFORCEMENT_ACK).toBe('anvil/enforcement/ack');
  });

  it('gate/request is anvil/gate/request', () => {
    // M3 council finding: the method was missing from §3.2's table.
    // Pin the canonical name.
    expect(ANVIL_GATE_REQUEST).toBe('anvil/gate/request');
  });

  it('suppression/apply is anvil/suppression/apply', () => {
    expect(ANVIL_SUPPRESSION_APPLY).toBe('anvil/suppression/apply');
  });

  it('status/query is anvil/status/query', () => {
    expect(ANVIL_STATUS_QUERY).toBe('anvil/status/query');
  });

  it('ALL_ANVIL_METHODS lists every constant exactly once', () => {
    const set = new Set(ALL_ANVIL_METHODS);
    expect(set.size).toBe(ALL_ANVIL_METHODS.length);
    expect(set).toContain(ANVIL_PUBLISH_DIAGNOSTICS);
    expect(set).toContain(ANVIL_SCAN_BUFFER);
    expect(set).toContain(ANVIL_ENFORCEMENT_ACK);
    expect(set).toContain(ANVIL_GATE_REQUEST);
    expect(set).toContain(ANVIL_SUPPRESSION_APPLY);
    expect(set).toContain(ANVIL_STATUS_QUERY);
  });
});

describe('DRVR-002 capability vocabulary', () => {
  it('exports both lattice values', () => {
    // Pin the kebab-case forms — these cross the JSON-RPC transport
    // and must match the Rust enum's serde rename.
    expect(ALL_CAPABILITIES).toEqual(['attached', 'participating']);
  });

  it('attached is the read-only floor', () => {
    const c: Capability = 'attached';
    expect(c).toBe('attached');
  });

  it('participating models enforcement-candidate state', () => {
    const c: Capability = 'participating';
    expect(c).toBe('participating');
  });
});

describe('DRVR-008 manifest slice', () => {
  it('round-trips through JSON when supported_anvil_methods is empty', () => {
    // A stock LSP client with no `anvil/` support; daemon caps at
    // Attached per DRVR-008.
    const manifest: DriverManifestSlice = {
      workspace_roots: ['/tmp/wt'],
      supported_anvil_methods: [],
    };
    const wire = JSON.stringify(manifest);
    const back = JSON.parse(wire) as DriverManifestSlice;
    expect(back).toEqual(manifest);
  });

  it('round-trips through JSON when supported_anvil_methods is populated', () => {
    const manifest: DriverManifestSlice = {
      workspace_roots: ['/tmp/wt'],
      supported_anvil_methods: [ANVIL_PUBLISH_DIAGNOSTICS, ANVIL_ENFORCEMENT_ACK],
    };
    const wire = JSON.stringify(manifest);
    const back = JSON.parse(wire) as DriverManifestSlice;
    expect(back).toEqual(manifest);
    expect(back.supported_anvil_methods).toContain(ANVIL_ENFORCEMENT_ACK);
  });

  it('uses snake_case field names on the wire', () => {
    // The wire format must match the Rust serde-default snake_case
    // convention — drivers connecting from non-TS hosts (Neovim Lua,
    // Helix Rust) need to spell the field consistently.
    const manifest: DriverManifestSlice = {
      workspace_roots: ['/tmp/wt'],
      supported_anvil_methods: [ANVIL_ENFORCEMENT_ACK],
    };
    const wire = JSON.stringify(manifest);
    expect(wire).toContain('"workspace_roots"');
    expect(wire).toContain('"supported_anvil_methods"');
    expect(wire).not.toContain('workspaceRoots');
    expect(wire).not.toContain('supportedAnvilMethods');
  });
});

describe('DRVR-008 capability-downgrade event', () => {
  it('round-trips a missing-enforcement-ack-method event', () => {
    // Event the daemon emits to a stock LSP driver that asked for
    // participation. The reason is the structured kebab-case string
    // the operator sees.
    const downgrade: CapabilityDowngrade = {
      requested: 'participating',
      negotiated: 'attached',
      reason: 'missing-enforcement-ack-method',
      advertised_methods: [ANVIL_PUBLISH_DIAGNOSTICS],
    };
    const back = JSON.parse(JSON.stringify(downgrade)) as CapabilityDowngrade;
    expect(back).toEqual(downgrade);
    expect(back.negotiated).toBe('attached');
  });

  it('preserves the kebab-case reason string', () => {
    const downgrade: CapabilityDowngrade = {
      requested: 'participating',
      negotiated: 'attached',
      reason: 'not-enforcement-candidate',
      advertised_methods: [],
    };
    expect(JSON.stringify(downgrade)).toContain('"reason":"not-enforcement-candidate"');
  });
});

describe('DRVR-002 method parameter shapes', () => {
  it('publishDiagnostics carries canonical Diagnostic[]', () => {
    // The protocol layer reuses the inner Diagnostic shape from
    // `../diagnostics/types.ts`. This test asserts that contract by
    // declaring a Diagnostic and embedding it in the params. If the
    // import drifts (e.g. someone tries to redefine Diagnostic in
    // protocol/types.ts) the type signature will fail to compile.
    const diag: Diagnostic = {
      schema_version: 'anvil.diagnostic.v1',
      id: 'diag_x',
      severity: 'error',
      summary: 'leaked secret',
      location: { file: 'src/api.ts', line: 1, column: 1 },
      category: 'secret',
      source: { rule_id: 'r', source_module: 'm' },
      mode: 'save-time',
    };
    const params: AnvilPublishDiagnosticsParams = {
      uri: 'file:///workspace/src/api.ts',
      version: 1,
      diagnostics: [diag],
    };
    expect(params.diagnostics).toHaveLength(1);
    expect(params.diagnostics[0]?.id).toBe('diag_x');
  });

  it('scan_buffer result carries the truncation flag', () => {
    // Per the envelope spec §3 control-form: mid-edit responses set
    // `truncated` when the daemon caps the diagnostic set.
    const result: AnvilScanBufferResult = {
      version: 17,
      diagnostics: [],
      truncated: false,
    };
    expect(result.truncated).toBe(false);
  });
});
