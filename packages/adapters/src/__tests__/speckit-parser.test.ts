import { describe, it, expect, beforeEach } from 'vitest';
import { SpecKitParser } from '../speckit/parser.js';

describe('SpecKitParser', () => {
  let parser: SpecKitParser;

  beforeEach(() => {
    parser = new SpecKitParser();
  });

  describe('parseSpecMarkdown', () => {
    it('should parse intent section', () => {
      const markdown = `# Specification
      
## Intent

Build a REST API for user management with CRUD operations.

## Other Section

Some content`;

      const result = parser.parseSpecMarkdown(markdown);

      expect(result.intent).toBe('Build a REST API for user management with CRUD operations.');
    });

    it('should parse overview section', () => {
      const markdown = `# Spec

## Overview

This system provides comprehensive user management capabilities including create, read, update, and delete operations.

## Details`;

      const result = parser.parseSpecMarkdown(markdown);

      expect(result.overview).toContain('comprehensive user management');
    });

    it('should parse goals as list items', () => {
      const markdown = `# Specification

## Goals

- Implement secure authentication
- Support role-based access control
- Provide RESTful API endpoints
- Enable user profile management`;

      const result = parser.parseSpecMarkdown(markdown);

      expect(result.goals).toBeDefined();
      expect(result.goals).toHaveLength(4);
      expect(result.goals?.[0]).toBe('Implement secure authentication');
      expect(result.goals?.[3]).toBe('Enable user profile management');
    });

    it('should parse numbered list items', () => {
      const markdown = `# Spec

## Requirements

1. Node.js version 18 or higher
2. PostgreSQL database
3. Redis for caching
4. Docker for containerization`;

      const result = parser.parseSpecMarkdown(markdown);

      expect(result.requirements).toBeDefined();
      expect(result.requirements).toHaveLength(4);
      expect(result.requirements?.[0]).toBe('Node.js version 18 or higher');
    });

    it('should handle multi-line list items', () => {
      const markdown = `# Spec

## Goals

- Implement authentication system
  with JWT tokens and refresh mechanism
- Support multiple authentication providers
  including OAuth2 and SAML`;

      const result = parser.parseSpecMarkdown(markdown);

      expect(result.goals).toHaveLength(2);
      expect(result.goals?.[0]).toBe(
        'Implement authentication system with JWT tokens and refresh mechanism'
      );
    });

    it('should parse changes section with subsections', () => {
      const markdown = `# Specification

## Changes

### Create authentication controller

Create a new controller at \`src/auth.controller.ts\`

\`\`\`typescript
export class AuthController {
  // Implementation
}
\`\`\`

### Update main application

Modify \`src/app.ts\` to include auth routes`;

      const result = parser.parseSpecMarkdown(markdown);

      expect(result.changes).toBeDefined();
      expect(result.changes).toHaveLength(2);

      const createChange = result.changes?.[0];
      expect(createChange?.type).toBe('file_create');
      expect(createChange?.description).toContain('Create authentication controller');
      expect(createChange?.path).toBe('src/auth.controller.ts');
      expect(createChange?.content).toContain('export class AuthController');

      const updateChange = result.changes?.[1];
      expect(updateChange?.type).toBe('file_update');
      expect(updateChange?.path).toBe('src/app.ts');
    });

    it('should parse changes from list items', () => {
      const markdown = `# Specification

## Changes

- Create new file \`config/database.ts\` for database configuration
- Update \`package.json\` to add PostgreSQL dependency
- Delete obsolete \`src/old-db.ts\` file
- Install new dependency: express-validator
- Run migration script to update database schema`;

      const result = parser.parseSpecMarkdown(markdown);

      expect(result.changes).toHaveLength(5);

      expect(result.changes?.[0].type).toBe('file_create');
      expect(result.changes?.[0].path).toBe('config/database.ts');

      expect(result.changes?.[1].type).toBe('file_update');
      expect(result.changes?.[1].path).toBe('package.json');

      expect(result.changes?.[2].type).toBe('file_delete');
      expect(result.changes?.[2].path).toBe('src/old-db.ts');

      expect(result.changes?.[3].type).toBe('dependency_add');

      expect(result.changes?.[4].type).toBe('script_execute');
    });

    it('should handle empty sections gracefully', () => {
      const markdown = `# Specification

## Intent

## Goals

## Requirements`;

      const result = parser.parseSpecMarkdown(markdown);

      expect(result.intent).toBe('');
      expect(result.goals).toEqual([]);
      expect(result.requirements).toEqual([]);
    });

    it('should handle alternative section names', () => {
      const markdown = `# Specification

## Purpose

Implement user authentication

## Objectives

- Secure the application
- Manage user sessions

## Prerequisites

- Database setup
- SSL certificates`;

      const result = parser.parseSpecMarkdown(markdown);

      expect(result.intent).toBe('Implement user authentication');
      expect(result.goals).toContain('Secure the application');
      expect(result.requirements).toContain('Database setup');
    });

    it('should extract code blocks with correct language', () => {
      const markdown = `# Spec

## Changes

### Update TypeScript config

\`\`\`json
{
  "compilerOptions": {
    "strict": true
  }
}
\`\`\``;

      const result = parser.parseSpecMarkdown(markdown);

      expect(result.changes?.[0].content).toContain('"strict": true');
    });

    it('should parse code blocks with backticks in content without ReDoS', () => {
      // Regression test for SECB-008: the code-block regex must not
      // catastrophically backtrack on content containing isolated backticks.
      const backtickHeavy = '`'.repeat(60);
      const markdown = `# Spec

## Changes

### Create template

\`\`\`typescript
const tmpl = ${backtickHeavy};
\`\`\``;

      const start = performance.now();
      const result = parser.parseSpecMarkdown(markdown);
      const elapsed = performance.now() - start;

      // Must complete in well under a second — exponential backtracking
      // would blow past this on 60 backticks.
      expect(elapsed).toBeLessThan(1000);
      expect(result.changes).toBeDefined();
    });
  });
});
