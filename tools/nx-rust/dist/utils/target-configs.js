'use strict';
Object.defineProperty(exports, '__esModule', { value: true });
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
    executor: 'nxrust:build',
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
    executor: 'nxrust:check',
    cache: true,
    outputs: [],
    options,
  };
}
function clippyTargetConfig(options = {}) {
  return {
    executor: 'nxrust:clippy',
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
    executor: 'nxrust:fmt',
    options,
  };
}
/**
 * Lint-only formatter target — runs `cargo fmt --check`, safe to cache by
 * exit code. Use this in `nx run-many --target=fmt-check` CI gates.
 */
function fmtCheckTargetConfig(options = {}) {
  return {
    executor: 'nxrust:fmt',
    cache: true,
    outputs: [],
    options: { check: true, ...options },
  };
}
function testTargetConfig(options = {}) {
  return {
    executor: 'nxrust:test',
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
    executor: 'nxrust:run',
    options,
    configurations: {
      production: { release: true },
    },
  };
}
//# sourceMappingURL=target-configs.js.map
