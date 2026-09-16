// Shared ignore-rule matcher for the pre-commit gate (CIB-403 repair round 1).
//
// Both oxfmt and oxlint exit non-zero when EVERY path handed to them is
// excluded by their own ignore rules:
//
//   oxfmt : exit 2, "Expected at least one target file. All matched files may
//           have been excluded by ignore rules."
//   oxlint: exit 1, "No files found to lint. Please check your paths and
//           ignore patterns."
//
// A mixed batch is fine — both silently skip the excluded entries — so the
// failure is specifically the all-excluded batch. That makes an ignored file a
// commit-blocker for any lint-staged key whose only staged match is ignored,
// with `--no-verify` as the contributor's only escape.
//
// Every other key in `.lintstagedrc.cjs` dodges this with a bespoke predicate
// (isRootPlansDoc, isGeneratedMarkdown, isAuditJson, isInitAnvilFixture) — one
// per ignore rule someone tripped over, added after the fact. CIB-403's
// `*.{mjs,cjs}` key shipped without one and tripped over `archive/`, which
// excludes docs/archive/experiments/html-vs-markdown-tokens/{bench,convert}.mjs
// from both tools. This reads the real ignore files instead of naming those two
// paths, so the next archived `.mjs` — or the next ignore rule — is covered
// without a code change.
//
// The two tools do NOT share an ignore set: oxfmt reads `.prettierignore`
// (which also excludes `/plans`, ACKNOWLEDGEMENTS.md, vendored schemas …) while
// oxlint reads `ignorePatterns` in `.oxlintrc.json` (a shorter list). They are
// compiled by the same code but exposed as two predicates, because filtering
// one tool's batch by the other's rules would either re-introduce the blocking
// failure or silently skip a file the tool would have checked.
//
// gitignore semantics implemented: comments and blank lines, `!` negation with
// last-match-wins, trailing `/` for directory-only, a leading `/` or an interior
// `/` anchoring to the repository root, `**`, `*` and `?`. A pattern is tested
// against the path itself and against each of its ancestor directories, which
// is what makes a directory rule exclude everything beneath it.

const { readFileSync } = require('node:fs');
const { relative, resolve, sep } = require('node:path');

// Built at runtime, never written as a literal NUL in this file: a source
// file containing a NUL byte is treated as binary by git, which kills diffs
// and review on the very module the gate depends on.
const NUL = String.fromCharCode(0);

const escapeLiteral = (char) => char.replace(/[.+^${}()|[\]\\]/g, '\\$&');

// Translate one gitignore-style pattern into an anchored RegExp plus the
// directory-only flag the caller needs to decide whether a leaf path can match.
const compilePattern = (rawPattern) => {
  let pattern = rawPattern;
  const negated = pattern.startsWith('!');
  if (negated) pattern = pattern.slice(1);

  const directoryOnly = pattern.endsWith('/');
  if (directoryOnly) pattern = pattern.slice(0, -1);

  // Anchoring is decided BEFORE the leading slash is stripped: gitignore anchors
  // a pattern to the ignore file's directory when it contains a slash anywhere
  // other than the very end, which a trailing-slash-only pattern such as
  // `archive/` does not.
  const anchored = pattern.startsWith('/') || pattern.slice(0, -1).includes('/');
  if (pattern.startsWith('/')) pattern = pattern.slice(1);

  let source = '';
  for (let i = 0; i < pattern.length; i += 1) {
    const char = pattern[i];
    if (char === '*') {
      if (pattern[i + 1] === '*') {
        if (pattern[i + 2] === '/') {
          source += '(?:.*/)?';
          i += 2;
        } else {
          source += '.*';
          i += 1;
        }
      } else {
        source += '[^/]*';
      }
    } else if (char === '?') {
      source += '[^/]';
    } else {
      source += escapeLiteral(char);
    }
  }

  return {
    negated,
    directoryOnly,
    regex: new RegExp(`${anchored ? '^' : '^(?:.*/)?'}${source}$`),
  };
};

const parseIgnoreLines = (contents) =>
  contents
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0 && !line.startsWith('#'))
    .map(compilePattern);

// Reading either config can fail (absent file, unparseable JSON). Failing open
// — treating nothing as ignored — is the safe direction: the gate then hands the
// tool every staged file, which is stricter, not weaker.
const readPrettierIgnore = (root) => {
  try {
    return parseIgnoreLines(readFileSync(resolve(root, '.prettierignore'), 'utf8'));
  } catch {
    return [];
  }
};

const readOxlintIgnore = (root) => {
  try {
    const config = JSON.parse(readFileSync(resolve(root, '.oxlintrc.json'), 'utf8'));
    const patterns = Array.isArray(config.ignorePatterns) ? config.ignorePatterns : [];
    return patterns.map((pattern) => compilePattern(pattern));
  } catch {
    return [];
  }
};

const caches = new Map();

const rulesFor = (key, root, load) => {
  const cacheKey = `${key}|${root}`;
  if (!caches.has(cacheKey)) caches.set(cacheKey, load(root));
  return caches.get(cacheKey);
};

// Repo-relative, forward-slashed, no leading `./`.
const toRelative = (file, root) => relative(root, resolve(root, file)).split(sep).join('/');

const matches = (rules, file, root) => {
  const relativePath = toRelative(file, root);
  if (relativePath === '' || relativePath.startsWith('..')) return false;

  // The path plus every ancestor directory. `a/b/c.mjs` also tests `a/b` and
  // `a`, so a directory rule such as `archive/` excludes the whole subtree.
  const segments = relativePath.split('/');
  const candidates = segments.map((_, index) => segments.slice(0, index + 1).join('/'));

  let ignored = false;
  for (const rule of rules) {
    for (let index = 0; index < candidates.length; index += 1) {
      const isLeaf = index === candidates.length - 1;
      if (rule.directoryOnly && isLeaf) continue;
      if (rule.regex.test(candidates[index])) {
        ignored = !rule.negated;
        break;
      }
    }
  }
  return ignored;
};

// `files.filter(isOxfmtIgnored)` is the shape everyone reaches for, and it
// passes the array INDEX as the second argument — which used to land in `root`
// and throw ERR_INVALID_ARG_TYPE out of path.resolve. On a module the commit
// gate depends on, a predicate that explodes when used the obvious way is a
// trap worth closing rather than documenting: anything that is not a string is
// not a root, so fall back to the default.
const rootOrCwd = (root) => (typeof root === 'string' ? root : process.cwd());

const isOxfmtIgnored = (file, root) =>
  matches(rulesFor('prettier', rootOrCwd(root), readPrettierIgnore), file, rootOrCwd(root));

const isOxlintIgnored = (file, root) =>
  matches(rulesFor('oxlint', rootOrCwd(root), readOxlintIgnore), file, rootOrCwd(root));

module.exports = { isOxfmtIgnored, isOxlintIgnored };

// CLI filter for `.husky/pre-commit`: NUL-separated paths in on stdin, the ones
// oxfmt will actually accept out on stdout, same separator. The hook's second
// pass runs `oxfmt --check` only, so it filters on the oxfmt rules. Sharing this
// module with `.lintstagedrc.cjs` is what stops the two passes drifting apart,
// which is how this class of bug recurs.
if (require.main === module) {
  const chunks = [];
  process.stdin.on('data', (chunk) => chunks.push(chunk));
  process.stdin.on('end', () => {
    const input = Buffer.concat(chunks).toString('utf8');
    const kept = input
      .split(NUL)
      .filter((entry) => entry.length > 0)
      .filter((entry) => !isOxfmtIgnored(entry));
    process.stdout.write(kept.map((entry) => entry + NUL).join(''));
  });
}
