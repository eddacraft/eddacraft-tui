// lint-staged config. Migrated from JSON so vendored package output (e.g.
// tools/nx-rust/dist/) can be filtered before lint-staged hands files to
// oxlint/eslint — those tools exit non-zero when every file passed in is
// ignored by their own config.

// Normalise Windows backslash separators so the vendored-output filter matches
// regardless of how lint-staged hands paths in. lint-staged on Windows can
// emit either form depending on git config (`core.autocrlf`, etc.).
const normalisePath = (file) => file.replace(/\\/g, '/');

const isVendoredOutput = (file) => {
  const normalised = normalisePath(file);
  return (
    normalised.includes('/tools/nx-rust/dist/') ||
    normalised.startsWith('tools/nx-rust/dist/') ||
    // Vendored upstream SARIF 2.1.0 schema (SARIFOUT): kept byte-identical to
    // upstream and listed in .prettierignore, so oxfmt rejects it as an
    // excluded target. Filter it here like other vendored output. Lives in
    // anvil-sarif since GITGOV-008 (relocated from anvil-cli/src/output/).
    normalised.endsWith('crates/anvil-sarif/src/sarif-schema-2.1.0.json')
  );
};

const isAuditJson = (file) => {
  const normalised = normalisePath(file);
  return (
    normalised.endsWith('.json') &&
    (normalised.includes('/plans/audits/') || normalised.startsWith('plans/audits/'))
  );
};

// Agent-config class dirs (.claude/, .codex/, .opencode/) are excluded from
// oxfmt via .prettierignore to avoid mangling embedded ```markdown fences in
// skill files. Filter their JSON files (*.meta.json, skill.meta.json) so
// lint-staged doesn't pass them to oxfmt and trigger "no target files" errors.
const isAgentConfig = (file) => {
  const normalised = normalisePath(file);
  return (
    normalised.includes('/.claude/') ||
    normalised.startsWith('.claude/') ||
    normalised.includes('/.codex/') ||
    normalised.startsWith('.codex/') ||
    normalised.includes('/.opencode/') ||
    normalised.startsWith('.opencode/')
  );
};

const filter = (files) => files.filter((f) => !isVendoredOutput(f));

// Quote each file with JSON.stringify so paths containing spaces (common on
// macOS / Windows) survive the shell join. JSON.stringify gives us the right
// double-quoted form with backslash escapes for inner quotes — which is what
// every shell on every supported platform expects for argument quoting.
const toCommandList = (files) => files.map((file) => JSON.stringify(file)).join(' ');

module.exports = {
  '*.{js,jsx,ts,tsx}': (files) => {
    // Agent-config dirs are prettierignored (skill fences); skip them so
    // lint-staged does not hand oxfmt an empty target set (CIB-191).
    const kept = filter(files).filter((f) => !isAgentConfig(f));
    if (kept.length === 0) return [];
    const list = toCommandList(kept);
    return [`oxfmt --write ${list}`, `oxlint --fix ${list}`, `eslint --fix ${list}`];
  },
  '*.json': (files) => {
    const kept = filter(files).filter((f) => !isAgentConfig(f));
    if (kept.length === 0) return [];
    const formatted = kept.filter((file) => !isAuditJson(file));
    const auditJson = kept.filter(isAuditJson);
    const tasks = [];
    if (formatted.length > 0) {
      const list = toCommandList(formatted);
      tasks.push(`oxfmt --write ${list}`, `eslint --fix ${list}`);
    }
    if (auditJson.length > 0) {
      // Validate each audit JSON, naming the offending file on failure so a
      // bad file in a multi-file stage is obvious (a bare JSON.parse throws
      // without saying which file).
      tasks.push(
        `node -e "for (const file of process.argv.slice(1)) { try { JSON.parse(require('node:fs').readFileSync(file, 'utf8')); } catch (err) { console.error('Invalid JSON in ' + file + ': ' + err.message); process.exit(1); } }" ${toCommandList(auditJson)}`
      );
    }
    return tasks;
  },
  '!(pnpm-lock|temper).{yml,yaml}': (files) => {
    const kept = filter(files);
    if (kept.length === 0) return [];
    const list = toCommandList(kept);
    return [`yamllint ${list}`, `oxfmt --write ${list}`];
  },
  'temper.{yml,yaml}': (files) => {
    const kept = filter(files);
    if (kept.length === 0) return [];
    return [`oxfmt --write ${toCommandList(kept)}`];
  },
  '*.md': (files) => {
    const kept = filter(files);
    if (kept.length === 0) return [];
    return [`markdownlint --fix ${toCommandList(kept)}`];
  },
};
