//! `anvil dashboard --web` — the local browser dashboard.
//!
//! Binds the read-only dashboard API to a loopback port and serves the
//! embedded UI from the same origin, so the whole surface is one process and
//! one URL. Nothing is exposed off-host: the listener refuses a non-loopback
//! address, and the server rejects requests whose `Host`/`Origin` are not the
//! loopback authority it bound.

use std::net::Ipv4Addr;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{GlobalArgs, util};

use super::DashboardArgs;

/// The machine-readable envelope, emitted once before the server takes over so
/// a caller driving this under `--json` can discover the URL without scraping.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WebDashboardStart<'a> {
    url: &'a str,
    workspace: &'a str,
    access: &'static str,
    ui_bundled: bool,
}

pub fn run(args: &DashboardArgs, global: &GlobalArgs) -> Result<()> {
    let root = util::workspace_root()
        .context("`anvil dashboard --web` needs a workspace; run it inside your project")?;

    // A current-thread runtime is enough: this command does nothing but serve.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("could not start the dashboard runtime")?;

    runtime.block_on(async move {
        // Port 0 asks the OS for a free port, so a second dashboard on the same
        // machine does not collide with the first.
        let port = args.port.unwrap_or(0);
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port))
            .await
            .with_context(|| match port {
                0 => "could not bind a loopback port for the dashboard".to_owned(),
                port => format!(
                    "could not bind 127.0.0.1:{port} — it may already be in use; \
                     omit --port to let the OS choose"
                ),
            })?;
        let address = listener
            .local_addr()
            .context("dashboard listener address")?;
        let url = format!("http://127.0.0.1:{}/", address.port());
        let bundled = anvil_dashboard_server::is_bundled();

        if global.json {
            let envelope = WebDashboardStart {
                url: &url,
                workspace: &root.display().to_string(),
                access: "read-only",
                ui_bundled: bundled,
            };
            println!("{}", serde_json::to_string_pretty(&envelope)?);
        } else {
            print_banner(&url, &root.display().to_string(), bundled, args.no_open);
        }

        if !global.json && !args.no_open {
            match util::open_in_browser(&url) {
                Ok(()) => println!("  Opened in your browser."),
                Err(reason) => {
                    println!("  Could not open a browser ({reason}) — open the URL above.");
                }
            }
        }
        if !global.json {
            println!("\nPress Ctrl-C to stop.");
        }

        anvil_dashboard_server::serve(listener, &root)
            .await
            .context("dashboard server stopped unexpectedly")
    })
}

fn print_banner(url: &str, workspace: &str, bundled: bool, no_open: bool) {
    println!("anvil dashboard\n");
    println!("  URL        {url}");
    println!("  Workspace  {workspace}");
    println!("  Access     read-only, this machine only");
    if !bundled {
        // The API is genuinely up; only the UI is missing. Say which, so the
        // reader does not chase a broken install.
        println!(
            "\n  Note: this build carries no dashboard UI assets, so the URL \
             serves the\n        read-only API only. See \
             docs/guides/local-dashboard.md."
        );
    }
    if no_open {
        println!();
    }
}
