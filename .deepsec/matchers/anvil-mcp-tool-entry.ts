import type { CandidateMatch, MatcherPlugin } from 'deepsec/config';

/**
 * Anvil's MCP server (`anvil mcp`) is a JSON-RPC entry point driven by
 * coding agents — every tool handler under `crates/anvil-cli/src/mcp/`
 * parses agent-supplied arguments (paths, buffers, suppression ids).
 * deepsec ships no Rust matchers, so this surface had zero FileRecords.
 * Noisy on purpose: every file in the MCP module should get AI review.
 */
const ENTRY_RE =
  /\bpub(?:\(crate\))?\s+(?:async\s+)?fn\s+\w+|serde_json::from_\w+|#\[derive\([^)]*Deserialize/;
const MAX_PER_FILE = 3;

export const anvilMcpToolEntry: MatcherPlugin = {
  slug: 'anvil-mcp-tool-entry',
  description: 'Anvil MCP server tool handler — agent-supplied JSON-RPC input',
  noiseTier: 'noisy',
  filePatterns: ['crates/anvil-cli/src/mcp/**/*.rs'],
  examples: [
    'pub(crate) async fn handle_validate_write(args: Value) -> Result<ToolOutput> {',
    'let req: SuppressRequest = serde_json::from_value(params)?;',
  ],
  match(content, filePath): CandidateMatch[] {
    if (/\/tests?\/|_test\.rs$/.test(filePath)) return [];

    const lines = content.split('\n');
    const matches: CandidateMatch[] = [];

    for (let i = 0; i < lines.length && matches.length < MAX_PER_FILE; i++) {
      // Inline test modules sit at the bottom of the file by convention.
      if (/^\s*#\[cfg\(test\)\]/.test(lines[i])) break;
      if (/^\s*(?:\/\/|\*)/.test(lines[i])) continue;
      if (!ENTRY_RE.test(lines[i])) continue;
      const start = Math.max(0, i - 1);
      const end = Math.min(lines.length, i + 6);
      matches.push({
        vulnSlug: 'anvil-mcp-tool-entry',
        lineNumbers: [i + 1],
        snippet: lines.slice(start, end).join('\n'),
        matchedPattern: 'MCP tool handler or agent-input deserialization',
      });
    }

    // Noisy entry-point coverage: a file in the MCP module with no
    // textual hit should still become a candidate.
    if (matches.length === 0 && content.trim().length > 0) {
      matches.push({
        vulnSlug: 'anvil-mcp-tool-entry',
        lineNumbers: [1],
        snippet: lines.slice(0, 8).join('\n'),
        matchedPattern: 'file inside the MCP server module',
      });
    }

    return matches;
  },
};
