import { readFileSync, readdirSync } from 'node:fs';

const appDir = new URL('../app/', import.meta.url);
const componentsDir = new URL('../components/', import.meta.url);

// Walk a directory recursively so nested routes (app/security, app/privacy, ...)
// are held to the same contract as the landing page.
function walk(dir, keep) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const url = new URL(entry.isDirectory() ? `${entry.name}/` : entry.name, dir);
    if (entry.isDirectory()) return walk(url, keep);
    return keep(entry.name) ? [url] : [];
  });
}

const files = [
  ...walk(appDir, (name) => name.endsWith('.tsx') || name.endsWith('.css')),
  ...walk(componentsDir, (name) => name.endsWith('.tsx')),
];
const content = files.map((file) => readFileSync(file, 'utf8')).join('\n');

const qualifiedFiles = new Map([
  ['decision-integrity-flywheel.tsx', ['still being built', 'system being completed']],
  ['delivery-boundary.tsx', ['operating today', 'system being completed']],
  ['decision-model.tsx', ['built toward', 'delivery boundary']],
  ['trust-gap.tsx', ['Today, anvil', 'foundation for a broader system']],
  ['product-stages.tsx', ['being built']],
  [
    'cli-footer.tsx',
    ['Dialog.Root', 'Dialog.Title', 'Dialog.Description', 'Dialog.Close', 'Close'],
  ],
  [
    'terminal-window.tsx',
    ['(max-width: 767px)', 'md:min-h-[28rem]', 'hidden md:', 'backend', 'protection_claim'],
  ],
]);
const heroContent = readFileSync(
  new URL('../components/hero-section.tsx', import.meta.url),
  'utf8'
);
const trustGapContent = readFileSync(
  new URL('../components/trust-gap.tsx', import.meta.url),
  'utf8'
);

const sourceContractsByFile = new Map([
  ['../app/layout.tsx', ['Inter']],
  [
    '../components/hero-section.tsx',
    ['Dialog.Title', 'Dialog.Description', 'Dialog.Close', 'Copy install command'],
  ],
  ['../components/trust-gap.tsx', ["label === 'ANVIL'"]],
  ['../components/terminal-window.tsx', ['MCP REQUEST :: anvil_validate_write']],
  ['../components/cli-footer.tsx', ['Dialog.Title', 'Dialog.Description', 'Dialog.Close']],
]);

const forbidden = [
  'FORCE PROBABILISTIC TOOLS',
  'Anvil',
  'EddaCraft',
  'THE LOOP IS ALREADY RUNNING',
  'IBM_Plex_Sans',
  '--font-ibm-plex-sans',
  '$ anvil_validate_write',
];

const allowedColours = new Set(['#0d0d0f', '#2a2a2e', '#ebebeb', '#cc5500', '#2e8b57']);
const offPaletteColours = [...content.matchAll(/#[\da-f]{6}/gi)]
  .map(([colour]) => colour.toLowerCase())
  .filter((colour) => !allowedColours.has(colour));
const shellLabelledMcpRequests = content
  .split('\n')
  .filter((line) => line.includes('anvil_validate_write') && line.includes('$'));

const failures = [
  ...[...sourceContractsByFile].flatMap(([path, values]) => {
    const fileContent = readFileSync(new URL(path, import.meta.url), 'utf8');
    return values
      .filter((value) => !fileContent.includes(value))
      .map((value) => `missing in ${path}: ${value}`);
  }),
  ...forbidden.filter((value) => content.includes(value)).map((value) => `retired: ${value}`),
  ...[...new Set(offPaletteColours)].map((value) => `off-palette: ${value}`),
  ...(shellLabelledMcpRequests.length > 0 ? ['MCP request presented as a shell command'] : []),
  ...(content.includes("scrollIntoView({ behavior: 'smooth' })")
    ? ['scrollIntoView forces smooth without reduced-motion check']
    : []),
  ...(content.includes('--ghost-grey: var(--off-white)')
    ? ['ghost-grey aliased to off-white']
    : []),
  ...(!/<TerminalWindow(?:\s[^>]*)?\s*\/?>/.test(heroContent)
    ? ['hero does not compose TerminalWindow']
    : []),
  ...(heroContent.includes('hidden md:block') ? ['hero hides the terminal on mobile'] : []),
  ...(trustGapContent.includes('index === 2') ? ['trust-gap highlights ANVIL by array index'] : []),
  ...[...qualifiedFiles].flatMap(([name, qualifiers]) => {
    const fileContent = readFileSync(new URL(`../components/${name}`, import.meta.url), 'utf8');
    return qualifiers
      .filter((qualifier) => !fileContent.includes(qualifier))
      .map((qualifier) => `missing qualifier in ${name}: ${qualifier}`);
  }),
];

if (failures.length > 0) {
  console.error(failures.join('\n'));
  process.exit(1);
}

console.log('website positioning contract: ok');
