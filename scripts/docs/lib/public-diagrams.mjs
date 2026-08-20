import { createHash } from 'node:crypto';
import { lstat, readFile, readdir } from 'node:fs/promises';
import { basename, dirname, extname, join, relative, resolve, sep } from 'node:path';
import { Window } from 'happy-dom';
import remarkGfm from 'remark-gfm';
import remarkParse from 'remark-parse';
import ts from 'typescript';
import { unified } from 'unified';

const CONTRACT_PATH = 'scripts/docs/public-diagrams.json';
const PROVENANCE_ID = 'anvil-drawio-provenance';
const LOWER_KEBAB = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const RASTER_EXTENSIONS = new Set(['.png', '.jpg', '.jpeg', '.gif', '.webp']);
const SVG_NAMESPACE = 'http://www.w3.org/2000/svg';
const XML_NAMESPACE = 'http://www.w3.org/XML/1998/namespace';
const XMLNS_NAMESPACE = 'http://www.w3.org/2000/xmlns/';
const SVG_DOM = new Window();

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
  if (/&(?!(?:#x[0-9a-f]+|#[0-9]+|amp|apos|gt|lt|quot);)/i.test(value)) {
    throw new Error('undeclared XML entity');
  }
  return value.replace(/&(#x[0-9a-f]+|#[0-9]+|amp|apos|gt|lt|quot);/gi, (_, entity) => {
    const lower = entity.toLowerCase();
    if (lower === 'amp') return '&';
    if (lower === 'apos') return "'";
    if (lower === 'gt') return '>';
    if (lower === 'lt') return '<';
    if (lower === 'quot') return '"';
    const radix = lower.startsWith('#x') ? 16 : 10;
    const digits = lower.slice(radix === 16 ? 2 : 1);
    const codePoint = Number.parseInt(digits, radix);
    if (!Number.isInteger(codePoint) || codePoint === 0 || codePoint > 0x10ffff) {
      throw new Error('invalid XML character reference');
    }
    return String.fromCodePoint(codePoint);
  });
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

export function canonicalDrawioSource(source) {
  const normalised = source.replace(/\r\n?/g, '\n').trim();
  if (
    /<!DOCTYPE|<!ENTITY|<\?|<!\[CDATA\[/i.test(normalised) ||
    !/^<mxfile\b[\s\S]*<\/mxfile>$/.test(normalised)
  ) {
    throw new Error('embedded Draw.io source is not canonical safe mxfile XML');
  }
  xmlDecode(normalised);
  return normalised.replace(/>\s+</g, '><');
}

function stripProvenance(svg) {
  return svg.replace(new RegExp(`\\s*<metadata\\s+id="${PROVENANCE_ID}"[^>]*/>\\s*`), '');
}

export function annotateSvg({
  svg,
  source,
  sourceName,
  contract,
  actualVersionOutput = contract.drawioDesktopVersionOutput,
}) {
  const { title, description } = sourceAccessibility(source);
  if (!title || !description) {
    throw new Error(
      'Draw.io source must declare non-empty anvil-title and anvil-description attributes'
    );
  }
  if (sourcePageCount(source) !== 1) {
    throw new Error('Draw.io source must contain exactly one diagram page');
  }
  const embedded = embeddedDrawioSource(svg);
  if (!embedded) {
    throw new Error('Draw.io SVG export must contain an embedded source content attribute');
  }
  if (canonicalDrawioSource(embedded) !== canonicalDrawioSource(source)) {
    throw new Error('embedded Draw.io source must match the sibling source');
  }
  if (actualVersionOutput !== contract.drawioDesktopVersionOutput) {
    throw new Error('actual Draw.io Desktop version output does not match the pinned contract');
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
    ` data-embedded-source-sha256="${sha256(canonicalDrawioSource(embedded))}"`,
    ` data-export-sha256="${sha256(accessible)}"`,
    ` data-drawio-version="${xmlEscape(contract.drawioDesktopVersion)}"`,
    ` data-drawio-version-output="${xmlEscape(actualVersionOutput)}"`,
    ` data-export-args="${xmlEscape(contract.exportArgs.join(' '))}"/>`,
  ].join('');

  return accessible.replace(/(<desc\b[^>]*>.*?<\/desc>)/s, `$1${provenance}`);
}

async function walk(root) {
  const found = [];
  const symlinks = [];
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
      if (entry.isSymbolicLink()) symlinks.push(path);
      else if (entry.isDirectory()) await visit(path);
      else if (entry.isFile()) found.push(path);
    }
  }
  try {
    if ((await lstat(root)).isSymbolicLink()) return { files: [], symlinks: [root] };
  } catch (error) {
    if (error.code === 'ENOENT') return { files: [], symlinks: [] };
    throw error;
  }
  await visit(root);
  return { files: found.sort(), symlinks: symlinks.sort() };
}

function finding(code, path, message) {
  return { code, path, message };
}

function mountedPublicRoots(renderer, rendererPath) {
  const source = ts.createSourceFile(
    rendererPath,
    renderer,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TS
  );
  const mounts = new Set();
  function visit(node) {
    if (
      ts.isPropertyAssignment(node) &&
      ((ts.isIdentifier(node.name) && node.name.text === 'path') ||
        (ts.isStringLiteral(node.name) && node.name.text === 'path')) &&
      (ts.isStringLiteral(node.initializer) || ts.isNoSubstitutionTemplateLiteral(node.initializer))
    ) {
      const value = node.initializer.text;
      if (value.startsWith('../../docs/public/')) mounts.add(value);
    }
    ts.forEachChild(node, visit);
  }
  visit(source);
  return mounts;
}

function provenanceFrom(svg) {
  const metadata = svg.match(new RegExp(`<metadata\\s+id="${PROVENANCE_ID}"[^>]*/>`))?.[0];
  if (!metadata) return undefined;
  return {
    source: attr(metadata, 'data-source'),
    sourceSha256: attr(metadata, 'data-source-sha256'),
    embeddedSourceSha256: attr(metadata, 'data-embedded-source-sha256'),
    exportSha256: attr(metadata, 'data-export-sha256'),
    version: attr(metadata, 'data-drawio-version'),
    versionOutput: attr(metadata, 'data-drawio-version-output'),
    args: attr(metadata, 'data-export-args'),
  };
}

const SAFE_SVG_ELEMENTS = new Set([
  'a',
  'circle',
  'clippath',
  'defs',
  'desc',
  'ellipse',
  'feblend',
  'fecolormatrix',
  'fecomposite',
  'fegaussianblur',
  'feoffset',
  'filter',
  'g',
  'image',
  'line',
  'lineargradient',
  'marker',
  'mask',
  'metadata',
  'path',
  'pattern',
  'polygon',
  'polyline',
  'radialgradient',
  'rect',
  'stop',
  'style',
  'svg',
  'switch',
  'symbol',
  'text',
  'title',
  'tspan',
  'use',
]);

function decodeCssEscapes(value) {
  return value
    .replace(/\\([0-9a-f]{1,6})\s?/gi, (_, digits) =>
      String.fromCodePoint(Number.parseInt(digits, 16))
    )
    .replace(/\\(.)/gs, '$1');
}

function normaliseReference(value) {
  let decoded = decodeCssEscapes(value).trim();
  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      const next = decodeURIComponent(decoded);
      if (next === decoded) break;
      decoded = next;
    } catch {
      return undefined;
    }
  }
  return decoded.replace(/[\u0000-\u0020\u007f]+/g, '');
}

function unsafeCss(value) {
  const css = decodeCssEscapes(value);
  if (/@import\b|expression\s*\(|(?:^|[;{])\s*(?:behavior|-moz-binding)\s*:/i.test(css)) {
    return true;
  }
  for (const match of css.matchAll(/url\s*\(([^)]*)\)/gi)) {
    const raw = match[1].trim().replace(/^(["'])(.*)\1$/s, '$2');
    const reference = normaliseReference(raw);
    if (!reference || !/^#[A-Za-z_][A-Za-z0-9_.:-]*$/.test(reference)) return true;
  }
  return false;
}

function hasUnsafeSvg(svg) {
  try {
    if (/<!DOCTYPE|<!ENTITY|<\?|<!\[CDATA\[/i.test(svg)) return true;
    for (const entity of svg.matchAll(/&([^;\s]+);/g)) {
      xmlDecode(entity[0]);
    }
    const document = new SVG_DOM.DOMParser().parseFromString(svg, 'image/svg+xml');
    if (
      document.querySelector('parsererror') ||
      document.documentElement?.localName !== 'svg' ||
      document.documentElement.namespaceURI !== SVG_NAMESPACE
    ) {
      return true;
    }

    for (const element of [document.documentElement, ...document.querySelectorAll('*')]) {
      const lowerName = element.localName.toLowerCase();
      if (element.namespaceURI !== SVG_NAMESPACE || !SAFE_SVG_ELEMENTS.has(lowerName)) return true;
      if (lowerName === 'style' && unsafeCss(element.textContent ?? '')) return true;

      for (const attribute of element.attributes) {
        const lowerAttribute = attribute.localName.toLowerCase();
        const qualifiedName = attribute.name.toLowerCase();
        const value = attribute.value;
        if (/^on/i.test(qualifiedName)) return true;
        if (attribute.namespaceURI === XMLNS_NAMESPACE) {
          if (
            (qualifiedName === 'xmlns' && value !== SVG_NAMESPACE) ||
            (qualifiedName === 'xmlns:xlink' && value !== 'http://www.w3.org/1999/xlink') ||
            (qualifiedName !== 'xmlns' && qualifiedName !== 'xmlns:xlink')
          ) {
            return true;
          }
          continue;
        }
        if (
          attribute.namespaceURI &&
          !(attribute.namespaceURI === XML_NAMESPACE && qualifiedName === 'xml:space')
        ) {
          return true;
        }
        if (lowerAttribute === 'href' || lowerAttribute === 'src') {
          const reference = normaliseReference(value);
          if (!reference || !/^#[A-Za-z_][A-Za-z0-9_.:-]*$/.test(reference)) return true;
        }
        if (lowerAttribute === 'style' && unsafeCss(value)) return true;
      }
    }
    return false;
  } catch {
    return true;
  }
}

function embeddedDrawioSource(svg) {
  const svgOpen = svg.match(/<svg\b[^>]*>/)?.[0] ?? '';
  const content = attr(svgOpen, 'content');
  return content?.trim().startsWith('<mxfile') ? content : undefined;
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

function nodeText(node) {
  if (typeof node.value === 'string' && (node.type === 'text' || node.type === 'inlineCode')) {
    return node.value;
  }
  return (node.children ?? []).map(nodeText).join(' ');
}

function meaningfulText(value) {
  const words = value.trim().match(/[\p{L}\p{N}]+/gu) ?? [];
  return value.trim().length >= 10 && words.length >= 2;
}

function htmlAttributes(value) {
  const attributes = new Map();
  for (const match of value.matchAll(/([A-Za-z_:][\w:.-]*)\s*=\s*(?:"([^"]*)"|'([^']*)')/g)) {
    attributes.set(match[1].toLowerCase(), match[2] ?? match[3] ?? '');
  }
  return attributes;
}

function markdownImageReferences(markdown) {
  const tree = unified().use(remarkParse).use(remarkGfm).parse(markdown);
  const definitions = new Map(
    tree.children
      .filter((node) => node.type === 'definition')
      .map((node) => [node.identifier.toLowerCase(), node.url])
  );
  const references = [];
  for (const [index, block] of tree.children.entries()) {
    const images = [];
    function collect(node) {
      if (node.type === 'code' || node.type === 'inlineCode' || node.type === 'html') return;
      if (node.type === 'image' || node.type === 'imageReference') images.push(node);
      for (const child of node.children ?? []) collect(child);
    }
    collect(block);
    for (const image of images) {
      const target =
        image.type === 'image' ? image.url : definitions.get(image.identifier.toLowerCase());
      if (!target) continue;
      const marker = tree.children[index - 2];
      const description = tree.children[index - 1];
      const associated =
        marker?.type === 'html' &&
        marker.value.trim() === '<!-- diagram-description: ' + target + ' -->' &&
        description?.type === 'paragraph' &&
        meaningfulText(nodeText(description));
      references.push({ target, alt: image.alt ?? '', associated });
    }
    if (block.type === 'html' && !block.value.trimStart().startsWith('<!--')) {
      for (const match of block.value.matchAll(/<img\b[^>]*>/gi)) {
        const attributes = htmlAttributes(match[0]);
        if (attributes.has('src')) {
          references.push({
            target: attributes.get('src'),
            alt: attributes.get('alt') ?? '',
            associated: false,
          });
        }
      }
    }
  }
  return references;
}

function referencedWithAlt(svgPath, markdownFiles, markdownByPath) {
  for (const markdownPath of markdownFiles) {
    const markdown = markdownByPath.get(markdownPath);
    for (const reference of markdownImageReferences(markdown)) {
      const target = reference.target.split(/[?#]/, 1)[0];
      try {
        if (
          (meaningfulText(reference.alt) || reference.associated) &&
          resolve(dirname(markdownPath), decodeURIComponent(target)) === svgPath
        ) {
          return true;
        }
      } catch {
        continue;
      }
    }
  }
  return false;
}

export async function validatePublicDiagrams(repoRoot, contract) {
  const findings = [];

  const expectedByRenderer = new Map(
    contract.productionRenderers.map((renderer) => [renderer, new Set()])
  );
  for (const family of contract.families) {
    const expected = expectedByRenderer.get(family.renderer);
    if (!expected) {
      findings.push(
        finding(
          'undeclared-production-renderer',
          family.renderer,
          `${family.root} maps to a renderer outside productionRenderers`
        )
      );
      continue;
    }
    expected.add(`../../${family.root}`);
  }
  for (const [rendererPath, expected] of expectedByRenderer) {
    let renderer = '';
    try {
      renderer = await readFile(resolve(repoRoot, rendererPath), 'utf8');
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
    const actual = mountedPublicRoots(renderer, rendererPath);
    for (const mount of expected) {
      if (!actual.has(mount)) {
        findings.push(
          finding(
            'family-not-mounted',
            rendererPath,
            `${mount.slice(6)} is not structurally mounted by its declared production renderer`
          )
        );
      }
    }
    for (const mount of actual) {
      if (!expected.has(mount)) {
        findings.push(
          finding(
            'undeclared-family-mount',
            rendererPath,
            `${mount.slice(6)} is mounted by production but absent from the manifest mapping`
          )
        );
      }
    }
  }

  for (const family of contract.families) {
    const familyRoot = resolve(repoRoot, family.root);
    const { files, symlinks } = await walk(familyRoot);
    for (const path of symlinks) {
      findings.push(
        finding(
          'symlink-path',
          relative(repoRoot, path).split(sep).join('/'),
          'governed diagram paths must not contain or traverse symlinks'
        )
      );
    }
    const byStem = new Map();
    const rasterFiles = [];
    for (const path of files) {
      const extension = extname(path).toLowerCase();
      if (RASTER_EXTENSIONS.has(extension)) {
        rasterFiles.push(path);
        continue;
      }
      if (extension !== '.drawio' && extension !== '.svg') continue;
      const stem = path.slice(0, -extension.length);
      const values = byStem.get(stem) ?? new Map();
      values.set(extension, path);
      byStem.set(stem, values);
    }
    for (const path of rasterFiles) {
      const extension = extname(path).toLowerCase();
      const stem = path.slice(0, -extension.length);
      const familyDirectory = relative(familyRoot, dirname(path)).split(sep);
      const isCandidate = byStem.has(stem) || familyDirectory.includes('diagrams');
      if (!isCandidate) continue;
      const relativePath = relative(repoRoot, path).split(sep).join('/');
      const exception = (contract.rasterExceptions ?? []).find(
        (candidate) => candidate.path === relativePath
      );
      if (exception && exception.consumer?.trim() && exception.reviewedAgainst === 'ADR-123') {
        continue;
      }
      findings.push(
        finding(
          exception ? 'invalid-raster-exception' : 'raster-diagram',
          relativePath,
          exception
            ? 'raster exception requires a consumer reason and ADR-123 review'
            : 'governed diagram candidates require paired Draw.io/SVG or a reviewed raster exception'
        )
      );
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
      const embedded = embeddedDrawioSource(svg);
      if (!embedded) {
        findings.push(
          finding('source-not-embedded', relativeSvg, 'SVG lacks embedded Draw.io source')
        );
      } else {
        try {
          if (canonicalDrawioSource(embedded) !== canonicalDrawioSource(source)) {
            findings.push(
              finding(
                'embedded-source-mismatch',
                relativeSvg,
                'embedded Draw.io source does not match its sibling source'
              )
            );
          }
        } catch {
          findings.push(
            finding(
              'embedded-source-mismatch',
              relativeSvg,
              'embedded Draw.io source is invalid or does not match its sibling source'
            )
          );
        }
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
        let canonicalSourceSha256;
        try {
          canonicalSourceSha256 = sha256(canonicalDrawioSource(source));
        } catch {
          canonicalSourceSha256 = undefined;
        }
        if (
          provenance.source !== basename(sourcePath) ||
          provenance.sourceSha256 !== sha256(source) ||
          provenance.embeddedSourceSha256 !== canonicalSourceSha256
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
          provenance.versionOutput !== contract.drawioDesktopVersionOutput ||
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
            'SVG reference requires meaningful alt text or an explicit adjacent description association'
          )
        );
      }
    }
  }

  return findings.sort(
    (left, right) => left.path.localeCompare(right.path) || left.code.localeCompare(right.code)
  );
}
