import { describe, expect, it } from 'vitest';
import { scanFile, scanFiles } from './scanner.js';

describe('Scanner', () => {
  describe('scanFile', () => {
    describe('AP-001: Broad eslint-disable', () => {
      it('should detect block comment eslint-disable', () => {
        const content = `/* eslint-disable */\nconst x = 1;`;
        const result = scanFile('test.ts', content);

        expect(result.warnings).toHaveLength(1);
        expect(result.warnings[0].id).toBe('AP-001');
        expect(result.warnings[0].location.line).toBe(1);
      });

      it('should detect line comment eslint-disable', () => {
        const content = `// eslint-disable\nconst x = 1;`;
        const result = scanFile('test.ts', content);

        expect(result.warnings).toHaveLength(1);
        expect(result.warnings[0].id).toBe('AP-001');
      });

      it('should NOT match eslint-disable-next-line', () => {
        const content = `// eslint-disable-next-line no-console\nconsole.log('ok');`;
        const result = scanFile('test.ts', content, { patterns: ['AP-001'] });

        expect(result.warnings).toHaveLength(0);
      });
    });

    describe('AP-003: Explicit any type', () => {
      it('should detect type annotation with any', () => {
        const content = `const data: any = {};`;
        const result = scanFile('test.ts', content);

        expect(result.warnings).toHaveLength(1);
        expect(result.warnings[0].id).toBe('AP-003');
        expect(result.warnings[0].location.line).toBe(1);
      });

      it('should detect as any cast', () => {
        const content = `const x = value as any;`;
        const result = scanFile('test.ts', content);

        expect(result.warnings).toHaveLength(1);
        expect(result.warnings[0].id).toBe('AP-003');
      });

      it('should detect <any> generic', () => {
        const content = `const arr = new Array<any>();`;
        const result = scanFile('test.ts', content);

        expect(result.warnings).toHaveLength(1);
        expect(result.warnings[0].id).toBe('AP-003');
      });

      it('should NOT match words containing any', () => {
        const content = `const company = 'Acme';`;
        const result = scanFile('test.ts', content, { patterns: ['AP-003'] });

        expect(result.warnings).toHaveLength(0);
      });
    });

    describe('AP-004: @ts-ignore', () => {
      it('should detect @ts-ignore', () => {
        const content = `// @ts-ignore\nconst x = undefined.foo;`;
        const result = scanFile('test.ts', content);

        expect(result.warnings).toHaveLength(1);
        expect(result.warnings[0].id).toBe('AP-004');
        expect(result.warnings[0].location.line).toBe(1);
      });
    });

    describe('AP-006: Empty catch block', () => {
      it('should detect empty catch block', () => {
        const content = `try { doSomething(); } catch (e) {}`;
        const result = scanFile('test.ts', content);

        expect(result.warnings).toHaveLength(1);
        expect(result.warnings[0].id).toBe('AP-006');
      });

      it('should detect catch with whitespace only', () => {
        const content = `try { x(); } catch (e) {   }`;
        const result = scanFile('test.ts', content, { patterns: ['AP-006'] });

        expect(result.warnings).toHaveLength(1);
      });
    });

    describe('opt-in patterns', () => {
      it('should NOT detect AP-002 by default', () => {
        const content = `// eslint-disable-next-line no-console`;
        const result = scanFile('test.ts', content);

        const ap002 = result.warnings.filter((w) => w.id === 'AP-002');
        expect(ap002).toHaveLength(0);
      });

      it('should detect AP-002 when includeOptIn is true', () => {
        const content = `// eslint-disable-next-line no-console`;
        const result = scanFile('test.ts', content, { includeOptIn: true });

        const ap002 = result.warnings.filter((w) => w.id === 'AP-002');
        expect(ap002).toHaveLength(1);
      });

      it('should NOT detect AP-005 by default', () => {
        const content = `// @ts-expect-error\nconst x = 1;`;
        const result = scanFile('test.ts', content);

        const ap005 = result.warnings.filter((w) => w.id === 'AP-005');
        expect(ap005).toHaveLength(0);
      });

      it('should NOT detect AP-007 by default', () => {
        const content = `console.log('hello');`;
        const result = scanFile('test.ts', content);

        const ap007 = result.warnings.filter((w) => w.id === 'AP-007');
        expect(ap007).toHaveLength(0);
      });

      it('should detect AP-007 when includeOptIn is true', () => {
        const content = `console.log('hello');`;
        const result = scanFile('test.ts', content, { includeOptIn: true });

        const ap007 = result.warnings.filter((w) => w.id === 'AP-007');
        expect(ap007).toHaveLength(1);
      });
    });

    describe('options', () => {
      it('should only check specified patterns', () => {
        const content = `const x: any = 1;\n// @ts-ignore\nconst y = 2;`;
        const result = scanFile('test.ts', content, { patterns: ['AP-003'] });

        expect(result.warnings).toHaveLength(1);
        expect(result.warnings[0].id).toBe('AP-003');
        expect(result.patternsChecked).toEqual(['AP-003']);
      });

      it('should return patternsChecked for default patterns', () => {
        const result = scanFile('test.ts', 'const x = 1;');

        expect(result.patternsChecked).toContain('AP-001');
        expect(result.patternsChecked).toContain('AP-003');
        expect(result.patternsChecked).toContain('AP-004');
        expect(result.patternsChecked).toContain('AP-006');
        expect(result.patternsChecked).not.toContain('AP-002');
        expect(result.patternsChecked).not.toContain('AP-005');
        expect(result.patternsChecked).not.toContain('AP-007');
      });
    });

    describe('location accuracy', () => {
      it('should report correct line numbers', () => {
        const content = `const x = 1;\nconst y: any = 2;\nconst z = 3;`;
        const result = scanFile('test.ts', content);

        expect(result.warnings[0].location.line).toBe(2);
      });

      it('should report correct column numbers', () => {
        const content = `const data: any = {};`;
        const result = scanFile('test.ts', content);

        expect(result.warnings[0].location.column).toBe(10);
      });

      it('should report correct file path', () => {
        const result = scanFile('src/utils/helper.ts', 'const x: any = 1;');

        expect(result.file).toBe('src/utils/helper.ts');
        expect(result.warnings[0].location.file).toBe('src/utils/helper.ts');
      });
    });

    describe('multiple matches', () => {
      it('should detect multiple patterns in same file', () => {
        const content = `const x: any = 1;\n// @ts-ignore\nconst y = 2;`;
        const result = scanFile('test.ts', content);

        expect(result.warnings).toHaveLength(2);
        const ids = result.warnings.map((w) => w.id);
        expect(ids).toContain('AP-003');
        expect(ids).toContain('AP-004');
      });

      it('should detect same pattern multiple times', () => {
        const content = `const a: any = 1;\nconst b: any = 2;\nconst c: any = 3;`;
        const result = scanFile('test.ts', content);

        const anyWarnings = result.warnings.filter((w) => w.id === 'AP-003');
        expect(anyWarnings).toHaveLength(3);
      });

      it('should detect multiple matches on same line', () => {
        const content = `const x: any = y as any;`;
        const result = scanFile('test.ts', content, { patterns: ['AP-003'] });

        expect(result.warnings).toHaveLength(2);
      });
    });

    describe('clean files', () => {
      it('should return empty warnings for clean file', () => {
        const content = `const x: number = 1;\nconst y: string = 'hello';`;
        const result = scanFile('test.ts', content);

        expect(result.warnings).toHaveLength(0);
      });
    });

    describe('allowlist filtering', () => {
      it('should skip AP-003 for .d.ts files', () => {
        const content = `const x: any = 1;`;
        const result = scanFile('types.d.ts', content);

        const ap003 = result.warnings.filter((w) => w.id === 'AP-003');
        expect(ap003).toHaveLength(0);
      });

      it('should skip AP-003 for __mocks__ directory', () => {
        const content = `const x: any = 1;`;
        const result = scanFile('src/__mocks__/module.ts', content);

        const ap003 = result.warnings.filter((w) => w.id === 'AP-003');
        expect(ap003).toHaveLength(0);
      });

      it('should skip AP-003 for test files', () => {
        const content = `const x: any = 1;`;
        const result = scanFile('src/test/helper.ts', content);

        const ap003 = result.warnings.filter((w) => w.id === 'AP-003');
        expect(ap003).toHaveLength(0);
      });

      it('should detect AP-003 in non-allowlisted files', () => {
        const content = `const x: any = 1;`;
        const result = scanFile('src/utils/helper.ts', content);

        const ap003 = result.warnings.filter((w) => w.id === 'AP-003');
        expect(ap003).toHaveLength(1);
      });

      it('should still detect other patterns in allowlisted files', () => {
        const content = `const x: any = 1;\n// @ts-ignore\nconst y = 2;`;
        const result = scanFile('types.d.ts', content);

        expect(result.warnings.some((w) => w.id === 'AP-003')).toBe(false);
        expect(result.warnings.some((w) => w.id === 'AP-004')).toBe(true);
      });
    });

    describe('warning structure', () => {
      it('should include all required fields with correct values', () => {
        const content = `const x: any = 1;`;
        const result = scanFile('src/file.ts', content);
        const warning = result.warnings[0];

        expect(warning.id).toBe('AP-003');
        expect(warning.category).toBe('anti-pattern');
        expect(warning.severity).toBe('warning');
        expect(warning.confidence).toBe('high');
        expect(warning.title).toContain('any');
        expect(typeof warning.message).toBe('string');
        expect(warning.message.length).toBeGreaterThan(0);
        expect(typeof warning.explanation).toBe('string');
        expect(warning.explanation.length).toBeGreaterThan(0);
        expect(typeof warning.suggestion).toBe('string');
        expect(warning.suggestion.length).toBeGreaterThan(0);
        expect(warning.location).toEqual(expect.objectContaining({ file: 'src/file.ts', line: 1 }));
        expect(warning.pattern).toBe('AP-003');
      });
    });
  });

  describe('legacy pattern scoping to JS/TS files', () => {
    it('should NOT detect JS/TS-only patterns on HTML files', () => {
      // AP-003 (explicit any) has no fileExtensions set, so it defaults to JS/TS only
      const content = `<div>const x: any = 1;</div>`;
      const result = scanFile('page.html', content, { patterns: ['AP-003'] });

      expect(result.warnings).toHaveLength(0);
    });

    it('should NOT detect JS/TS-only patterns on CSS files', () => {
      const content = `/* eslint-disable */`;
      const result = scanFile('style.css', content, { patterns: ['AP-001'] });

      expect(result.warnings).toHaveLength(0);
    });

    it('should still detect JS/TS-only patterns on .ts files', () => {
      const content = `const x: any = 1;`;
      const result = scanFile('test.ts', content, { patterns: ['AP-003'] });

      expect(result.warnings).toHaveLength(1);
    });

    it('should still detect JS/TS-only patterns on .jsx files', () => {
      const content = `const x: any = 1;`;
      const result = scanFile('component.jsx', content, { patterns: ['AP-003'] });

      expect(result.warnings).toHaveLength(1);
    });
  });

  describe('fileExtensions filtering', () => {
    it('should skip pattern when file extension does not match fileExtensions', () => {
      // AP-008 (inline style) has fileExtensions: ['.html', '.htm']
      // scanning a .ts file should skip it
      const content = `const style = 'style="color:red"';`;
      const result = scanFile('test.ts', content, { patterns: ['AP-008'] });

      expect(result.warnings).toHaveLength(0);
    });

    it('should detect pattern when file extension matches fileExtensions', () => {
      const content = `<div style="color: red">Hello</div>`;
      const result = scanFile('page.html', content, { patterns: ['AP-008'] });

      expect(result.warnings).toHaveLength(1);
      expect(result.warnings[0].id).toBe('AP-008');
    });

    it('should detect pattern when file has .htm extension', () => {
      const content = `<div style="color: red">Hello</div>`;
      const result = scanFile('page.htm', content, { patterns: ['AP-008'] });

      expect(result.warnings).toHaveLength(1);
      expect(result.warnings[0].id).toBe('AP-008');
    });

    it('should apply CSS pattern only to CSS files', () => {
      const content = `color: red !important;`;
      const resultCss = scanFile('style.css', content, { patterns: ['AP-012'] });
      const resultTs = scanFile('style.ts', content, { patterns: ['AP-012'] });

      expect(resultCss.warnings).toHaveLength(1);
      expect(resultTs.warnings).toHaveLength(0);
    });

    it('should apply CSS pattern to SCSS files', () => {
      const content = `color: red !important;`;
      const result = scanFile('style.scss', content, { patterns: ['AP-012'] });

      expect(result.warnings).toHaveLength(1);
    });
  });

  describe('scanFiles', () => {
    it('should scan multiple files', () => {
      const files = [
        { path: 'a.ts', content: 'const x: any = 1;' },
        { path: 'b.ts', content: 'const y: any = 2;' },
      ];
      const results = scanFiles(files);

      expect(results).toHaveLength(2);
      expect(results[0].file).toBe('a.ts');
      expect(results[1].file).toBe('b.ts');
    });

    it('should return results for each file', () => {
      const files = [
        { path: 'clean.ts', content: 'const x = 1;' },
        { path: 'dirty.ts', content: 'const y: any = 2;' },
      ];
      const results = scanFiles(files);

      expect(results[0].warnings).toHaveLength(0);
      expect(results[1].warnings).toHaveLength(1);
    });

    it('should pass options to each file scan', () => {
      const files = [{ path: 'test.ts', content: 'console.log("hi");' }];
      const results = scanFiles(files, { includeOptIn: true });

      const ap007 = results[0].warnings.filter((w) => w.id === 'AP-007');
      expect(ap007).toHaveLength(1);
    });
  });
});
