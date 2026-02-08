import { describe, expect, it } from 'vitest';
import { HTML_PATTERNS } from './patterns-html.js';
import { getPattern } from './patterns.js';
import { scanFile } from './scanner.js';

describe('HTML Patterns', () => {
  describe('HTML_PATTERNS collection', () => {
    it('should contain 4 HTML patterns', () => {
      expect(HTML_PATTERNS).toHaveLength(4);
    });

    it('should have unique IDs', () => {
      const ids = HTML_PATTERNS.map((p) => p.id);
      expect(new Set(ids).size).toBe(ids.length);
    });

    it('should all be in the html category', () => {
      for (const pattern of HTML_PATTERNS) {
        expect(pattern.category).toBe('html');
      }
    });

    it('should all be opt-in', () => {
      for (const pattern of HTML_PATTERNS) {
        expect(pattern.optIn).toBe(true);
      }
    });

    it('should all target HTML file extensions', () => {
      for (const pattern of HTML_PATTERNS) {
        expect(pattern.fileExtensions).toEqual(['.html', '.htm']);
      }
    });

    it('should all have email allowlist', () => {
      for (const pattern of HTML_PATTERNS) {
        expect(pattern.allowlist).toContain('**/email/**');
      }
    });
  });

  describe('AP-008: Inline style attribute', () => {
    const pattern = getPattern('AP-008')!;

    it('should be registered in pattern catalogue', () => {
      expect(pattern).toBeDefined();
      expect(pattern.name).toBe('Inline style attribute');
    });

    it('should detect style="..." in HTML', () => {
      const content = `<div style="color: red">Hello</div>`;
      const result = scanFile('page.html', content, { patterns: ['AP-008'] });

      expect(result.warnings).toHaveLength(1);
      expect(result.warnings[0].id).toBe('AP-008');
    });

    it('should detect style with single quotes', () => {
      const content = `<p style='font-size: 14px'>Text</p>`;
      const result = scanFile('page.html', content, { patterns: ['AP-008'] });

      expect(result.warnings).toHaveLength(1);
    });

    it('should NOT detect in .ts files', () => {
      const content = `const el = '<div style="color: red">';`;
      const result = scanFile('component.ts', content, { patterns: ['AP-008'] });

      expect(result.warnings).toHaveLength(0);
    });

    it('should detect in .htm files', () => {
      const content = `<span style="display:none">Hidden</span>`;
      const result = scanFile('legacy.htm', content, { patterns: ['AP-008'] });

      expect(result.warnings).toHaveLength(1);
    });

    it('should skip files in email directory', () => {
      const content = `<td style="padding: 10px">Cell</td>`;
      const result = scanFile('templates/email/welcome.html', content, { patterns: ['AP-008'] });

      expect(result.warnings).toHaveLength(0);
    });
  });

  describe('AP-009: Inline script block', () => {
    const pattern = getPattern('AP-009')!;

    it('should be registered in pattern catalogue', () => {
      expect(pattern).toBeDefined();
      expect(pattern.name).toBe('Inline script block');
    });

    it('should detect inline script with content', () => {
      const content = `<script>console.log('hello');</script>`;
      const result = scanFile('page.html', content, { patterns: ['AP-009'] });

      expect(result.warnings).toHaveLength(1);
      expect(result.warnings[0].id).toBe('AP-009');
    });

    it('should detect script with attributes and content', () => {
      const content = `<script type="text/javascript">var x = 1;</script>`;
      const result = scanFile('page.html', content, { patterns: ['AP-009'] });

      expect(result.warnings).toHaveLength(1);
    });

    it('should NOT detect in .ts files', () => {
      const content = `const html = '<script>alert(1)</script>';`;
      const result = scanFile('test.ts', content, { patterns: ['AP-009'] });

      expect(result.warnings).toHaveLength(0);
    });
  });

  describe('AP-010: Inline event handler', () => {
    const pattern = getPattern('AP-010')!;

    it('should be registered in pattern catalogue', () => {
      expect(pattern).toBeDefined();
      expect(pattern.name).toBe('Inline event handler');
    });

    it('should detect onclick handler', () => {
      const content = `<button onclick="doSomething()">Click</button>`;
      const result = scanFile('page.html', content, { patterns: ['AP-010'] });

      expect(result.warnings).toHaveLength(1);
      expect(result.warnings[0].id).toBe('AP-010');
    });

    it('should detect onload handler', () => {
      const content = `<body onload="init()">`;
      const result = scanFile('page.html', content, { patterns: ['AP-010'] });

      expect(result.warnings).toHaveLength(1);
    });

    it('should detect onmouseover handler', () => {
      const content = `<div onmouseover="highlight()">Hover me</div>`;
      const result = scanFile('page.html', content, { patterns: ['AP-010'] });

      expect(result.warnings).toHaveLength(1);
    });

    it('should NOT detect in .ts files', () => {
      const content = `element.onclick = handler;`;
      const result = scanFile('app.ts', content, { patterns: ['AP-010'] });

      expect(result.warnings).toHaveLength(0);
    });
  });

  describe('AP-011: Deprecated HTML tags', () => {
    const pattern = getPattern('AP-011')!;

    it('should be registered in pattern catalogue', () => {
      expect(pattern).toBeDefined();
      expect(pattern.name).toBe('Deprecated HTML tag');
    });

    it('should detect <font> tag', () => {
      const content = `<font color="red">Old text</font>`;
      const result = scanFile('page.html', content, { patterns: ['AP-011'] });

      expect(result.warnings).toHaveLength(1);
      expect(result.warnings[0].id).toBe('AP-011');
    });

    it('should detect <center> tag', () => {
      const content = `<center>Centered text</center>`;
      const result = scanFile('page.html', content, { patterns: ['AP-011'] });

      expect(result.warnings).toHaveLength(1);
    });

    it('should detect <marquee> tag', () => {
      const content = `<marquee>Scrolling text</marquee>`;
      const result = scanFile('page.html', content, { patterns: ['AP-011'] });

      expect(result.warnings).toHaveLength(1);
    });

    it('should detect <blink> tag', () => {
      const content = `<blink>Blinking text</blink>`;
      const result = scanFile('page.html', content, { patterns: ['AP-011'] });

      expect(result.warnings).toHaveLength(1);
    });

    it('should detect <big> tag', () => {
      const content = `<big>Big text</big>`;
      const result = scanFile('page.html', content, { patterns: ['AP-011'] });

      expect(result.warnings).toHaveLength(1);
    });

    it('should detect <strike> tag', () => {
      const content = `<strike>Struck through</strike>`;
      const result = scanFile('page.html', content, { patterns: ['AP-011'] });

      expect(result.warnings).toHaveLength(1);
    });

    it('should NOT detect in .ts files', () => {
      const content = `const html = '<center>test</center>';`;
      const result = scanFile('test.ts', content, { patterns: ['AP-011'] });

      expect(result.warnings).toHaveLength(0);
    });

    it('should skip files in email directory', () => {
      const content = `<center>Email centered content</center>`;
      const result = scanFile('templates/email/newsletter.html', content, { patterns: ['AP-011'] });

      expect(result.warnings).toHaveLength(0);
    });
  });
});
