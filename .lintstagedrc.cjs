// lint-staged config. Migrated from JSON so vendored package output (e.g.
// tools/nx-rust/dist/) can be filtered before lint-staged hands files to
// oxlint/eslint — those tools exit non-zero when every file passed in is
// ignored by their own config.

const { relative } = require('node:path');

// Normalise Windows backslash separators so the vendored-output filter matches
// regardless of how lint-staged hands paths in. lint-staged on Windows can
// emit either form depending on git config (`core.autocrlf`, etc.).
const normalisePath = (file) => file.replace(/\\/g, '/');

// `.prettierignore` ignores `/plans` — root-anchored, so only the top-level
// planning tree. Anchoring here has to match: `docs/archive/plans/` is NOT
// prettierignored and must keep being formatted, so a bare `/plans/` substring
// test would wrongly skip it. lint-staged hands absolute paths and runs with
// cwd at the repository root, so resolve against cwd and require the result to
// start at `plans/`.
const isRootPlansDoc = (file) => {
  const rel = normalisePath(relative(process.cwd(), file));
  return rel === 'plans' || rel.startsWith('plans/');
};

// The root acknowledgements file is generated and intentionally excluded from
// oxfmt. Keep markdownlint coverage, but do not hand it to the formatter.
const isGeneratedMarkdown = (file) =>
  normalisePath(relative(process.cwd(), file)) === 'ACKNOWLEDGEMENTS.md';

// Init-file fixture must stay byte-identical to `anvil init` (double-quoted
// YAML). `.prettierignore` excludes it, so oxfmt --write on that path alone
// exits non-zero with "Expected at least one target file".
const isInitAnvilFixture = (file) =>
  normalisePath(relative(process.cwd(), file)) === 'scripts/docs/fixtures/anvil-init.yaml';

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
    const formatted = kept.filter((f) => !isInitAnvilFixture(f));
    const tasks = [`yamllint ${toCommandList(kept)}`];
    if (formatted.length > 0) {
      tasks.push(`oxfmt --write ${toCommandList(formatted)}`);
    }
    return tasks;
  },
  'temper.{yml,yaml}': (files) => {
    const kept = filter(files);
    if (kept.length === 0) return [];
    return [`oxfmt --write ${toCommandList(kept)}`];
  },
  '*.md': (files) => {
    // Align with CI `pnpm format:check` (oxfmt --check .), which formats
    // Markdown under docs/ and similar. Agent-config dirs are prettierignored
    // so their skill fences are not reflowed — skip them here the same way
    // as the TS/JSON globs (CIB-191 empty-target avoidance). Root plans/
    // stay in `kept`; oxfmt skips them via isRootPlansDoc so plans-only
    // stages do not hand oxfmt an empty target set.
    const kept = filter(files).filter((f) => !isAgentConfig(f));
    if (kept.length === 0) return [];
    const tasks = [];
    // `/plans` is prettierignored wholesale, so a commit staging only planning
    // markdown hands oxfmt an all-excluded target set and it exits non-zero
    // with "Expected at least one target file", failing the whole pre-commit
    // hook. markdownlint is safe to hand the same files: it exits 0 when every
    // input is excluded, so dropping planning docs from the formatter only —
    // not from the glob — is enough to keep the hook green.
    //
    // markdownlint does not lint them either. `.markdownlintignore` excludes
    // `plans/**` and markdownlint-cli loads that file automatically (the
    // explicit `--ignore-path` in `pnpm lint:check` is redundant, not the
    // thing that enables it), and the exclusion still matches the absolute
    // paths lint-staged passes. Planning docs reaching markdownlint here are
    // a deliberate no-op, not coverage.
    const formatted = kept.filter((f) => !isRootPlansDoc(f) && !isGeneratedMarkdown(f));
    if (formatted.length > 0) {
      tasks.push(`oxfmt --write ${toCommandList(formatted)}`);
    }
    tasks.push(`markdownlint --fix ${toCommandList(kept)}`);
    return tasks;
  },
};
