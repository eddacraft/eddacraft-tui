//! Redaction deny-list for span attributes in JSON-formatted trace output.
//!
//! This module owns the TRACE-003 local tracing-pipe redaction layer:
//! span values whose field names match [`SENSITIVE_FIELDS`] are replaced
//! with [`REDACTED`] before the JSON formatter writes them to stderr or
//! a local trace file. The deny-list is intentionally exact-match and
//! case-insensitive so safe fields such as `token_type` are not
//! accidentally destroyed. The broader INTD-015 / EXPORT policy parity work
//! remains blocked, but local JSON span and event output both use this
//! formatter before writing to stderr or a local trace file.
//!
//! The marker [`REDACTED`] is the canonical replacement string the
//! TRACE-003 layer emits; tests pin it so a contract change is loud.

use std::collections::BTreeMap;
use std::fmt;

use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};
use tracing::{Event, Subscriber, field, span};
use tracing_subscriber::field::RecordFields;
use tracing_subscriber::fmt::format::{FormatFields, Writer};
use tracing_subscriber::fmt::time::{FormatTime, SystemTime};
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormattedFields};
use tracing_subscriber::registry::LookupSpan;

/// Canonical replacement value for redacted span attributes / log fields.
pub const REDACTED: &str = "<redacted>";

/// Field names whose **values** must never be forwarded into a span
/// attribute. Lower-case; comparison is case-insensitive.
///
/// Sourced from the OWASP secret-name patterns Anvil's secret-detection
/// rule already recognises, plus the deny-list INTD-013 reviewers
/// flagged on `notification.context`.
pub const SENSITIVE_FIELDS: &[&str] = &[
    "api_key",
    "apikey",
    "access_key",
    "auth",
    "authorization",
    "bearer",
    "client_secret",
    "context",
    "credential",
    "credentials",
    "notification.context",
    "notification_context",
    "password",
    "passwd",
    "pwd",
    "private_key",
    "secret",
    "session_token",
    "token",
];

/// Returns `true` if `field` exactly matches a known-sensitive field
/// name (case-insensitive). Substrings do **not** match: `token_type`
/// is allowed even though `token` is on the list. Callers that need
/// pattern matching must layer their own logic on top.
///
/// Consulted at runtime: [`init_tracing`](super::init_tracing) installs
/// [`RedactingJsonFields`]/[`RedactingJsonEventFormatter`], which call this
/// to decide whether a span/event field value is replaced with
/// [`REDACTED`]. See the module-level note.
#[must_use]
pub fn is_sensitive_field(field: &str) -> bool {
    SENSITIVE_FIELDS
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(field))
}

/// Coarse classification of an argument value's *shape*, recorded
/// without the value itself.
///
/// Captured for usage analytics (USAGE-001) so the founder can ask
/// "which commands take which kinds of arguments" without any raw
/// value ever landing in a Kindling row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArgValueType {
    /// A value that parses as a signed integer.
    Integer,
    /// A value equal to `true`/`false` (case-insensitive).
    Boolean,
    /// Any other supplied value.
    String,
    /// A bare flag with no attached value (e.g. `--json`).
    Flag,
}

/// Coarse length bucket for an argument value.
///
/// Deliberately *not* the exact character count: an exact length lets a
/// reader confirm a fixed-length secret (a 40-char token, a 64-hex
/// digest, a JWT of known size) passed under a non-sensitive flag name.
/// Buckets keep the analytics signal ("are these short flags or long
/// paths?") without disclosing a value-confirming measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArgLength {
    /// Empty value (`""`).
    Empty,
    /// 1–8 characters.
    Short,
    /// 9–32 characters.
    Medium,
    /// 33 or more characters.
    Long,
}

#[must_use]
fn length_bucket(len: usize) -> ArgLength {
    match len {
        0 => ArgLength::Empty,
        1..=8 => ArgLength::Short,
        9..=32 => ArgLength::Medium,
        _ => ArgLength::Long,
    }
}

/// Redacted, value-free description of a single command-line argument.
///
/// Records the argument *name* and the *shape* of its value (type,
/// coarse length bucket, presence) but never the value itself. For
/// arguments whose name matches [`SENSITIVE_FIELDS`], even the shape is
/// elided: `shape`/`length` are `None` and `redacted` carries the
/// [`REDACTED`] marker, so a sensitive argument's existence stays
/// visible while nothing about its value leaks. This type is the
/// USAGE-001 privacy contract expressed in code.
///
/// Note: redaction keys off the argument *name* only. A secret passed
/// under a non-sensitive flag name (e.g. `--output <token>`) or as a
/// positional is **not** name-redacted; the [`ArgLength`] bucket is the
/// backstop that keeps its exact length from leaking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArgShape {
    /// The argument name as typed (e.g. `path`, `json`), without the
    /// leading dashes. Never a value.
    pub name: String,
    /// Redaction marker. `Some(REDACTED)` when the name is sensitive,
    /// `None` otherwise. When set, `shape`/`length`/`present` are
    /// suppressed. Owned (not `&'static str`) so the type stays
    /// `Deserialize`-clean when nested in observation rows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted: Option<String>,
    /// The value's coarse type, when not redacted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape: Option<ArgValueType>,
    /// The value's coarse length bucket, when not redacted and a value
    /// was supplied. Never the exact length (see [`ArgLength`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<ArgLength>,
    /// Whether a value was supplied (`true`) versus a bare flag
    /// (`false`).
    #[serde(default, skip_serializing_if = "is_false")]
    pub present: bool,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_false(b: &bool) -> bool {
    !*b
}

/// Build a redacted [`ArgShape`] for one argument.
///
/// `value` is `None` for a bare flag and `Some(_)` for an option that
/// carried a value. The raw `value` is consumed only to measure its
/// shape (coarse length bucket + coarse type) and is never stored. If
/// `name` matches [`SENSITIVE_FIELDS`], the shape is elided and the
/// [`REDACTED`] marker is set instead — a sensitive argument
/// contributes only its name plus the marker.
#[must_use]
pub fn redact_arg(name: &str, value: Option<&str>) -> ArgShape {
    if is_sensitive_field(name) {
        return ArgShape {
            name: name.to_owned(),
            redacted: Some(REDACTED.to_owned()),
            shape: None,
            length: None,
            present: false,
        };
    }
    match value {
        None => ArgShape {
            name: name.to_owned(),
            redacted: None,
            shape: Some(ArgValueType::Flag),
            length: None,
            present: false,
        },
        Some(v) => ArgShape {
            name: name.to_owned(),
            redacted: None,
            shape: Some(classify_value(v)),
            length: Some(length_bucket(v.chars().count())),
            present: true,
        },
    }
}

fn classify_value(value: &str) -> ArgValueType {
    if value.parse::<i64>().is_ok() {
        ArgValueType::Integer
    } else if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false") {
        ArgValueType::Boolean
    } else {
        ArgValueType::String
    }
}

/// JSON [`FormatFields`] implementation that redacts sensitive span
/// field values before subscriber output is formatted.
#[derive(Debug, Default, Clone, Copy)]
pub struct RedactingJsonFields;

/// JSON event formatter paired with [`RedactingJsonFields`].
///
/// `tracing-subscriber`'s stock JSON event formatter serialises event fields
/// through `tracing-serde`, bypassing the configured [`FormatFields`]. This
/// formatter keeps Anvil's existing JSON-line shape while routing event fields
/// through the same redaction visitor used for spans.
#[derive(Debug, Default, Clone, Copy)]
pub struct RedactingJsonEventFormatter {
    timer: SystemTime,
}

impl<S> FormatEvent<S, RedactingJsonFields> for RedactingJsonEventFormatter
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, RedactingJsonFields>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let mut timestamp = String::new();
        self.timer.format_time(&mut Writer::new(&mut timestamp))?;

        let mut fields = RedactingJsonVisitor::default();
        event.record(&mut fields);

        let meta = event.metadata();
        let current_span = event
            .parent()
            .and_then(|id| ctx.span(id))
            .or_else(|| ctx.lookup_current());

        let mut serialiser = serde_json::Serializer::new(WriteAdaptor::new(&mut writer));
        let mut map = serialiser.serialize_map(None).map_err(|_| fmt::Error)?;
        map.serialize_entry("timestamp", &timestamp)
            .map_err(|_| fmt::Error)?;
        map.serialize_entry("level", &meta.level().to_string())
            .map_err(|_| fmt::Error)?;
        map.serialize_entry("fields", &fields.values)
            .map_err(|_| fmt::Error)?;
        map.serialize_entry("target", meta.target())
            .map_err(|_| fmt::Error)?;

        if let Some(span) = current_span {
            let span_fields = span
                .extensions()
                .get::<FormattedFields<RedactingJsonFields>>()
                .map_or_else(serde_json::Map::new, formatted_fields_to_map);
            let mut span_value = span_fields;
            span_value.insert(
                "name".to_owned(),
                serde_json::Value::String(span.metadata().name().to_owned()),
            );
            map.serialize_entry("span", &span_value)
                .map_err(|_| fmt::Error)?;
        }

        map.end().map_err(|_| fmt::Error)?;
        writeln!(writer)
    }
}

impl<'writer> FormatFields<'writer> for RedactingJsonFields {
    fn format_fields<R: RecordFields>(
        &self,
        mut writer: Writer<'writer>,
        fields: R,
    ) -> fmt::Result {
        let mut visitor = RedactingJsonVisitor::default();
        fields.record(&mut visitor);
        write_json_fields(&mut writer, visitor.values)
    }

    fn add_fields(
        &self,
        current: &'writer mut FormattedFields<Self>,
        fields: &span::Record<'_>,
    ) -> fmt::Result {
        let mut visitor = RedactingJsonVisitor {
            values: if current.is_empty() {
                BTreeMap::new()
            } else {
                serde_json::from_str(current).map_err(|_| fmt::Error)?
            },
        };
        fields.record(&mut visitor);

        let mut next = String::new();
        write_json_fields(&mut Writer::new(&mut next), visitor.values)?;
        current.fields = next;
        Ok(())
    }
}

#[derive(Default)]
struct RedactingJsonVisitor {
    values: BTreeMap<String, serde_json::Value>,
}

impl field::Visit for RedactingJsonVisitor {
    fn record_f64(&mut self, field: &field::Field, value: f64) {
        self.insert(field, serde_json::Value::from(value));
    }

    fn record_i64(&mut self, field: &field::Field, value: i64) {
        self.insert(field, serde_json::Value::from(value));
    }

    fn record_u64(&mut self, field: &field::Field, value: u64) {
        self.insert(field, serde_json::Value::from(value));
    }

    fn record_bool(&mut self, field: &field::Field, value: bool) {
        self.insert(field, serde_json::Value::from(value));
    }

    fn record_str(&mut self, field: &field::Field, value: &str) {
        self.insert(field, serde_json::Value::from(value));
    }

    fn record_bytes(&mut self, field: &field::Field, value: &[u8]) {
        self.insert(field, serde_json::Value::from(value));
    }

    fn record_debug(&mut self, field: &field::Field, value: &dyn fmt::Debug) {
        self.insert(field, serde_json::Value::from(format!("{value:?}")));
    }
}

impl RedactingJsonVisitor {
    fn insert(&mut self, field: &field::Field, value: serde_json::Value) {
        let name = json_field_name(field.name());
        let value = if is_sensitive_field(name) {
            serde_json::Value::from(REDACTED)
        } else {
            value
        };
        self.values.insert(name.to_owned(), value);
    }
}

fn json_field_name(name: &str) -> &str {
    name.strip_prefix("r#").unwrap_or(name)
}

fn write_json_fields(
    writer: &mut dyn fmt::Write,
    values: BTreeMap<String, serde_json::Value>,
) -> fmt::Result {
    let inner = || {
        let mut serialiser = serde_json::Serializer::new(WriteAdaptor::new(writer));
        let mut map = serialiser.serialize_map(None)?;
        for (key, value) in values {
            map.serialize_entry(&key, &value)?;
        }
        map.end()
    };

    inner().map_err(|_| fmt::Error)
}

fn formatted_fields_to_map(
    fields: &FormattedFields<RedactingJsonFields>,
) -> serde_json::Map<String, serde_json::Value> {
    serde_json::from_str(fields).unwrap_or_default()
}

struct WriteAdaptor<'a> {
    fmt: &'a mut dyn fmt::Write,
}

impl<'a> WriteAdaptor<'a> {
    fn new(fmt: &'a mut dyn fmt::Write) -> Self {
        Self { fmt }
    }
}

impl std::io::Write for WriteAdaptor<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let s = std::str::from_utf8(buf)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        self.fmt
            .write_str(s)
            .map_err(|_| std::io::Error::other("format writer failed"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    use tracing::subscriber::with_default;
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;

    #[test]
    fn matches_lowercase_canonical_form() {
        assert!(is_sensitive_field("api_key"));
        assert!(is_sensitive_field("authorization"));
        assert!(is_sensitive_field("notification.context"));
        assert!(is_sensitive_field("password"));
    }

    #[test]
    fn matches_case_insensitively() {
        assert!(is_sensitive_field("API_KEY"));
        assert!(is_sensitive_field("Authorization"));
        assert!(is_sensitive_field("Password"));
    }

    #[test]
    fn redact_arg_records_string_shape_not_value() {
        let shape = redact_arg("path", Some("/home/secret/repo"));
        assert_eq!(shape.name, "path");
        assert_eq!(shape.redacted, None);
        assert_eq!(shape.shape, Some(ArgValueType::String));
        // 17 chars → Medium bucket (coarse, not the exact length).
        assert_eq!(shape.length, Some(ArgLength::Medium));
        assert!(shape.present);
        // The raw value must never appear in the serialised row.
        let json = serde_json::to_string(&shape).expect("serialise");
        assert!(
            !json.contains("/home/secret/repo"),
            "raw value leaked into arg shape: {json}"
        );
    }

    #[test]
    fn redact_arg_classifies_integer_and_boolean() {
        assert_eq!(
            redact_arg("threshold", Some("42")).shape,
            Some(ArgValueType::Integer)
        );
        assert_eq!(
            redact_arg("rescan", Some("true")).shape,
            Some(ArgValueType::Boolean)
        );
        assert_eq!(
            redact_arg("rescan", Some("FALSE")).shape,
            Some(ArgValueType::Boolean)
        );
    }

    #[test]
    fn redact_arg_marks_bare_flag() {
        let shape = redact_arg("json", None);
        assert_eq!(shape.shape, Some(ArgValueType::Flag));
        assert_eq!(shape.length, None);
        assert!(!shape.present);
    }

    #[test]
    fn redact_arg_buckets_length_not_exact() {
        assert_eq!(redact_arg("a", Some("")).length, Some(ArgLength::Empty));
        assert_eq!(redact_arg("a", Some("abcd")).length, Some(ArgLength::Short));
        assert_eq!(
            redact_arg("a", Some(&"x".repeat(20))).length,
            Some(ArgLength::Medium)
        );
        assert_eq!(
            redact_arg("a", Some(&"x".repeat(64))).length,
            Some(ArgLength::Long)
        );
    }

    #[test]
    fn redact_arg_elides_sensitive_named_argument() {
        // A sensitive name redacts the shape entirely, even when a
        // value (and its length) was supplied.
        let shape = redact_arg("token", Some("super-secret-value"));
        assert_eq!(shape.name, "token");
        assert_eq!(shape.redacted.as_deref(), Some(REDACTED));
        assert_eq!(shape.shape, None);
        // Length is elided entirely for a sensitive-named argument
        // (structural assertion — no exact length is computed at all).
        assert_eq!(shape.length, None);
        assert!(!shape.present);
        let json = serde_json::to_string(&shape).expect("serialise");
        assert!(json.contains(REDACTED), "redaction marker missing: {json}");
        assert!(
            !json.contains("super-secret-value"),
            "sensitive value leaked: {json}"
        );
    }

    #[test]
    fn redact_arg_sensitive_check_is_case_insensitive() {
        assert_eq!(
            redact_arg("Authorization", Some("Bearer x"))
                .redacted
                .as_deref(),
            Some(REDACTED)
        );
        assert_eq!(
            redact_arg("PASSWORD", Some("hunter2")).redacted.as_deref(),
            Some(REDACTED)
        );
    }

    #[test]
    fn rejects_unrelated_names() {
        assert!(!is_sensitive_field("path"));
        assert!(!is_sensitive_field("trace_id"));
        assert!(!is_sensitive_field("trace_context"));
        assert!(!is_sensitive_field(""));
    }

    #[test]
    fn matches_exactly_not_substring() {
        // `token` is on the list; `token_type` and `pagination_token`
        // are common safe field names that must not be redacted.
        assert!(is_sensitive_field("token"));
        assert!(!is_sensitive_field("token_type"));
        assert!(!is_sensitive_field("pagination_token"));
        assert!(!is_sensitive_field("session_token_type"));
    }

    #[test]
    fn redacted_marker_is_stable() {
        // Pinned: TRACE-003 layer asserts on this exact string. Changing
        // it is a contract break across binary boundaries.
        assert_eq!(REDACTED, "<redacted>");
    }

    #[test]
    fn json_formatter_redacts_sensitive_span_fields_before_output() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let writer = TestWriter(output.clone());
        let subscriber = tracing_subscriber::registry().with(
            fmt::layer()
                .json()
                .fmt_fields(RedactingJsonFields)
                .with_ansi(false)
                .with_writer(move || writer.clone()),
        );

        with_default(subscriber, || {
            let span = tracing::info_span!(
                "redaction_probe",
                password = "super-secret",
                token_type = "bearer",
                path = "src/lib.rs"
            );
            let _entered = span.enter();
            tracing::info!("inside redaction probe");
        });

        let line = String::from_utf8(output.lock().expect("output").clone()).expect("utf8");
        assert!(
            line.contains(r#""password":"<redacted>""#),
            "sensitive field was not redacted: {line}"
        );
        assert!(
            !line.contains("super-secret"),
            "secret value leaked into trace output: {line}"
        );
        assert!(
            line.contains(r#""token_type":"bearer""#),
            "safe substring field should not be redacted: {line}"
        );
    }

    #[test]
    fn json_formatter_redacts_sensitive_event_fields_before_output() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let writer = TestWriter(output.clone());
        let subscriber = tracing_subscriber::registry().with(
            fmt::layer()
                .json()
                .fmt_fields(RedactingJsonFields)
                .event_format(RedactingJsonEventFormatter::default())
                .with_ansi(false)
                .with_writer(move || writer.clone()),
        );

        with_default(subscriber, || {
            tracing::info!(
                token = "secret-token",
                token_type = "bearer",
                path = "src/lib.rs",
                "event redaction probe"
            );
        });

        let line = String::from_utf8(output.lock().expect("output").clone()).expect("utf8");
        assert!(
            line.contains(r#""token":"<redacted>""#),
            "sensitive event field was not redacted: {line}"
        );
        assert!(
            !line.contains("secret-token"),
            "secret value leaked into event output: {line}"
        );
        assert!(
            line.contains(r#""token_type":"bearer""#),
            "safe event substring field should not be redacted: {line}"
        );
    }

    #[derive(Clone)]
    struct TestWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().expect("output").extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
}
