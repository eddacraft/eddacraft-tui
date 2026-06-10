import type { CandidateMatch, MatcherPlugin } from 'deepsec/config';

/**
 * Rust process spawns where the program is not a string literal
 * (`Command::new(&self.binary_path)`, `Command::new(exe)`) or where the
 * program is a shell wrapper (`Command::new("sh")` + `-c` string
 * building). Rust has no implicit shell, so literal-program spawns with
 * dynamic args are usually fine — these two shapes are where argument or
 * path control becomes code execution. deepsec ships no Rust matchers;
 * this is a generic CWE-78-adjacent shape, candidate for upstreaming.
 */
const DYNAMIC_PROGRAM_RE = /\bCommand::new\s*\(\s*(?!["'])\S/;
const SHELL_WRAPPER_RE =
  /\bCommand::new\s*\(\s*"(?:sh|bash|zsh|dash|cmd|powershell|pwsh)(?:\.exe)?"/;

export const rsSpawnDynamic: MatcherPlugin = {
  slug: 'rs-spawn-dynamic',
  description: 'Rust Command::new with a non-literal program or a shell wrapper',
  noiseTier: 'precise',
  filePatterns: ['crates/**/src/**/*.rs', 'tools/**/src/**/*.rs'],
  examples: [
    'let output = Command::new(&self.binary_path).arg("eval").output()?;',
    'let mut cmd = std::process::Command::new(exe);',
    'std::process::Command::new("sh").arg("-c").arg(user_script);',
  ],
  match(content, filePath): CandidateMatch[] {
    if (/\/tests?\/|_test\.rs$|\/benches\/|\/examples\//.test(filePath)) return [];
    // The bench harness crate spawns binaries by design; not a security surface.
    if (/crates\/anvil-bench\//.test(filePath)) return [];

    const lines = content.split('\n');
    const matches: CandidateMatch[] = [];
    let inTestMod = false;

    for (let i = 0; i < lines.length; i++) {
      if (/^\s*#\[cfg\(test\)\]/.test(lines[i])) inTestMod = true;
      if (inTestMod) continue;
      if (/^\s*(?:\/\/|\*)/.test(lines[i])) continue;
      const shell = SHELL_WRAPPER_RE.test(lines[i]);
      const dynamic = !shell && DYNAMIC_PROGRAM_RE.test(lines[i]);
      if (!shell && !dynamic) continue;
      const start = Math.max(0, i - 2);
      const end = Math.min(lines.length, i + 6);
      matches.push({
        vulnSlug: 'rs-spawn-dynamic',
        lineNumbers: [i + 1],
        snippet: lines.slice(start, end).join('\n'),
        matchedPattern: shell
          ? 'shell-wrapper process spawn'
          : 'process spawn with non-literal program path',
      });
    }

    return matches;
  },
};
