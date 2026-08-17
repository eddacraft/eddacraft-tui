import { readFileSync, readdirSync } from 'node:fs';

const componentsDir = new URL('../components/', import.meta.url);
const files = [
  new URL('../app/page.tsx', import.meta.url),
  new URL('../app/layout.tsx', import.meta.url),
  ...readdirSync(componentsDir)
    .filter((name) => name.endsWith('.tsx'))
    .map((name) => new URL(`../components/${name}`, import.meta.url)),
];
const content = files.map((file) => readFileSync(file, 'utf8')).join('\n');

const required = [
  'TRUST THE CODE',
  'PROTECTION IS THE ENTRY POINT',
  'DECISION INTEGRITY IS THE SYSTEM AROUND IT',
  'TRUST INFRASTRUCTURE',
];

const forbidden = [
  'FORCE PROBABILISTIC TOOLS',
  'Anvil',
  'EddaCraft',
  'THE LOOP IS ALREADY RUNNING',
];

const failures = [
  ...required.filter((value) => !content.includes(value)).map((value) => `missing: ${value}`),
  ...forbidden.filter((value) => content.includes(value)).map((value) => `retired: ${value}`),
];

if (failures.length > 0) {
  console.error(failures.join('\n'));
  process.exit(1);
}

console.log('website positioning contract: ok');
