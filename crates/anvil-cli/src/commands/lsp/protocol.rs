use std::fmt;
use std::io::{self, BufRead, Read};
use std::path::{Path, PathBuf};

pub(super) const MAX_LSP_FRAME_BYTES: usize = 4 * 1024 * 1024;
pub(super) const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;
pub(super) const MAX_DOCUMENT_URI_BYTES: usize = 4096;
const MAX_HEADER_BYTES: usize = 8 * 1024;
const MAX_HEADER_LINE_BYTES: usize = 1024;
const MAX_HEADER_COUNT: usize = 64;

#[derive(Debug)]
pub(super) enum FrameError {
    Io(io::Error),
    MissingContentLength,
    DuplicateContentLength,
    InvalidContentLength,
    FrameTooLarge,
    HeadersTooLarge,
}

impl fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "LSP frame I/O failed: {error}"),
            Self::MissingContentLength => formatter.write_str("missing Content-Length header"),
            Self::DuplicateContentLength => formatter.write_str("duplicate Content-Length header"),
            Self::InvalidContentLength => formatter.write_str("invalid Content-Length header"),
            Self::FrameTooLarge => formatter.write_str("LSP frame exceeds 4 MiB limit"),
            Self::HeadersTooLarge => formatter.write_str("LSP headers exceed bounded limits"),
        }
    }
}

impl std::error::Error for FrameError {}

impl From<io::Error> for FrameError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct UriError;

impl fmt::Display for UriError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("only absolute file URIs are supported")
    }
}

impl std::error::Error for UriError {}

pub(super) fn file_uri_to_path(uri: &str) -> Result<PathBuf, UriError> {
    let encoded = uri.strip_prefix("file://").ok_or(UriError)?;
    let mut decoded = Vec::with_capacity(encoded.len());
    let bytes = encoded.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = bytes.get(index + 1).and_then(|byte| hex_value(*byte));
            let low = bytes.get(index + 2).and_then(|byte| hex_value(*byte));
            let (Some(high), Some(low)) = (high, low) else {
                return Err(UriError);
            };
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    if decoded.contains(&0) {
        return Err(UriError);
    }
    let decoded = String::from_utf8(decoded).map_err(|_| UriError)?;
    #[cfg(windows)]
    let mut decoded = decoded
        .strip_prefix('/')
        .filter(|path| path.as_bytes().get(1) == Some(&b':'))
        .unwrap_or(&decoded)
        .to_string();
    #[cfg(windows)]
    if decoded.as_bytes().get(1) == Some(&b':') {
        decoded[..1].make_ascii_uppercase();
    }
    let path = PathBuf::from(decoded);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(UriError)
    }
}

#[derive(Debug, Default)]
pub(super) struct WorkspaceRoots(Vec<PathBuf>);

impl WorkspaceRoots {
    pub fn from_initialize(message: &serde_json::Value) -> Self {
        Self::from_initialize_with_fallback(message, std::env::current_dir().ok())
    }

    fn from_initialize_with_fallback(
        message: &serde_json::Value,
        fallback_root: Option<PathBuf>,
    ) -> Self {
        let folder_uris = message
            .pointer("/params/workspaceFolders")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|folder| folder.get("uri").and_then(serde_json::Value::as_str))
            .filter(|uri| uri.len() <= MAX_DOCUMENT_URI_BYTES)
            .take(64);
        let mut roots = folder_uris
            .filter_map(|uri| file_uri_to_path(uri).ok())
            .collect::<Vec<_>>();
        if roots.is_empty()
            && let Some(uri) = message
                .pointer("/params/rootUri")
                .and_then(serde_json::Value::as_str)
                .filter(|uri| uri.len() <= MAX_DOCUMENT_URI_BYTES)
        {
            roots.extend(file_uri_to_path(uri).ok());
        }
        if roots.is_empty() {
            roots.extend(fallback_root.filter(|root| root.is_absolute()));
        }
        roots.sort_by(|left, right| {
            right
                .components()
                .count()
                .cmp(&left.components().count())
                .then_with(|| left.cmp(right))
        });
        roots.dedup();
        Self(roots)
    }

    pub fn relative_path(&self, uri: &str) -> Result<PathBuf, UriError> {
        if uri.len() > MAX_DOCUMENT_URI_BYTES {
            return Err(UriError);
        }
        let path = file_uri_to_path(uri)?;
        self.0
            .iter()
            .find_map(|root| path.strip_prefix(root).ok().map(Path::to_path_buf))
            .filter(|relative| {
                !relative.as_os_str().is_empty()
                    && relative
                        .components()
                        .all(|component| matches!(component, std::path::Component::Normal(_)))
            })
            .filter(|relative| relative.to_string_lossy().len() <= MAX_DOCUMENT_URI_BYTES)
            .ok_or(UriError)
    }
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub(super) fn read_lsp_frame(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>, FrameError> {
    let mut content_length = None;
    let mut header_bytes = 0usize;
    let mut header_count = 0usize;
    loop {
        let mut line = Vec::new();
        let read = reader
            .take((MAX_HEADER_LINE_BYTES + 1) as u64)
            .read_until(b'\n', &mut line)?;
        if read == 0 {
            return if content_length.is_none() {
                Ok(None)
            } else {
                Err(FrameError::InvalidContentLength)
            };
        }
        header_bytes = header_bytes.saturating_add(read);
        if read > MAX_HEADER_LINE_BYTES || header_bytes > MAX_HEADER_BYTES || !line.ends_with(b"\n")
        {
            return Err(FrameError::HeadersTooLarge);
        }
        let line = std::str::from_utf8(&line).map_err(|_| FrameError::InvalidContentLength)?;
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        header_count += 1;
        if header_count > MAX_HEADER_COUNT {
            return Err(FrameError::HeadersTooLarge);
        }
        let Some((name, value)) = trimmed.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            if content_length.is_some() {
                return Err(FrameError::DuplicateContentLength);
            }
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| FrameError::InvalidContentLength)?,
            );
        }
    }

    let length = content_length.ok_or(FrameError::MissingContentLength)?;
    if length > MAX_LSP_FRAME_BYTES {
        return Err(FrameError::FrameTooLarge);
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    Ok(Some(body))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use serde_json::json;

    use super::{FrameError, MAX_HEADER_COUNT, WorkspaceRoots, file_uri_to_path, read_lsp_frame};

    #[test]
    fn frame_headers_are_case_insensitive() {
        let mut input = Cursor::new(b"content-length: 2\r\n\r\n{}".to_vec());

        let body = read_lsp_frame(&mut input)
            .expect("valid frame")
            .expect("frame body");

        assert_eq!(body, b"{}");
    }

    #[test]
    fn duplicate_content_length_is_rejected() {
        let mut input = Cursor::new(b"Content-Length: 2\r\ncontent-length: 2\r\n\r\n{}".to_vec());

        assert!(matches!(
            read_lsp_frame(&mut input),
            Err(FrameError::DuplicateContentLength)
        ));
    }

    #[test]
    fn oversized_header_line_is_rejected_without_waiting_for_a_body() {
        let mut input = Cursor::new(vec![b'a'; 1025]);
        assert!(matches!(
            read_lsp_frame(&mut input),
            Err(FrameError::HeadersTooLarge)
        ));
    }

    #[test]
    fn header_count_limit_excludes_the_blank_terminator() {
        let mut accepted = b"X-Ignored: value\r\n".repeat(MAX_HEADER_COUNT - 1);
        accepted.extend_from_slice(b"Content-Length: 2\r\n\r\n{}");
        let mut accepted = Cursor::new(accepted);
        assert_eq!(
            read_lsp_frame(&mut accepted)
                .expect("the declared header limit should be accepted")
                .expect("frame body"),
            b"{}"
        );

        let mut rejected = b"X-Ignored: value\r\n".repeat(MAX_HEADER_COUNT);
        rejected.extend_from_slice(b"Content-Length: 2\r\n\r\n{}");
        let mut rejected = Cursor::new(rejected);
        assert!(matches!(
            read_lsp_frame(&mut rejected),
            Err(FrameError::HeadersTooLarge)
        ));
    }

    #[test]
    #[cfg(unix)]
    fn most_specific_workspace_root_supplies_a_relative_path() {
        let roots = WorkspaceRoots::from_initialize(&json!({"params":{"workspaceFolders":[
            {"uri":"file:///tmp/anvil%20project"},
            {"uri":"file:///tmp/anvil%20project/crates/cli"}
        ]}}));
        assert_eq!(
            roots
                .relative_path("file:///tmp/anvil%20project/crates/cli/src/main.rs")
                .unwrap(),
            std::path::Path::new("src/main.rs")
        );
        assert!(roots.relative_path("file:///tmp/outside.rs").is_err());
        assert!(
            roots
                .relative_path("file:///tmp/anvil%20project/crates/cli/../outside.rs")
                .is_err()
        );
        let oversized_uri = format!("file:///tmp/{}", "a".repeat(4096));
        assert!(roots.relative_path(&oversized_uri).is_err());
    }

    #[test]
    #[cfg(unix)]
    fn duplicate_same_depth_workspace_roots_are_removed() {
        let roots = WorkspaceRoots::from_initialize(&json!({"params":{"workspaceFolders":[
            {"uri":"file:///tmp/a"},
            {"uri":"file:///tmp/b"},
            {"uri":"file:///tmp/a"}
        ]}}));

        assert_eq!(
            roots.0,
            vec![
                std::path::PathBuf::from("/tmp/a"),
                std::path::PathBuf::from("/tmp/b")
            ]
        );
    }

    #[test]
    #[cfg(unix)]
    fn missing_workspace_metadata_falls_back_to_process_working_directory() {
        let fallback = std::path::PathBuf::from("/tmp/anvil-fallback");
        let roots = WorkspaceRoots::from_initialize_with_fallback(
            &json!({"params": {}}),
            Some(fallback.clone()),
        );

        assert_eq!(roots.0, vec![fallback]);
        assert_eq!(
            roots
                .relative_path("file:///tmp/anvil-fallback/src/main.rs")
                .unwrap(),
            std::path::Path::new("src/main.rs")
        );
    }

    #[test]
    #[cfg(unix)]
    fn file_uri_percent_encoding_is_decoded_without_filesystem_access() {
        let path =
            file_uri_to_path("file:///tmp/anvil%20project/src/main.rs").expect("valid file URI");

        assert_eq!(path, std::path::Path::new("/tmp/anvil project/src/main.rs"));
        assert!(file_uri_to_path("untitled:buffer").is_err());
    }

    #[test]
    #[cfg(windows)]
    fn windows_file_uri_drive_prefix_is_normalised() {
        let path = file_uri_to_path("file:///c:/anvil%20project/src/main.rs")
            .expect("valid Windows file URI");

        assert_eq!(path, std::path::Path::new("C:/anvil project/src/main.rs"));

        let roots = WorkspaceRoots::from_initialize(&json!({
            "params": {"rootUri": "file:///C:/anvil%20project"}
        }));
        assert_eq!(
            roots
                .relative_path("file:///c:/anvil%20project/src/main.rs")
                .expect("drive-letter case must not change workspace membership"),
            std::path::Path::new("src/main.rs")
        );
    }
}
