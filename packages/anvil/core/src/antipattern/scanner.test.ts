import { describe, expect, it } from 'vitest';
import { scanArtifact, scanArtifacts, scanFile, scanFiles } from './scanner.js';

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

    describe('nudge propagation', () => {
      it('should propagate nudge from pattern to warning', () => {
        const content = `const x: any = 1;`;
        const result = scanFile('src/file.ts', content);
        const warning = result.warnings[0];

        expect(warning.nudge).toBeDefined();
        expect(warning.nudge).toContain("Don't use `any` here");
      });

      it('should propagate nudge for AP-001', () => {
        const content = `/* eslint-disable */`;
        const result = scanFile('test.ts', content, { patterns: ['AP-001'] });

        expect(result.warnings[0].nudge).toContain('Blanket disables');
      });

      it('should propagate nudge for AP-004', () => {
        const content = `// @ts-ignore`;
        const result = scanFile('test.ts', content, { patterns: ['AP-004'] });

        expect(result.warnings[0].nudge).toContain('@ts-expect-error');
      });

      it('should propagate nudge for AP-006', () => {
        const content = `try { x(); } catch (e) {}`;
        const result = scanFile('test.ts', content, { patterns: ['AP-006'] });

        expect(result.warnings[0].nudge).toContain('swallow');
      });
    });
  });

  describe('JS/TS file scoping', () => {
    it('should detect JS/TS patterns on .ts files', () => {
      const content = `const x: any = 1;`;
      const result = scanFile('test.ts', content, { patterns: ['AP-003'] });

      expect(result.warnings).toHaveLength(1);
    });

    it('should detect JS/TS patterns on .jsx files', () => {
      const content = `const x: any = 1;`;
      const result = scanFile('component.jsx', content, { patterns: ['AP-003'] });

      expect(result.warnings).toHaveLength(1);
    });
  });

  describe('scanArtifact', () => {
    it('should mirror scanFile for source artifacts', () => {
      const content = `const x: any = 1;`;
      const fileResult = scanFile('src/a.ts', content);
      const artifactResult = scanArtifact({ type: 'source', ref: 'src/a.ts', content });

      expect(artifactResult.warnings.map((w) => w.id)).toEqual(
        fileResult.warnings.map((w) => w.id)
      );
      expect(artifactResult.file).toBe('src/a.ts');
      expect(artifactResult.artifactType).toBe('source');
    });

    it('should skip legacy source-only rules for non-source artifacts', () => {
      // AP-003 targets source only — scanning its detection content against
      // a pr-description should produce no AP-003 warnings.
      const prBody = `const x: any = 1;\n// @ts-ignore`;
      const result = scanArtifact(
        { type: 'pr-description', ref: 'pr/42', content: prBody },
        { patterns: ['AP-003', 'AP-004'] }
      );

      expect(result.warnings).toEqual([]);
      expect(result.artifactType).toBe('pr-description');
      expect(result.file).toBe('pr/42');
    });

    it('should run compiled rules that target pr-description on PR bodies', () => {
      // RL-001 targets [agent-output, pr-description] and matches "pre-existing"
      // without a run link / verification.
      const prBody = 'This failure is pre-existing and unrelated to my change.';
      const result = scanArtifact(
        { type: 'pr-description', ref: 'pr/42', content: prBody },
        { patterns: ['RL-001'] }
      );

      expect(result.warnings.length).toBeGreaterThan(0);
      expect(result.warnings[0].id).toBe('RL-001');
      expect(result.warnings[0].location.file).toBe('pr/42');
    });

    it('should skip pr-description-only rules when scanning source files', () => {
      const source = 'const note = "pre-existing issue"';
      const result = scanArtifact(
        { type: 'source', ref: 'src/a.ts', content: source },
        { patterns: ['RL-001'] }
      );

      expect(result.warnings).toEqual([]);
    });

    it('should propagate artifactType onto the result', () => {
      const result = scanArtifact({
        type: 'commit-message',
        ref: 'abc123',
        content: 'fix stuff',
      });

      expect(result.artifactType).toBe('commit-message');
      expect(result.file).toBe('abc123');
    });

    it('should honour pattern filtering via options', () => {
      const content = `const x: any = 1;\n// @ts-ignore`;
      const result = scanArtifact(
        { type: 'source', ref: 'src/a.ts', content },
        { patterns: ['AP-003'] }
      );

      expect(result.warnings.map((w) => w.id)).toEqual(['AP-003']);
      expect(result.patternsChecked).toEqual(['AP-003']);
    });
  });

  describe('scanArtifacts', () => {
    it('should scan multiple artifacts of mixed types', () => {
      const results = scanArtifacts([
        { type: 'source', ref: 'src/a.ts', content: 'const x: any = 1;' },
        {
          type: 'pr-description',
          ref: 'pr/42',
          content: 'This is pre-existing unrelated to my change.',
        },
      ]);

      expect(results).toHaveLength(2);
      expect(results[0].artifactType).toBe('source');
      expect(results[0].warnings.some((w) => w.id === 'AP-003')).toBe(true);
      expect(results[1].artifactType).toBe('pr-description');
      // RL-001 fires against the unverified "pre-existing" claim.
      expect(results[1].warnings.some((w) => w.id === 'RL-001')).toBe(true);
    });
  });

  describe('family provenance on warnings', () => {
    it('should attach family, definition_ref, spectrum_position for compiled patterns', () => {
      const content = '/* eslint-disable */';
      const result = scanFile('src/a.ts', content, { patterns: ['AP-001'] });

      expect(result.warnings).toHaveLength(1);
      const w = result.warnings[0];
      expect(w.family).toBe('guardrail-suppression');
      expect(w.definition_ref).toBe('patterns/guardrail-suppression/definition.anvil');
      expect(w.spectrum_position).toBe(1);
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
