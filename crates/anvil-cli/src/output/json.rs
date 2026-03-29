use serde::Serialize;

pub fn print<T: Serialize>(value: &T) -> anyhow::Result<()> {
    let output = serde_json::to_string_pretty(value)?;
    println!("{output}");
    Ok(())
}

#[allow(dead_code)]
pub fn print_compact<T: Serialize>(value: &T) -> anyhow::Result<()> {
    let output = serde_json::to_string(value)?;
    println!("{output}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct Sample {
        name: String,
        score: f64,
    }

    #[test]
    fn print_produces_valid_json() {
        let s = Sample {
            name: "test".into(),
            score: 42.0,
        };
        // Just verify it doesn't error
        print(&s).unwrap();
    }

    #[test]
    fn print_compact_produces_valid_json() {
        let s = Sample {
            name: "test".into(),
            score: 42.0,
        };
        print_compact(&s).unwrap();
    }
}
