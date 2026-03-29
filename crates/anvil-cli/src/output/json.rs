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
