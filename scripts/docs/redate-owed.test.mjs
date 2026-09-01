import assert from 'node:assert/strict';
import { test } from 'node:test';

import {
  findFreshnessCell,
  hasUnescapedPipe,
  localDate,
  rewriteFreshness,
  tableCells,
} from './redate-owed.mjs';

const metadataTable = [
  '# Example',
  '',
  '| Type  | Authority     | Owner | Status | Freshness                       |',
  '| ----- | ------------- | ----- | ------ | ------------------------------- |',
  '| Guide | Authoritative | DOCRB | Live   | Last reviewed 2026-08-30 vs `x` |',
  '',
].join('\n');

test('tableCells strips the leading and trailing pipe', () => {
  assert.deepEqual(tableCells('| a | b | c |'), ['a', 'b', 'c']);
});

test('findFreshnessCell locates the DOCGOV-002 value row', () => {
  const cell = findFreshnessCell(metadataTable);
  assert.equal(cell.row, 4);
  assert.equal(cell.freshness, 'Last reviewed 2026-08-30 vs `x`');
  assert.equal(cell.cells.length, 5);
});

test('findFreshnessCell returns null for an ungoverned document', () => {
  assert.equal(findFreshnessCell('# No table here\n\nProse only.\n'), null);
});

test('findFreshnessCell returns null when the value row is short', () => {
  const truncated = metadataTable.replace(
    '| Guide | Authoritative | DOCRB | Live   |',
    '| Guide |'
  );
  assert.equal(findFreshnessCell(truncated), null);
});

test('rewriteFreshness leads with the new review and demotes the old one', () => {
  const rewritten = rewriteFreshness(
    'Last reviewed 2026-08-30 against `scripts/x.sh`; nothing moved',
    {
      date: '2026-08-31',
      note: 'Re-dated for the probe.',
    }
  );
  assert.equal(
    rewritten,
    'Last reviewed 2026-08-31 Re-dated for the probe. Prior review 2026-08-30 against `scripts/x.sh`; nothing moved'
  );
});

test('rewriteFreshness handles the plans/specs bare-date shape', () => {
  const rewritten = rewriteFreshness('2026-08-30 — freshness bump; diagrams unchanged', {
    date: '2026-08-31',
    note: 'Re-dated for the probe.',
  });
  assert.equal(
    rewritten,
    '2026-08-31 — Re-dated for the probe. Prior 2026-08-30 — freshness bump; diagrams unchanged'
  );
});

test('rewriteFreshness keeps an unrecognised cell verbatim as the prior review', () => {
  const rewritten = rewriteFreshness('Reviewed at some point, somehow', {
    date: '2026-08-31',
    note: 'Re-dated.',
  });
  assert.equal(
    rewritten,
    'Last reviewed 2026-08-31 Re-dated. Prior review: Reviewed at some point, somehow'
  );
});

test('rewriteFreshness never drops the previous provenance', () => {
  const previous = 'Last reviewed 2026-08-29 at `abc1234` against `a.md`, `b.md`; prior 2026-08-01';
  const rewritten = rewriteFreshness(previous, { date: '2026-08-31', note: 'Note.' });
  for (const fragment of ['`abc1234`', '`a.md`', '`b.md`', 'prior 2026-08-01', '2026-08-29']) {
    assert.ok(rewritten.includes(fragment), `lost ${fragment}`);
  }
});

test('rewriteFreshness collapses whitespace so the markdown table stays one row', () => {
  const rewritten = rewriteFreshness('Last reviewed 2026-08-30\n  against `x`', {
    date: '2026-08-31',
    note: 'Note.',
  });
  assert.ok(!rewritten.includes('\n'));
});

test('hasUnescapedPipe catches a note that would break the metadata table', () => {
  assert.equal(hasUnescapedPipe('no pipes here'), false);
  assert.equal(hasUnescapedPipe('adds a | column'), true);
  assert.equal(hasUnescapedPipe('|leading pipe'), true);
  assert.equal(hasUnescapedPipe('escaped \\| pipe is fine'), false);
});

test('a rejected note leaves the metadata table with its five columns', () => {
  // The guard exists because the note is joined back with ' | ': an unescaped
  // pipe silently adds a column to every document in the cascade.
  const note = 'broke | it';
  assert.equal(hasUnescapedPipe(note), true);
  const cells = [...findFreshnessCell(metadataTable).cells];
  cells[4] = rewriteFreshness(cells[4], { date: '2026-08-31', note });
  assert.equal(`| ${cells.join(' | ')} |`.split('|').length - 2, 6);
});

test('localDate uses the local calendar, not UTC', () => {
  // 07:09 on 1 Sep in UTC+8 is still 31 Aug in UTC. The corpus dates in local
  // time, so the UTC form would stamp yesterday onto a review done today.
  const morningInUtcPlus8 = new Date('2026-09-01T07:09:00+08:00');
  assert.equal(morningInUtcPlus8.toISOString().slice(0, 10), '2026-08-31');
  assert.equal(localDate(morningInUtcPlus8), '2026-09-01');
});

test('localDate zero-pads month and day', () => {
  assert.equal(localDate(new Date(2026, 0, 5)), '2026-01-05');
  assert.equal(localDate(new Date(2026, 11, 31)), '2026-12-31');
});
