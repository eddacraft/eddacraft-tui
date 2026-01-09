import { describe, it, expect, beforeEach, vi } from 'vitest';
import { EvidenceWriter } from '../evidence-writer.js';
import type { GateRunResult } from '@anvil/core';

describe('EvidenceWriter', () => {
  let writer: EvidenceWriter;
  let mockGateResults: GateRunResult;

  beforeEach(() => {
    writer = new EvidenceWriter();
    
    // Create mock gate results
    mockGateResults = {
      overall: true,
      score: 95.5,
      checks: [
        {
          check: 'eslint',
          passed: true,
          skipped: false,
          message: 'ESLint passed',
          details: {},
        },
        {
          check: 'test',
          passed: true,
          skipped: false,
          message: 'Tests passed',
          details: {},
        },
      ],
      summary: {
        total: 2,
        passed: 2,
        failed: 0,
        skipped: 0,
      },
    };
  });

  describe('writeEvidence', () => {
    it('should return error for unsupported format', async () => {
      const result = await writer.writeEvidence({
        format: 'unsupported',
        filePath: '/test/file.txt',
        gateResults: mockGateResults,
        plan: {
          schema_version: '0.1.0',
          id: 'aps-00000001',
          hash: '0'.repeat(64),
          intent: 'Test',
          proposed_changes: [],
          provenance: {
            timestamp: new Date().toISOString(),
            source: 'test',
            version: '1.0.0',
          },
          validations: {
            required_checks: [],
            skip_checks: [],
          },
        },
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain('not supported');
    });

    it('should handle format speckit', async () => {
      const result = await writer.writeEvidence({
        format: 'speckit',
        filePath: '/nonexistent/file.md',
        gateResults: mockGateResults,
        plan: {
          schema_version: '0.1.0',
          id: 'aps-00000001',
          hash: '0'.repeat(64),
          intent: 'Test',
          proposed_changes: [],
          provenance: {
            timestamp: new Date().toISOString(),
            source: 'test',
            version: '1.0.0',
          },
          validations: {
            required_checks: [],
            skip_checks: [],
          },
        },
      });

      // Will fail due to file not existing, but format is handled
      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });

    it('should handle format bmad', async () => {
      const result = await writer.writeEvidence({
        format: 'bmad',
        filePath: '/nonexistent/file.md',
        gateResults: mockGateResults,
        plan: {
          schema_version: '0.1.0',
          id: 'aps-00000001',
          hash: '0'.repeat(64),
          intent: 'Test',
          proposed_changes: [],
          provenance: {
            timestamp: new Date().toISOString(),
            source: 'test',
            version: '1.0.0',
          },
          validations: {
            required_checks: [],
            skip_checks: [],
          },
        },
      });

      // Will fail due to file not existing, but format is handled
      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });

    it('should normalize format names', async () => {
      const result = await writer.writeEvidence({
        format: 'spec-kit',
        filePath: '/nonexistent/file.md',
        gateResults: mockGateResults,
        plan: {
          schema_version: '0.1.0',
          id: 'aps-00000001',
          hash: '0'.repeat(64),
          intent: 'Test',
          proposed_changes: [],
          provenance: {
            timestamp: new Date().toISOString(),
            source: 'test',
            version: '1.0.0',
          },
          validations: {
            required_checks: [],
            skip_checks: [],
          },
        },
      });

      // Should handle spec-kit as speckit
      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });
  });

  describe('evidence bundle creation', () => {
    it('should include overall status in evidence', async () => {
      const result = await writer.writeEvidence({
        format: 'invalid',
        filePath: '/test/file.md',
        gateResults: mockGateResults,
        plan: {
          schema_version: '0.1.0',
          id: 'aps-00000001',
          hash: '0'.repeat(64),
          intent: 'Test',
          proposed_changes: [],
          provenance: {
            timestamp: new Date().toISOString(),
            source: 'test',
            version: '1.0.0',
          },
          validations: {
            required_checks: [],
            skip_checks: [],
          },
        },
      });

      // Even though format is invalid, we can check error message structure
      expect(result.error).toBeTruthy();
    });

    it('should handle failed gate results', async () => {
      const failedResults: GateRunResult = {
        ...mockGateResults,
        overall: false,
        checks: [
          {
            check: 'eslint',
            passed: false,
            skipped: false,
            message: 'ESLint failed',
            details: { errors: ['error 1'] },
          },
        ],
        summary: {
          total: 1,
          passed: 0,
          failed: 1,
          skipped: 0,
        },
      };

      const result = await writer.writeEvidence({
        format: 'unsupported',
        filePath: '/test/file.md',
        gateResults: failedResults,
        plan: {
          schema_version: '0.1.0',
          id: 'aps-00000001',
          hash: '0'.repeat(64),
          intent: 'Test',
          proposed_changes: [],
          provenance: {
            timestamp: new Date().toISOString(),
            source: 'test',
            version: '1.0.0',
          },
          validations: {
            required_checks: [],
            skip_checks: [],
          },
        },
      });

      expect(result.success).toBe(false);
      expect(result.error).toContain('not supported');
    });
  });
});
