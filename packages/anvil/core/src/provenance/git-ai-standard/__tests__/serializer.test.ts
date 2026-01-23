import { describe, it, expect } from 'vitest';
import {
  serializeAuthorshipLog,
  parseAuthorshipLog,
  isAuthorshipLog,
  expandLineRanges,
  compactLineRanges,
} from '../serializer.js';
import { SCHEMA_VERSION, type AuthorshipLog } from '../types.js';

describe('AuthorshipLog Serializer', () => {
  const sampleLog: AuthorshipLog = {
    attestations: {
      'src/auth/login.ts': [{ sessionHash: 'a1b2c3d4e5f67890', lineRanges: '1-50,55-60' }],
      'src/auth/jwt.ts': [{ sessionHash: 'a1b2c3d4e5f67890', lineRanges: '1-30' }],
    },
    metadata: {
      schema_version: SCHEMA_VERSION,
      base_commit_sha: 'abc123def456789012345678901234567890abcd',
      prompts: {
        a1b2c3d4e5f67890: {
          agent_id: {
            tool: 'claude-code',
            id: 'session-123',
            model: 'claude-3-opus',
          },
          messages: [
            { type: 'user', text: 'Implement login endpoint' },
            { type: 'assistant', text: 'Creating login.ts...' },
          ],
          total_additions: 80,
          total_deletions: 0,
          accepted_lines: 75,
          overriden_lines: 5,
          human_author: 'Alice <alice@example.com>',
        },
      },
    },
  };

  describe('serializeAuthorshipLog', () => {
    it('produces valid log format with attestation and metadata sections', () => {
      const output = serializeAuthorshipLog(sampleLog);

      // Should have attestation section with files
      expect(output).toContain('src/auth/login.ts');
      expect(output).toContain('a1b2c3d4e5f67890 1-50,55-60');

      // Should have separator
      expect(output).toContain('---');

      // Should have JSON metadata
      expect(output).toContain(`"schema_version": "${SCHEMA_VERSION}"`);
      expect(output).toContain('"claude-code"');
    });

    it('quotes file paths with special characters', () => {
      const logWithSpaces: AuthorshipLog = {
        attestations: {
          'src/My Component/index.ts': [{ sessionHash: 'a1b2c3d4e5f67890', lineRanges: '1-10' }],
        },
        metadata: sampleLog.metadata,
      };

      const output = serializeAuthorshipLog(logWithSpaces);
      expect(output).toContain('"src/My Component/index.ts"');
    });

    it('sorts file paths alphabetically', () => {
      const output = serializeAuthorshipLog(sampleLog);
      const lines = output.split('\n');

      const jwtIndex = lines.findIndex((l) => l.includes('jwt.ts'));
      const loginIndex = lines.findIndex((l) => l.includes('login.ts'));

      expect(jwtIndex).toBeLessThan(loginIndex);
    });
  });

  describe('parseAuthorshipLog', () => {
    it('parses a serialized log correctly', () => {
      const serialized = serializeAuthorshipLog(sampleLog);
      const parsed = parseAuthorshipLog(serialized);

      expect(parsed.metadata.schema_version).toBe(SCHEMA_VERSION);
      expect(parsed.metadata.prompts['a1b2c3d4e5f67890'].agent_id.tool).toBe('claude-code');
    });

    it('round-trips correctly', () => {
      const serialized = serializeAuthorshipLog(sampleLog);
      const parsed = parseAuthorshipLog(serialized);

      expect(parsed.attestations['src/auth/login.ts']).toHaveLength(1);
      expect(parsed.attestations['src/auth/login.ts'][0].sessionHash).toBe('a1b2c3d4e5f67890');
      expect(parsed.attestations['src/auth/login.ts'][0].lineRanges).toBe('1-50,55-60');
    });

    it('throws on missing separator', () => {
      expect(() => parseAuthorshipLog('no separator here')).toThrow('missing --- separator');
    });

    it('throws on invalid JSON', () => {
      expect(() => parseAuthorshipLog('file.ts\n  abc12345 1-10\n---\n{invalid}')).toThrow(
        'malformed JSON'
      );
    });
  });

  describe('isAuthorshipLog', () => {
    it('returns true for valid log content', () => {
      const serialized = serializeAuthorshipLog(sampleLog);
      expect(isAuthorshipLog(serialized)).toBe(true);
    });

    it('returns false for non-log content', () => {
      expect(isAuthorshipLog('just some random text')).toBe(false);
      expect(isAuthorshipLog('has --- but no schema')).toBe(false);
    });
  });

  describe('expandLineRanges', () => {
    it('expands single line', () => {
      expect(expandLineRanges('42')).toEqual([42]);
    });

    it('expands range', () => {
      expect(expandLineRanges('1-5')).toEqual([1, 2, 3, 4, 5]);
    });

    it('expands mixed format', () => {
      expect(expandLineRanges('1,3-5,10')).toEqual([1, 3, 4, 5, 10]);
    });
  });

  describe('compactLineRanges', () => {
    it('compacts consecutive lines into range', () => {
      expect(compactLineRanges([1, 2, 3, 4, 5])).toBe('1-5');
    });

    it('handles single line', () => {
      expect(compactLineRanges([42])).toBe('42');
    });

    it('handles mixed ranges and singles', () => {
      expect(compactLineRanges([1, 3, 4, 5, 10])).toBe('1,3-5,10');
    });

    it('handles empty array', () => {
      expect(compactLineRanges([])).toBe('');
    });

    it('deduplicates and sorts', () => {
      expect(compactLineRanges([5, 3, 1, 3, 5])).toBe('1,3,5');
    });
  });
});
