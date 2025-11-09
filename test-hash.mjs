import { generateHash } from './core/dist/crypto/hash.js';

const plan = {
  schema_version: '0.1.0',
  id: 'aps-1ef574b9',
  intent: 'fix-the-init-process',
  proposed_changes: [],
  provenance: {
    timestamp: '2025-11-09T15:24:28.140Z',
    author: 'aneki',
    source: 'cli',
    version: '0.0.0',
    repository: '/home/aneki/anvil-001',
    branch: 'main',
    commit: '',
  },
  validations: {
    required_checks: ['lint', 'test', 'coverage', 'secrets'],
    skip_checks: [],
  },
  evidence: [],
  executions: [],
};

const computedHash = generateHash(plan);
const storedHash = '421ee97d38a45f9f721841f7ffa472485b257ad596d6848d8c5f749cde4a4a58';

console.log('Computed hash (without hash field):', computedHash);
console.log('Stored hash:                       ', storedHash);
console.log('Match:', computedHash === storedHash);
