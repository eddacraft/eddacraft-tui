// lint-staged config. Migrated from JSON so vendored package output (e.g.
// tools/nx-rust/dist/) can be filtered before lint-staged hands files to
// oxlint/eslint — those tools exit non-zero when every file passed in is
// ignored by their own config.

const isVendoredOutput = (file) =>
  file.includes('/tools/nx-rust/dist/') || file.startsWith('tools/nx-rust/dist/');

const filter = (files) => files.filter((f) => !isVendoredOutput(f));

module.exports = {
  '*.{js,jsx,ts,tsx}': (files) => {
    const kept = filter(files);
    if (kept.length === 0) return [];
    const list = kept.join(' ');
    return [`oxfmt --write ${list}`, `oxlint --fix ${list}`, `eslint --fix ${list}`];
  },
  '*.json': (files) => {
    const kept = filter(files);
    if (kept.length === 0) return [];
    const list = kept.join(' ');
    return [`oxfmt --write ${list}`, `eslint --fix ${list}`];
  },
  '!(pnpm-lock|temper).{yml,yaml}': (files) => {
    const kept = filter(files);
    if (kept.length === 0) return [];
    const list = kept.join(' ');
    return [`yamllint ${list}`, `oxfmt --write ${list}`];
  },
  'temper.{yml,yaml}': (files) => {
    const kept = filter(files);
    if (kept.length === 0) return [];
    return [`oxfmt --write ${kept.join(' ')}`];
  },
  '*.md': (files) => {
    const kept = filter(files);
    if (kept.length === 0) return [];
    return [`markdownlint --fix ${kept.join(' ')}`];
  },
};
