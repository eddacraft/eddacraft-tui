#!/usr/bin/env node
// CIB-390-adjacent honesty guard for the brand's text colours.
//
// Covers the accent and the two signal colours. The signal colours are here
// because they shipped unguarded and wrong: the website aliased both to
// --anvil, so an error, a warning and a decorative highlight rendered in one
// orange, and the dashboard's brick red sat at 4.22:1 while the terminal's had
// already been retuned. A guard that only knew about the accent could not see
// either.
//
// anvil Ember is used as text on both a near-white and a near-black ground, and
// no single value clears the WCAG AA 4.5:1 floor on both: #cc5500 measured
// 4.13:1 on #fafafa and 4.4998:1 on #0d0d0f — below the floor, not on it — and the dim hover tone (#a34400)
// measured 3.13:1 in dark mode — worse than the value it was meant to improve.
// So the accent is theme-specific, and this asserts every pairing rather than
// trusting that a hex "looks brand-correct".
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const AA = 4.5;

function channel(value) {
  const c = value / 255;
  return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}
function luminance(hex) {
  const h = hex.replace('#', '');
  const [r, g, b] = [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16));
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}
export function contrastRatio(a, b) {
  const [x, y] = [luminance(a), luminance(b)];
  const [hi, lo] = x > y ? [x, y] : [y, x];
  return (hi + 0.05) / (lo + 0.05);
}

function token(file, name, { scope } = {}) {
  const css = readFileSync(resolve(ROOT, file), 'utf8');
  const body = scope
    ? (css.split(scope)[1] ?? '').split('}')[0]
    : css.split("[data-theme='dark']")[0];
  const match = body.match(new RegExp(`${name}:\\s*(#[0-9a-fA-F]{6})`));
  if (!match) {
    // Name the aliasing case explicitly. It is the defect this guard was
    // extended for, and "not found" would read as a typo in the token name.
    const alias = body.match(new RegExp(`${name}:\\s*(var\\([^)]+\\))`));
    if (alias) {
      throw new Error(
        `${file}: ${name} is aliased to ${alias[1]}, so it has no contrast of its own. ` +
          'Signal colours must be literal values distinct from the accent.'
      );
    }
    throw new Error(`${file}: ${name} not found${scope ? ` in ${scope}` : ''}`);
  }
  return match[1];
}

const VOID = '#0d0d0f';
const PAPER = '#fafafa';
const DARK = "[data-theme='dark']";

// [label, foreground, background]
export const PAIRS = [
  ['website --anvil on --void', token('apps/website/app/globals.css', '--anvil'), VOID],
  ['docs-shell --anvil on --void', token('apps/docs-shell/app/globals.css', '--anvil'), VOID],
  ['dashboard --anvil on --void', token('apps/dashboard/src/styles.css', '--anvil'), VOID],
];

// Signal colours, on every surface that defines them. `token` reads a literal
// hex, so a surface that aliases one back to another token (the defect this
// covers) fails to parse rather than silently reporting the accent's ratio.
for (const [site, file] of [
  ['website', 'apps/website/app/globals.css'],
  ['dashboard', 'apps/dashboard/src/styles.css'],
]) {
  PAIRS.push(
    [`${site} --brick-red on --void`, token(file, '--brick-red'), VOID],
    [`${site} --dull-amber on --void`, token(file, '--dull-amber'), VOID]
  );
}

for (const [site, file] of [
  ['docs-public', 'apps/docs-public/src/css/custom.css'],
  ['anvil-docs-private', 'apps/anvil-docs-private/src/css/custom.css'],
]) {
  PAIRS.push(
    [`${site} accent on paper (light)`, token(file, '--ec-anvil-accent'), PAPER],
    [`${site} accent-dim on paper (light hover)`, token(file, '--ec-anvil-accent-dim'), PAPER],
    [`${site} accent on void (dark)`, token(file, '--ec-anvil-accent', { scope: DARK }), VOID],
    [
      `${site} accent-dim on void (dark hover)`,
      token(file, '--ec-anvil-accent-dim', { scope: DARK }),
      VOID,
    ]
  );
}

// Compare resolved paths rather than string-building a file:// URL: the
// concatenation happens to match for plain paths, but not for one needing URL
// encoding (a space or non-ASCII character in the checkout path).
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const failures = [];
  for (const [label, fg, bg] of PAIRS) {
    const ratio = contrastRatio(fg, bg);
    const ok = ratio >= AA;
    if (!ok) failures.push(`${label}: ${fg} on ${bg} is ${ratio.toFixed(2)}:1`);
    process.stdout.write(`  ${ok ? 'ok  ' : 'FAIL'} ${label.padEnd(46)} ${ratio.toFixed(2)}:1\n`);
  }
  if (failures.length > 0) {
    process.stdout.write(
      `\n[accent-contrast] ${failures.length} pairing(s) below the WCAG AA ${AA}:1 floor:\n` +
        failures.map((f) => `  - ${f}`).join('\n') +
        '\n'
    );
    process.exit(1);
  }
  process.stdout.write(`\n[accent-contrast] ${PAIRS.length} pairings checked, all >= ${AA}:1\n`);
}
