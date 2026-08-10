//! Tiny public API surface for DEVACC rename scenarios.

pub fn greet(name: &str) -> String {
    format!("hello, {name}")
}

pub fn shout(name: &str) -> String {
    greet(name).to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greet_works() {
        assert_eq!(greet("a"), "hello, a");
    }
}
