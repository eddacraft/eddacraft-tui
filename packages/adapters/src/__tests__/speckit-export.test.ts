import { describe, it, expect, beforeEach } from 'vitest';
import { SpecKitExportAdapter } from '../speckit/export.js';
import type { APSPlan } from '@eddacraft/anvil-core';

describe('SpecKitExportAdapter', () => {
  let adapter: SpecKitExportAdapter;
  let sampleAPSPlan: APSPlan;

  beforeEach(() => {
    adapter = new SpecKitExportAdapter();

    sampleAPSPlan = {
      id: 'aps-12345678',
      hash: '0'.repeat(64),
      intent: 'Implement a user authentication system with JWT tokens to secure API endpoints',
      schema_version: '0.1.0',
      proposed_changes: [
        {
          type: 'file_create',
          path: 'src/auth/controller.ts',
          description: 'Create authentication controller',
          content: 'export class AuthController { /* ... */ }',
        },
        {
          type: 'file_update',
          path: 'src/app.ts',
          description: 'Update main app to include auth routes',
        },
        {
          type: 'dependency_add',
          path: 'package.json',
          description: 'Add jsonwebtoken dependency',
        },
      ],
      provenance: {
        timestamp: '2024-01-15T10:00:00Z',
        source: 'cli',
        version: '1.0.0',
        author: 'test@example.com',
      },
      validations: {
        required_checks: ['lint', 'test'],
        skip_checks: [],
      },
      metadata: {
        goals: ['Secure API endpoints', 'Implement JWT authentication'],
        requirements: ['Node.js 18+', 'Express.js'],
        overview: 'JWT-based authentication system',
      },
    };
  });

  describe('basic properties', () => {
    it('should have correct name and version', () => {
      expect(adapter.name).toBe('speckit-export');
      expect(adapter.version).toBe('1.0.0');
    });

    it('should support speckit export formats', () => {
      expect(adapter.canExport('speckit')).toBe(true);
      expect(adapter.canExport('spec.md')).toBe(true);
      expect(adapter.canExport('unknown')).toBe(false);
    });
  });

  describe('convertFromAPS', () => {
    it('should convert APS to SpecKit format', async () => {
      const result = await adapter.convertFromAPS(sampleAPSPlan);

      expect(result.success).toBe(true);
      expect(result.data).toBeDefined();

      if (result.success && result.data) {
        expect(result.data.format).toBe('speckit');
        expect(result.data.version).toBe('1.0.0');

        const content = result.data.content as any;
        expect(content.specContent).toBeDefined();
        expect(content.planContent).toBeDefined();
        expect(content.tasksContent).toBeDefined();

        // Check spec content includes intent
        expect(content.specContent).toContain('## Intent');
        expect(content.specContent).toContain(sampleAPSPlan.intent);

        // Check goals and requirements are included
        expect(content.specContent).toContain('## Goals');
        expect(content.specContent).toContain('Secure API endpoints');

        // Check changes are properly formatted
        expect(content.specContent).toContain('## Changes');
        expect(content.specContent).toContain('### Files to Create');
        expect(content.specContent).toContain('src/auth/controller.ts');
      }
    });

    it('should generate valid markdown for spec.md', async () => {
      const result = await adapter.convertFromAPS(sampleAPSPlan);

      expect(result.success).toBe(true);
      const content = result.data?.content as any;
      const specMarkdown = content.specContent;

      // Check markdown structure
      expect(specMarkdown).toMatch(/^# Specification/);
      expect(specMarkdown).toContain('## Intent');
      expect(specMarkdown).toContain('## Goals');
      expect(specMarkdown).toContain('## Requirements');
      expect(specMarkdown).toContain('## Changes');

      // Check code blocks are properly formatted
      expect(specMarkdown).toMatch(/```\w+\n[\s\S]*?```/);
    });

    it('should generate valid plan.md', async () => {
      const result = await adapter.convertFromAPS(sampleAPSPlan);

      expect(result.success).toBe(true);
      const content = result.data?.content as any;
      const planMarkdown = content.planContent;

      // Check plan structure
      expect(planMarkdown).toContain('# Implementation Plan');
      expect(planMarkdown).toContain(`Generated from APS: ${sampleAPSPlan.id}`);
      expect(planMarkdown).toContain('## Summary');
      expect(planMarkdown).toContain('## Implementation Steps');

      // Check steps are numbered
      expect(planMarkdown).toMatch(/1\. \*\*.+\*\*/);
      expect(planMarkdown).toMatch(/2\. \*\*.+\*\*/);
    });

    it('should generate valid tasks.md', async () => {
      const result = await adapter.convertFromAPS(sampleAPSPlan);

      expect(result.success).toBe(true);
      const content = result.data?.content as any;
      const tasksMarkdown = content.tasksContent;

      // Check tasks structure
      expect(tasksMarkdown).toContain('# Tasks');
      expect(tasksMarkdown).toContain(`Generated from APS: ${sampleAPSPlan.id}`);
      expect(tasksMarkdown).toContain('## Task List');
      expect(tasksMarkdown).toContain('## Progress');

      // Check task formatting
      expect(tasksMarkdown).toMatch(/- \[ \] ⏳ .+/);

      // Check progress calculation
      expect(tasksMarkdown).toContain('Total tasks:');
      expect(tasksMarkdown).toContain('Progress: 0%');
    });

    it('should handle APS with execution history', async () => {
      const planWithExecutions: APSPlan = {
        ...sampleAPSPlan,
        executions: [
          {
            operation: 'apply' as const,
            timestamp: '2024-01-16T12:00:00Z',
            status: 'success',
            executed_by: 'ci-bot',
            changes_applied: ['change-1', 'change-2'],
          },
          {
            operation: 'apply' as const,
            timestamp: '2024-01-16T13:00:00Z',
            status: 'failed',
            executed_by: 'developer@example.com',
            logs: ['Error: Build failed'],
          },
        ],
      };

      const result = await adapter.convertFromAPS(planWithExecutions);

      expect(result.success).toBe(true);
      const content = result.data?.content as any;
      const tasksMarkdown = content.tasksContent;

      expect(tasksMarkdown).toContain('## Execution History');
      expect(tasksMarkdown).toContain('Status: success');
      expect(tasksMarkdown).toContain('Status: failed');
      expect(tasksMarkdown).toContain('Error: Build failed');
    });

    it('should preserve metadata in export', async () => {
      const result = await adapter.convertFromAPS(sampleAPSPlan);

      expect(result.success).toBe(true);
      expect(result.data?.metadata).toBeDefined();
      expect(result.data?.metadata?.aps_id).toBe(sampleAPSPlan.id);
      expect(result.data?.metadata?.aps_hash).toBe(sampleAPSPlan.hash);
      expect(result.data?.metadata?.generator).toBe('speckit-export');
    });
  });

  describe('validateSpec', () => {
    it('should validate specs before export', async () => {
      const invalidSpec: APSPlan = {
        ...sampleAPSPlan,
        intent: 'Short', // Too short
      };

      const result = await adapter.validateSpec(invalidSpec);

      expect(result.valid).toBe(false);
      expect(result.issues).toBeDefined();
      expect(result.issues?.[0].path).toBe('intent');
      expect(result.issues?.[0].severity).toBe('error');
    });

    it('should warn about empty changes', async () => {
      const emptySpec: APSPlan = {
        ...sampleAPSPlan,
        proposed_changes: [],
      };

      const result = await adapter.validateSpec(emptySpec);

      expect(result.valid).toBe(true);
      expect(result.issues).toBeDefined();
      expect(result.issues?.[0].severity).toBe('warning');
      expect(result.issues?.[0].message).toContain('No changes');
    });
  });
});
