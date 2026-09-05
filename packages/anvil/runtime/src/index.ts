/**
 * Remaining TypeScript cache utilities. API/docs flag consumers use the
 * explicit /feature-flags subpath. Watch and agent/lock/queue orchestration
 * were retired under EMBERRS-001; the Rust CLI owns engine execution.
 */
export * from './cache/index.js';
