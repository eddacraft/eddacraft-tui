use std::fmt;
use std::io::{self, BufRead, Read};
use std::path::PathBuf;

pub(super) const MAX_LSP_FRAME_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug)]
pub(super) enum FrameError {
    Io(io::Error),
    MissingContentLength,
    DuplicateContentLength,
    InvalidContentLength,
    FrameTooLarge,
}

impl fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "LSP frame I/O failed: {error}"),
            Self::MissingContentLength => formatter.write_str("missing Content-Length header"),
            Self::DuplicateContentLength => formatter.write_str("duplicate Content-Length header"),
            Self::InvalidContentLength => formatter.write_str("invalid Content-Length header"),
            Self::FrameTooLarge => formatter.write_str("LSP frame exceeds 4 MiB limit"),
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
    let decoded = String::from_utf8(decoded).map_err(|_| UriError)?;
    #[cfg(windows)]
    let decoded = decoded
        .strip_prefix('/')
        .filter(|path| path.as_bytes().get(1) == Some(&b':'))
        .unwrap_or(&decoded)
        .to_string();
    let path = PathBuf::from(decoded);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(UriError)
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
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return if content_length.is_none() {
                Ok(None)
            } else {
                Err(FrameError::InvalidContentLength)
            };
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
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
        io::copy(&mut reader.take(length as u64), &mut io::sink())?;
        return Err(FrameError::FrameTooLarge);
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    Ok(Some(body))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{FrameError, file_uri_to_path, read_lsp_frame};

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
        let path = file_uri_to_path("file:///C:/anvil%20project/src/main.rs")
            .expect("valid Windows file URI");

        assert_eq!(path, std::path::Path::new("C:/anvil project/src/main.rs"));
    }
}
