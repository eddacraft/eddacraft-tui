use std::time::Instant;

use tree_sitter::StreamingIterator;

fn main() {
    println!("=== KERN-001: tree-sitter TypeScript/JavaScript parsing spike ===\n");

    let mut parser = tree_sitter::Parser::new();

    let ts_language: tree_sitter::Language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
    let js_language: tree_sitter::Language = tree_sitter_javascript::LANGUAGE.into();

    parser
        .set_language(&ts_language)
        .expect("failed to set TypeScript language");

    let samples: &[(&str, &str)] = &[
        ("tiny (10 LOC)", SAMPLE_TINY),
        ("small (50 LOC)", SAMPLE_SMALL),
        ("medium (200 LOC)", SAMPLE_MEDIUM),
        ("large (500 LOC)", SAMPLE_LARGE),
    ];

    println!("--- TypeScript parsing ---\n");
    for (label, source) in samples {
        benchmark_parse(&mut parser, label, source);
    }

    let js_samples: &[(&str, &str)] = &[
        ("tiny (10 LOC)", JS_SAMPLE_TINY),
        ("small (50 LOC)", JS_SAMPLE_SMALL),
        ("medium (200 LOC)", JS_SAMPLE_MEDIUM),
        ("large (500 LOC)", JS_SAMPLE_LARGE),
    ];

    println!("\n--- JavaScript parsing ---\n");
    parser
        .set_language(&js_language)
        .expect("failed to set JavaScript language");
    for (label, source) in js_samples {
        benchmark_parse(&mut parser, label, source);
    }

    println!("\n--- Symbol extraction via tree-sitter queries ---\n");
    parser
        .set_language(&ts_language)
        .expect("failed to set TypeScript language");
    benchmark_symbol_extraction(&mut parser, &ts_language);

    println!("\n=== Spike complete ===");
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn benchmark_parse(parser: &mut tree_sitter::Parser, label: &str, source: &str) {
    let iterations = 1000;
    let source_bytes = source.as_bytes();

    for _ in 0..100 {
        let _ = parser.parse(source_bytes, None);
    }

    let mut durations = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        let tree = parser.parse(source_bytes, None).expect("parse failed");
        let elapsed = start.elapsed();
        durations.push(elapsed);
        drop(tree);
    }

    durations.sort();
    let median = durations[iterations / 2];
    let p99 = durations[((iterations as f64 * 0.99) as usize).min(durations.len() - 1)];
    let mean: std::time::Duration =
        durations.iter().sum::<std::time::Duration>() / iterations as u32;

    let loc = source.lines().count();
    println!(
        "  {label} ({loc} LOC): mean={mean:.1?}, median={median:.1?}, p99={p99:.1?}  [target: <1ms]"
    );

    if p99 < std::time::Duration::from_millis(1) {
        println!("    ✓ PASS — p99 under 1ms");
    } else {
        println!("    ✗ FAIL — p99 exceeds 1ms");
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn benchmark_symbol_extraction(parser: &mut tree_sitter::Parser, language: &tree_sitter::Language) {
    let source = SAMPLE_MEDIUM;
    let tree = parser.parse(source.as_bytes(), None).expect("parse failed");
    let root = tree.root_node();

    let query = tree_sitter::Query::new(
        language,
        r"
        (function_declaration name: (identifier) @fn.name)
        (class_declaration name: (type_identifier) @class.name)
        (export_statement) @export
        (import_statement) @import
        ",
    )
    .expect("invalid query");

    let iterations = 1000;
    let mut durations = Vec::with_capacity(iterations);
    let mut symbol_count = 0;

    for i in 0..iterations {
        let mut cursor = tree_sitter::QueryCursor::new();
        let start = Instant::now();
        // tree-sitter 0.26: QueryMatches implements StreamingIterator, not Iterator.
        // Use while-let .next() loop instead of .collect().
        let mut matches = cursor.matches(&query, root, source.as_bytes());
        let mut match_count = 0_usize;
        let mut capture_count = 0_usize;
        while let Some(m) = matches.next() {
            match_count += 1;
            capture_count += m.captures.len();
        }
        let elapsed = start.elapsed();
        durations.push(elapsed);

        if i == 0 {
            symbol_count = capture_count;
            let _ = match_count;
        }
    }

    durations.sort();
    let median = durations[iterations / 2];
    let p99 = durations[((iterations as f64 * 0.99) as usize).min(durations.len() - 1)];

    println!("  Query extraction ({symbol_count} symbols): median={median:.1?}, p99={p99:.1?}");
}

const SAMPLE_TINY: &str = r"
import { readFileSync } from 'node:fs';

export function greet(name: string): string {
  return `Hello, ${name}!`;
}

export const VERSION = '1.0.0';

console.log(greet('world'));
";

const SAMPLE_SMALL: &str = r"
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

export interface Config {
  name: string;
  version: string;
  debug: boolean;
}

export class ConfigLoader {
  private path: string;

  constructor(path: string) {
    this.path = path;
  }

  load(): Config {
    const raw = readFileSync(this.path, 'utf-8');
    return JSON.parse(raw) as Config;
  }

  save(config: Config): void {
    writeFileSync(this.path, JSON.stringify(config, null, 2));
  }

  validate(config: Config): boolean {
    if (!config.name) return false;
    if (!config.version) return false;
    return true;
  }
}

export function loadConfig(dir: string): Config {
  const loader = new ConfigLoader(join(dir, 'config.json'));
  const config = loader.load();
  if (!loader.validate(config)) {
    throw new Error('Invalid config');
  }
  return config;
}

export function mergeConfigs(base: Config, override_: Partial<Config>): Config {
  return { ...base, ...override_ };
}

export const DEFAULT_CONFIG: Config = {
  name: 'default',
  version: '0.0.0',
  debug: false,
};
";

const SAMPLE_MEDIUM: &str = r"
import { EventEmitter } from 'node:events';
import { readFileSync, existsSync } from 'node:fs';
import { join, resolve, relative } from 'node:path';

export interface SymbolNode {
  id: string;
  kind: 'function' | 'class' | 'module' | 'export';
  name: string;
  visibility: 'public' | 'internal';
  file: string;
  trustLevel: TrustLevel;
}

export enum TrustLevel {
  Unknown = 'unknown',
  Internal = 'internal',
  Boundary = 'boundary',
  External = 'external',
  Privileged = 'privileged',
}

export interface SymbolEdge {
  from: string;
  to: string;
  edgeType: 'contains' | 'references' | 'calls' | 'imports';
}

export class SymbolGraph extends EventEmitter {
  private nodes: Map<string, SymbolNode> = new Map();
  private edges: SymbolEdge[] = [];
  private fileIndex: Map<string, Set<string>> = new Map();

  addNode(node: SymbolNode): void {
    this.nodes.set(node.id, node);
    const fileNodes = this.fileIndex.get(node.file) ?? new Set();
    fileNodes.add(node.id);
    this.fileIndex.set(node.file, fileNodes);
    this.emit('node:added', node);
  }

  removeNode(id: string): boolean {
    const node = this.nodes.get(id);
    if (!node) return false;
    this.nodes.delete(id);
    this.edges = this.edges.filter(e => e.from !== id && e.to !== id);
    const fileNodes = this.fileIndex.get(node.file);
    if (fileNodes) {
      fileNodes.delete(id);
      if (fileNodes.size === 0) this.fileIndex.delete(node.file);
    }
    this.emit('node:removed', node);
    return true;
  }

  addEdge(edge: SymbolEdge): void {
    this.edges.push(edge);
    this.emit('edge:added', edge);
  }

  getNode(id: string): SymbolNode | undefined {
    return this.nodes.get(id);
  }

  getNodesByFile(file: string): SymbolNode[] {
    const ids = this.fileIndex.get(file);
    if (!ids) return [];
    return [...ids].map(id => this.nodes.get(id)!).filter(Boolean);
  }

  getEdgesFrom(nodeId: string): SymbolEdge[] {
    return this.edges.filter(e => e.from === nodeId);
  }

  getEdgesTo(nodeId: string): SymbolEdge[] {
    return this.edges.filter(e => e.to === nodeId);
  }

  get nodeCount(): number { return this.nodes.size; }
  get edgeCount(): number { return this.edges.length; }

  clear(): void {
    this.nodes.clear();
    this.edges = [];
    this.fileIndex.clear();
    this.emit('graph:cleared');
  }

  toJSON(): { nodes: SymbolNode[]; edges: SymbolEdge[] } {
    return {
      nodes: [...this.nodes.values()],
      edges: [...this.edges],
    };
  }
}

export class GraphAnalyser {
  constructor(private graph: SymbolGraph) {}

  findOrphans(): SymbolNode[] {
    const referenced = new Set<string>();
    const json = this.graph.toJSON();
    for (const edge of json.edges) {
      referenced.add(edge.from);
      referenced.add(edge.to);
    }
    return json.nodes.filter(n => !referenced.has(n.id));
  }

  findCycles(): string[][] {
    const json = this.graph.toJSON();
    const adj = new Map<string, string[]>();
    for (const edge of json.edges) {
      const targets = adj.get(edge.from) ?? [];
      targets.push(edge.to);
      adj.set(edge.from, targets);
    }
    const cycles: string[][] = [];
    const visited = new Set<string>();
    const stack = new Set<string>();

    function dfs(node: string, path: string[]): void {
      if (stack.has(node)) {
        const cycleStart = path.indexOf(node);
        cycles.push(path.slice(cycleStart));
        return;
      }
      if (visited.has(node)) return;
      visited.add(node);
      stack.add(node);
      for (const next of adj.get(node) ?? []) {
        dfs(next, [...path, node]);
      }
      stack.delete(node);
    }

    for (const node of json.nodes) {
      dfs(node.id, []);
    }
    return cycles;
  }

  getBoundaryNodes(): SymbolNode[] {
    const json = this.graph.toJSON();
    return json.nodes.filter(n => n.trustLevel === TrustLevel.Boundary);
  }

  getExternalDependencies(): SymbolEdge[] {
    const json = this.graph.toJSON();
    return json.edges.filter(e => {
      const target = this.graph.getNode(e.to);
      return target?.trustLevel === TrustLevel.External;
    });
  }
}

export function createGraph(): SymbolGraph {
  return new SymbolGraph();
}

export function loadGraphFromFile(path: string): SymbolGraph {
  if (!existsSync(path)) throw new Error(`Graph file not found: ${path}`);
  const raw = readFileSync(path, 'utf-8');
  const data = JSON.parse(raw);
  const graph = new SymbolGraph();
  for (const node of data.nodes ?? []) graph.addNode(node);
  for (const edge of data.edges ?? []) graph.addEdge(edge);
  return graph;
}
";

// JavaScript-valid equivalents — no TS-only constructs (type annotations, interfaces, enums)
const JS_SAMPLE_TINY: &str = r"
import { readFileSync } from 'node:fs';

export function greet(name) {
  return `Hello, ${name}!`;
}

export const VERSION = '1.0.0';

console.log(greet('world'));
";

const JS_SAMPLE_SMALL: &str = r"
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

export class ConfigLoader {
  #path;

  constructor(path) {
    this.#path = path;
  }

  load() {
    const raw = readFileSync(this.#path, 'utf-8');
    return JSON.parse(raw);
  }

  save(config) {
    writeFileSync(this.#path, JSON.stringify(config, null, 2));
  }

  validate(config) {
    if (!config.name) return false;
    if (!config.version) return false;
    return true;
  }
}

export function loadConfig(dir) {
  const loader = new ConfigLoader(join(dir, 'config.json'));
  const config = loader.load();
  if (!loader.validate(config)) {
    throw new Error('Invalid config');
  }
  return config;
}

export function mergeConfigs(base, override_) {
  return { ...base, ...override_ };
}

export const DEFAULT_CONFIG = {
  name: 'default',
  version: '0.0.0',
  debug: false,
};
";

const JS_SAMPLE_MEDIUM: &str = r"
import { EventEmitter } from 'node:events';
import { readFileSync, existsSync } from 'node:fs';
import { join, resolve, relative } from 'node:path';

const TrustLevel = Object.freeze({
  Unknown: 'unknown',
  Internal: 'internal',
  Boundary: 'boundary',
  External: 'external',
  Privileged: 'privileged',
});

export class SymbolGraph extends EventEmitter {
  #nodes = new Map();
  #edges = [];
  #fileIndex = new Map();

  addNode(node) {
    this.#nodes.set(node.id, node);
    const fileNodes = this.#fileIndex.get(node.file) ?? new Set();
    fileNodes.add(node.id);
    this.#fileIndex.set(node.file, fileNodes);
    this.emit('node:added', node);
  }

  removeNode(id) {
    const node = this.#nodes.get(id);
    if (!node) return false;
    this.#nodes.delete(id);
    this.#edges = this.#edges.filter(e => e.from !== id && e.to !== id);
    const fileNodes = this.#fileIndex.get(node.file);
    if (fileNodes) {
      fileNodes.delete(id);
      if (fileNodes.size === 0) this.#fileIndex.delete(node.file);
    }
    this.emit('node:removed', node);
    return true;
  }

  addEdge(edge) {
    this.#edges.push(edge);
    this.emit('edge:added', edge);
  }

  getNode(id) {
    return this.#nodes.get(id);
  }

  getNodesByFile(file) {
    const ids = this.#fileIndex.get(file);
    if (!ids) return [];
    return [...ids].map(id => this.#nodes.get(id)).filter(Boolean);
  }

  getEdgesFrom(nodeId) {
    return this.#edges.filter(e => e.from === nodeId);
  }

  getEdgesTo(nodeId) {
    return this.#edges.filter(e => e.to === nodeId);
  }

  get nodeCount() { return this.#nodes.size; }
  get edgeCount() { return this.#edges.length; }

  clear() {
    this.#nodes.clear();
    this.#edges = [];
    this.#fileIndex.clear();
    this.emit('graph:cleared');
  }

  toJSON() {
    return {
      nodes: [...this.#nodes.values()],
      edges: [...this.#edges],
    };
  }
}

export class GraphAnalyser {
  #graph;

  constructor(graph) {
    this.#graph = graph;
  }

  findOrphans() {
    const referenced = new Set();
    const json = this.#graph.toJSON();
    for (const edge of json.edges) {
      referenced.add(edge.from);
      referenced.add(edge.to);
    }
    return json.nodes.filter(n => !referenced.has(n.id));
  }

  findCycles() {
    const json = this.#graph.toJSON();
    const adj = new Map();
    for (const edge of json.edges) {
      const targets = adj.get(edge.from) ?? [];
      targets.push(edge.to);
      adj.set(edge.from, targets);
    }
    const cycles = [];
    const visited = new Set();
    const stack = new Set();

    function dfs(node, path) {
      if (stack.has(node)) {
        const cycleStart = path.indexOf(node);
        cycles.push(path.slice(cycleStart));
        return;
      }
      if (visited.has(node)) return;
      visited.add(node);
      stack.add(node);
      for (const next of adj.get(node) ?? []) {
        dfs(next, [...path, node]);
      }
      stack.delete(node);
    }

    for (const node of json.nodes) {
      dfs(node.id, []);
    }
    return cycles;
  }

  getBoundaryNodes() {
    const json = this.#graph.toJSON();
    return json.nodes.filter(n => n.trustLevel === TrustLevel.Boundary);
  }

  getExternalDependencies() {
    const json = this.#graph.toJSON();
    return json.edges.filter(e => {
      const target = this.#graph.getNode(e.to);
      return target?.trustLevel === TrustLevel.External;
    });
  }
}

export function createGraph() {
  return new SymbolGraph();
}

export function loadGraphFromFile(path) {
  if (!existsSync(path)) throw new Error(`Graph file not found: ${path}`);
  const raw = readFileSync(path, 'utf-8');
  const data = JSON.parse(raw);
  const graph = new SymbolGraph();
  for (const node of data.nodes ?? []) graph.addNode(node);
  for (const edge of data.edges ?? []) graph.addEdge(edge);
  return graph;
}
";

const JS_SAMPLE_LARGE: &str = r"
import { EventEmitter } from 'node:events';
import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'node:fs';
import { join, resolve, relative, dirname, basename, extname } from 'node:path';
import { createHash } from 'node:crypto';

export class ChangeCoalescer {
  #pending = new Map();
  #timer = null;
  #batchCount = 0;
  #config;
  #onBatch;

  constructor(config, onBatch) {
    this.#config = config;
    this.#onBatch = onBatch;
  }

  push(change) {
    const existing = this.#pending.get(change.path);
    if (existing) {
      existing.kind = this.#mergeKinds(existing.kind, change.kind);
      existing.timestamp = change.timestamp;
      existing.contentHash = change.contentHash;
    } else {
      this.#pending.set(change.path, { ...change });
    }

    if (this.#pending.size >= this.#config.maxBatchSize) {
      this.flush();
      return;
    }

    if (this.#timer) clearTimeout(this.#timer);
    this.#timer = setTimeout(() => this.flush(), this.#config.debounceMs);
  }

  flush() {
    if (this.#pending.size === 0) return;
    if (this.#timer) {
      clearTimeout(this.#timer);
      this.#timer = null;
    }

    this.#batchCount += 1;
    const batch = {
      id: `batch-${this.#batchCount}`,
      changes: [...this.#pending.values()],
      timestamp: Date.now(),
      debounced: this.#pending.size > 1,
    };
    this.#pending.clear();
    this.#onBatch(batch);
  }

  #mergeKinds(existing, incoming) {
    if (existing === 'create' && incoming === 'delete') return 'delete';
    if (existing === 'delete' && incoming === 'create') return 'modify';
    if (existing === 'create' && incoming === 'modify') return 'create';
    return incoming;
  }

  get pendingCount() { return this.#pending.size; }

  clear() {
    this.#pending.clear();
    if (this.#timer) {
      clearTimeout(this.#timer);
      this.#timer = null;
    }
  }
}

export class FileHasher {
  #cache = new Map();

  hash(path) {
    try {
      const content = readFileSync(path);
      const hash = createHash('sha256').update(content).digest('hex');
      this.#cache.set(path, hash);
      return hash;
    } catch {
      this.#cache.delete(path);
      return '';
    }
  }

  getCached(path) {
    return this.#cache.get(path);
  }

  hasChanged(path) {
    const cached = this.#cache.get(path);
    if (!cached) return true;
    const current = this.hash(path);
    return current !== cached;
  }

  invalidate(path) {
    this.#cache.delete(path);
  }

  clear() {
    this.#cache.clear();
  }

  get cacheSize() { return this.#cache.size; }
}

export class PolicyEngine {
  #rules = [];

  register(rule) {
    this.#rules.push(rule);
  }

  evaluate(context) {
    const violations = [];
    for (const rule of this.#rules) {
      try {
        const result = rule.evaluate(context);
        violations.push(...result);
      } catch (err) {
        violations.push({
          ruleId: rule.id,
          file: '<policy-engine>',
          message: `Rule ${rule.id} threw: ${err}`,
          severity: 'error',
        });
      }
    }
    return this.#deduplicate(violations);
  }

  #deduplicate(violations) {
    const seen = new Set();
    return violations.filter(v => {
      const key = `${v.ruleId}:${v.file}:${v.symbol ?? ''}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
  }

  get ruleCount() { return this.#rules.length; }
}

export class EventEmitterBus extends EventEmitter {
  #seq = 0;

  emit(event, ...args) {
    this.#seq += 1;
    return super.emit(event, { seq: this.#seq, timestamp: Date.now() }, ...args);
  }

  get sequence() { return this.#seq; }
}

export function hashContent(content) {
  return createHash('sha256').update(content).digest('hex');
}

export function isSourceFile(path) {
  const ext = extname(path).toLowerCase();
  return ['.ts', '.tsx', '.js', '.jsx', '.rs'].includes(ext);
}

export function shouldIgnore(path, patterns) {
  for (const pattern of patterns) {
    if (path.includes(pattern)) return true;
  }
  return false;
}

export function ensureDir(path) {
  if (!existsSync(path)) {
    mkdirSync(path, { recursive: true });
  }
}

export function relativePath(from, to) {
  return relative(from, to).replace(/\\/g, '/');
}

export function generateId() {
  return createHash('sha256')
    .update(Date.now().toString())
    .update(Math.random().toString())
    .digest('hex')
    .slice(0, 16);
}

export class BatchProcessor {
  #queue = [];
  #processing = false;
  #handler;
  #maxBatchSize;

  constructor(handler, maxBatchSize = 50) {
    this.#handler = handler;
    this.#maxBatchSize = maxBatchSize;
  }

  async enqueue(item) {
    this.#queue.push(item);
    if (this.#queue.length >= this.#maxBatchSize) {
      await this.process();
    }
  }

  async process() {
    if (this.#processing || this.#queue.length === 0) return;
    this.#processing = true;
    try {
      const batch = this.#queue.splice(0, this.#maxBatchSize);
      await this.#handler(batch);
    } finally {
      this.#processing = false;
    }
  }

  get pending() { return this.#queue.length; }
  get isProcessing() { return this.#processing; }
}

export class RingBuffer {
  #buffer;
  #head = 0;
  #count = 0;
  #capacity;

  constructor(capacity) {
    this.#capacity = capacity;
    this.#buffer = new Array(capacity);
  }

  push(item) {
    this.#buffer[this.#head] = item;
    this.#head = (this.#head + 1) % this.#capacity;
    if (this.#count < this.#capacity) this.#count += 1;
  }

  toArray() {
    const result = [];
    const start = this.#count < this.#capacity ? 0 : this.#head;
    for (let i = 0; i < this.#count; i++) {
      const idx = (start + i) % this.#capacity;
      result.push(this.#buffer[idx]);
    }
    return result;
  }

  get size() { return this.#count; }
  get isFull() { return this.#count === this.#capacity; }
}
";

const SAMPLE_LARGE: &str = r"
import { EventEmitter } from 'node:events';
import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'node:fs';
import { join, resolve, relative, dirname, basename, extname } from 'node:path';
import { createHash } from 'node:crypto';

export interface WatcherConfig {
  root: string;
  ignore: string[];
  debounceMs: number;
  maxBatchSize: number;
  recursive: boolean;
}

export interface FileChange {
  path: string;
  kind: 'create' | 'modify' | 'delete' | 'rename';
  timestamp: number;
  contentHash?: string;
}

export interface ChangeBatch {
  id: string;
  changes: FileChange[];
  timestamp: number;
  debounced: boolean;
}

export class ChangeCoalescer {
  private pending: Map<string, FileChange> = new Map();
  private timer: ReturnType<typeof setTimeout> | null = null;
  private batchCount = 0;

  constructor(
    private config: WatcherConfig,
    private onBatch: (batch: ChangeBatch) => void,
  ) {}

  push(change: FileChange): void {
    const existing = this.pending.get(change.path);
    if (existing) {
      existing.kind = this.mergeKinds(existing.kind, change.kind);
      existing.timestamp = change.timestamp;
      existing.contentHash = change.contentHash;
    } else {
      this.pending.set(change.path, { ...change });
    }

    if (this.pending.size >= this.config.maxBatchSize) {
      this.flush();
      return;
    }

    if (this.timer) clearTimeout(this.timer);
    this.timer = setTimeout(() => this.flush(), this.config.debounceMs);
  }

  flush(): void {
    if (this.pending.size === 0) return;
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }

    this.batchCount += 1;
    const batch: ChangeBatch = {
      id: `batch-${this.batchCount}`,
      changes: [...this.pending.values()],
      timestamp: Date.now(),
      debounced: this.pending.size > 1,
    };
    this.pending.clear();
    this.onBatch(batch);
  }

  private mergeKinds(
    existing: FileChange['kind'],
    incoming: FileChange['kind'],
  ): FileChange['kind'] {
    if (existing === 'create' && incoming === 'delete') return 'delete';
    if (existing === 'delete' && incoming === 'create') return 'modify';
    if (existing === 'create' && incoming === 'modify') return 'create';
    return incoming;
  }

  get pendingCount(): number { return this.pending.size; }

  clear(): void {
    this.pending.clear();
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
  }
}

export class FileHasher {
  private cache: Map<string, string> = new Map();

  hash(path: string): string {
    try {
      const content = readFileSync(path);
      const hash = createHash('sha256').update(content).digest('hex');
      this.cache.set(path, hash);
      return hash;
    } catch {
      this.cache.delete(path);
      return '';
    }
  }

  getCached(path: string): string | undefined {
    return this.cache.get(path);
  }

  hasChanged(path: string): boolean {
    const cached = this.cache.get(path);
    if (!cached) return true;
    const current = this.hash(path);
    return current !== cached;
  }

  invalidate(path: string): void {
    this.cache.delete(path);
  }

  clear(): void {
    this.cache.clear();
  }

  get cacheSize(): number { return this.cache.size; }
}

export interface PolicyRule {
  id: string;
  name: string;
  severity: 'error' | 'warning' | 'info';
  evaluate: (context: PolicyContext) => PolicyViolation[];
}

export interface PolicyContext {
  changes: FileChange[];
  graph: { nodeCount: number; edgeCount: number };
  config: Record<string, unknown>;
}

export interface PolicyViolation {
  ruleId: string;
  file: string;
  symbol?: string;
  message: string;
  severity: 'error' | 'warning' | 'info';
}

export class PolicyEngine {
  private rules: PolicyRule[] = [];

  register(rule: PolicyRule): void {
    this.rules.push(rule);
  }

  evaluate(context: PolicyContext): PolicyViolation[] {
    const violations: PolicyViolation[] = [];
    for (const rule of this.rules) {
      try {
        const result = rule.evaluate(context);
        violations.push(...result);
      } catch (err) {
        violations.push({
          ruleId: rule.id,
          file: '<policy-engine>',
          message: `Rule ${rule.id} threw: ${err}`,
          severity: 'error',
        });
      }
    }
    return this.deduplicate(violations);
  }

  private deduplicate(violations: PolicyViolation[]): PolicyViolation[] {
    const seen = new Set<string>();
    return violations.filter(v => {
      const key = `${v.ruleId}:${v.file}:${v.symbol ?? ''}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
  }

  get ruleCount(): number { return this.rules.length; }
}

export class EventEmitterBus extends EventEmitter {
  private seq = 0;

  emit(event: string, ...args: unknown[]): boolean {
    this.seq += 1;
    return super.emit(event, { seq: this.seq, timestamp: Date.now() }, ...args);
  }

  get sequence(): number { return this.seq; }
}

export function hashContent(content: string): string {
  return createHash('sha256').update(content).digest('hex');
}

export function isSourceFile(path: string): boolean {
  const ext = extname(path).toLowerCase();
  return ['.ts', '.tsx', '.js', '.jsx', '.rs'].includes(ext);
}

export function shouldIgnore(path: string, patterns: string[]): boolean {
  for (const pattern of patterns) {
    if (path.includes(pattern)) return true;
  }
  return false;
}

export function ensureDir(path: string): void {
  if (!existsSync(path)) {
    mkdirSync(path, { recursive: true });
  }
}

export function relativePath(from: string, to: string): string {
  return relative(from, to).replace(/\\/g, '/');
}

export function generateId(): string {
  return createHash('sha256')
    .update(Date.now().toString())
    .update(Math.random().toString())
    .digest('hex')
    .slice(0, 16);
}

export class BatchProcessor<T> {
  private queue: T[] = [];
  private processing = false;

  constructor(
    private handler: (items: T[]) => Promise<void>,
    private maxBatchSize: number = 50,
  ) {}

  async enqueue(item: T): Promise<void> {
    this.queue.push(item);
    if (this.queue.length >= this.maxBatchSize) {
      await this.process();
    }
  }

  async process(): Promise<void> {
    if (this.processing || this.queue.length === 0) return;
    this.processing = true;
    try {
      const batch = this.queue.splice(0, this.maxBatchSize);
      await this.handler(batch);
    } finally {
      this.processing = false;
    }
  }

  get pending(): number { return this.queue.length; }
  get isProcessing(): boolean { return this.processing; }
}

export class RingBuffer<T> {
  private buffer: (T | undefined)[];
  private head = 0;
  private count = 0;

  constructor(private capacity: number) {
    this.buffer = new Array(capacity);
  }

  push(item: T): void {
    this.buffer[this.head] = item;
    this.head = (this.head + 1) % this.capacity;
    if (this.count < this.capacity) this.count += 1;
  }

  toArray(): T[] {
    const result: T[] = [];
    const start = this.count < this.capacity ? 0 : this.head;
    for (let i = 0; i < this.count; i++) {
      const idx = (start + i) % this.capacity;
      result.push(this.buffer[idx]!);
    }
    return result;
  }

  get size(): number { return this.count; }
  get isFull(): boolean { return this.count === this.capacity; }
}
";
