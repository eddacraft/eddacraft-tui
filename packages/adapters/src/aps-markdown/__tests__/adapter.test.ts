/**
 * APSMarkdownAdapter Tests
 * Tests for format detection and parsing of APS markdown documents
 */

import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect } from 'vitest';
import { APSMarkdownAdapter } from '../adapter.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

describe('APSMarkdownAdapter', () => {
  const adapter = new APSMarkdownAdapter();

  describe('metadata', () => {
    it('has correct metadata', () => {
      expect(adapter.metadata.name).toBe('aps-markdown');
      expect(adapter.metadata.extensions).toContain('.aps.md');
      expect(adapter.metadata.formats).toContain('aps');
    });
  });

  describe('detect', () => {
    it('detects .aps.md content with Tasks section', () => {
      const content = `# Feature Plan

**Scope:** AUTH **Owner:** @alice

## Tasks

### AUTH-001: Implement login

**Intent:** Create login endpoint
`;
      const result = adapter.detect(content);
      expect(result.detected).toBe(true);
      expect(result.confidence).toBeGreaterThanOrEqual(80);
    });

    it('detects index file with Modules section', () => {
      const content = `# Project Plan

## Modules

### auth

- **Path:** [./modules/auth.aps.md](./modules/auth.aps.md)
- **Scope:** AUTH
`;
      const result = adapter.detect(content);
      expect(result.detected).toBe(true);
      // modules(20) + aps-link(25) + scope(10) = 55
      expect(result.confidence).toBeGreaterThanOrEqual(55);
    });

    it('does not detect regular markdown', () => {
      const content = `# README

This is a regular readme file.

## Installation

Run npm install.
`;
      const result = adapter.detect(content);
      expect(result.detected).toBe(false);
    });

    it('does not detect SpecKit format', () => {
      const content = `# Feature: User Login

## User Story
As a user I want to login

## Acceptance Criteria
- Given valid credentials
- When I submit login form
- Then I am authenticated
`;
      const result = adapter.detect(content);
      expect(result.detected).toBe(false);
    });
  });

  describe('canImport / canExport', () => {
    it('should support importing aps format', () => {
      expect(adapter.canImport('aps')).toBe(true);
      expect(adapter.canImport('aps-markdown')).toBe(true);
      expect(adapter.canImport('.aps.md')).toBe(true);
    });

    it('should support exporting to aps format', () => {
      expect(adapter.canExport('aps')).toBe(true);
      expect(adapter.canExport('.aps.md')).toBe(true);
    });

    it('should not support unknown formats', () => {
      expect(adapter.canImport('speckit')).toBe(false);
      expect(adapter.canImport('bmad')).toBe(false);
    });
  });

  describe('confidence scoring', () => {
    it('has high confidence (90+) for leaf spec with SCOPE-NNN task and Intent', () => {
      const content = `# Authentication Module

**Scope:** AUTH **Owner:** @alice **Priority:** high

> Handles user authentication and session management.

## Tasks

### AUTH-001: Implement login endpoint

**Intent:** Create POST /auth/login endpoint with JWT response
**Confidence:** high
**Expected Outcome:** Returns JWT token on success, 401 on failure
**Tags:** security, api

### AUTH-002: Add password reset

**Intent:** Implement password reset flow with email verification
**Confidence:** medium
**Dependencies:** AUTH-001
`;
      const result = adapter.detect(content);
      expect(result.detected).toBe(true);
      expect(result.confidence).toBeGreaterThanOrEqual(90);
    });

    it('has medium-high confidence (70+) for index with Modules and .aps.md paths', () => {
      const content = `# Project Plan

> A multi-module project plan.

## Modules

### auth

- **Path:** [./modules/auth.aps.md](./modules/auth.aps.md)
- **Scope:** AUTH
- **Owner:** @alice
- **Priority:** high

### payments

- **Path:** [./modules/payments.aps.md](./modules/payments.aps.md)
- **Scope:** PAY
- **Owner:** @bob
- **Priority:** medium
- **Dependencies:** auth
`;
      const result = adapter.detect(content);
      expect(result.detected).toBe(true);
      // modules(20) + aps-links(25+5) + scope(10) + owner(5) + priority(5) = 70
      expect(result.confidence).toBeGreaterThanOrEqual(70);
    });

    it('has low confidence for markdown without APS markers', () => {
      const content = `# Some Document

This has a header but no APS-specific content.

## Section One

Just regular text here.
`;
      const result = adapter.detect(content);
      expect(result.confidence).toBeLessThan(50);
      expect(result.detected).toBe(false);
    });

    it('has partial confidence for partial APS markers', () => {
      const content = `# Feature Plan

**Scope:** TEST

## Tasks

Some tasks without proper formatting.
`;
      const result = adapter.detect(content);
      // Has scope(10) and Tasks section(15) but no SCOPE-NNN pattern = 25
      expect(result.confidence).toBeGreaterThanOrEqual(25);
      expect(result.confidence).toBeLessThan(80);
    });
  });

  describe('detection reasons', () => {
    it('provides reason for leaf spec detection', () => {
      const content = `# Feature

**Scope:** AUTH

## Tasks

### AUTH-001: Task one

**Intent:** Do something
`;
      const result = adapter.detect(content);
      expect(result.reason).toBeDefined();
      expect(result.reason).toContain('tasks-section');
    });

    it('provides reason for index detection', () => {
      const content = `# Project

## Modules

### auth

- **Path:** [./auth.aps.md](./auth.aps.md)
`;
      const result = adapter.detect(content);
      expect(result.reason).toBeDefined();
      expect(result.reason).toContain('modules-section');
    });
  });

  describe('parse', () => {
    it('parses a leaf spec to APSPlan', async () => {
      const content = readFileSync(join(__dirname, '__fixtures__/simple-leaf.aps.md'), 'utf-8');

      const result = await adapter.parse(content, {
        repositoryPath: '/test/repo',
        author: 'test-user',
      });

      expect(result.success).toBe(true);
      expect(result.data).toBeDefined();
      expect(result.data!.intent).toContain('Authentication Feature');
      expect(result.data!.proposed_changes).toHaveLength(2);
    });

    it('maps task fields to change metadata', async () => {
      const content = readFileSync(join(__dirname, '__fixtures__/simple-leaf.aps.md'), 'utf-8');

      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      const change = result.data!.proposed_changes[0];
      expect(change.description).toContain('AUTH-001');
      expect(change.metadata?.taskId).toBe('AUTH-001');
      expect(change.metadata?.confidence).toBe('high');
    });

    it('preserves task dependencies', async () => {
      const content = readFileSync(join(__dirname, '__fixtures__/simple-leaf.aps.md'), 'utf-8');

      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      const change2 = result.data!.proposed_changes[1];
      expect(change2.metadata?.dependencies).toEqual(['AUTH-001']);
    });

    it('infers file_create type from create/add intent', async () => {
      const content = `# Test Feature

**Scope:** TEST

## Tasks

### TEST-001: Create new file

**Intent:** Create a new configuration file
**Confidence:** high
`;

      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      expect(result.data!.proposed_changes[0].type).toBe('file_create');
    });

    it('infers file_update type from update/modify intent', async () => {
      const content = `# Test Feature

**Scope:** TEST

## Tasks

### TEST-001: Update existing code

**Intent:** Update the existing handler logic
**Confidence:** high
`;

      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      expect(result.data!.proposed_changes[0].type).toBe('file_update');
    });

    it('infers file_delete type from delete/remove intent', async () => {
      const content = `# Test Feature

**Scope:** TEST

## Tasks

### TEST-001: Remove deprecated file

**Intent:** Delete the legacy module
**Confidence:** high
`;

      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      expect(result.data!.proposed_changes[0].type).toBe('file_delete');
    });

    it('generates valid plan ID and hash', async () => {
      const content = readFileSync(join(__dirname, '__fixtures__/simple-leaf.aps.md'), 'utf-8');

      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      expect(result.data!.id).toMatch(/^aps-[a-f0-9]{8}$/);
      expect(result.data!.hash).toMatch(/^[a-f0-9]{64}$/);
    });

    it('uses provided plan ID from context', async () => {
      const content = readFileSync(join(__dirname, '__fixtures__/simple-leaf.aps.md'), 'utf-8');

      const result = await adapter.parse(content, {
        planId: 'aps-12345678',
      });

      expect(result.success).toBe(true);
      expect(result.data!.id).toBe('aps-12345678');
    });

    it('returns error for invalid content', async () => {
      const content = 'Invalid content without H1 title';

      const result = await adapter.parse(content);

      expect(result.success).toBe(false);
      expect(result.errors).toBeDefined();
      expect(result.errors!.length).toBeGreaterThan(0);
      expect(result.errors![0].code).toBe('PARSE_ERROR');
    });

    it('uses first file from files array as change path', async () => {
      const content = readFileSync(join(__dirname, '__fixtures__/simple-leaf.aps.md'), 'utf-8');

      const result = await adapter.parse(content);

      expect(result.success).toBe(true);
      // AUTH-001 has Files: src/auth/login.ts, src/auth/jwt.ts
      expect(result.data!.proposed_changes[0].path).toBe('src/auth/login.ts');
    });
  });
});
