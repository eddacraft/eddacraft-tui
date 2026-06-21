/// Stable version token for the GCTX estimator contract.
pub const GCTX_TOKEN_ESTIMATOR_VERSION: &str = "gctx-simple-v1";

/// Keep estimator work bounded for interactive MCP planning calls.
pub const MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenEstimate {
    pub estimator: &'static str,
    pub tokens: usize,
    pub input_bytes: usize,
    /// Reserved for a future cap-on-overflow mode. The current estimator
    /// *rejects* oversized input with [`TokenEstimateError::InputTooLarge`]
    /// rather than truncating it, so this is always `false` today; it exists so
    /// callers can branch on truncation without an API break when capping lands.
    pub capped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TokenEstimateError {
    #[error(
        "input is too large for GCTX token estimation: {input_bytes} bytes > {max_bytes} bytes"
    )]
    InputTooLarge {
        input_bytes: usize,
        max_bytes: usize,
    },
}

pub fn estimate_gctx_tokens(
    input: &str,
    language: Option<&str>,
) -> Result<TokenEstimate, TokenEstimateError> {
    let _ = language;
    let input_bytes = input.len();
    if input_bytes > MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES {
        return Err(TokenEstimateError::InputTooLarge {
            input_bytes,
            max_bytes: MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES,
        });
    }

    Ok(TokenEstimate {
        estimator: GCTX_TOKEN_ESTIMATOR_VERSION,
        tokens: estimate_conservative_tokens(input),
        input_bytes,
        capped: false,
    })
}

/// Accuracy envelope: this is a deliberately conservative *upper-leaning*
/// approximation, not a model-faithful tokenizer. It takes the larger of two
/// signals — counted lexical units (word runs, newlines, punctuation) and the
/// classic `bytes / 4` chars-per-token ceiling — so it tends to over-count rather
/// than under-count for the corpora GCTX serves (source snippets and
/// identity-only graph summaries). Across the cl100k / o200k BPE families this
/// keeps real counts at or below the estimate for ordinary code and prose; the
/// one regime it can under-shoot is high-entropy unbroken blobs (long base64 or
/// minified runs with no separators), where BPE may exceed `bytes / 4`. That is
/// out of scope for the identity-surface inputs here, so callers should treat
/// the result as a planning budget, not a billing figure.
fn estimate_conservative_tokens(input: &str) -> usize {
    if input.is_empty() {
        return 0;
    }

    lexical_units(input).max(input.len().div_ceil(4))
}

fn lexical_units(input: &str) -> usize {
    let mut count = 0;
    let mut in_word = false;

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            if !in_word {
                count += 1;
                in_word = true;
            }
            continue;
        }

        in_word = false;

        if ch.is_whitespace() {
            if ch == '\n' {
                count += 1;
            }
        } else if ch.is_ascii() {
            count += 1;
        } else {
            count += ch.len_utf8().div_ceil(3);
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gctx_token_estimator_is_deterministic_for_fixed_corpus() {
        let source = "export function alpha(value: string) {\n  return value.trim();\n}\n";

        let first = estimate_gctx_tokens(source, Some("typescript")).unwrap();
        let second = estimate_gctx_tokens(source, Some("typescript")).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.estimator, GCTX_TOKEN_ESTIMATOR_VERSION);
        assert_eq!(first.input_bytes, source.len());
        assert!(!first.capped);
    }

    #[test]
    fn gctx_token_estimator_is_conservative_for_reference_corpus() {
        let corpus = [
            (
                "typescript function",
                "export function alpha(value: string) {\n  return value.trim();\n}\n",
                16,
            ),
            (
                "rust impl",
                "impl Runner {\n    pub fn run(&self) -> Result<()> { Ok(()) }\n}\n",
                18,
            ),
            (
                "graph summary",
                "symbol alpha function public src/a.ts depends_on src/b.ts",
                12,
            ),
        ];

        for (name, input, reference_count) in corpus {
            let estimate = estimate_gctx_tokens(input, None).unwrap();

            assert!(
                estimate.tokens >= reference_count,
                "{name}: estimated {} tokens but reference was {reference_count}",
                estimate.tokens
            );
        }
    }

    #[test]
    fn gctx_token_estimator_rejects_oversized_input() {
        let input = "a".repeat(MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES + 1);

        let err = estimate_gctx_tokens(&input, None).unwrap_err();

        assert_eq!(
            err,
            TokenEstimateError::InputTooLarge {
                input_bytes: input.len(),
                max_bytes: MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES,
            }
        );
    }
}
