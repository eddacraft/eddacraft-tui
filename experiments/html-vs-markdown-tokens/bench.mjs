#!/usr/bin/env node
// Compare token cost of loading & processing the same APS plan in three
// formats: markdown, minimal HTML, and structured (data-attribute) HTML.
//
// Usage:
//   ANTHROPIC_API_KEY=sk-ant-... \
//     node experiments/html-vs-markdown-tokens/bench.mjs [--model NAME] [--runs N]
//
// Defaults: model=claude-sonnet-4-6, runs=1.
//
// Output: a table with byte size, count_tokens result, and round-trip
// input/output tokens + latency per variant.

import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));

const args = process.argv.slice(2);
const getArg = (flag, fallback) => {
  const i = args.indexOf(flag);
  return i >= 0 && args[i + 1] ? args[i + 1] : fallback;
};
const MODEL = getArg('--model', 'claude-sonnet-4-6');
const RUNS = Number(getArg('--runs', '1'));
const API_KEY = process.env.ANTHROPIC_API_KEY;

if (!API_KEY) {
  console.error('ANTHROPIC_API_KEY is not set. Aborting.');
  process.exit(1);
}

const VARIANTS = [
  { name: 'markdown', file: 'source.md' },
  { name: 'html-minimal', file: 'source.minimal.html' },
  { name: 'html-structured', file: 'source.structured.html' },
];

// A realistic processing task that forces the model to read the whole plan,
// pick out structured fields, and emit a constrained shape. JSON output keeps
// the response size comparable across formats.
const PROCESSING_PROMPT = `You are given an Anvil Plan Spec (APS) module document below.
Extract every work item and return a JSON array. Each element must have:
  - id (e.g. "TUIDASH-003")
  - priority (High|Medium|Low|null)
  - confidence (high|medium|low|null)
  - dependencyCount (number of declared dependencies, 0 if none)
Return ONLY the JSON array, no prose.

Document:
---
{{BODY}}
---`;

const HEADERS = {
  'x-api-key': API_KEY,
  'anthropic-version': '2023-06-01',
  'content-type': 'application/json',
};

async function countTokens(body) {
  const res = await fetch('https://api.anthropic.com/v1/messages/count_tokens', {
    method: 'POST',
    headers: HEADERS,
    body: JSON.stringify({
      model: MODEL,
      messages: [{ role: 'user', content: body }],
    }),
  });
  if (!res.ok) throw new Error(`count_tokens ${res.status}: ${await res.text()}`);
  return res.json();
}

async function runPrompt(body) {
  const prompt = PROCESSING_PROMPT.replace('{{BODY}}', body);
  const t0 = Date.now();
  const res = await fetch('https://api.anthropic.com/v1/messages', {
    method: 'POST',
    headers: HEADERS,
    body: JSON.stringify({
      model: MODEL,
      max_tokens: 4096,
      messages: [{ role: 'user', content: prompt }],
    }),
  });
  const latencyMs = Date.now() - t0;
  if (!res.ok) throw new Error(`messages ${res.status}: ${await res.text()}`);
  const json = await res.json();
  return { usage: json.usage, latencyMs };
}

function avg(nums) {
  return nums.reduce((a, b) => a + b, 0) / nums.length;
}

const rows = [];
for (const v of VARIANTS) {
  const body = readFileSync(join(here, v.file), 'utf8');
  const bytes = Buffer.byteLength(body, 'utf8');
  process.stderr.write(`[${v.name}] count_tokens...`);
  const { input_tokens: rawTokens } = await countTokens(body);
  process.stderr.write(` ${rawTokens} tokens\n`);

  const runs = [];
  for (let i = 0; i < RUNS; i++) {
    process.stderr.write(`[${v.name}] processing run ${i + 1}/${RUNS}...`);
    const r = await runPrompt(body);
    process.stderr.write(
      ` in=${r.usage.input_tokens} out=${r.usage.output_tokens} ${r.latencyMs}ms\n`
    );
    runs.push(r);
  }

  rows.push({
    variant: v.name,
    bytes,
    rawTokens,
    promptInTokens: Math.round(avg(runs.map((r) => r.usage.input_tokens))),
    promptOutTokens: Math.round(avg(runs.map((r) => r.usage.output_tokens))),
    latencyMs: Math.round(avg(runs.map((r) => r.latencyMs))),
  });
}

const base = rows.find((r) => r.variant === 'markdown');
console.log(`\nModel: ${MODEL}  Runs: ${RUNS}\n`);
console.log(
  'variant           bytes   count_tokens   prompt_in   prompt_out   latency_ms   tokens_vs_md'
);
console.log(
  '----------------  ------  ------------   ---------   ----------   ----------   ------------'
);
for (const r of rows) {
  const delta =
    r.variant === 'markdown'
      ? '  (baseline)'
      : `${((r.rawTokens / base.rawTokens - 1) * 100).toFixed(1).padStart(6)}%`;
  console.log(
    [
      r.variant.padEnd(16),
      String(r.bytes).padStart(6),
      String(r.rawTokens).padStart(12),
      String(r.promptInTokens).padStart(9),
      String(r.promptOutTokens).padStart(10),
      String(r.latencyMs).padStart(10),
      delta.padStart(12),
    ].join('  ')
  );
}
