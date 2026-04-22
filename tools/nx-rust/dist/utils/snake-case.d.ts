/**
 * Convert a name to `snake_case`. Cargo package names allow `-` or `_` but the
 * Rust `[lib]` / `[bin]` `name` field requires an identifier, so we normalise
 * aggressively for generated source files.
 */
export declare function toSnakeCase(input: string): string;
//# sourceMappingURL=snake-case.d.ts.map