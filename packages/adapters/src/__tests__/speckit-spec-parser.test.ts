import { describe, it, expect } from 'vitest';
import { readFile } from 'node:fs/promises';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { SpecParser } from '../speckit/parsers/spec-parser.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const fixturesDir = join(__dirname, 'fixtures/speckit-official/auth-feature');

describe('SpecParser - Official Spec-Kit Format', () => {
  let parser: SpecParser;

  beforeEach(() => {
    parser = new SpecParser();
  });

  it('should parse spec.md metadata', async () => {
    const content = await readFile(join(fixturesDir, 'spec.md'), 'utf-8');
    const parsed = parser.parseSpec(content);

    expect(parsed.metadata.feature).toBe('User Authentication System');
    expect(parsed.metadata.branch).toBe('feature/001-auth-system');
    expect(parsed.metadata.date).toBe('2025-01-15');
    expect(parsed.metadata.status).toBe('Draft');
  });

  it('should parse user scenarios with priorities', async () => {
    const content = await readFile(join(fixturesDir, 'spec.md'), 'utf-8');
    const parsed = parser.parseSpec(content);

    expect(parsed.userScenarios).toHaveLength(4);

    const registration = parsed.userScenarios[0];
    expect(registration.priority).toBe('P1');
    expect(registration.title).toBe('User Registration');
    expect(registration.asA).toContain('new user');
    expect(registration.iWantTo).toContain('create an account');
    expect(registration.soThat).toContain('access the application securely');
    expect(registration.acceptanceScenarios.length).toBeGreaterThan(0);
    expect(registration.edgeCases.length).toBeGreaterThan(0);
  });

  it('should parse functional requirements', async () => {
    const content = await readFile(join(fixturesDir, 'spec.md'), 'utf-8');
    const parsed = parser.parseSpec(content);

    expect(parsed.requirements.functional.length).toBeGreaterThan(0);

    const fr001 = parsed.requirements.functional.find((r) => r.code === 'FR-001');
    expect(fr001).toBeDefined();
    expect(fr001?.description).toContain('RFC 5322');
    expect(fr001?.needsClarification).toBe(false);
  });

  it('should identify requirements needing clarification', async () => {
    const content = await readFile(join(fixturesDir, 'spec.md'), 'utf-8');
    const parsed = parser.parseSpec(content);

    const clarificationReqs = parsed.requirements.functional.filter((r) => r.needsClarification);
    expect(clarificationReqs.length).toBeGreaterThan(0);

    const fr008 = parsed.requirements.functional.find((r) => r.code === 'FR-008');
    expect(fr008?.needsClarification).toBe(true);
    expect(fr008?.clarificationQuestion).toContain('Token expiration time');
  });

  it('should parse key entities', async () => {
    const content = await readFile(join(fixturesDir, 'spec.md'), 'utf-8');
    const parsed = parser.parseSpec(content);

    expect(parsed.requirements.entities.length).toBeGreaterThan(0);

    const userEntity = parsed.requirements.entities.find((e) => e.name === 'User');
    expect(userEntity).toBeDefined();
    expect(userEntity?.represents).toContain('person with access');
    expect(userEntity?.keyAttributes).toContain('email');
    expect(userEntity?.relationships.length).toBeGreaterThan(0);
  });

  it('should parse success criteria', async () => {
    const content = await readFile(join(fixturesDir, 'spec.md'), 'utf-8');
    const parsed = parser.parseSpec(content);

    expect(parsed.successCriteria.quantitative.length).toBeGreaterThan(0);
    expect(parsed.successCriteria.qualitative.length).toBeGreaterThan(0);
    expect(parsed.successCriteria.security).toBeDefined();
    expect(parsed.successCriteria.security!.length).toBeGreaterThan(0);
  });

  it('should extract all clarification markers', async () => {
    const content = await readFile(join(fixturesDir, 'spec.md'), 'utf-8');
    const parsed = parser.parseSpec(content);

    expect(parsed.clarifications.length).toBeGreaterThan(0);
    expect(parsed.clarifications.some((c) => c.includes('remember me'))).toBe(true);
    expect(parsed.clarifications.some((c) => c.includes('token expiration'))).toBe(true);
  });

  it('should handle edge cases in user scenarios', async () => {
    const content = await readFile(join(fixturesDir, 'spec.md'), 'utf-8');
    const parsed = parser.parseSpec(content);

    const login = parsed.userScenarios.find((s) => s.title === 'User Login');
    expect(login).toBeDefined();
    expect(login?.edgeCases.length).toBeGreaterThan(0);
    expect(login?.edgeCases.some((e) => e.includes('Invalid credentials'))).toBe(true);
  });

  it('should parse P2 and P3 scenarios', async () => {
    const content = await readFile(join(fixturesDir, 'spec.md'), 'utf-8');
    const parsed = parser.parseSpec(content);

    const p2Scenarios = parsed.userScenarios.filter((s) => s.priority === 'P2');
    const p3Scenarios = parsed.userScenarios.filter((s) => s.priority === 'P3');

    expect(p2Scenarios.length).toBeGreaterThan(0);
    expect(p3Scenarios.length).toBeGreaterThan(0);
  });
});
