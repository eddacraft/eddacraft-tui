import type { CandidateMatch, MatcherPlugin } from 'deepsec/config';

/**
 * Lines where the daemon-side Rust crates ingest data across a trust
 * boundary: socket accept/recv, line/stream reads, wire deserialization,
 * and filesystem-event delivery. Covers the intercept daemon, its wire
 * protocol crate, the win32 port, the run wrapper, and the git hook —
 * all outside deepsec's default (TS-tilted) glob set.
 *
 * Normal tier: the marker is broad (any ingestion point); the AI decides
 * whether the input is validated/canonicalised before use. Capped at 3
 * candidates per file so marker-dense files (ipc.rs has ~50 hits) don't
 * drown the queue.
 */
const INGEST_RE =
  /\bserde_json::from_(?:str|slice|reader|value)|\bbincode::deserialize|\bfrom_reader\b|\bread_line\b|\bread_to_string\b|\bread_until\b|\.recv\(|\.accept\(|\bUnixListener\b|\bUnixStream\b|\bLocalSocket|\bnamed_pipe|\bnotify::|\bwatcher\b/i;
const MAX_PER_FILE = 3;

export const anvilDaemonWireInput: MatcherPlugin = {
  slug: 'anvil-daemon-wire-input',
  description: 'Daemon-side Rust crate ingesting IPC, wire, or file-event input',
  noiseTier: 'normal',
  filePatterns: [
    'crates/anvil-intercept/src/**/*.rs',
    'crates/anvil-intercept-proto/src/**/*.rs',
    'crates/anvil-intercept-win32/src/**/*.rs',
    'crates/anvil-run/src/**/*.rs',
    'crates/anvil-hook/src/**/*.rs',
  ],
  examples: [
    'let msg: WireRequest = serde_json::from_str(&line)?;',
    'let (stream, _addr) = listener.accept().await?;',
    'reader.read_line(&mut buf)?;',
  ],
  match(content, filePath): CandidateMatch[] {
    if (/\/tests?\/|_test\.rs$|\/benches\/|\/examples\//.test(filePath)) return [];

    const lines = content.split('\n');
    const matches: CandidateMatch[] = [];

    for (let i = 0; i < lines.length && matches.length < MAX_PER_FILE; i++) {
      if (/^\s*(?:\/\/|\*)/.test(lines[i])) continue;
      if (/^\s*(?:pub(?:\([^)]*\))?\s+)?(?:mod|use)\s/.test(lines[i])) continue;
      if (!INGEST_RE.test(lines[i])) continue;
      const start = Math.max(0, i - 2);
      const end = Math.min(lines.length, i + 5);
      matches.push({
        vulnSlug: 'anvil-daemon-wire-input',
        lineNumbers: [i + 1],
        snippet: lines.slice(start, end).join('\n'),
        matchedPattern: 'IPC/wire/file-event ingestion point',
      });
    }

    return matches;
  },
};
