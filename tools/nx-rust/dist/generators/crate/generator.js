"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.default = crateGenerator;
const devkit_1 = require("@nx/devkit");
const path = __importStar(require("node:path"));
const add_to_workspace_1 = require("../../utils/add-to-workspace");
const normalize_options_1 = require("../../utils/normalize-options");
const target_configs_1 = require("../../utils/target-configs");
const generator_1 = __importDefault(require("../init/generator"));
/**
 * Escape a string for use inside a TOML basic string (`"..."`). Only the
 * four characters TOML forbids in bare basic strings get escaped; we stay
 * ASCII-safe for the common case.
 */
function tomlEscape(value) {
    return value
        .replace(/\\/g, '\\\\')
        .replace(/"/g, '\\"')
        .replace(/\n/g, '\\n')
        .replace(/\r/g, '\\r');
}
async function crateGenerator(tree, options) {
    await (0, generator_1.default)(tree, { skipFormat: true });
    const normalized = (0, normalize_options_1.normalizeOptions)(tree, options);
    const targets = {
        build: (0, target_configs_1.buildTargetConfig)(),
        check: (0, target_configs_1.checkTargetConfig)(),
        clippy: (0, target_configs_1.clippyTargetConfig)(),
        fmt: (0, target_configs_1.fmtTargetConfig)(),
        'fmt-check': (0, target_configs_1.fmtCheckTargetConfig)(),
        test: (0, target_configs_1.testTargetConfig)(),
        ...(options.bin ? { run: (0, target_configs_1.runTargetConfig)() } : {}),
    };
    (0, devkit_1.addProjectConfiguration)(tree, normalized.projectName, {
        root: normalized.projectRoot,
        projectType: options.bin ? 'application' : 'library',
        sourceRoot: `${normalized.projectRoot}/src`,
        tags: normalized.parsedTags,
        targets,
    });
    const templateDir = options.bin
        ? path.join(__dirname, 'files', 'bin')
        : path.join(__dirname, 'files', 'lib');
    (0, devkit_1.generateFiles)(tree, templateDir, normalized.projectRoot, {
        ...normalized,
        ...(0, devkit_1.names)(normalized.cargoName),
        // TOML-escape the free-form description so a name like `He said "hi"` or
        // a backslash can't produce a corrupt manifest.
        description: normalized.description
            ? tomlEscape(normalized.description)
            : normalized.description,
        offsetFromRoot: (0, devkit_1.offsetFromRoot)(normalized.projectRoot),
        template: '',
    });
    (0, add_to_workspace_1.addToCargoWorkspace)(tree, normalized.projectRoot);
    if (!options.skipFormat) {
        await (0, devkit_1.formatFiles)(tree);
    }
}
//# sourceMappingURL=generator.js.map