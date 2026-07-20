use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use anvil_kernel_types::Diagnostic;
use serde::{Deserialize, Serialize};

use crate::mcp::gctx_client::{DaemonRpcError, daemon_rpc_call};

const SCAN_BUFFER_VERSION: u64 = 1;
static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScanMode {
    MidEdit,
    PreWrite,
}

impl ScanMode {
    const fn wire_name(self) -> &'static str {
        match self {
            Self::MidEdit => "midEdit",
            Self::PreWrite => "preWrite",
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct ScanBufferParams<'a> {
    path: &'a str,
    text: &'a str,
    version: u64,
    mode: &'static str,
}

pub(crate) fn build_scan_buffer_params<'a>(
    mode: ScanMode,
    path: &'a str,
    text: &'a str,
) -> ScanBufferParams<'a> {
    ScanBufferParams {
        path,
        text,
        version: SCAN_BUFFER_VERSION,
        mode: mode.wire_name(),
    }
}

#[derive(Debug, Deserialize)]
struct ScanBufferResult {
    version: u64,
    diagnostics: Vec<Diagnostic>,
    truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScanBufferError {
    Unavailable,
    Failed,
    Truncated,
    VersionMismatch,
}

impl fmt::Display for ScanBufferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable => "daemon validation is unavailable",
            Self::Failed => "daemon validation failed",
            Self::Truncated => "daemon validation response was truncated",
            Self::VersionMismatch => "daemon validation response version did not match",
        })
    }
}

impl std::error::Error for ScanBufferError {}

pub(crate) fn scan_buffer(
    mode: ScanMode,
    path: &str,
    text: &str,
) -> Result<Vec<Diagnostic>, ScanBufferError> {
    let params = build_scan_buffer_params(mode, path, text);
    let request_id = format!(
        "cli-scan-buffer-{}",
        NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed)
    );
    let result: ScanBufferResult =
        daemon_rpc_call("scan_buffer", &params, &request_id).map_err(|error| match error {
            DaemonRpcError::Unavailable => ScanBufferError::Unavailable,
            DaemonRpcError::Failure => ScanBufferError::Failed,
        })?;
    if result.version != SCAN_BUFFER_VERSION {
        return Err(ScanBufferError::VersionMismatch);
    }
    if result.truncated {
        return Err(ScanBufferError::Truncated);
    }
    Ok(result.diagnostics)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{ScanMode, build_scan_buffer_params};

    #[test]
    fn mid_edit_request_uses_the_existing_scan_buffer_contract() {
        let params = build_scan_buffer_params(ScanMode::MidEdit, "src/main.rs", "fn main() {}");

        assert_eq!(
            serde_json::to_value(params).expect("serialise params"),
            json!({
                "path": "src/main.rs",
                "text": "fn main() {}",
                "version": 1,
                "mode": "midEdit"
            })
        );
    }

    #[test]
    fn pre_write_request_preserves_the_mcp_mode() {
        let params = build_scan_buffer_params(ScanMode::PreWrite, "src/main.rs", "fn main() {}");

        assert_eq!(
            serde_json::to_value(params).expect("serialise params")["mode"],
            "preWrite"
        );
    }
}
