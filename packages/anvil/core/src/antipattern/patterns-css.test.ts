import { describe, expect, it } from 'vitest';
import { CSS_PATTERNS } from './patterns-css.js';
import { getPattern } from './patterns.js';
import { scanFile } from './scanner.js';

describe('CSS Patterns', () => {
  describe('CSS_PATTERNS collection', () => {
    it('should contain 2 CSS patterns', () => {
      expect(CSS_PATTERNS).toHaveLength(2);
    });

    it('should have unique IDs', () => {
      const ids = CSS_PATTERNS.map((p) => p.id);
      expect(new Set(ids).size).toBe(ids.length);
    });

    it('should all be in the css category', () => {
      for (const pattern of CSS_PATTERNS) {
        expect(pattern.category).toBe('css');
      }
    });

    it('should all be opt-in', () => {
      for (const pattern of CSS_PATTERNS) {
        expect(pattern.optIn).toBe(true);
      }
    });

    it('should all target CSS file extensions', () => {
      for (const pattern of CSS_PATTERNS) {
        expect(pattern.fileExtensions).toEqual(['.css', '.scss', '.less']);
      }
    });
  });

  describe('AP-012: !important in CSS', () => {
    const pattern = getPattern('AP-012')!;

    it('should be registered in pattern catalogue', () => {
      expect(pattern).toBeDefined();
      expect(pattern.name).toBe('!important in CSS');
    });

    it('should detect !important in CSS', () => {
      const content = `color: red !important;`;
      const result = scanFile('style.css', content, { patterns: ['AP-012'] });

      expect(result.warnings).toHaveLength(1);
      expect(result.warnings[0].id).toBe('AP-012');
    });

    it('should detect !important with no space', () => {
      const content = `display: none!important;`;
      const result = scanFile('style.css', content, { patterns: ['AP-012'] });

      expect(result.warnings).toHaveLength(1);
    });

    it('should detect !important in SCSS', () => {
      const content = `.class { color: red !important; }`;
      const result = scanFile('style.scss', content, { patterns: ['AP-012'] });

      expect(result.warnings).toHaveLength(1);
    });

    it('should detect !important in LESS', () => {
      const content = `.class { color: red !important; }`;
      const result = scanFile('style.less', content, { patterns: ['AP-012'] });

      expect(result.warnings).toHaveLength(1);
    });

    it('should NOT detect in .ts files', () => {
      const content = `const css = 'color: red !important';`;
      const result = scanFile('styles.ts', content, { patterns: ['AP-012'] });

      expect(result.warnings).toHaveLength(0);
    });

    it('should skip reset.css files', () => {
      const content = `* { margin: 0 !important; }`;
      const result = scanFile('vendor/reset.css', content, { patterns: ['AP-012'] });

      expect(result.warnings).toHaveLength(0);
    });

    it('should skip normalize.css files', () => {
      const content = `button { overflow: visible !important; }`;
      const result = scanFile('vendor/normalize.css', content, { patterns: ['AP-012'] });

      expect(result.warnings).toHaveLength(0);
    });

    it('should detect multiple !important on different lines', () => {
      const content = `color: red !important;\nfont-size: 14px !important;`;
      const result = scanFile('style.css', content, { patterns: ['AP-012'] });

      expect(result.warnings).toHaveLength(2);
    });
  });

  describe('AP-013: CSS @import', () => {
    const pattern = getPattern('AP-013')!;

    it('should be registered in pattern catalogue', () => {
      expect(pattern).toBeDefined();
      expect(pattern.name).toBe('CSS @import');
    });

    it('should have info severity', () => {
      expect(pattern.severity).toBe('info');
    });

    it('should detect @import with double quotes', () => {
      const content = `@import "reset.css";`;
      const result = scanFile('style.css', content, { patterns: ['AP-013'] });

      expect(result.warnings).toHaveLength(1);
      expect(result.warnings[0].id).toBe('AP-013');
    });

    it('should detect @import with single quotes', () => {
      const content = `@import 'typography.css';`;
      const result = scanFile('style.css', content, { patterns: ['AP-013'] });

      expect(result.warnings).toHaveLength(1);
    });

    it('should detect @import url() with quotes', () => {
      const content = `@import url("fonts.css");`;
      const result = scanFile('style.css', content, { patterns: ['AP-013'] });

      expect(result.warnings).toHaveLength(1);
    });

    it('should detect @import in SCSS', () => {
      const content = `@import "variables";`;
      const result = scanFile('main.scss', content, { patterns: ['AP-013'] });

      expect(result.warnings).toHaveLength(1);
    });

    it('should detect @import in LESS', () => {
      const content = `@import "mixins.less";`;
      const result = scanFile('main.less', content, { patterns: ['AP-013'] });

      expect(result.warnings).toHaveLength(1);
    });

    it('should NOT detect in .ts files', () => {
      const content = `// @import 'something'`;
      const result = scanFile('test.ts', content, { patterns: ['AP-013'] });

      expect(result.warnings).toHaveLength(0);
    });
  });
});
