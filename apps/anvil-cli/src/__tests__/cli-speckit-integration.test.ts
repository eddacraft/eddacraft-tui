/**
 * CLI Integration Tests for SpecKit Format
 *
 * Tests end-to-end workflows with SpecKit documents:
 * - Format detection and validation
 * - Gate execution with evidence injection
 * - Export/import roundtrip fidelity
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { readFile, writeFile, mkdir, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { PlanLoader } from '../services/plan-loader.js';
import { EvidenceWriter } from '../services/evidence-writer.js';
import { SpecKitFormatAdapter } from '@eddacraft/anvil-adapters';
import type { GateRunResult } from '@eddacraft/anvil-core';

describe('CLI SpecKit Integration', () => {
  let testDir: string;
  let specKitFixture: string;

  beforeEach(async () => {
    // Create temp directory for tests
    testDir = join(tmpdir(), `anvil-test-${Date.now()}`);
    await mkdir(testDir, { recursive: true });

    // Create a sample SpecKit document
    specKitFixture = `# Specification

## Intent

Implement user authentication with JWT tokens

## Overview

This specification outlines a JWT-based authentication system.

## Goals

- Implement secure user authentication
- Add middleware for protecting routes
- Support token refresh mechanism

## Requirements

- Node.js 18+ runtime
- Express.js web framework
- jsonwebtoken library

## Changes

### Files to Create

#### Create \`src/auth/controller.ts\`

Authentication controller for handling login/register endpoints.

\`\`\`typescript
export class AuthController {
  async login(req, res) {
    // Login implementation
  }
}
\`\`\`

### Files to Update

#### Update \`src/app.ts\`

Add authentication routes to Express app.
`;
  });

  afterEach(async () => {
    // Clean up test directory
    try {
      await rm(testDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors
    }
  });

  describe('Format Detection and Validation', () => {
    it('should detect SpecKit format with high confidence', async () => {
      const filePath = join(testDir, 'spec.md');
      await writeFile(filePath, specKitFixture, 'utf-8');

      const planLoader = new PlanLoader();
      const result = await planLoader.loadPlan(filePath);

      expect(result.sourceFormat).toBeDefined();
      expect(result.sourceFormat?.format).toBe('speckit');
      expect(result.sourceFormat?.confidence).toBeGreaterThanOrEqual(50);
      expect(result.validation.valid).toBe(true);
    });

    it('should parse SpecKit document to valid APS plan', async () => {
      const filePath = join(testDir, 'spec.md');
      await writeFile(filePath, specKitFixture, 'utf-8');

      const planLoader = new PlanLoader();
      const result = await planLoader.loadPlan(filePath);

      expect(result.plan).toBeDefined();
      expect(result.plan.id).toMatch(/^aps-[a-f0-9]{8}$/);
      expect(result.plan.intent).toContain('authentication');
      expect(result.plan.schema_version).toBe('0.1.0');
      expect(result.plan.proposed_changes.length).toBeGreaterThan(0);
    });

    it('should extract metadata from SpecKit sections', async () => {
      const filePath = join(testDir, 'spec.md');
      await writeFile(filePath, specKitFixture, 'utf-8');

      const planLoader = new PlanLoader();
      const result = await planLoader.loadPlan(filePath);

      expect(result.plan.metadata).toBeDefined();
      expect(result.plan.metadata?.source_format).toBe('speckit');
      expect(result.plan.metadata?.overview).toContain('JWT-based');
      expect(result.plan.metadata?.goals).toBeDefined();
      expect(Array.isArray(result.plan.metadata?.goals)).toBe(true);
    });

    it('should handle explicit format specification', async () => {
      const filePath = join(testDir, 'document.txt');
      await writeFile(filePath, specKitFixture, 'utf-8');

      const planLoader = new PlanLoader();
      const result = await planLoader.loadPlan(filePath, {
        format: 'speckit',
      });

      expect(result.sourceFormat?.format).toBe('speckit');
      expect(result.validation.valid).toBe(true);
    });

    it('should fail gracefully on invalid SpecKit content', async () => {
      const invalidContent = '# Random Document\n\nSome random content';
      const filePath = join(testDir, 'invalid.md');
      await writeFile(filePath, invalidContent, 'utf-8');

      const planLoader = new PlanLoader();

      await expect(planLoader.loadPlan(filePath)).rejects.toThrow();
    });
  });

  describe('Evidence Injection', () => {
    it('should inject gate evidence into SpecKit document', async () => {
      const filePath = join(testDir, 'spec.md');
      await writeFile(filePath, specKitFixture, 'utf-8');

      // Load plan
      const planLoader = new PlanLoader();
      const loadResult = await planLoader.loadPlan(filePath);

      // Create mock gate results
      const gateResults: GateRunResult = {
        overall: true,
        score: 95.5,
        checks: [
          {
            check: 'eslint',
            passed: true,
            message: 'No linting errors found',
            details: { errors: 0, warnings: 0 },
          },
          {
            check: 'vitest',
            passed: true,
            message: 'All tests passed',
            details: { total: 42, passed: 42, failed: 0 },
          },
          {
            check: 'coverage',
            passed: true,
            score: 85,
            message: 'Coverage threshold met',
            details: { line: 85, branch: 80, function: 90 },
          },
        ],
        summary: {
          total: 3,
          passed: 3,
          failed: 0,
          skipped: 0,
        },
      };

      // Inject evidence
      const evidenceWriter = new EvidenceWriter();
      const writeResult = await evidenceWriter.writeEvidence({
        format: 'speckit',
        filePath,
        gateResults,
        plan: loadResult.plan,
        mode: 'replace',
      });

      expect(writeResult.success).toBe(true);
      expect(writeResult.filePath).toBe(filePath);
      expect(writeResult.evidence).toBeDefined();

      // Verify evidence was added to file
      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).toContain('## Gate Evidence');
      expect(updatedContent).toContain('✅ PASSED');
      expect(updatedContent).toContain('95.5%');
      expect(updatedContent).toContain('eslint');
      expect(updatedContent).toContain('vitest');
      expect(updatedContent).toContain('coverage');
    });

    it('should preserve original content when injecting evidence', async () => {
      const filePath = join(testDir, 'spec.md');
      await writeFile(filePath, specKitFixture, 'utf-8');

      const planLoader = new PlanLoader();
      const loadResult = await planLoader.loadPlan(filePath);

      const gateResults: GateRunResult = {
        overall: true,
        score: 100,
        checks: [
          {
            check: 'test',
            passed: true,
            message: 'OK',
          },
        ],
        summary: { total: 1, passed: 1, failed: 0, skipped: 0 },
      };

      const evidenceWriter = new EvidenceWriter();
      await evidenceWriter.writeEvidence({
        format: 'speckit',
        filePath,
        gateResults,
        plan: loadResult.plan,
      });

      const updatedContent = await readFile(filePath, 'utf-8');

      // Verify original sections are preserved
      expect(updatedContent).toContain('## Intent');
      expect(updatedContent).toContain('user authentication');
      expect(updatedContent).toContain('## Changes');
      expect(updatedContent).toContain('AuthController');
    });

    it('should handle failed gate results', async () => {
      const filePath = join(testDir, 'spec.md');
      await writeFile(filePath, specKitFixture, 'utf-8');

      const planLoader = new PlanLoader();
      const loadResult = await planLoader.loadPlan(filePath);

      const gateResults: GateRunResult = {
        overall: false,
        score: 45,
        checks: [
          {
            check: 'eslint',
            passed: false,
            message: 'Linting errors found',
            details: { errors: 5, warnings: 10 },
          },
          {
            check: 'vitest',
            passed: true,
            message: 'Tests passed',
          },
        ],
        summary: { total: 2, passed: 1, failed: 1, skipped: 0 },
      };

      const evidenceWriter = new EvidenceWriter();
      const writeResult = await evidenceWriter.writeEvidence({
        format: 'speckit',
        filePath,
        gateResults,
        plan: loadResult.plan,
      });

      expect(writeResult.success).toBe(true);

      const updatedContent = await readFile(filePath, 'utf-8');
      expect(updatedContent).toContain('❌ FAILED');
      expect(updatedContent).toContain('45.0%');
    });

    it('should support append mode for multiple gate runs', async () => {
      const filePath = join(testDir, 'spec.md');
      await writeFile(filePath, specKitFixture, 'utf-8');

      const planLoader = new PlanLoader();
      const loadResult = await planLoader.loadPlan(filePath);

      const gateResults: GateRunResult = {
        overall: true,
        score: 90,
        checks: [{ check: 'test1', passed: true, message: 'OK' }],
        summary: { total: 1, passed: 1, failed: 0, skipped: 0 },
      };

      const evidenceWriter = new EvidenceWriter();

      // First run - replace
      await evidenceWriter.writeEvidence({
        format: 'speckit',
        filePath,
        gateResults,
        plan: loadResult.plan,
        mode: 'replace',
      });

      // Second run - append
      await evidenceWriter.writeEvidence({
        format: 'speckit',
        filePath,
        gateResults: { ...gateResults, score: 95 },
        plan: loadResult.plan,
        mode: 'append',
      });

      const updatedContent = await readFile(filePath, 'utf-8');

      // Should contain evidence section with multiple runs
      expect(updatedContent).toContain('## Gate Evidence');
      expect((updatedContent.match(/### Run:/g) || []).length).toBeGreaterThan(0);
    });
  });

  describe('Roundtrip Fidelity', () => {
    it('should maintain content integrity in SpecKit → APS → SpecKit roundtrip', async () => {
      const filePath = join(testDir, 'spec.md');
      await writeFile(filePath, specKitFixture, 'utf-8');

      // Parse SpecKit → APS
      const adapter = new SpecKitFormatAdapter();
      const parseResult = await adapter.parse(specKitFixture);

      expect(parseResult.success).toBe(true);
      expect(parseResult.data).toBeDefined();

      // Serialize APS → SpecKit
      const serializeResult = await adapter.serialize(parseResult.data!);

      expect(serializeResult.success).toBe(true);
      expect(serializeResult.content).toBeDefined();

      // Parse again to verify
      const parseResult2 = await adapter.parse(serializeResult.content!);

      expect(parseResult2.success).toBe(true);
      expect(parseResult2.data?.intent).toBe(parseResult.data?.intent);
      expect(parseResult2.data?.proposed_changes.length).toBe(
        parseResult.data?.proposed_changes.length
      );
    });

    it('should preserve metadata through roundtrip', async () => {
      const adapter = new SpecKitFormatAdapter();

      // Parse
      const parseResult = await adapter.parse(specKitFixture);
      expect(parseResult.success).toBe(true);

      const originalMetadata = parseResult.data?.metadata;

      // Serialize
      const serializeResult = await adapter.serialize(parseResult.data!);
      expect(serializeResult.success).toBe(true);

      // Parse again
      const parseResult2 = await adapter.parse(serializeResult.content!);

      expect(parseResult2.data?.metadata?.overview).toBe(originalMetadata?.overview);
      expect(parseResult2.data?.metadata?.goals).toEqual(originalMetadata?.goals);
      expect(parseResult2.data?.metadata?.requirements).toEqual(originalMetadata?.requirements);
    });

    it('should handle changes correctly through roundtrip', async () => {
      const adapter = new SpecKitFormatAdapter();

      const parseResult = await adapter.parse(specKitFixture);
      expect(parseResult.success).toBe(true);

      const originalChanges = parseResult.data?.proposed_changes;
      expect(originalChanges).toBeDefined();
      expect(originalChanges!.length).toBeGreaterThan(0);

      const serializeResult = await adapter.serialize(parseResult.data!);
      expect(serializeResult.success).toBe(true);

      const parseResult2 = await adapter.parse(serializeResult.content!);
      const roundtripChanges = parseResult2.data?.proposed_changes;

      expect(roundtripChanges?.length).toBe(originalChanges?.length);

      // Verify each change is preserved (allowing minor formatting differences)
      for (let i = 0; i < originalChanges!.length; i++) {
        expect(roundtripChanges![i].type).toBe(originalChanges![i].type);
        expect(roundtripChanges![i].path).toBe(originalChanges![i].path);
        // Description should be semantically equivalent (allowing formatting variations)
        expect(roundtripChanges![i].description).toBeTruthy();
        expect(roundtripChanges![i].description).toContain(
          originalChanges![i].path.replace(/`/g, '')
        );
      }
    });
  });

  describe('Error Handling', () => {
    it('should provide helpful error for non-existent file', async () => {
      const planLoader = new PlanLoader();
      const nonExistentPath = join(testDir, 'does-not-exist.md');

      await expect(planLoader.loadPlan(nonExistentPath)).rejects.toThrow(/Failed to load plan/);
    });

    it('should handle corrupted SpecKit document', async () => {
      const corruptedContent = `# Specification

## Intent

Missing required content...

## Changes

Incomplete change description
`;

      const filePath = join(testDir, 'corrupted.md');
      await writeFile(filePath, corruptedContent, 'utf-8');

      const planLoader = new PlanLoader();
      const result = await planLoader.loadPlan(filePath);

      // Should still parse but may have warnings
      expect(result.plan).toBeDefined();
      expect(result.validation.valid).toBe(true);
    });

    it('should reject evidence injection for unsupported format', async () => {
      const evidenceWriter = new EvidenceWriter();

      const gateResults: GateRunResult = {
        overall: true,
        score: 100,
        checks: [],
        summary: { total: 0, passed: 0, failed: 0, skipped: 0 },
      };

      const writeResult = await evidenceWriter.writeEvidence({
        format: 'unsupported-format',
        filePath: '/tmp/test.txt',
        gateResults,
        plan: {} as any,
      });

      expect(writeResult.success).toBe(false);
      expect(writeResult.error).toContain('not supported');
    });
  });
});
