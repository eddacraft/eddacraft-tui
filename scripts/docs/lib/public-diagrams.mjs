import { createHash } from 'node:crypto';
import { readFile, readdir } from 'node:fs/promises';
import { basename, dirname, extname, join, relative, resolve, sep } from 'node:path';

const CONTRACT_PATH = 'scripts/docs/public-diagrams.json';
const PROVENANCE_ID = 'anvil-drawio-provenance';
const LOWER_KEBAB = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const RASTER_EXTENSIONS = new Set(['.png', '.jpg', '.jpeg', '.gif', '.webp']);

export async function loadContract(repoRoot) {
  return JSON.parse(await readFile(resolve(repoRoot, CONTRACT_PATH), 'utf8'));
}

export function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function xmlEscape(value) {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('"', '&quot;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;');
}

function xmlDecode(value) {
  return value
    .replaceAll('&quot;', '"')
    .replaceAll('&gt;', '>')
    .replaceAll('&lt;', '<')
    .replaceAll('&amp;', '&');
}

function attr(text, name) {
  const match = text.match(new RegExp(`\\s${name}="([^"]*)"`));
  return match ? xmlDecode(match[1]) : undefined;
}

function sourceAccessibility(source) {
  const mxfile = source.match(/<mxfile\b[^>]*>/)?.[0] ?? '';
  return {
    title: attr(mxfile, 'anvil-title'),
    description: attr(mxfile, 'anvil-description'),
  };
}

function sourcePageCount(source) {
  return [...source.matchAll(/<diagram\b/g)].length;
}

function stripProvenance(svg) {
  return svg.replace(new RegExp(`\\s*<metadata\\s+id="${PROVENANCE_ID}"[^>]*/>\\s*`), '');
}

export function annotateSvg({ svg, source, sourceName, contract }) {
  const { title, description } = sourceAccessibility(source);
  if (!title || !description) {
    throw new Error(
      'Draw.io source must declare non-empty anvil-title and anvil-description attributes'
    );
  }
  if (sourcePageCount(source) !== 1) {
    throw new Error('Draw.io source must contain exactly one diagram page');
  }
  if (!embeddedDrawioSource(svg)) {
    throw new Error('Draw.io SVG export must contain an embedded source content attribute');
  }

  const idBase = sourceName.replace(/\.drawio$/, '');
  const titleId = `${idBase}-title`;
  const descriptionId = `${idBase}-description`;
  let accessible = svg.replace(
    /<svg\b/,
    `<svg role="img" aria-labelledby="${titleId} ${descriptionId}"`
  );
  accessible = accessible.replace(
    /(<svg\b[^>]*>)/,
    `$1<title id="${titleId}">${xmlEscape(title)}</title><desc id="${descriptionId}">${xmlEscape(description)}</desc>`
  );

  const provenance = [
    `<metadata id="${PROVENANCE_ID}"`,
    ` data-source="${xmlEscape(sourceName)}"`,
    ` data-source-sha256="${sha256(source)}"`,
    ` data-export-sha256="${sha256(accessible)}"`,
    ` data-drawio-version="${xmlEscape(contract.drawioDesktopVersion)}"`,
    ` data-export-args="${xmlEscape(contract.exportArgs.join(' '))}"/>`,
  ].join('');

  return accessible.replace(/(<desc\b[^>]*>.*?<\/desc>)/s, `$1${provenance}`);
}

async function walk(root) {
  const found = [];
  async function visit(directory) {
    let entries;
    try {
      entries = await readdir(directory, { withFileTypes: true });
    } catch (error) {
      if (error.code === 'ENOENT') return;
      throw error;
    }
    for (const entry of entries) {
      const path = join(directory, entry.name);
      if (entry.isDirectory()) await visit(path);
      else if (entry.isFile()) found.push(path);
    }
  }
  await visit(root);
  return found.sort();
}

function finding(code, path, message) {
  return { code, path, message };
}

function provenanceFrom(svg) {
  const metadata = svg.match(new RegExp(`<metadata\\s+id="${PROVENANCE_ID}"[^>]*/>`))?.[0];
  if (!metadata) return undefined;
  return {
    source: attr(metadata, 'data-source'),
    sourceSha256: attr(metadata, 'data-source-sha256'),
    exportSha256: attr(metadata, 'data-export-sha256'),
    version: attr(metadata, 'data-drawio-version'),
    args: attr(metadata, 'data-export-args'),
  };
}

function hasUnsafeSvg(svg) {
  return (
    /<script\b/i.test(svg) ||
    /<foreignObject\b/i.test(svg) ||
    /\son[a-z]+\s*=/i.test(svg) ||
    /\b(?:href|src)\s*=\s*["'](?:https?:|\/\/|javascript:)/i.test(svg)
  );
}

function embeddedDrawioSource(svg) {
  const svgOpen = svg.match(/<svg\b[^>]*>/)?.[0] ?? '';
  const content = attr(svgOpen, 'content');
  return Boolean(content?.trim().startsWith('<mxfile'));
}

function hasAccessibleSvg(svg, expected) {
  const role = /<svg\b[^>]*\srole="img"/.test(svg);
  const labelled = attr(svg.match(/<svg\b[^>]*>/)?.[0] ?? '', 'aria-labelledby');
  const title = svg.match(/<title\s+id="([^"]+)">([^<]+)<\/title>/);
  const description = svg.match(/<desc\s+id="([^"]+)">([^<]+)<\/desc>/);
  return Boolean(
    role &&
    labelled &&
    title &&
    description &&
    labelled.split(/\s+/).includes(title[1]) &&
    labelled.split(/\s+/).includes(description[1]) &&
    xmlDecode(title[2]).trim() === expected.title &&
    xmlDecode(description[2]).trim() === expected.description
  );
}

function referencedWithAlt(svgPath, markdownFiles, markdownByPath) {
  for (const markdownPath of markdownFiles) {
    const markdown = markdownByPath.get(markdownPath);
    const imagePattern = /!\[([^\]]*)\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g;
    for (const match of markdown.matchAll(imagePattern)) {
      const target = match[2].split(/[?#]/, 1)[0];
      if (
        (match[1].trim() || hasAdjacentEquivalentProse(markdown, match.index, match[0].length)) &&
        resolve(dirname(markdownPath), decodeURIComponent(target)) === svgPath
      ) {
        return true;
      }
    }
    const htmlPattern = /<img\b[^>]*\bsrc=["']([^"']+)["'][^>]*\balt=["']([^"']+)["'][^>]*>/gi;
    for (const match of markdown.matchAll(htmlPattern)) {
      const target = match[1].split(/[?#]/, 1)[0];
      if (
        match[2].trim() &&
        resolve(dirname(markdownPath), decodeURIComponent(target)) === svgPath
      ) {
        return true;
      }
    }
  }
  return false;
}

function hasAdjacentEquivalentProse(markdown, imageStart, imageLength) {
  const before = markdown
    .slice(0, imageStart)
    .trimEnd()
    .split(/\n\s*\n/)
    .at(-1)
    ?.trim();
  const after = markdown
    .slice(imageStart + imageLength)
    .trimStart()
    .split(/\n\s*\n/)
    .at(0)
    ?.trim();
  return [before, after].some(
    (paragraph) =>
      paragraph &&
      !paragraph.startsWith('#') &&
      !paragraph.startsWith('![') &&
      !paragraph.startsWith('<img') &&
      paragraph.replace(/[`*_>[\]()]/g, '').trim().length >= 20
  );
}

export async function validatePublicDiagrams(repoRoot, contract) {
  const findings = [];

  for (const family of contract.families) {
    const rendererPath = resolve(repoRoot, family.renderer);
    let renderer = '';
    try {
      renderer = await readFile(rendererPath, 'utf8');
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
    const mount = `../../${family.root}`;
    if (!renderer.includes(`path: '${mount}'`) && !renderer.includes(`path: "${mount}"`)) {
      findings.push(
        finding(
          'family-not-mounted',
          family.renderer,
          `${family.root} is not mounted by its declared production renderer`
        )
      );
    }

    const familyRoot = resolve(repoRoot, family.root);
    const files = await walk(familyRoot);
    const byStem = new Map();
    for (const path of files) {
      const extension = extname(path).toLowerCase();
      if (extension !== '.drawio' && extension !== '.svg' && !RASTER_EXTENSIONS.has(extension)) {
        continue;
      }
      const stem = path.slice(0, -extension.length);
      const values = byStem.get(stem) ?? new Map();
      values.set(extension, path);
      byStem.set(stem, values);
    }

    const markdownFiles = files.filter((path) => /\.mdx?$/.test(path));
    const markdownByPath = new Map(
      await Promise.all(markdownFiles.map(async (path) => [path, await readFile(path, 'utf8')]))
    );

    for (const [stem, variants] of byStem) {
      const display = relative(repoRoot, stem).split(sep).join('/');
      const name = basename(stem);
      if (!LOWER_KEBAB.test(name)) {
        findings.push(
          finding('invalid-name', display, 'diagram basename must be lower-kebab-case')
        );
      }

      for (const extension of RASTER_EXTENSIONS) {
        if (variants.has(extension)) {
          findings.push(
            finding(
              'raster-diagram',
              relative(repoRoot, variants.get(extension)).split(sep).join('/'),
              'governed public diagrams use paired Draw.io and SVG, not raster exports'
            )
          );
        }
      }

      const sourcePath = variants.get('.drawio');
      const svgPath = variants.get('.svg');
      if (!sourcePath || !svgPath) {
        findings.push(
          finding(
            'unpaired-diagram',
            display,
            'governed public diagram requires sibling .drawio and .svg files'
          )
        );
        continue;
      }

      const [source, svg] = await Promise.all([
        readFile(sourcePath, 'utf8'),
        readFile(svgPath, 'utf8'),
      ]);
      const relativeSvg = relative(repoRoot, svgPath).split(sep).join('/');
      const provenance = provenanceFrom(svg);
      const accessibility = sourceAccessibility(source);

      if (sourcePageCount(source) !== 1) {
        findings.push(
          finding(
            'multi-page-source',
            relative(repoRoot, sourcePath).split(sep).join('/'),
            'Draw.io source must contain exactly one diagram page for a sibling SVG export'
          )
        );
      }
      if (!embeddedDrawioSource(svg)) {
        findings.push(
          finding('source-not-embedded', relativeSvg, 'SVG lacks embedded Draw.io source')
        );
      }
      if (hasUnsafeSvg(svg)) {
        findings.push(
          finding('unsafe-svg', relativeSvg, 'SVG contains active content or an external reference')
        );
      }
      if (
        !accessibility.title ||
        !accessibility.description ||
        !hasAccessibleSvg(svg, accessibility)
      ) {
        findings.push(
          finding(
            'inaccessible-svg',
            relativeSvg,
            'SVG title/description must match source anvil-title/anvil-description'
          )
        );
      }
      if (!provenance) {
        findings.push(
          finding('missing-provenance', relativeSvg, 'SVG lacks deterministic export provenance')
        );
      } else {
        if (
          provenance.source !== basename(sourcePath) ||
          provenance.sourceSha256 !== sha256(source)
        ) {
          findings.push(
            finding('stale-source', relativeSvg, 'SVG provenance does not match Draw.io source')
          );
        }
        if (provenance.exportSha256 !== sha256(stripProvenance(svg))) {
          findings.push(
            finding('stale-export', relativeSvg, 'SVG content does not match its provenance hash')
          );
        }
        if (
          provenance.version !== contract.drawioDesktopVersion ||
          provenance.args !== contract.exportArgs.join(' ')
        ) {
          findings.push(
            finding(
              'wrong-exporter',
              relativeSvg,
              'SVG provenance does not match the pinned Draw.io Desktop contract'
            )
          );
        }
      }
      if (!referencedWithAlt(svgPath, markdownFiles, markdownByPath)) {
        findings.push(
          finding(
            'unreferenced-svg',
            relativeSvg,
            'SVG reference requires non-empty alt text or adjacent equivalent prose'
          )
        );
      }
    }
  }

  return findings.sort(
    (left, right) => left.path.localeCompare(right.path) || left.code.localeCompare(right.code)
  );
}
