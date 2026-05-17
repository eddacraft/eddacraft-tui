//! Minimal rust fixture for the editor-coexistence harness.
//!
//! Intentionally small: just enough surface for rust-analyzer to index
//! and report no diagnostics. Do not grow this without updating
//! `docs/policies/editor-coexistence.md`.

pub fn greet(name: &str) -> String {
    format!("hello, {name}")
}

#[cfg(test)]
mod tests {
    use super::greet;

    #[test]
    fn greets_by_name() {
        assert_eq!(greet("anvil"), "hello, anvil");
    }
}
