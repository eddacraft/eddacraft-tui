export type Severity = 'HIGH' | 'MEDIUM' | 'LOW';

export interface ProtectionRun {
  id: string;
  started: string;
  duration: string;
  result: 'Issues' | 'Clean';
  violations: number;
  newViolations: number;
  changedFiles: number;
}

export interface EvidenceLine {
  number: number;
  text: string;
  highlighted?: boolean;
}

export interface ProtectionWarning {
  id: string;
  severity: Severity;
  rule: string;
  category: string;
  file: string;
  line: number;
  age: string;
  evidence: boolean;
  explanation: string;
  matchedPattern: string;
  evidenceId: string;
  code: EvidenceLine[];
}

export const workspace = {
  name: 'anvil-001',
  root: '~/dev/anvil-001',
  refreshedAt: '2025-05-28 14:32:10',
  freshness: '2m 14s ago',
};

export const protectionRuns: ProtectionRun[] = [
  {
    id: 'run-143207',
    started: '2025-05-28 14:32:07',
    duration: '18.4s',
    result: 'Issues',
    violations: 12,
    newViolations: 3,
    changedFiles: 2,
  },
  {
    id: 'run-142145',
    started: '2025-05-28 14:21:45',
    duration: '17.1s',
    result: 'Issues',
    violations: 10,
    newViolations: 1,
    changedFiles: 1,
  },
  {
    id: 'run-141012',
    started: '2025-05-28 14:10:12',
    duration: '16.7s',
    result: 'Clean',
    violations: 0,
    newViolations: 0,
    changedFiles: 0,
  },
  {
    id: 'run-135833',
    started: '2025-05-28 13:58:33',
    duration: '17.0s',
    result: 'Issues',
    violations: 9,
    newViolations: 2,
    changedFiles: 0,
  },
  {
    id: 'run-134702',
    started: '2025-05-28 13:47:02',
    duration: '16.3s',
    result: 'Clean',
    violations: 0,
    newViolations: 0,
    changedFiles: 0,
  },
];

const genericCode = (line: number, file: string): EvidenceLine[] => [
  { number: line - 1, text: `// ${file}` },
  { number: line, text: 'const flaggedValue = readConfiguration();', highlighted: true },
  { number: line + 1, text: 'return flaggedValue;' },
];

export const protectionWarnings: ProtectionWarning[] = [
  {
    id: 'warning-api-key',
    severity: 'HIGH',
    rule: 'hardcoded-api-key',
    category: 'Secrets',
    file: 'src/services/payment/gateway.ts',
    line: 27,
    age: '2m 14s',
    evidence: true,
    explanation:
      'A string that matches a known API key format was detected. Hard-coded secrets can be committed and leaked. Use environment variables or a secrets manager.',
    matchedPattern: '(?i)\\b(pk_live|sk_live|rk_live)_[0-9a-zA-Z]{20,}\\b',
    evidenceId: 'anvil://evidence/8f2e3c7d-7b2a-4a1d-9c9a-2f3b6e9a1d55',
    code: [
      {
        number: 24,
        text: "const PAYMENTS_BASE = process.env.PAYMENTS_BASE || 'https://api.payments.example.com';",
      },
      { number: 25, text: 'const RETRY_LIMIT = 3;' },
      { number: 26, text: '' },
      {
        number: 27,
        text: "const API_KEY = 'pk_live_51H8x9Q2eZvKYlo2C0XYz4bAabcdef1234567890';",
        highlighted: true,
      },
      { number: 28, text: '' },
      { number: 29, text: 'export function charge(amount: number, token: string) {' },
      { number: 30, text: '  return fetch(`${PAYMENTS_BASE}/charge`, {' },
    ],
  },
  {
    id: 'warning-sql-injection',
    severity: 'HIGH',
    rule: 'sql-injection-risk',
    category: 'Injection',
    file: 'src/routes/users.ts',
    line: 112,
    age: '2m 14s',
    evidence: true,
    explanation:
      'User-controlled input is interpolated into a database query. Use a parameterised query at this boundary.',
    matchedPattern: 'query\\s*\\(`[^`]*\\$\\{[^}]+\\}',
    evidenceId: 'anvil://evidence/5f11f2bb-c72f-438a-847f-d85eb0dd8071',
    code: genericCode(112, 'src/routes/users.ts'),
  },
  {
    id: 'warning-weak-crypto',
    severity: 'MEDIUM',
    rule: 'weak-crypto-algorithm',
    category: 'Cryptography',
    file: 'src/utils/crypto.ts',
    line: 41,
    age: '4m 53s',
    evidence: true,
    explanation:
      'A legacy cryptographic algorithm was detected. Replace it with a supported modern algorithm.',
    matchedPattern: 'createHash\\([\'"]md5[\'"]\\)',
    evidenceId: 'anvil://evidence/b77d22d7-6a58-43de-aacc-10a44938f6c3',
    code: genericCode(41, 'src/utils/crypto.ts'),
  },
  {
    id: 'warning-input-validation',
    severity: 'MEDIUM',
    rule: 'missing-input-validation',
    category: 'Validation',
    file: 'src/controllers/user.controller.ts',
    line: 68,
    age: '4m 53s',
    evidence: false,
    explanation:
      'Request data reaches a privileged operation without an observable validation step at the boundary.',
    matchedPattern: 'No deterministic evidence captured',
    evidenceId: 'anvil://evidence/unavailable',
    code: genericCode(68, 'src/controllers/user.controller.ts'),
  },
  {
    id: 'warning-console-log',
    severity: 'LOW',
    rule: 'console-log',
    category: 'Code Quality',
    file: 'src/services/logger.ts',
    line: 15,
    age: '9m 01s',
    evidence: true,
    explanation:
      'A direct console call bypasses the workspace logger and may expose uncontrolled output.',
    matchedPattern: 'console\\.(log|debug|info)\\(',
    evidenceId: 'anvil://evidence/41bbb864-66e3-4192-b21b-b19d7f29f5d0',
    code: genericCode(15, 'src/services/logger.ts'),
  },
  {
    id: 'warning-todo-comment',
    severity: 'LOW',
    rule: 'todo-comment',
    category: 'Code Quality',
    file: 'src/routes/orders.ts',
    line: 201,
    age: '9m 01s',
    evidence: false,
    explanation:
      'A deferred-work comment is present in runtime code and is not tracked by the owning system.',
    matchedPattern: 'No deterministic evidence captured',
    evidenceId: 'anvil://evidence/unavailable',
    code: genericCode(201, 'src/routes/orders.ts'),
  },
];

export const latestRun = protectionRuns[0];
export const nextAttention = protectionWarnings[0];
