//! W3C Trace Context `traceparent` parse/generate helpers.
//!
//! Implements the version-`00` shape of the W3C Trace Context spec
//! (<https://www.w3.org/TR/trace-context/#traceparent-header>):
//!
//! ```text
//! version "-" trace-id "-" parent-id "-" trace-flags
//! 00      -   32 hex    -   16 hex    -   2 hex
//! ```
//!
//! Per ADR-035 this is the **cross-pipe correlation key** Anvil pins on
//! every span, on the JSON-RPC envelope, and on the notification
//! envelope. Parsing is strict (lower-case hex, exact lengths); rejects
//! the all-zero trace-id / parent-id forbidden by the spec; and rejects
//! the version byte `ff` reserved by the spec.

use std::fmt;
use std::str::FromStr;

const VERSION_BYTES: usize = 2;
const TRACE_ID_BYTES: usize = 32;
const PARENT_ID_BYTES: usize = 16;
const FLAGS_BYTES: usize = 2;

/// Total length of a version-`00` `traceparent` header value.
pub const TRACEPARENT_LEN: usize =
    VERSION_BYTES + 1 + TRACE_ID_BYTES + 1 + PARENT_ID_BYTES + 1 + FLAGS_BYTES;

const ZERO_TRACE_ID: &str = "00000000000000000000000000000000";
const ZERO_PARENT_ID: &str = "0000000000000000";
const RESERVED_VERSION: &str = "ff";

/// Parsed W3C `traceparent` header (version `00`).
///
/// Hex strings are stored in lower-case canonical form; `Display` /
/// [`as_header`](TraceContext::as_header) emit the exact bytes the spec
/// defines.
///
/// Intentionally does **not** derive `Serialize` / `Deserialize`. A
/// derived deserialiser would let callers construct a `TraceContext`
/// without going through [`TraceContext::parse`] and bypass the
/// canonical-form invariants. Callers that need to round-trip a
/// `TraceContext` over the wire serialise [`as_header`](TraceContext::as_header)
/// and parse on the receiving end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceContext {
    trace_id: String,
    parent_id: String,
    flags: u8,
}

/// Errors produced by [`TraceContext::parse`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TraceContextError {
    /// Header bytes were not the expected length.
    #[error("traceparent must be {TRACEPARENT_LEN} bytes, got {0}")]
    Length(usize),
    /// Header did not have the four `-`-separated fields.
    #[error("traceparent must have version, trace-id, parent-id, and flags fields")]
    Shape,
    /// Field contained non-lower-hex characters.
    #[error("traceparent {field} field must be lower-case hex")]
    NotHex {
        /// Which field failed validation.
        field: &'static str,
    },
    /// Version byte was the reserved `ff`.
    #[error("traceparent version ff is reserved")]
    ReservedVersion,
    /// Version byte was not `00`. Only version `00` is implemented.
    /// The submitted bytes are deliberately not echoed in the error
    /// message — every error path is reflected back into log streams,
    /// and there is nothing useful in showing the rejected version.
    #[error("traceparent version is not supported (only 00 is implemented)")]
    UnsupportedVersion,
    /// `trace-id` was the all-zero form forbidden by the spec.
    #[error("traceparent trace-id must not be all zero")]
    AllZeroTraceId,
    /// `parent-id` was the all-zero form forbidden by the spec.
    #[error("traceparent parent-id must not be all zero")]
    AllZeroParentId,
}

impl TraceContext {
    /// Parse a `traceparent` header value.
    ///
    /// Strict: rejects upper-case hex, wrong lengths, the all-zero
    /// trace-id/parent-id forms, and the reserved `ff` version byte.
    ///
    /// # Errors
    ///
    /// Returns [`TraceContextError`] describing which validation rule
    /// the input violated.
    pub fn parse(input: &str) -> Result<Self, TraceContextError> {
        // The W3C header is fixed-length lower-hex ASCII. Asserting
        // ASCII up-front turns the byte-level hex check below into an
        // explicit invariant rather than an accidental byproduct of
        // multi-byte UTF-8 falling outside the hex range.
        if !input.is_ascii() {
            return Err(TraceContextError::Shape);
        }
        if input.len() != TRACEPARENT_LEN {
            return Err(TraceContextError::Length(input.len()));
        }

        let mut parts = input.split('-');
        let version = parts.next().ok_or(TraceContextError::Shape)?;
        let trace_id = parts.next().ok_or(TraceContextError::Shape)?;
        let parent_id = parts.next().ok_or(TraceContextError::Shape)?;
        let flags = parts.next().ok_or(TraceContextError::Shape)?;
        if parts.next().is_some()
            || version.len() != VERSION_BYTES
            || trace_id.len() != TRACE_ID_BYTES
            || parent_id.len() != PARENT_ID_BYTES
            || flags.len() != FLAGS_BYTES
        {
            return Err(TraceContextError::Shape);
        }

        // Validate and reject by version first so a wrong version pays
        // for one hex check instead of all four.
        ensure_lower_hex(version, "version")?;
        if version == RESERVED_VERSION {
            return Err(TraceContextError::ReservedVersion);
        }
        if version != "00" {
            return Err(TraceContextError::UnsupportedVersion);
        }

        ensure_lower_hex(trace_id, "trace-id")?;
        ensure_lower_hex(parent_id, "parent-id")?;
        ensure_lower_hex(flags, "flags")?;

        if trace_id == ZERO_TRACE_ID {
            return Err(TraceContextError::AllZeroTraceId);
        }
        if parent_id == ZERO_PARENT_ID {
            return Err(TraceContextError::AllZeroParentId);
        }

        let flags = u8::from_str_radix(flags, 16).expect("validated lower-hex flags");

        Ok(Self {
            trace_id: trace_id.to_owned(),
            parent_id: parent_id.to_owned(),
            flags,
        })
    }

    /// 32-character lower-hex trace-id.
    #[must_use]
    pub fn trace_id(&self) -> &str {
        &self.trace_id
    }

    /// 16-character lower-hex parent-id (the upstream span id).
    #[must_use]
    pub fn parent_id(&self) -> &str {
        &self.parent_id
    }

    /// W3C `trace-flags` byte. Bit 0 (`01`) is the `sampled` flag.
    #[must_use]
    pub fn flags(&self) -> u8 {
        self.flags
    }

    /// True if the upstream caller marked the trace as sampled.
    #[must_use]
    pub fn is_sampled(&self) -> bool {
        self.flags & 0x01 != 0
    }

    /// Render the canonical `traceparent` header value.
    #[must_use]
    pub fn as_header(&self) -> String {
        format!("00-{}-{}-{:02x}", self.trace_id, self.parent_id, self.flags)
    }
}

impl fmt::Display for TraceContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_header())
    }
}

impl FromStr for TraceContext {
    type Err = TraceContextError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

fn ensure_lower_hex(value: &str, field: &'static str) -> Result<(), TraceContextError> {
    if value
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err(TraceContextError::NotHex { field })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";

    #[test]
    fn parses_canonical_header() {
        let ctx = TraceContext::parse(VALID).expect("valid header");
        assert_eq!(ctx.trace_id(), "0af7651916cd43dd8448eb211c80319c");
        assert_eq!(ctx.parent_id(), "b7ad6b7169203331");
        assert_eq!(ctx.flags(), 0x01);
        assert!(ctx.is_sampled());
    }

    #[test]
    fn round_trip_is_byte_for_byte_identical() {
        let ctx = TraceContext::parse(VALID).expect("valid");
        assert_eq!(ctx.as_header(), VALID);
    }

    #[test]
    fn rejects_upper_case_hex() {
        let upper = "00-0AF7651916CD43DD8448EB211C80319C-b7ad6b7169203331-01";
        let err = TraceContext::parse(upper).expect_err("upper-case must be rejected");
        assert!(matches!(
            err,
            TraceContextError::NotHex { field: "trace-id" }
        ));
    }

    #[test]
    fn rejects_wrong_length() {
        let err = TraceContext::parse("00-too-short").expect_err("too short");
        assert!(matches!(err, TraceContextError::Length(_)));
    }

    #[test]
    fn rejects_all_zero_trace_id() {
        let zero = "00-00000000000000000000000000000000-b7ad6b7169203331-00";
        assert_eq!(
            TraceContext::parse(zero),
            Err(TraceContextError::AllZeroTraceId)
        );
    }

    #[test]
    fn rejects_all_zero_parent_id() {
        let zero = "00-0af7651916cd43dd8448eb211c80319c-0000000000000000-00";
        assert_eq!(
            TraceContext::parse(zero),
            Err(TraceContextError::AllZeroParentId)
        );
    }

    #[test]
    fn rejects_reserved_version_ff() {
        let reserved = "ff-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        assert_eq!(
            TraceContext::parse(reserved),
            Err(TraceContextError::ReservedVersion)
        );
    }

    #[test]
    fn rejects_unsupported_version() {
        let unsupported = "01-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        assert_eq!(
            TraceContext::parse(unsupported),
            Err(TraceContextError::UnsupportedVersion)
        );
    }

    #[test]
    fn rejects_misplaced_hyphen_inside_field() {
        // Exactly 55 bytes with an extra hyphen embedded inside the
        // trace-id field (compensated by dropping one byte from flags).
        // split('-') sees five segments; the shape check rejects on
        // either the per-field length mismatch or the leftover segment.
        let crafted = "00-0af7651916cd43dd8448eb211c8031-9c-b7ad6b7169203331-1";
        assert_eq!(crafted.len(), TRACEPARENT_LEN);
        let err = TraceContext::parse(crafted).expect_err("must reject");
        assert!(matches!(err, TraceContextError::Shape));
    }

    #[test]
    fn rejects_non_ascii_input_over_length() {
        // 55 displayed chars but >55 bytes once a multi-byte char is in.
        // Could be caught by either the is_ascii guard or the length
        // check; the guard fires first by construction.
        let non_ascii = "00-0af7651916cd43dd8448eb211c80319é-b7ad6b7169203331-01";
        assert!(!non_ascii.is_ascii());
        assert!(non_ascii.len() > TRACEPARENT_LEN);
        assert_eq!(
            TraceContext::parse(non_ascii),
            Err(TraceContextError::Shape)
        );
    }

    #[test]
    fn rejects_non_ascii_input_at_exact_length() {
        // 55 bytes exactly: the trailing "01" (2 bytes) is replaced
        // with "é" (2 bytes in UTF-8). Length check passes; only the
        // is_ascii guard fires before the per-field hex check is
        // reached. Pins the guard as the primary defence for inputs
        // that satisfy the length check but smuggle non-ASCII bytes.
        let non_ascii = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-é";
        assert!(!non_ascii.is_ascii());
        assert_eq!(non_ascii.len(), TRACEPARENT_LEN);
        assert_eq!(
            TraceContext::parse(non_ascii),
            Err(TraceContextError::Shape)
        );
    }

    #[test]
    fn from_str_works() {
        let ctx: TraceContext = VALID.parse().expect("from_str");
        assert_eq!(ctx.as_header(), VALID);
    }

    #[test]
    fn unsampled_flag() {
        let unsampled = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-00";
        let ctx = TraceContext::parse(unsampled).expect("valid");
        assert!(!ctx.is_sampled());
    }
}
