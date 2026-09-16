// lint-staged config. Migrated from JSON so vendored package output (e.g.
// tools/nx-rust/dist/) can be filtered before lint-staged hands files to
// oxlint/eslint — those tools exit non-zero when every file passed in is
// ignored by their own config.

const { relative } = require('node:path');
// CIB-403 repair: real ignore-rule matchers, shared with the oxfmt re-check in
// `.husky/pre-commit` so the two passes cannot drift apart.
const { isOxfmtIgnored, isOxlintIgnored } = require('./scripts/lint/ignore-rules.cjs');

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

// Agent-config class dirs are excluded from oxfmt via .prettierignore to avoid
// mangling embedded ```markdown fences in skill files. Filter them so
// lint-staged doesn't pass them to oxfmt and trigger "no target files" errors.
const isAgentConfig = (file) => {
  const normalised = normalisePath(file);
  return (
    normalised.includes('/.claude/') ||
    normalised.startsWith('.claude/') ||
    normalised.includes('/.codex/') ||
    normalised.startsWith('.codex/') ||
    normalised.includes('/.opencode/') ||
    normalised.startsWith('.opencode/') ||
    normalised.includes('/.agents/') ||
    normalised.startsWith('.agents/') ||
    normalised.includes('/.grok/') ||
    normalised.startsWith('.grok/') ||
    normalised.includes('/.github/agents/') ||
    normalised.startsWith('.github/agents/')
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
  // CIB-403: `.mjs`/`.cjs` were outside every other glob here, so 87 tracked
  // files (84 `.mjs`, 3 `.cjs` — including this config and `eslint.config.mjs`)
  // got NO pre-commit formatting or linting at all. lint-staged reported "could
  // not find any staged files matching configured tasks" and exited 0; the only
  // thing that caught them was CI's `oxfmt --check .`, a far slower loop than
  // the one every other executable source extension gets.
  //
  // A separate key rather than widening `*.{js,jsx,ts,tsx}`, because the task
  // list is deliberately narrower: oxfmt + oxlint, no `eslint --fix`. ESLint is
  // what differs, and it differs on purpose. The root flat config applies
  // `js.configs.recommended` and typescript-eslint's recommended set to
  // `.mjs`/`.cjs` without the `**/*.{ts,tsx,mts,cts}` block that downgrades
  // `preserve-caught-error` and `no-useless-assignment` to warnings, so
  // `eslint` reports 13 errors across 9 files that are on `main` today and that
  // CI does not enforce (`pnpm lint` runs `oxlint .` tree-wide plus per-project
  // nx lint targets, which do not cover these root scripts). Adding `eslint`
  // here would not move a CI check earlier — it would invent a new blocking
  // condition that fails the hook for anyone who stages an untouched file.
  // Parity with what CI actually enforces is the goal.
  '*.{mjs,cjs}': (files) => {
    const kept = filter(files).filter((f) => !isAgentConfig(f));
    if (kept.length === 0) return [];
    // Drop each tool's own ignored paths before invoking it, the same shape as
    // the `*.md` and `*.json` keys. oxfmt exits 2 and oxlint exits 1 on an
    // all-excluded batch, so without this, staging only
    // `docs/archive/.../bench.mjs` refuses the commit and leaves `--no-verify`
    // as the only way out — the new blocking condition this key's own comment
    // argues against, arriving via ignore rules instead of lint rules.
    //
    // Two predicates, not one: `.prettierignore` and `.oxlintrc.json`
    // `ignorePatterns` are different lists, and both are read from the files
    // themselves rather than naming paths, so the next archived `.mjs` is
    // covered without a code change.
    const formatted = kept.filter((f) => !isOxfmtIgnored(f));
    const linted = kept.filter((f) => !isOxlintIgnored(f));
    const tasks = [];
    if (formatted.length > 0) {
      tasks.push(`oxfmt --write ${toCommandList(formatted)}`);
    }
    if (linted.length > 0) {
      tasks.push(`oxlint --fix ${toCommandList(linted)}`);
    }
    return tasks;
  },
  '*.json': (files) => {
    const kept = filter(files).filter((f) => !isAgentConfig(f));
    if (kept.length === 0) return [];
    // `/plans` is prettierignored wholesale (same as Markdown). A plans-only
    // JSON stage otherwise hands oxfmt an all-excluded target set and fails
    // with "Expected at least one target file" — CCTX live V1 run records live
    // under plans/evals/. Skip oxfmt for root plans JSON; still JSON.parse.
    const formatted = kept.filter((file) => !isAuditJson(file) && !isRootPlansDoc(file));
    const parseOnly = kept.filter((file) => isAuditJson(file) || isRootPlansDoc(file));
    const tasks = [];
    if (formatted.length > 0) {
      const list = toCommandList(formatted);
      tasks.push(`oxfmt --write ${list}`, `eslint --fix ${list}`);
    }
    if (parseOnly.length > 0) {
      // Validate each plans/audit JSON and any root plans/** JSON, naming the offending file on failure so a
      // bad file in a multi-file stage is obvious (a bare JSON.parse throws
      // without saying which file).
      tasks.push(
        `node -e "for (const file of process.argv.slice(1)) { try { JSON.parse(require('node:fs').readFileSync(file, 'utf8')); } catch (err) { console.error('Invalid JSON in ' + file + ': ' + err.message); process.exit(1); } }" ${toCommandList(parseOnly)}`
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
    // hook. Drop planning docs from the formatter only; markdownlint is handled
    // below via the CIB-390 wrapper.
    const formatted = kept.filter((f) => !isRootPlansDoc(f) && !isGeneratedMarkdown(f));
    if (formatted.length > 0) {
      tasks.push(`oxfmt --write ${toCommandList(formatted)}`);
    }
    // CIB-390: markdownlint-cli exits 0 when every named input is ignored, so
    // the wrapper is the only entry point that can tell "checked" from
    // "examined nothing". `--omit-ignored` drops plans/** and other ignore
    // hits rather than failing the hook — those files still get conflict-marker
    // coverage below.
    tasks.push(
      `node scripts/docs/run-markdownlint.mjs --fix --omit-ignored ${toCommandList(kept)}`
    );
    // Conflict markers, on EVERY staged Markdown file including `plans/**`.
    //
    // Scope matters here. `.markdownlintignore` excludes `plans/**`, so the
    // markdownlint wrapper above omits planning docs — and a planning doc,
    // `plans/index.aps.md`, is exactly where a committed diff3 marker reached
    // main (#4187). Hanging this off `kept` rather than `formatted` is what
    // gives the planning tree its only staged-file coverage. `docs:check`
    // catches a marker after it is committed; this is the half that stops it
    // being committed at all.
    tasks.push(`node scripts/docs/check-conflict-markers.mjs ${toCommandList(kept)}`);
    return tasks;
  },
};
