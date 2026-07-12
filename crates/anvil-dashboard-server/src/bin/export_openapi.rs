use std::path::PathBuf;

use anvil_dashboard_server::openapi_document;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: export-openapi <output-path>")?;
    let mut document = serde_json::to_vec_pretty(&openapi_document())?;
    document.push(b'\n');
    std::fs::write(output, document)?;
    Ok(())
}
