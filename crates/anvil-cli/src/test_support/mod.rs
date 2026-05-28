//! Test-only support utilities shared across the crate's `#[cfg(test)]`
//! modules. Compiled only under test, so nothing here ships in the binary.

pub mod cwd;
