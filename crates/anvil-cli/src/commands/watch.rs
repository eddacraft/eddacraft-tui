#![allow(dead_code)]
use std::io::IsTerminal;
use std::sync::mpsc;

use anyhow::{Context, Result};
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct WatchArgs {
    /// Specific file to watch
    file: Option<String>,

    /// Action to run on change: validate, gate, check
    #[arg(long, short)]
    action: Option<String>,

    /// Watch planning documents
    #[arg(long)]
    plans: bool,

    /// Watch source files
    #[arg(long)]
    source: bool,

    /// Watch everything
    #[arg(long)]
    all: bool,

    /// Glob patterns to watch (comma-separated)
    #[arg(long)]
    patterns: Option<String>,

    /// Patterns to exclude (comma-separated)
    #[arg(long)]
    exclude: Option<String>,

    /// Debounce interval in milliseconds
    #[arg(long)]
    debounce: Option<u64>,
}

const DEFAULT_WATCH_PATTERNS: &[&str] = &[
    "**/*.md",
    "**/*.aps.md",
    "**/prd.*",
    "**/plan.*",
    "**/spec.*",
];

const SOURCE_PATTERNS: &[&str] = &[
    "src/**/*.ts",
    "src/**/*.tsx",
    "lib/**/*.ts",
    "crates/**/*.rs",
];

const DEFAULT_EXCLUDE: &[&str] = &[
    "node_modules/**",
    "dist/**",
    "build/**",
    ".git/**",
    "target/**",
    "coverage/**",
];

#[derive(Debug, Serialize)]
struct WatchEvent {
    timestamp: String,
    event_type: String,
    detail: String,
}

pub fn run(args: &WatchArgs, global: &GlobalArgs) -> Result<()> {
    let workspace_root = std::env::current_dir().context("getting current directory")?;

    let _patterns: Vec<String> = if let Some(ref p) = args.patterns {
        p.split(',').map(|s| s.trim().to_string()).collect()
    } else if args.all || (args.source && args.plans) {
        DEFAULT_WATCH_PATTERNS
            .iter()
            .chain(SOURCE_PATTERNS.iter())
            .map(ToString::to_string)
            .collect()
    } else if args.source {
        SOURCE_PATTERNS.iter().map(ToString::to_string).collect()
    } else {
        DEFAULT_WATCH_PATTERNS
            .iter()
            .map(ToString::to_string)
            .collect()
    };

    let _exclude: Vec<String> = args.exclude.as_ref().map_or_else(
        || DEFAULT_EXCLUDE.iter().map(ToString::to_string).collect(),
        |s| s.split(',').map(|s| s.trim().to_string()).collect(),
    );

    let arch_config_path = workspace_root.join(".anvil").join("architecture.yaml");
    let arch_config = if arch_config_path.exists() {
        Some(arch_config_path)
    } else {
        None
    };

    let watcher_config = anvil_kernel::watcher::WatcherConfig {
        root: workspace_root.clone(),
        debounce_window: std::time::Duration::from_millis(args.debounce.unwrap_or(300)),
        ..Default::default()
    };

    let watch_config = anvil_kernel::watch::WatchConfig {
        root: workspace_root.clone(),
        architecture_config: arch_config.clone(),
        watcher: watcher_config,
    };

    let (event_tx, event_rx) = mpsc::channel();

    let handle = anvil_kernel::watch::run_watch(&watch_config, event_tx)
        .context("starting kernel watcher")?;

    if global.json || !std::io::stdout().is_terminal() || global.no_tui {
        for event in &event_rx {
            if global.json {
                let watch_event = WatchEvent {
                    timestamp: event.timestamp.clone(),
                    event_type: format!("{:?}", event.event_type),
                    detail: format!("{:?}", event.payload),
                };
                println!("{}", serde_json::to_string(&watch_event)?);
            } else {
                print_event_plain(&event);
            }
        }
    } else {
        let state =
            anvil_tui::surfaces::watch::WatchState::new(anvil_tui::surfaces::watch::WatchData {
                status: anvil_tui::surfaces::watch::WatchStatus::Idle,
                queue: Vec::new(),
                history: Vec::new(),
                stats: anvil_tui::surfaces::watch::WatchStats {
                    total_runs: 0,
                    pass_rate: 0.0,
                    avg_duration_ms: 0,
                    files_watched: 0,
                },
            });
        crate::tui::run_watch(state, &event_rx)?;
    }

    handle.stop().context("stopping watcher")?;
    Ok(())
}

fn print_event_plain(event: &anvil_kernel_types::EngineEvent) {
    use anvil_kernel_types::{EventPayload, EventType};

    let prefix = match event.event_type {
        EventType::Progress => "\u{25b6}",
        EventType::Snapshot => "\u{1f4f8}",
        EventType::Violation => "\u{26a0}",
        EventType::Error => "\u{2717}",
    };

    match &event.payload {
        EventPayload::Progress {
            phase,
            current,
            total,
        } => {
            eprintln!("{prefix} {phase}: {current}/{total}");
        }
        EventPayload::Snapshot {
            node_count,
            edge_count,
            files_watched,
        } => {
            eprintln!(
                "{prefix} Snapshot: {node_count} nodes, {edge_count} edges, {files_watched} files"
            );
        }
        EventPayload::Violation {
            policy_id,
            file,
            message,
            ..
        } => {
            eprintln!("{prefix} [{policy_id}] {file}: {message}");
        }
        EventPayload::Error(err) => {
            eprintln!("{prefix} Error: {}", err.message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Wrapper {
        #[command(flatten)]
        inner: WatchArgs,
    }

    #[test]
    fn args_parses_empty() {
        let w = Wrapper::try_parse_from(["test"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_source() {
        let w = Wrapper::try_parse_from(["test", "--source"]).unwrap();
        assert!(w.inner.source);
    }

    #[test]
    fn args_parses_all() {
        let w = Wrapper::try_parse_from(["test", "--all"]).unwrap();
        assert!(w.inner.all);
    }

    #[test]
    fn args_parses_patterns() {
        let w = Wrapper::try_parse_from(["test", "--patterns", "**/*.ts,**/*.tsx"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }
}
