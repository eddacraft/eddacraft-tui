import { createHash } from 'node:crypto';
import { lstat, readFile, readdir } from 'node:fs/promises';
import { basename, dirname, extname, join, relative, resolve, sep } from 'node:path';
import { Window } from 'happy-dom';
import remarkGfm from 'remark-gfm';
import remarkParse from 'remark-parse';
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

function parseDrawioSource(source) {
  const normalised = source.replace(/\r\n?/g, '\n').trim();
  if (/<!DOCTYPE|<!ENTITY|<\?|<!\[CDATA\[/i.test(normalised)) {
    throw new Error('embedded Draw.io source is not canonical safe mxfile XML');
  }
  const document = new SVG_DOM.DOMParser().parseFromString(normalised, 'application/xml');
  const root = document.documentElement;
  if (
    document.querySelector('parsererror') ||
    !root ||
    root.localName !== 'mxfile' ||
    root.namespaceURI
  ) {
    throw new Error('embedded Draw.io source must have one non-namespaced mxfile root');
  }
  const elements = [root, ...root.querySelectorAll('*')];
  for (const element of elements) {
    if (element.namespaceURI)
      throw new Error('Draw.io source must not contain namespaced elements');
    for (const attribute of element.attributes) {
      if (attribute.namespaceURI) {
        throw new Error('Draw.io source must not contain namespaced attributes');
      }
    }
  }
  const pages = [...root.children].filter((element) => element.localName === 'diagram');
  if (pages.length !== 1 || root.children.length !== 1) {
    throw new Error('Draw.io source must contain exactly one diagram page');
  }
  const diagram = pages[0];
  if (!diagram.getAttribute('id')?.trim()) {
    throw new Error('Draw.io diagram page must have a non-empty id');
  }
  const elementChildren = [...diagram.children];
  if (elementChildren.length === 0) {
    if (!diagram.textContent?.trim()) {
      throw new Error('Draw.io diagram page must contain a graph model or compressed content');
    }
  } else {
    if (elementChildren.length !== 1 || elementChildren[0].localName !== 'mxGraphModel') {
      throw new Error('Draw.io diagram page must contain exactly one graph model');
    }
    const graphModel = elementChildren[0];
    const graphRoots = [...graphModel.children].filter((element) => element.localName === 'root');
    if (
      graphRoots.length !== 1 ||
      graphModel.children.length !== 1 ||
      graphRoots[0].querySelectorAll('mxCell').length === 0
    ) {
      throw new Error('Draw.io graph model must contain exactly one populated root');
    }
  }
  return { root, diagram };
}

function canonicalXmlNode(node) {
  if (node.nodeType === 3) {
    const value = node.data.trim();
    return value ? xmlEscape(value) : '';
  }
  if (node.nodeType !== 1) {
    throw new Error('Draw.io source contains unsupported XML nodes');
  }
  const attributes = [...node.attributes]
    .sort((left, right) => left.name.localeCompare(right.name))
    .map((attribute) => ` ${attribute.name}="${xmlEscape(attribute.value)}"`)
    .join('');
  const content = [...node.childNodes].map(canonicalXmlNode).join('');
  return content
    ? `<${node.localName}${attributes}>${content}</${node.localName}>`
    : `<${node.localName}${attributes}/>`;
}

function sourceAccessibility(source) {
  try {
    const { root } = parseDrawioSource(source);
    return {
      title: root.getAttribute('anvil-title') ?? undefined,
      description: root.getAttribute('anvil-description') ?? undefined,
    };
  } catch {
    return {};
  }
}

function sourcePageCount(source) {
  try {
    parseDrawioSource(source);
    return 1;
  } catch {
    return 0;
  }
}

export function canonicalDrawioSource(source) {
  return canonicalXmlNode(parseDrawioSource(source).root);
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

async function symlinkAncestors(repoRoot, target) {
  const path = relative(repoRoot, target);
  if (path === '..' || path.startsWith(`..${sep}`) || resolve(repoRoot, path) !== target) {
    return [target];
  }
  const symlinks = [];
  let current = repoRoot;
  const candidates = [
    repoRoot,
    ...path
      .split(sep)
      .filter(Boolean)
      .map((part) => {
        current = join(current, part);
        return current;
      }),
  ];
  for (const candidate of candidates) {
    try {
      if ((await lstat(candidate)).isSymbolicLink()) {
        symlinks.push(candidate);
        break;
      }
    } catch (error) {
      if (error.code === 'ENOENT') break;
      throw error;
    }
  }
  return symlinks;
}

function finding(code, path, message) {
  return { code, path, message };
}

function contractFindings(contract) {
  const issues = [];
  const unique = (values) => new Set(values).size === values.length;
  const familyRoots = (contract.families ?? []).map(({ root }) => root);
  const familyNames = (contract.families ?? []).map(({ name }) => name);
  const diagramDirectories = contract.diagramDirectories ?? [];
  const expectedDiagramDirectories = familyRoots.map((root) => `${root}/assets/diagrams`);
  if (
    !unique(familyRoots) ||
    !unique(familyNames) ||
    !unique(diagramDirectories) ||
    diagramDirectories.length !== expectedDiagramDirectories.length ||
    diagramDirectories.some((directory) => !expectedDiagramDirectories.includes(directory))
  ) {
    issues.push(
      finding(
        'invalid-contract',
        CONTRACT_PATH,
        'manifest family roots, names and diagram directories must be unique and correspond exactly'
      )
    );
  }
  return issues;
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
        if (lowerAttribute === 'ping') return true;
        if (lowerAttribute === 'href' || lowerAttribute === 'src') {
          const reference = normaliseReference(value);
          if (!reference || !/^#[A-Za-z_][A-Za-z0-9_.:-]*$/.test(reference)) return true;
        }
        if (lowerAttribute === 'style' && unsafeCss(value)) return true;
        if (/url\s*\(/i.test(decodeCssEscapes(value)) && unsafeCss(value)) return true;
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

function rawHtmlImageReferences(value) {
  const document = new SVG_DOM.DOMParser().parseFromString(value, 'text/html');
  const references = [];
  for (const image of document.querySelectorAll('img')) {
    let visible = true;
    for (let element = image; element; element = element.parentElement) {
      const style = element.getAttribute('style') ?? '';
      if (
        element.hasAttribute('hidden') ||
        element.getAttribute('aria-hidden')?.toLowerCase() === 'true' ||
        /(?:^|;)\s*(?:display\s*:\s*none|visibility\s*:\s*hidden)\s*(?:;|$)/i.test(style)
      ) {
        visible = false;
        break;
      }
    }
    if (visible && image.hasAttribute('src')) {
      references.push({
        target: image.getAttribute('src'),
        alt: image.getAttribute('alt') ?? '',
        associated: false,
      });
    }
  }
  return references;
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
      references.push(...rawHtmlImageReferences(block.value));
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
  const findings = contractFindings(contract);

  for (const family of contract.families) {
    const familyRoot = resolve(repoRoot, family.root);
    const { files: familyFiles } = await walk(familyRoot);
    const diagramRoots = (contract.diagramDirectories ?? [])
      .filter((directory) => directory.startsWith(`${family.root}/`))
      .map((directory) => resolve(repoRoot, directory));
    const files = [];
    const symlinks = new Set();
    for (const diagramRoot of diagramRoots) {
      const ancestors = await symlinkAncestors(repoRoot, diagramRoot);
      if (ancestors.length > 0) {
        ancestors.forEach((path) => symlinks.add(path));
        continue;
      }
      const governed = await walk(diagramRoot);
      governed.files.forEach((path) => files.push(path));
      governed.symlinks.forEach((path) => symlinks.add(path));
    }
    for (const path of symlinks) {
      findings.push(
        finding(
          'symlink-path',
          relative(repoRoot, path).split(sep).join('/'),
          'governed diagram paths must not contain or traverse symlinks'
        )
      );
    }
    const markdownFiles = familyFiles.filter((path) => /\.mdx?$/.test(path));
    const markdownByPath = new Map(
      await Promise.all(markdownFiles.map(async (path) => [path, await readFile(path, 'utf8')]))
    );
    const byStem = new Map();
    const rasterFiles = [];
    for (const path of files) {
      const rawExtension = extname(path);
      const extension = rawExtension.toLowerCase();
      if (
        rawExtension !== extension &&
        (RASTER_EXTENSIONS.has(extension) || extension === '.drawio' || extension === '.svg')
      ) {
        findings.push(
          finding(
            'invalid-extension',
            relative(repoRoot, path).split(sep).join('/'),
            'governed diagram file extensions must be lower-case'
          )
        );
      }
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
      const relativePath = relative(repoRoot, path).split(sep).join('/');
      const exception = (contract.rasterExceptions ?? []).find(
        (candidate) => candidate.path === relativePath
      );
      if (
        exception &&
        exception.consumer?.trim() &&
        exception.reviewedAgainst === 'ADR-123' &&
        referencedWithAlt(path, markdownFiles, markdownByPath)
      ) {
        continue;
      }
      findings.push(
        finding(
          exception ? 'invalid-raster-exception' : 'raster-diagram',
          relativePath,
          exception
            ? 'raster exception requires a consumer reason, ADR-123 review and an accessible real reference'
            : 'governed diagram candidates require paired Draw.io/SVG or a reviewed raster exception'
        )
      );
    }

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
