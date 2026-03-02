/**
 * Generic Parser Tests
 * Tests for parseGeneric input size guard (byte-length)
 */

import { describe, it, expect } from 'vitest';
import { parseGeneric } from '../parser.js';

const MAX_INPUT_SIZE = 2 * 1024 * 1024; // 2 MiB

describe('parseGeneric', () => {
  describe('input size guard', () => {
    it('should reject content exceeding the byte limit', () => {
      // ASCII content just over the limit
      const content = 'a'.repeat(MAX_INPUT_SIZE + 1);
      expect(() => parseGeneric(content)).toThrow(/maximum size/);
    });

    it('should accept content at exactly the byte limit', () => {
      // Build a minimal valid markdown document that is exactly MAX_INPUT_SIZE bytes.
      // The parser may not find any planning sections, but it should not throw
      // the size error.
      const header = '# Title\n\n';
      const padding = 'x'.repeat(MAX_INPUT_SIZE - Buffer.byteLength(header, 'utf8'));
      const content = header + padding;

      expect(Buffer.byteLength(content, 'utf8')).toBe(MAX_INPUT_SIZE);
      // Should not throw size error (may throw other validation errors, that's fine)
      expect(() => parseGeneric(content)).not.toThrow(/maximum size/);
    });

    it('should reject multi-byte content that exceeds byte limit despite short length', () => {
      // Each emoji is 4 bytes in UTF-8 but only 2 UTF-16 code units.
      // We build a string whose .length (UTF-16 code units) is under the limit
      // but whose UTF-8 byte length exceeds it.
      const emoji = '\u{1F600}'; // U+1F600 GRINNING FACE, 4 bytes UTF-8, 2 code units
      const emojiByteLen = Buffer.byteLength(emoji, 'utf8'); // 4

      // We need: count * emojiByteLen > MAX_INPUT_SIZE
      //          count * emoji.length  <= MAX_INPUT_SIZE  (old check would pass)
      // count > MAX_INPUT_SIZE / emojiByteLen
      // count <= MAX_INPUT_SIZE / emoji.length
      const count = Math.floor(MAX_INPUT_SIZE / emojiByteLen) + 1;
      const content = emoji.repeat(count);

      // Verify preconditions: string length is under the limit but byte length exceeds it
      expect(content.length).toBeLessThanOrEqual(MAX_INPUT_SIZE);
      expect(Buffer.byteLength(content, 'utf8')).toBeGreaterThan(MAX_INPUT_SIZE);

      expect(() => parseGeneric(content)).toThrow(/maximum size/);
    });
  });
});
