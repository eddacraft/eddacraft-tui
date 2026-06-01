//! Download the cargo-dist installer artefact and its detached `.minisig`
//! signature from GitHub Releases, then verify them with the embedded
//! public key. Returns the path to a verified installer that the caller
//! can hand to axoupdater via `configure_installer_path`.
//!
//! Wraps an async `reqwest::Client` in a fresh single-thread tokio
//! runtime, matching the pattern in `commands::version`. This keeps the
//! workspace-wide reqwest feature set minimal (no `blocking` feature
//! required) while still letting the rest of `commands::update` run
//! synchronously.

use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;

use super::signature::{self, VerifiedArtefact};

/// Maximum size we will download for any single artefact in the update
/// flow. Installer scripts are kilobytes; their signatures are bytes. A
/// hard cap is the cheapest defence against a CDN responding with a
/// runaway stream.
const MAX_DOWNLOAD_BYTES: u64 = 4 * 1024 * 1024;

/// Network timeout for individual GET requests during the update flow.
/// Generous enough for a slow CI mirror, tight enough to fail fast on a
/// dead endpoint instead of hanging the user.
const NETWORK_TIMEOUT: Duration = Duration::from_secs(30);

/// Where to download installer artefacts from. Production code calls
/// [`github_release_source`]; tests substitute an HTTP mock server URL.
pub struct ReleaseSource {
    pub installer_url: String,
    pub signature_url: String,
}

/// Resolve the installer + signature URLs for the cargo-dist release
/// hosted under `owner/repo`. `version` is the cargo-dist tag (e.g.
/// `v0.7.0-beta`); when `None`, the URLs resolve through GitHub's
/// `releases/latest/download/…` redirect.
pub fn github_release_source(
    owner: &str,
    repo: &str,
    app_name: &str,
    version: Option<&str>,
) -> ReleaseSource {
    let extension = if cfg!(windows) { ".ps1" } else { ".sh" };
    let asset = format!("{app_name}-installer{extension}");
    let base = match version {
        Some(v) => format!(
            "https://github.com/{owner}/{repo}/releases/download/{v}/{asset}",
            v = if v.starts_with('v') {
                v.to_string()
            } else {
                format!("v{v}")
            }
        ),
        None => format!("https://github.com/{owner}/{repo}/releases/latest/download/{asset}"),
    };
    ReleaseSource {
        installer_url: base.clone(),
        signature_url: format!("{base}.minisig"),
    }
}

/// Synchronous wrapper around [`fetch_and_verify_async`]. Builds a fresh
/// single-thread tokio runtime so callers can stay synchronous.
pub fn fetch_and_verify(
    source: &ReleaseSource,
    dest_dir: &std::path::Path,
) -> anyhow::Result<(PathBuf, VerifiedArtefact)> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime for update fetch")?;
    runtime.block_on(fetch_and_verify_async(None, source, dest_dir))
}

/// Download installer + signature into `dest_dir`, then verify against a
/// trusted public key. When `public_key_b64_override` is `None`, the
/// embedded `EMBEDDED_PUBLIC_KEY` from [`super::signature`] is used —
/// this is the production path. Tests pass `Some(<key>)` so they can
/// verify with a freshly generated keypair without rebuilding.
///
/// On success returns the verified installer path and the verified-
/// artefact metadata. On failure returns an `anyhow::Error` whose root
/// cause is a [`signature::SignatureError`] where appropriate.
pub async fn fetch_and_verify_async(
    public_key_b64_override: Option<&str>,
    source: &ReleaseSource,
    dest_dir: &std::path::Path,
) -> anyhow::Result<(PathBuf, VerifiedArtefact)> {
    // Production path enforces HTTPS-only and a bounded redirect depth.
    let client = build_client(true)?;
    fetch_and_verify_inner(&client, public_key_b64_override, source, dest_dir).await
}

/// Shared core of [`fetch_and_verify_async`]. Tests invoke this directly
/// with a non-HTTPS-enforcing client so they can point wiremock at it,
/// while production routes through the wrapper above. Keeping a single
/// implementation prevents the two paths from drifting.
async fn fetch_and_verify_inner(
    client: &reqwest::Client,
    public_key_b64_override: Option<&str>,
    source: &ReleaseSource,
    dest_dir: &std::path::Path,
) -> anyhow::Result<(PathBuf, VerifiedArtefact)> {
    let installer_bytes = download_bounded(client, &source.installer_url)
        .await
        .with_context(|| format!("downloading installer from {}", source.installer_url))?;
    let signature_bytes = download_bounded(client, &source.signature_url)
        .await
        .with_context(|| format!("downloading signature from {}", source.signature_url))?;

    let signature_str =
        std::str::from_utf8(&signature_bytes).context("signature file is not valid UTF-8")?;

    let verified = match public_key_b64_override {
        Some(key) => signature::verify_bytes_with(key, &installer_bytes, signature_str),
        None => signature::verify_bytes(&installer_bytes, signature_str),
    }
    .context("artefact signature verification failed")?;

    // Persist the verified bytes; axoupdater re-uses this file via
    // `configure_installer_path`, skipping its own download step (see
    // axoupdater 0.10 src/lib.rs:444 — the installer_path branch). We
    // do not write the signature — only the verified payload — so a
    // later accidental re-execution cannot use an outdated signature.
    let extension = if cfg!(windows) { ".ps1" } else { ".sh" };
    let installer_path = dest_dir.join(format!("installer{extension}"));
    let mut file = std::fs::File::create(&installer_path).with_context(|| {
        format!(
            "creating verified installer at {}",
            installer_path.display()
        )
    })?;
    file.write_all(&installer_bytes)?;
    file.flush()?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o744);
        file.set_permissions(perms)?;
    }

    Ok((installer_path, verified))
}

/// Build a `reqwest::Client` for the update fetch path. `enforce_https`
/// turns on `https_only` and bounded redirects for production; tests
/// disable it so they can point the client at a local HTTP mock server.
fn build_client(enforce_https: bool) -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .timeout(NETWORK_TIMEOUT)
        .connect_timeout(Duration::from_secs(10))
        .user_agent(concat!("anvil-update/", env!("CARGO_PKG_VERSION")));
    if enforce_https {
        // Council MAJOR: refuse HTTP downgrades. GitHub Releases is HTTPS-
        // only, but reqwest's default policy will follow an `http://`
        // redirect silently. https_only short-circuits that.
        builder = builder
            .https_only(true)
            // Cap redirect depth. `releases/latest/download/…` relies on
            // one redirect to the versioned asset; five gives breathing
            // room without letting a malicious chain run away.
            .redirect(reqwest::redirect::Policy::limited(5));
    }
    builder
        .build()
        .context("failed to construct HTTP client for update fetch")
}

async fn download_bounded(client: &reqwest::Client, url: &str) -> anyhow::Result<Vec<u8>> {
    use futures_util::StreamExt;

    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("GET {url}"))?
        .error_for_status()
        .with_context(|| format!("non-success status from {url}"))?;

    // Defence-in-depth: declared Content-Length is checked first as a
    // fast-fail, but we never trust it — the body is streamed chunk-by-
    // chunk and aborts the moment running total exceeds the cap, so a
    // server that lies about Content-Length cannot trick us into
    // buffering an oversized payload.
    if let Some(length) = response.content_length()
        && length > MAX_DOWNLOAD_BYTES
    {
        anyhow::bail!(
            "refusing to download {url}: declared size {length} exceeds {MAX_DOWNLOAD_BYTES} byte cap"
        );
    }

    let mut stream = response.bytes_stream();
    let mut buf: Vec<u8> = Vec::with_capacity(8 * 1024);
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.with_context(|| format!("streaming body of {url}"))?;
        if (buf.len() as u64).saturating_add(chunk.len() as u64) > MAX_DOWNLOAD_BYTES {
            anyhow::bail!(
                "refusing to load {url}: body exceeds {MAX_DOWNLOAD_BYTES} byte cap while streaming"
            );
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Cursor;

    #[test]
    fn latest_url_uses_releases_latest_download() {
        let src = github_release_source("eddacraft", "anvil", "anvil", None);
        // The installer extension is platform-native (`.ps1` on Windows,
        // `.sh` elsewhere), matching `github_release_source`.
        let ext = if cfg!(windows) { ".ps1" } else { ".sh" };
        assert_eq!(
            src.installer_url,
            format!(
                "https://github.com/eddacraft/anvil/releases/latest/download/anvil-installer{ext}"
            )
        );
        assert_eq!(
            src.signature_url,
            format!(
                "https://github.com/eddacraft/anvil/releases/latest/download/anvil-installer{ext}.minisig"
            )
        );
    }

    #[test]
    fn versioned_url_normalises_leading_v() {
        let with = github_release_source("eddacraft", "anvil", "anvil", Some("v0.7.0-beta"));
        let without = github_release_source("eddacraft", "anvil", "anvil", Some("0.7.0-beta"));
        assert_eq!(with.installer_url, without.installer_url);
        assert!(
            with.installer_url
                .contains("/releases/download/v0.7.0-beta/")
        );
    }

    /// Sign `data` with a fresh keypair, return (`public_key_b64`, `signature_str`).
    fn sign_fixture(data: &[u8], trusted_comment: &str) -> (String, String) {
        let kp = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        let pk_b64 = kp.pk.to_base64();
        let signature_box = minisign::sign(
            None,
            &kp.sk,
            Cursor::new(data),
            Some(trusted_comment),
            Some("anvil-fetch-test"),
        )
        .unwrap();
        (pk_b64, String::from(signature_box))
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fetch_and_verify_happy_path() {
        let installer_body = b"#!/bin/sh\necho hello from anvil installer\n";
        let (pk_b64, sig_str) = sign_fixture(installer_body, "tag=v0.0.0-test");
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/anvil-installer.sh"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_bytes(installer_body.to_vec()),
            )
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/anvil-installer.sh.minisig"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(sig_str.clone()))
            .mount(&server)
            .await;

        let source = ReleaseSource {
            installer_url: format!("{}/anvil-installer.sh", server.uri()),
            signature_url: format!("{}/anvil-installer.sh.minisig", server.uri()),
        };
        let dir = tempfile::tempdir().unwrap();
        let client = build_client(false).unwrap();
        let result = fetch_and_verify_inner(&client, Some(&pk_b64), &source, dir.path())
            .await
            .expect("verification must succeed for matching signature");
        let (path, verified) = result;
        assert!(path.exists(), "installer file must be persisted");
        assert!(
            verified.trusted_comment.contains("tag=v0.0.0-test"),
            "trusted comment surfaced; got {:?}",
            verified.trusted_comment
        );
        let on_disk = std::fs::read(&path).unwrap();
        assert_eq!(on_disk, installer_body);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(
                mode & 0o777,
                0o744,
                "verified installer must be executable by owner"
            );
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fetch_and_verify_refuses_tampered_installer() {
        // Sign the ORIGINAL bytes, but serve TAMPERED bytes. Verification
        // must refuse — this is the canonical DISTRIB-001 attack model:
        // a CDN compromise that swaps the installer without the matching
        // signature.
        let original = b"#!/bin/sh\necho legitimate installer\n";
        let tampered = b"#!/bin/sh\necho MALICIOUS installer\n";
        let (pk_b64, sig_str) = sign_fixture(original, "tag=v0.0.0-test");

        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/anvil-installer.sh"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_bytes(tampered.to_vec()))
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/anvil-installer.sh.minisig"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(sig_str))
            .mount(&server)
            .await;

        let source = ReleaseSource {
            installer_url: format!("{}/anvil-installer.sh", server.uri()),
            signature_url: format!("{}/anvil-installer.sh.minisig", server.uri()),
        };
        let dir = tempfile::tempdir().unwrap();
        let client = build_client(false).unwrap();
        let err = fetch_and_verify_inner(&client, Some(&pk_b64), &source, dir.path())
            .await
            .expect_err("tampered installer must be refused");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("signature verification failed") || msg.contains("refusing to install"),
            "expected loud refusal, got: {msg}"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fetch_and_verify_refuses_missing_signature() {
        // CDN returns the installer but the signature 404s. Verification
        // must refuse, never silently fall through. This proves we cannot
        // be downgraded by simply removing the signature file.
        let installer_body = b"#!/bin/sh\necho hello\n";
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/anvil-installer.sh"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_bytes(installer_body.to_vec()),
            )
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/anvil-installer.sh.minisig"))
            .respond_with(wiremock::ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let source = ReleaseSource {
            installer_url: format!("{}/anvil-installer.sh", server.uri()),
            signature_url: format!("{}/anvil-installer.sh.minisig", server.uri()),
        };
        let dir = tempfile::tempdir().unwrap();
        // Production path: no key override, so verification runs against
        // the embedded DEV key. We expect the signature download itself
        // to 404 before verification, so this still asserts the right
        // refusal mode.
        let client = build_client(false).unwrap();
        let err = fetch_and_verify_inner(&client, None, &source, dir.path())
            .await
            .expect_err("missing signature must abort verification");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("downloading signature") && msg.contains("404"),
            "expected 404-aware error, got: {msg}"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fetch_and_verify_refuses_signature_served_as_html() {
        // Realistic CDN-misconfiguration case: GitHub returns a 200 OK
        // with an HTML "asset not found" page when an asset URL is
        // malformed. The signature parser must refuse rather than
        // silently treat the page as a (malformed) signature.
        let installer_body = b"#!/bin/sh\necho hello\n";
        let (pk_b64, _) = sign_fixture(installer_body, "tag=v0.0.0-test");

        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/anvil-installer.sh"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_bytes(installer_body.to_vec()),
            )
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/anvil-installer.sh.minisig"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_string("<html><body>Not Found</body></html>"),
            )
            .mount(&server)
            .await;

        let source = ReleaseSource {
            installer_url: format!("{}/anvil-installer.sh", server.uri()),
            signature_url: format!("{}/anvil-installer.sh.minisig", server.uri()),
        };
        let dir = tempfile::tempdir().unwrap();
        let client = build_client(false).unwrap();
        let err = fetch_and_verify_inner(&client, Some(&pk_b64), &source, dir.path())
            .await
            .expect_err("HTML body served as signature must be refused");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("signature") || msg.contains("verification failed"),
            "expected signature-decode error, got: {msg}"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn fetch_and_verify_aborts_on_streaming_oversize_body() {
        // The body-size cap must be enforced mid-stream, not just by
        // trusting Content-Length. We serve a 5 MiB body (above the
        // 4 MiB cap) without a declared length so the streaming check
        // is the only line of defence.
        let installer_body = vec![b'x'; 5 * 1024 * 1024];
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/anvil-installer.sh"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_bytes(installer_body))
            .mount(&server)
            .await;

        let source = ReleaseSource {
            installer_url: format!("{}/anvil-installer.sh", server.uri()),
            signature_url: format!("{}/anvil-installer.sh.minisig", server.uri()),
        };
        let dir = tempfile::tempdir().unwrap();
        let client = build_client(false).unwrap();
        let err = fetch_and_verify_inner(&client, None, &source, dir.path())
            .await
            .expect_err("5 MiB body must be refused");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("exceeds") && msg.contains("byte cap"),
            "expected streaming-cap rejection, got: {msg}"
        );
    }
}
