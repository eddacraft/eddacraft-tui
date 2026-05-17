import type { CandidateMatch, MatcherPlugin } from 'deepsec/config';

const TRUST_BOUNDARY_RE =
  /\b(?:is_driver_allowed|DriverManifest|validate_workspace_roots|Cross::Spoofed|with_cross_check_context|SO_PEERCRED|canonicalize|canonicalise|workspace_roots|ANVIL_ENFORCEMENT_ACK|allowlist|scan_buffer|MAX_LINE_BYTES|MAX_JSONRPC_BATCH_ITEMS)\b/;

export const anvilInterceptTrustBoundary: MatcherPlugin = {
  slug: 'anvil-intercept-trust-boundary',
  description: 'Anvil Rust intercept daemon trust-boundary and IPC enforcement surface',
  noiseTier: 'noisy',
  filePatterns: [
    'crates/anvil-intercept/src/auth.rs',
    'crates/anvil-intercept/src/ipc.rs',
    'crates/anvil-intercept/src/registry.rs',
    'crates/anvil-intercept/src/fence.rs',
    'crates/anvil-intercept/src/enforcement.rs',
  ],
  match(content, filePath): CandidateMatch[] {
    if (/\/tests?\/|_test\.rs$/.test(filePath)) return [];

    const lines = content.split('\n');
    for (let i = 0; i < lines.length; i++) {
      if (!TRUST_BOUNDARY_RE.test(lines[i])) continue;
      const start = Math.max(0, i - 2);
      const end = Math.min(lines.length, i + 5);
      return [
        {
          vulnSlug: 'anvil-intercept-trust-boundary',
          lineNumbers: [i + 1],
          snippet: lines.slice(start, end).join('\n'),
          matchedPattern: 'intercept trust-boundary, driver, IPC, or canonicalisation surface',
        },
      ];
    }

    return [];
  },
};
