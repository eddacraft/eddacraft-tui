use std::io::Write;
use std::path::{Path, PathBuf};

use anvil_dashboard_server::openapi_document;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: export-openapi <output-path>")?;
    let mut document = serde_json::to_vec_pretty(&openapi_document())?;
    document.push(b'\n');
    atomic_write(&output, &document)?;
    Ok(())
}

/// Write `content` via a same-directory temp file + rename so an interrupted
/// export cannot leave the committed OpenAPI contract truncated.
fn atomic_write(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("no parent directory for {}", path.display()),
        )
    })?;

    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("openapi.json");
    let tmp_path = dir.join(format!(".{file_name}.{}.tmp", std::process::id()));

    {
        let mut file = std::fs::File::create(&tmp_path)?;
        file.write_all(content)?;
        file.sync_all()?;
    }

    // On Windows, rename fails if the destination exists. Remove first; on
    // Unix rename replaces atomically.
    #[cfg(windows)]
    {
        if let Err(e) = std::fs::remove_file(path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                let _ = std::fs::remove_file(&tmp_path);
                return Err(e);
            }
        }
    }

    match std::fs::rename(&tmp_path, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_path);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn atomic_write_replaces_existing_content() {
        let dir = std::env::temp_dir().join(format!("export-openapi-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("openapi.json");
        fs::write(&path, b"stale\n").expect("seed");

        atomic_write(&path, b"{\"ok\":true}\n").expect("atomic write");

        assert_eq!(fs::read_to_string(&path).expect("read"), "{\"ok\":true}\n");
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(Result::ok)
            .filter(|e| e.path() != path)
            .collect();
        assert!(leftovers.is_empty(), "temp file should be renamed away: {leftovers:?}");

        let _ = fs::remove_dir_all(&dir);
    }
}
