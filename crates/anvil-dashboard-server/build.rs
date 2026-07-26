//! Build script for `eddacraft-anvil-dashboard-server`.
//!
//! Embeds the built dashboard SPA (`apps/dashboard/dist`) into the binary so a
//! released `anvil` serves the UI with no Node toolchain, no repository
//! checkout, and no second process.
//!
//! The bundle is **optional**. A plain `cargo build` with no prior
//! `pnpm --filter @eddacraft/anvil-dashboard build` produces a server that
//! reports honestly that its UI assets are absent, rather than failing the
//! build — Rust contributors must never need a Node toolchain. The release
//! pipeline builds the SPA before `dist`, so shipped binaries always carry it.
//!
//! `ANVIL_DASHBOARD_DIST` overrides the source directory (used by the release
//! pipeline when the build tree is not the repository layout).

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=ANVIL_DASHBOARD_DIST");
    println!("cargo:rerun-if-env-changed=ANVIL_DASHBOARD_REQUIRE_BUNDLE");

    let dist = dist_dir();
    let mut assets = Vec::new();

    // `index.html` is the bundle's keystone: without it there is no SPA shell
    // to fall back to, so a partial directory is treated as no bundle at all.
    if dist.join("index.html").is_file() {
        // Cargo re-scans a watched directory recursively, so one directive
        // covers every hashed asset under it. Emitted only when the directory
        // exists — a `rerun-if-changed` on a missing path makes Cargo treat the
        // unit as perpetually dirty and rebuild on every invocation.
        println!("cargo:rerun-if-changed={}", dist.display());
        collect(&dist, &dist, &mut assets);
        // Deterministic output: `read_dir` order varies by filesystem, and the
        // generated table must not.
        assets.sort();
    }

    // The release pipeline sets this. Without it a mistyped path or a dropped
    // artifact would produce a perfectly green build of a binary whose only
    // symptom is that the dashboard does not exist — exactly the gap this work
    // item closes, reintroduced silently.
    assert!(
        !(assets.is_empty()
            && env::var_os("ANVIL_DASHBOARD_REQUIRE_BUNDLE").is_some_and(|v| v == "1")),
        "ANVIL_DASHBOARD_REQUIRE_BUNDLE=1 but no dashboard bundle was found at {}.\n\
         Build the SPA first (`pnpm --filter @eddacraft/anvil-dashboard build`), or point \
         ANVIL_DASHBOARD_DIST at an existing `dist` directory.",
        dist.display()
    );

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    fs::write(out.join("dashboard_assets.rs"), render(&assets)).expect("write asset table");
}

/// The directory holding the built SPA, defaulting to the in-repository path.
fn dist_dir() -> PathBuf {
    if let Some(over) = env::var_os("ANVIL_DASHBOARD_DIST") {
        return PathBuf::from(over);
    }
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    manifest.join("../../apps/dashboard/dist")
}

/// Recursively gather `(request path, absolute source path)` pairs.
///
/// Request paths are slash-separated and relative to the bundle root, matching
/// the URL the browser asks for. Non-UTF-8 names are skipped: they cannot be
/// requested over HTTP by a path we could match, and silently skipping keeps
/// the build working on filesystems that permit them.
fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, out);
            continue;
        }
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let (Some(rel), Some(abs)) = (rel.to_str(), path.to_str()) else {
            continue;
        };
        // Source maps are devtools-only and dwarf the code they describe (the
        // app's map is several times the size of the whole rest of the
        // bundle). They stay out of the shipped binary; a browser that asks
        // for one gets an ordinary 404.
        if Path::new(rel)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("map"))
        {
            continue;
        }
        out.push((rel.replace('\\', "/"), abs.to_owned()));
    }
}

fn render(assets: &[(String, String)]) -> String {
    let mut src = String::from("pub(crate) static ASSETS: &[Asset] = &[\n");
    for (rel, abs) in assets {
        // `{:?}` on a `&str` emits a correctly escaped Rust string literal,
        // which matters for Windows paths and any name containing a quote.
        let _ = writeln!(
            src,
            "    Asset {{ path: {rel:?}, content_type: {ct:?}, bytes: include_bytes!({abs:?}) }},",
            ct = content_type(rel),
        );
    }
    src.push_str("];\n");
    src
}

/// Map an extension to a `Content-Type`.
///
/// Responses carry `X-Content-Type-Options: nosniff`, so an asset typed
/// `application/octet-stream` by mistake is refused by the browser rather than
/// guessed. Every type Vite emits for this app is listed explicitly.
fn content_type(path: &str) -> &'static str {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/vnd.microsoft.icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}
