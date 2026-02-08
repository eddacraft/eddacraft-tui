import { describe, expect, it } from 'vitest';
import { extractHtmlEdges, extractCssEdges } from './edge-detector-html.js';

describe('HTML Edge Detector', () => {
  describe('extractHtmlEdges', () => {
    it('should extract <script src="..."> edge', () => {
      const content = `<script src="./app.js"></script>`;
      const edges = extractHtmlEdges('index.html', content);

      expect(edges).toHaveLength(1);
      expect(edges[0]).toMatchObject({
        from: 'index.html',
        to: 'app.js',
        line: 1,
        type: 'import',
        specifier: './app.js',
      });
    });

    it('should extract multiple <script src> edges', () => {
      const content = `<script src="./vendor.js"></script>
<script src="./app.js"></script>`;
      const edges = extractHtmlEdges('index.html', content);

      expect(edges).toHaveLength(2);
      expect(edges[0].specifier).toBe('./vendor.js');
      expect(edges[1].specifier).toBe('./app.js');
    });

    it('should skip external script URLs (https)', () => {
      const content = `<script src="https://cdn.example.com/lib.js"></script>`;
      const edges = extractHtmlEdges('index.html', content);

      expect(edges).toHaveLength(0);
    });

    it('should skip external script URLs (http)', () => {
      const content = `<script src="http://cdn.example.com/lib.js"></script>`;
      const edges = extractHtmlEdges('index.html', content);

      expect(edges).toHaveLength(0);
    });

    it('should skip protocol-relative script URLs', () => {
      const content = `<script src="//cdn.example.com/lib.js"></script>`;
      const edges = extractHtmlEdges('index.html', content);

      expect(edges).toHaveLength(0);
    });

    it('should extract <link rel="stylesheet" href="..."> edge', () => {
      const content = `<link rel="stylesheet" href="./styles.css">`;
      const edges = extractHtmlEdges('index.html', content);

      expect(edges).toHaveLength(1);
      expect(edges[0]).toMatchObject({
        from: 'index.html',
        to: 'styles.css',
        line: 1,
        type: 'import',
        specifier: './styles.css',
      });
    });

    it('should extract <link> with .css extension even without rel="stylesheet"', () => {
      const content = `<link href="./theme.css">`;
      const edges = extractHtmlEdges('index.html', content);

      expect(edges).toHaveLength(1);
      expect(edges[0].specifier).toBe('./theme.css');
    });

    it('should NOT extract <link> for non-stylesheet (e.g., icon)', () => {
      const content = `<link rel="icon" href="./favicon.ico">`;
      const edges = extractHtmlEdges('index.html', content);

      expect(edges).toHaveLength(0);
    });

    it('should skip external stylesheet URLs', () => {
      const content = `<link rel="stylesheet" href="https://fonts.googleapis.com/css">`;
      const edges = extractHtmlEdges('index.html', content);

      expect(edges).toHaveLength(0);
    });

    it('should resolve relative paths', () => {
      const content = `<script src="../scripts/app.js"></script>`;
      const edges = extractHtmlEdges('pages/index.html', content);

      expect(edges).toHaveLength(1);
      expect(edges[0].to).toBe('scripts/app.js');
    });

    it('should handle file with no edges', () => {
      const content = `<html><body><p>Hello</p></body></html>`;
      const edges = extractHtmlEdges('index.html', content);

      expect(edges).toHaveLength(0);
    });

    it('should report correct line numbers', () => {
      const content = `<html>
<head>
  <link rel="stylesheet" href="./style.css">
  <script src="./app.js"></script>
</head>
</html>`;
      const edges = extractHtmlEdges('index.html', content);

      expect(edges).toHaveLength(2);
      expect(edges[0].line).toBe(3);
      expect(edges[1].line).toBe(4);
    });
  });

  describe('extractCssEdges', () => {
    it('should extract @import with double quotes', () => {
      const content = `@import "reset.css";`;
      const edges = extractCssEdges('style.css', content);

      expect(edges).toHaveLength(1);
      expect(edges[0]).toMatchObject({
        from: 'style.css',
        to: 'reset.css',
        line: 1,
        type: 'import',
        specifier: 'reset.css',
      });
    });

    it('should extract @import with single quotes', () => {
      const content = `@import 'typography.css';`;
      const edges = extractCssEdges('style.css', content);

      expect(edges).toHaveLength(1);
      expect(edges[0].specifier).toBe('typography.css');
    });

    it('should extract @import url() with quotes', () => {
      const content = `@import url("fonts.css");`;
      const edges = extractCssEdges('style.css', content);

      expect(edges).toHaveLength(1);
      expect(edges[0].specifier).toBe('fonts.css');
    });

    it('should skip external @import URLs', () => {
      const content = `@import "https://fonts.googleapis.com/css?family=Roboto";`;
      const edges = extractCssEdges('style.css', content);

      expect(edges).toHaveLength(0);
    });

    it('should extract url() references', () => {
      const content = `background: url("./images/bg.png");`;
      const edges = extractCssEdges('style.css', content);

      expect(edges).toHaveLength(1);
      expect(edges[0]).toMatchObject({
        from: 'style.css',
        to: 'images/bg.png',
        line: 1,
        type: 'import',
        specifier: './images/bg.png',
      });
    });

    it('should extract url() without quotes', () => {
      const content = `background: url(./images/bg.png);`;
      const edges = extractCssEdges('style.css', content);

      expect(edges).toHaveLength(1);
      expect(edges[0].specifier).toBe('./images/bg.png');
    });

    it('should skip data: URIs in url()', () => {
      const content = `background: url(data:image/png;base64,abc123);`;
      const edges = extractCssEdges('style.css', content);

      expect(edges).toHaveLength(0);
    });

    it('should skip external URLs in url()', () => {
      const content = `background: url("https://example.com/bg.png");`;
      const edges = extractCssEdges('style.css', content);

      expect(edges).toHaveLength(0);
    });

    it('should resolve relative paths', () => {
      const content = `@import "../shared/variables.css";`;
      const edges = extractCssEdges('components/button.css', content);

      expect(edges).toHaveLength(1);
      expect(edges[0].to).toBe('shared/variables.css');
    });

    it('should handle multiple edges on different lines', () => {
      const content = `@import "reset.css";
@import "variables.css";
body { background: url("./bg.png"); }`;
      const edges = extractCssEdges('style.css', content);

      expect(edges).toHaveLength(3);
      expect(edges[0].line).toBe(1);
      expect(edges[1].line).toBe(2);
      expect(edges[2].line).toBe(3);
    });

    it('should handle file with no edges', () => {
      const content = `body { color: red; }`;
      const edges = extractCssEdges('style.css', content);

      expect(edges).toHaveLength(0);
    });

    it('should not duplicate edges from @import url() lines', () => {
      // @import url("foo.css") should produce one edge, not two
      // (the @import regex matches, but the url() regex should skip lines with @import)
      const content = `@import url("foo.css");`;
      const edges = extractCssEdges('style.css', content);

      expect(edges).toHaveLength(1);
    });
  });
});
