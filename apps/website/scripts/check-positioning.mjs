import { readFileSync, readdirSync } from 'node:fs';

const appDir = new URL('../app/', import.meta.url);
const componentsDir = new URL('../components/', import.meta.url);
const pageFile = new URL('../app/page.tsx', import.meta.url);
const files = [
  ...readdirSync(appDir)
    .filter((name) => name.endsWith('.tsx') || name.endsWith('.css'))
    .map((name) => new URL(`../app/${name}`, import.meta.url)),
  ...readdirSync(componentsDir)
    .filter((name) => name.endsWith('.tsx'))
    .map((name) => new URL(`../components/${name}`, import.meta.url)),
];
const content = files.map((file) => readFileSync(file, 'utf8')).join('\n');
const pageContent = readFileSync(pageFile, 'utf8');

const composedComponents = [
  'ShippingProof',
  'TrustGap',
  'DecisionIntegrityFlywheel',
  'ProductStages',
  'DeliveryBoundary',
  'DecisionModel',
  'CompanyBand',
  'CLIFooter',
];

const qualifiedFiles = new Map([
  ['decision-integrity-flywheel.tsx', ['still being built', 'system being completed']],
  ['delivery-boundary.tsx', ['operating today', 'system being completed']],
  ['decision-model.tsx', ['This is the target model', 'not presented as shipped capabilities']],
  ['trust-gap.tsx', ['Today, anvil protects', 'foundation for a broader system']],
  [
    'cli-footer.tsx',
    ['Dialog.Root', 'Dialog.Title', 'Dialog.Description', 'Dialog.Close', 'Close'],
  ],
]);

const required = [
  'TRUST THE CODE',
  'PROTECTION IS THE ENTRY POINT',
  'DECISION INTEGRITY IS THE SYSTEM AROUND IT',
  'TRUST INFRASTRUCTURE',
  'MCP REQUEST :: anvil_validate_write',
  'Inter',
  'Dialog.Title',
  'Dialog.Description',
  'Dialog.Close',
];

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
  ...required.filter((value) => !content.includes(value)).map((value) => `missing: ${value}`),
  ...forbidden.filter((value) => content.includes(value)).map((value) => `retired: ${value}`),
  ...[...new Set(offPaletteColours)].map((value) => `off-palette: ${value}`),
  ...(shellLabelledMcpRequests.length > 0 ? ['MCP request presented as a shell command'] : []),
  ...composedComponents.flatMap((name) => {
    const importPattern = new RegExp(`import \\{ ${name} \\} from ['"]@/components/`);
    return [
      ...(!importPattern.test(pageContent) ? [`not imported by page: ${name}`] : []),
      ...(!pageContent.includes(`<${name} />`) ? [`not composed by page: ${name}`] : []),
    ];
  }),
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
