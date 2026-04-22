"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.buildTargetConfig = buildTargetConfig;
exports.checkTargetConfig = checkTargetConfig;
exports.clippyTargetConfig = clippyTargetConfig;
exports.fmtTargetConfig = fmtTargetConfig;
exports.fmtCheckTargetConfig = fmtCheckTargetConfig;
exports.testTargetConfig = testTargetConfig;
exports.runTargetConfig = runTargetConfig;
const BINARY_OUTPUTS = ['{options.target-dir}', '{workspaceRoot}/target'];
function buildTargetConfig(options = {}) {
    return {
        executor: '@eddacraft/nx-rust:build',
        cache: true,
        outputs: BINARY_OUTPUTS,
        options,
        configurations: {
            production: { release: true },
        },
    };
}
function checkTargetConfig(options = {}) {
    return {
        executor: '@eddacraft/nx-rust:check',
        cache: true,
        outputs: [],
        options,
    };
}
function clippyTargetConfig(options = {}) {
    return {
        executor: '@eddacraft/nx-rust:clippy',
        cache: true,
        outputs: [],
        options,
    };
}
/**
 * Reformatting target — rewrites source files in place, so it is NOT safely
 * cacheable. Pair with `fmtCheckTargetConfig` for CI lint runs.
 */
function fmtTargetConfig(options = {}) {
    return {
        executor: '@eddacraft/nx-rust:fmt',
        options,
    };
}
/**
 * Lint-only formatter target — runs `cargo fmt --check`, safe to cache by
 * exit code. Use this in `nx run-many --target=fmt-check` CI gates.
 */
function fmtCheckTargetConfig(options = {}) {
    return {
        executor: '@eddacraft/nx-rust:fmt',
        cache: true,
        outputs: [],
        options: { check: true, ...options },
    };
}
function testTargetConfig(options = {}) {
    return {
        executor: '@eddacraft/nx-rust:test',
        cache: true,
        outputs: BINARY_OUTPUTS,
        options,
        configurations: {
            production: { release: true },
        },
    };
}
function runTargetConfig(options = {}) {
    return {
        executor: '@eddacraft/nx-rust:run',
        options,
        configurations: {
            production: { release: true },
        },
    };
}
//# sourceMappingURL=target-configs.js.map