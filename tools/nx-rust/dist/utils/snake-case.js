"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.toSnakeCase = toSnakeCase;
/**
 * Convert a name to `snake_case`. Cargo package names allow `-` or `_` but the
 * Rust `[lib]` / `[bin]` `name` field requires an identifier, so we normalise
 * aggressively for generated source files.
 */
function toSnakeCase(input) {
    return input
        .replace(/[^a-zA-Z0-9]+/g, '_')
        .replace(/([a-z0-9])([A-Z])/g, '$1_$2')
        .replace(/([A-Z])([A-Z][a-z])/g, '$1_$2')
        .replace(/^_+|_+$/g, '')
        .toLowerCase();
}
//# sourceMappingURL=snake-case.js.map