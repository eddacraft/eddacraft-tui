"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.cargoCommand = cargoCommand;
exports.cargoMetadata = cargoMetadata;
exports.isExternal = isExternal;
const node_child_process_1 = require("node:child_process");
const node_path_1 = require("node:path");
const run_process_1 = require("./run-process");
// Inline ANSI dim instead of pulling chalk in. The vendored package's chalk
// dep was being shadowed by chalk@5 hoisted at the workspace root, breaking
// chalk's CJS default-import interop at runtime under the Nx plugin loader.
// One log line doesn't justify that mess. Listed in SPLIT.md as a
// divergence from upstream.
const DIM = '[2m';
const RESET_DIM = '[22m';
/**
 * Spawn `cargo <args>` with inherited stdio and always-on colour. Returns the
 * success flag. Logs the command in dim text so failures are easy to
 * reproduce.
 *
 * Cargo rejects any flag before `+toolchain`, so if the first arg is a
 * toolchain selector we emit it ahead of `--color always`.
 */
async function cargoCommand(...args) {
    const [head, ...rest] = args;
    const ordered = head && head.startsWith('+')
        ? [head, '--color', 'always', ...rest]
        : ['--color', 'always', ...args];
    console.log(`${DIM}> cargo ${redactArgs(ordered).join(' ')}${RESET_DIM}`);
    return (0, run_process_1.runProcess)('cargo', ...ordered);
}
/**
 * Redact secret-bearing flag values in-place for log output. Cargo surfaces
 * tokens via `--token <value>`; that value must never appear in terminal
 * history, `/proc/<pid>/cmdline` readers, or CI log scrapers.
 */
function redactArgs(argv) {
    const SECRET_FLAGS = new Set(['--token']);
    const out = [];
    for (let i = 0; i < argv.length; i++) {
        const token = argv[i];
        out.push(token);
        if (SECRET_FLAGS.has(token) && i + 1 < argv.length) {
            out.push('***');
            i++;
        }
    }
    return out;
}
/**
 * Run `cargo metadata --format-version=1` and parse the JSON output. Returns
 * `null` on failure — graph resolution has to be resilient to transient cargo
 * errors (e.g. during `cargo clean`).
 *
 * `cargo metadata` is the supported stable contract for consuming a Cargo
 * workspace; parsing Cargo.toml by hand loses resolved versions, path-dep
 * resolution, and external dependency source info.
 *
 * Uses `execFileSync` (no shell) so cargo arg injection is not possible.
 */
function cargoMetadata(cwd) {
    try {
        const output = (0, node_child_process_1.execFileSync)('cargo', ['metadata', '--format-version=1'], {
            encoding: 'utf8',
            stdio: ['ignore', 'pipe', 'pipe'],
            maxBuffer: 1024 * 1024 * 64,
            cwd,
            windowsHide: true,
        });
        return JSON.parse(output);
    }
    catch {
        return null;
    }
}
/**
 * True if the package/dep resolves to a registry, git, or out-of-workspace
 * path. Used to decide whether a dep becomes an internal Nx edge or an
 * external `cargo:<name>` node.
 */
function isExternal(packageOrDep, workspaceRoot) {
    const source = packageOrDep.source ?? '';
    if (source.startsWith('registry+'))
        return true;
    if (source.startsWith('git+'))
        return true;
    // cargo metadata emits absolute manifest/path values, so the workspace root
    // must also be absolute for `relative()` to produce correct answers.
    const absRoot = (0, node_path_1.isAbsolute)(workspaceRoot) ? workspaceRoot : (0, node_path_1.resolve)(workspaceRoot);
    const candidate = ('manifest_path' in packageOrDep && packageOrDep.manifest_path) ||
        ('path' in packageOrDep && packageOrDep.path) ||
        null;
    // No source and no path → almost certainly a workspace-inherited registry
    // dep whose `source` is elided in the metadata. Treat as external; a missing
    // path cannot describe a local path dep.
    if (!candidate)
        return true;
    const absCandidate = (0, node_path_1.isAbsolute)(candidate) ? candidate : (0, node_path_1.resolve)(absRoot, candidate);
    const rel = (0, node_path_1.relative)(absRoot, absCandidate);
    return rel.startsWith('..') || (0, node_path_1.isAbsolute)(rel);
}
//# sourceMappingURL=cargo.js.map