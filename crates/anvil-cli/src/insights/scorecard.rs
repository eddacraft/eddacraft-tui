//! Rendering for the cumulative value scoreboard (CIB-073): the plain
//! terminal section and the shareable single-file HTML scorecard.
//!
//! ## Determinism
//!
//! Both renders are pure functions of [`CumulativeValue`], which itself
//! carries no generation timestamp — every date shown is one of the
//! evidence window's own bounds. Same inputs, byte-identical output.
//!
//! ## Redaction contract (the shareable artifact)
//!
//! The scorecard embeds **only** counts, evidence-window dates, and
//! static labels. It must never contain repository paths, repo names,
//! file names, branch names, secret values, hostnames, usernames, or
//! emails. That holds structurally — [`CumulativeValue`] cannot carry
//! them — and is proven by the redaction fixtures in
//! `commands::insights::tests`, which seed marker strings through every
//! source field and assert none appear here.
//!
//! The HTML card is fully self-contained: embedded styling, no scripts,
//! no external assets, no network references of any kind.

use std::fmt::Write as _;

use super::cumulative::CumulativeValue;

/// Honest empty-state line shared by the plain render and the `--share`
/// refusal path: an absent aggregate is stated, never zero-filled.
pub const NO_EVENTS_LINE: &str =
    "No recorded events yet — Anvil has not observed any activity to report.";

/// Render the scoreboard as plain terminal text.
#[must_use]
pub fn render_plain(value: &CumulativeValue) -> String {
    let mut out = String::new();
    out.push_str("anvil value scoreboard\n");
    let (Some(since), Some(as_of)) = (&value.since, &value.as_of) else {
        let _ = writeln!(out, "{NO_EVENTS_LINE}");
        return out;
    };
    let _ = writeln!(
        out,
        "Evidence window: {} to {} (since first recorded event)",
        date_part(since),
        date_part(as_of)
    );
    let _ = writeln!(
        out,
        "Witness events recorded: {} since first run · {} last 30 days · {} last 90 days",
        value.witness_events_total,
        value.witness_events_last_30_days,
        value.witness_events_last_90_days
    );

    let save = &value.save_time;
    if save.has_evidence() {
        let (Some(start), Some(end)) = (&save.window_start, &save.window_end) else {
            unreachable!("has_evidence guarantees both bounds");
        };
        let _ = writeln!(
            out,
            "Save-time protection (retained window {} to {}):",
            date_part(start),
            date_part(end)
        );
        let _ = writeln!(
            out,
            "  Save-time checks observed: {}",
            save.evaluations_observed
        );
        let _ = writeln!(out, "  Risky writes flagged: {}", save.risky_writes_flagged);
        let _ = writeln!(out, "  Writes blocked: {}", save.writes_blocked);
        let _ = writeln!(
            out,
            "  Secret findings caught: {}",
            save.secret_findings_caught
        );
        let _ = writeln!(out, "  Protective fences engaged: {}", save.fences_engaged);
    } else {
        let _ = writeln!(
            out,
            "Save-time protection: no recorded events in the retained window yet."
        );
    }
    let _ = writeln!(
        out,
        "Witness events cover this repository; save-time figures cover this machine."
    );
    out
}

/// Render the shareable single-file HTML scorecard.
///
/// Returns `None` when there is no evidence to share — an all-zero card
/// would read as a measured claim, so the caller renders
/// [`NO_EVENTS_LINE`] instead and writes nothing.
#[must_use]
pub fn render_html_card(value: &CumulativeValue) -> Option<String> {
    let (since, as_of) = (value.since.as_deref()?, value.as_of.as_deref()?);
    let save = &value.save_time;

    let mut stats = String::new();
    push_stat(
        &mut stats,
        value.witness_events_total,
        "witness events since first run",
    );
    push_stat(
        &mut stats,
        value.witness_events_last_30_days,
        "witness events, last 30 days",
    );
    push_stat(
        &mut stats,
        value.witness_events_last_90_days,
        "witness events, last 90 days",
    );

    let save_section = if save.has_evidence() {
        let start = save.window_start.as_deref().unwrap_or_default();
        let end = save.window_end.as_deref().unwrap_or_default();
        let mut section = format!(
            "    <h2>Save-time protection <span class=\"window\">retained window {} to {}</span></h2>\n    <div class=\"stats\">\n",
            date_part(start),
            date_part(end)
        );
        push_stat(&mut section, save.evaluations_observed, "save-time checks");
        push_stat(
            &mut section,
            save.risky_writes_flagged,
            "risky writes flagged",
        );
        push_stat(&mut section, save.writes_blocked, "writes blocked");
        push_stat(
            &mut section,
            save.secret_findings_caught,
            "secret findings caught",
        );
        push_stat(
            &mut section,
            save.fences_engaged,
            "protective fences engaged",
        );
        section.push_str("    </div>\n");
        section
    } else {
        "    <h2>Save-time protection</h2>\n    <p class=\"empty\">No recorded events in the retained window yet.</p>\n".to_string()
    };

    Some(format!(
        r#"<!doctype html>
<html lang="en-GB">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Anvil value scorecard</title>
<style>
  :root {{ color-scheme: light dark; }}
  body {{
    margin: 0; padding: 2rem 1rem;
    font-family: ui-sans-serif, system-ui, sans-serif;
    background: #101418; color: #e6e9ec;
    display: flex; justify-content: center;
  }}
  .card {{
    max-width: 34rem; width: 100%;
    background: #171d24; border: 1px solid #2a323c; border-radius: 12px;
    padding: 1.5rem 1.75rem;
  }}
  h1 {{ margin: 0; font-size: 1.35rem; letter-spacing: 0.02em; }}
  h2 {{ margin: 1.4rem 0 0.6rem; font-size: 0.95rem; color: #aeb6bf; }}
  .tagline {{ margin: 0.2rem 0 0.8rem; color: #aeb6bf; font-size: 0.9rem; }}
  .window {{ color: #7f8893; font-weight: normal; font-size: 0.8rem; }}
  .stats {{ display: flex; flex-wrap: wrap; gap: 0.75rem; }}
  .stat {{
    flex: 1 1 8.5rem; background: #10151b; border: 1px solid #232b34;
    border-radius: 8px; padding: 0.6rem 0.75rem;
  }}
  .stat .n {{ display: block; font-size: 1.5rem; font-weight: 700; }}
  .stat .l {{ display: block; margin-top: 0.15rem; color: #aeb6bf; font-size: 0.78rem; }}
  .empty {{ color: #7f8893; font-size: 0.85rem; }}
  footer {{ margin-top: 1.4rem; color: #7f8893; font-size: 0.75rem; }}
</style>
</head>
<body>
  <main class="card">
    <h1>Anvil</h1>
    <p class="tagline">Value caught, recorded locally</p>
    <p class="window">Evidence window: {since} to {as_of}</p>
    <div class="stats">
{stats}    </div>
{save_section}    <footer>Counts only — no repository details. Recorded and rendered locally by Anvil.</footer>
  </main>
</body>
</html>
"#,
        since = date_part(since),
        as_of = date_part(as_of),
    ))
}

fn push_stat(out: &mut String, n: u64, label: &str) {
    let _ = writeln!(
        out,
        "      <div class=\"stat\"><span class=\"n\">{n}</span><span class=\"l\">{label}</span></div>"
    );
}

/// The `YYYY-MM-DD` prefix of an RFC 3339 timestamp — the human-facing
/// date form used on both renders.
fn date_part(ts: &str) -> &str {
    ts.get(..10).unwrap_or(ts)
}
