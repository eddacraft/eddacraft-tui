import type { CandidateMatch, MatcherPlugin } from 'deepsec/config';

const TRUST_BOUNDARY_RE =
  /\b(?:is_driver_allowed|DriverManifest|validate_workspace_roots|Cross::Spoofed|with_cross_check_context|SO_PEERCRED|canonicalize|canonicalise|workspace_roots|ANVIL_ENFORCEMENT_ACK|allowlist|scan_buffer|MAX_LINE_BYTES|MAX_JSONRPC_BATCH_ITEMS|validate_paths|path_safety|workspace_admission|certify)\b/;

export const anvilInterceptTrustBoundary: MatcherPlugin = {
  slug: 'anvil-intercept-trust-boundary',
  description: 'Anvil Rust intercept daemon trust-boundary and IPC enforcement surface',
  noiseTier: 'noisy',
  filePatterns: ['crates/anvil-intercept/src/**/*.rs'],
  match(content, filePath): CandidateMatch[] {
    if (/\/tests?\/|_test\.rs$/.test(filePath)) return [];

    const lines = content.split('\n');
    const matches: CandidateMatch[] = [];

    // The keyword sweep is dense (ipc.rs alone has 300+ hits); the AI
    // reads the full file during process, so a handful of anchor
    // snippets per file is enough to queue it.
    const MAX_PER_FILE = 8;

    for (let i = 0; i < lines.length && matches.length < MAX_PER_FILE; i++) {
      if (/^\s*(?:\/\/|\*)/.test(lines[i])) continue;
      if (/^\s*(?:pub(?:\([^)]*\))?\s+)?(?:mod|use)\s/.test(lines[i])) continue;
      if (!TRUST_BOUNDARY_RE.test(lines[i])) continue;
      const start = Math.max(0, i - 2);
      const end = Math.min(lines.length, i + 5);
      matches.push({
        vulnSlug: 'anvil-intercept-trust-boundary',
        lineNumbers: [i + 1],
        snippet: lines.slice(start, end).join('\n'),
        matchedPattern: 'intercept trust-boundary, driver, IPC, or canonicalisation surface',
      });
    }

    return matches;
  },
};
