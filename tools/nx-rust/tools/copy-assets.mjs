#!/usr/bin/env node
// Copies non-TS assets (schema.json files, generator template trees) from
// src/ into dist/ so the compiled plugin has the companion files executors
// and generators expect at runtime.
import { copyFile, mkdir, readdir, stat } from 'node:fs/promises';
import { dirname, join, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = fileURLToPath(new URL('.', import.meta.url));
const root = join(here, '..');
const srcDir = join(root, 'src');
const distDir = join(root, 'dist');

async function* walk(dir) {
  for (const entry of await readdir(dir)) {
    const full = join(dir, entry);
    const st = await stat(full);
    if (st.isDirectory()) {
      yield* walk(full);
    } else {
      yield full;
    }
  }
}

const keep = (p) => {
  if (p.endsWith('.json')) return true;
  if (p.endsWith('.md')) return true;
  if (p.split(sep).includes('files')) return true;
  return false;
};

for await (const file of walk(srcDir)) {
  if (!keep(file)) continue;
  const rel = relative(srcDir, file);
  const dest = join(distDir, rel);
  await mkdir(dirname(dest), { recursive: true });
  await copyFile(file, dest);
}
