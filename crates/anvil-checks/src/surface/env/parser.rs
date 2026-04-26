//! Minimal `.env` parser.
//!
//! Supports the dotenv subset that 99% of `.env` files in the wild use:
//!
//! - `KEY=VALUE` lines.
//! - `#`-prefixed comments and trailing comments on unquoted values.
//! - Single-quoted values (literal — no escape processing).
//! - Double-quoted values with `\\`, `\"`, `\n`, `\r`, `\t` escapes.
//! - Optional leading `export ` (treated as no-op).
//! - Blank lines.
//!
//! Out of scope for SURFENV-001 (per
//! `plans/modules/surface-env-files.aps.md`): multi-line values,
//! `${VAR}` interpolation, `.env.vault`-style encrypted formats.
//!
//! The parser is intentionally permissive — a malformed line yields an
//! `EnvParseError` (returned alongside successful entries from
//! `parse_env`) rather than aborting the whole file, so a single typo
//! does not silently disable secret scanning for the rest of the values.

use std::ops::Range;

/// One key/value pair extracted from a `.env` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvEntry {
    /// The variable name (left of `=`). `parse_env` skips lines that
    /// do not declare a key (recording an `EnvParseError` in the second
    /// return value), so this field is never empty on entries returned
    /// from `parse_env` — the documentation reflects the contract for
    /// callers.
    pub key: String,
    /// The decoded value (right of `=`, with quotes stripped and escapes
    /// resolved for double-quoted strings).
    pub value: String,
    /// 1-indexed source line number. Surfaces verbatim on resulting
    /// findings so operators can jump to the offending line.
    pub line: usize,
    /// Byte range of the *value* within `line`'s source text. Used by the
    /// scanner to compute the column offset of a finding within the file.
    pub value_span: Range<usize>,
    /// `true` when the value was wrapped in quotes (single or double) in
    /// the source. The structural ruleset (SURFENV-002+) uses this to
    /// distinguish a literal empty value `KEY=""` from `KEY=`.
    pub quoted: bool,
}

/// A non-fatal parse problem. The parser emits one of these per malformed
/// line and continues so the rest of the file still produces entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvParseError {
    pub line: usize,
    pub message: String,
}

/// Parse a `.env` file's content into entries plus any parse warnings.
///
/// The first return value is the entries that parsed cleanly; the second
/// is the per-line warnings the caller may want to surface. Neither list
/// is sorted — both follow source order so callers can correlate.
#[must_use]
pub fn parse_env(content: &str) -> (Vec<EnvEntry>, Vec<EnvParseError>) {
    let mut entries = Vec::new();
    let mut errors = Vec::new();

    for (index, raw_line) in content.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = raw_line.trim_start();

        // Blank line or pure comment — skip.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Strip optional `export ` prefix.
        let body = trimmed.strip_prefix("export ").unwrap_or(trimmed);

        let Some(equals_index) = body.find('=') else {
            errors.push(EnvParseError {
                line: line_number,
                message: "missing `=` separator".to_string(),
            });
            continue;
        };

        let key = body[..equals_index].trim().to_string();
        if key.is_empty() {
            errors.push(EnvParseError {
                line: line_number,
                message: "empty key".to_string(),
            });
            continue;
        }
        if !is_valid_key(&key) {
            errors.push(EnvParseError {
                line: line_number,
                message: format!("invalid key `{key}`"),
            });
            continue;
        }

        // Recompute spans against the *raw* line so callers can report
        // accurate columns. `body_offset` is the byte offset of `body` in
        // the raw line (skipped leading whitespace + optional `export `).
        let body_offset = raw_line.len() - body.len();
        let value_start_in_body = equals_index + 1;
        let value_in_body = &body[value_start_in_body..];
        let value_offset_in_line = body_offset + value_start_in_body;

        match decode_value(value_in_body) {
            Ok((value, quoted, consumed)) => {
                let value_span = value_offset_in_line..(value_offset_in_line + consumed);
                entries.push(EnvEntry {
                    key,
                    value,
                    line: line_number,
                    value_span,
                    quoted,
                });
            }
            Err(message) => {
                errors.push(EnvParseError {
                    line: line_number,
                    message,
                });
            }
        }
    }

    (entries, errors)
}

fn is_valid_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Decode the right-hand side of a `KEY=VALUE` line.
///
/// Returns the decoded value, whether the source was quoted, and the byte
/// length of the source consumed (used to compute the value's span in the
/// raw line). On a malformed quoted string returns the parse error message.
fn decode_value(rest: &str) -> Result<(String, bool, usize), String> {
    let trimmed_start = rest.trim_start_matches([' ', '\t']);
    let leading_ws = rest.len() - trimmed_start.len();

    if let Some(after_quote) = trimmed_start.strip_prefix('\'') {
        let (value, consumed) = decode_single_quoted(after_quote)?;
        Ok((value, true, leading_ws + 1 + consumed))
    } else if let Some(after_quote) = trimmed_start.strip_prefix('"') {
        let (value, consumed) = decode_double_quoted(after_quote)?;
        Ok((value, true, leading_ws + 1 + consumed))
    } else {
        // Unquoted: value runs to end of line or `#` comment marker.
        // `#` is only treated as a comment when preceded by whitespace —
        // matches the dotenv convention so that `URL=https://x#frag` is
        // not silently truncated.
        let mut end = trimmed_start.len();
        let bytes = trimmed_start.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'#' && (i == 0 || bytes[i - 1].is_ascii_whitespace()) {
                end = i;
                break;
            }
        }
        let value = trimmed_start[..end].trim_end().to_string();
        Ok((value, false, leading_ws + end))
    }
}

fn decode_single_quoted(rest: &str) -> Result<(String, usize), String> {
    // No escapes inside single quotes — match dotenv-cli semantics.
    let close = rest
        .find('\'')
        .ok_or_else(|| "unterminated single-quoted value".to_string())?;
    Ok((rest[..close].to_string(), close + 1))
}

fn decode_double_quoted(rest: &str) -> Result<(String, usize), String> {
    let mut out = String::with_capacity(rest.len());
    let bytes = rest.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        if byte == b'"' {
            return Ok((out, i + 1));
        }
        if byte == b'\\' && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            match next {
                b'\\' => out.push('\\'),
                b'"' => out.push('"'),
                b'\'' => out.push('\''),
                b'n' => out.push('\n'),
                b'r' => out.push('\r'),
                b't' => out.push('\t'),
                _ => {
                    // Unknown escape — keep the backslash + char verbatim
                    // so the scanner sees what was actually in the source.
                    out.push('\\');
                    out.push(next as char);
                }
            }
            i += 2;
            continue;
        }
        // Multi-byte UTF-8 sequence: copy the whole codepoint.
        let char_len = char_len_at(bytes, i);
        out.push_str(
            std::str::from_utf8(&bytes[i..i + char_len])
                .map_err(|err| format!("invalid utf-8 in double-quoted value: {err}"))?,
        );
        i += char_len;
    }
    Err("unterminated double-quoted value".to_string())
}

fn char_len_at(bytes: &[u8], i: usize) -> usize {
    let byte = bytes[i];
    if byte < 0x80 {
        1
    } else if byte < 0xC0 {
        // Continuation byte — should not start a char, but be defensive.
        1
    } else if byte < 0xE0 {
        2
    } else if byte < 0xF0 {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::{EnvParseError, parse_env};

    #[test]
    fn parses_basic_key_value() {
        let (entries, errors) = parse_env("FOO=bar\nBAZ=qux\n");
        assert!(errors.is_empty(), "no errors expected, got {errors:?}");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "FOO");
        assert_eq!(entries[0].value, "bar");
        assert_eq!(entries[0].line, 1);
        assert!(!entries[0].quoted);
        assert_eq!(entries[1].key, "BAZ");
        assert_eq!(entries[1].value, "qux");
        assert_eq!(entries[1].line, 2);
    }

    #[test]
    fn skips_blank_lines_and_comments() {
        let content = "\n# top comment\nFOO=bar\n\n# another\nBAZ=qux\n";
        let (entries, errors) = parse_env(content);
        assert!(errors.is_empty());
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].line, 3);
        assert_eq!(entries[1].line, 6);
    }

    #[test]
    fn parses_double_quoted_with_escapes() {
        let (entries, errors) = parse_env("PASS=\"hunter\\n\\t\\\"two\"\n");
        assert!(errors.is_empty());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].value, "hunter\n\t\"two");
        assert!(entries[0].quoted);
    }

    #[test]
    fn parses_single_quoted_literally() {
        let (entries, errors) = parse_env("PASS='hunter\\n'\n");
        assert!(errors.is_empty());
        assert_eq!(entries.len(), 1);
        // Single quotes are literal — `\n` stays as backslash + n.
        assert_eq!(entries[0].value, "hunter\\n");
        assert!(entries[0].quoted);
    }

    #[test]
    fn strips_trailing_unquoted_comment() {
        let (entries, errors) = parse_env("URL=https://example.com # canonical\n");
        assert!(errors.is_empty());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].value, "https://example.com");
    }

    #[test]
    fn keeps_hash_inside_unquoted_url_fragment() {
        // Bare `#` with no preceding whitespace stays inside the value —
        // otherwise we'd silently truncate `https://example.com#section`.
        let (entries, errors) = parse_env("URL=https://example.com#frag\n");
        assert!(errors.is_empty());
        assert_eq!(entries[0].value, "https://example.com#frag");
    }

    #[test]
    fn handles_export_prefix() {
        let (entries, errors) = parse_env("export FOO=bar\n");
        assert!(errors.is_empty());
        assert_eq!(entries[0].key, "FOO");
        assert_eq!(entries[0].value, "bar");
    }

    #[test]
    fn warns_on_missing_equals() {
        let (entries, errors) = parse_env("BARE_LINE\nFOO=bar\n");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "FOO");
        assert_eq!(
            errors,
            vec![EnvParseError {
                line: 1,
                message: "missing `=` separator".to_string(),
            }]
        );
    }

    #[test]
    fn warns_on_invalid_key() {
        let (entries, errors) = parse_env("9NOPE=bad\nOK=good\n");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "OK");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 1);
        assert!(errors[0].message.contains("9NOPE"));
    }

    #[test]
    fn warns_on_unterminated_double_quote() {
        let (entries, errors) = parse_env("KEY=\"unfinished\nOK=value\n");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "OK");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("unterminated"));
    }

    #[test]
    fn value_span_points_at_value_in_raw_line() {
        let (entries, _errors) = parse_env("AWS_ACCESS_KEY_ID=AKIAABCDEFGHIJKLMNOP\n");
        let entry = &entries[0];
        assert_eq!(entry.value, "AKIAABCDEFGHIJKLMNOP");
        // Value span should slice the raw line back to the value text.
        let raw_line = "AWS_ACCESS_KEY_ID=AKIAABCDEFGHIJKLMNOP";
        assert_eq!(&raw_line[entry.value_span.clone()], "AKIAABCDEFGHIJKLMNOP");
    }
}
