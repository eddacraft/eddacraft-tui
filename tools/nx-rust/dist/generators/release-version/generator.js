"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.releaseVersionGenerator = releaseVersionGenerator;
const devkit_1 = require("@nx/devkit");
const toml_1 = require("../../utils/toml");
async function releaseVersionGenerator(tree, options) {
    const data = {};
    const projectList = normaliseProjects(options.projects, options.projectGraph);
    for (const project of projectList) {
        const cargoPath = `${project.root}/Cargo.toml`;
        const raw = tree.read(cargoPath)?.toString();
        if (!raw) {
            devkit_1.logger.warn(`${project.name}: no Cargo.toml at ${cargoPath} — skipping.`);
            data[project.name] = { currentVersion: null, newVersion: null };
            continue;
        }
        const toml = (0, toml_1.parseCargoToml)(raw);
        const rawVersion = toml.package?.version;
        // `version.workspace = true` parses as an object (not a string). Bumping
        // the member's manifest in-place would silently do the wrong thing and
        // make `nx release` report success with zero effect.
        if (rawVersion && typeof rawVersion !== 'string') {
            throw new Error(`${project.name}: Cargo.toml inherits its version from [workspace.package] ` +
                '(`version.workspace = true`). Run `nx release version` on the workspace root ' +
                'crate, or drop the inheritance to bump member versions directly.');
        }
        const currentVersion = (rawVersion ?? null);
        const newVersion = resolveNewVersion(currentVersion, options.specifier);
        // Only rewrite Cargo.toml when the version actually changes. Otherwise
        // re-serialising the TOML produces a noisy diff with no semantic effect
        // (e.g. when `specifier` is undefined or an unknown bump keyword falls
        // back to `currentVersion`).
        if (newVersion && newVersion !== currentVersion && toml.package) {
            toml.package.version = newVersion;
            tree.write(cargoPath, (0, toml_1.stringifyCargoToml)(toml));
        }
        data[project.name] = { currentVersion, newVersion };
    }
    return {
        data,
        callback: async () => {
            // Side effects like git staging are handled by `nx release` itself.
        },
    };
}
exports.default = releaseVersionGenerator;
function normaliseProjects(input, projectGraph) {
    if (!Array.isArray(input))
        return [];
    return input
        .map((p) => {
        if (typeof p !== 'string') {
            return { name: p.name, root: p.data?.root ?? '' };
        }
        // `nx release version` sometimes passes bare project names. Look the
        // root up from the project graph rather than silently dropping them.
        const node = projectGraph?.nodes?.[p];
        const root = node?.data?.root ?? '';
        if (!root) {
            throw new Error(`releaseVersionGenerator: cannot resolve project root for "${p}". ` +
                'Pass `projectGraph` alongside `projects`, or use the array-of-objects form.');
        }
        return { name: p, root };
    })
        .filter((p) => p.root);
}
function resolveNewVersion(current, specifier) {
    if (!specifier)
        return current;
    if (/^\d+\.\d+\.\d+(?:[-+].+)?$/.test(specifier))
        return specifier;
    if (!current)
        return null;
    const [baseStr, preRest] = current.split('-', 2);
    const parts = baseStr.split('.').map((n) => parseInt(n, 10));
    const [major = 0, minor = 0, patch = 0] = parts;
    switch (specifier) {
        case 'major':
            return `${major + 1}.0.0`;
        case 'minor':
            return `${major}.${minor + 1}.0`;
        case 'patch':
            return `${major}.${minor}.${patch + 1}`;
        default:
            void preRest;
            return current;
    }
}
//# sourceMappingURL=generator.js.map