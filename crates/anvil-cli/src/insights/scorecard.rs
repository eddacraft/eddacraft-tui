//! Rendering for the cumulative value scoreboard (CIB-073): the plain
//! terminal section and the shareable single-file HTML scorecard.
//!
//! ## Determinism
//!
//! Both renders are pure functions of [`CumulativeValue`], which itself
//! carries no generation timestamp — every date shown is one of the
//! evidence streams' own bounds. Same inputs, byte-identical output.
//! This claim covers the scoreboard section and the share card only;
//! the v1 rolling-window fields of the `--cumulative --json` document
//! stay wall-clock-anchored (see `insights::cumulative` module docs).
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
    if value.witness_has_evidence() {
        let (Some(first), Some(last)) = (&value.witness_first_event, &value.witness_last_event)
        else {
            unreachable!("witness_has_evidence guarantees both bounds");
        };
        let _ = writeln!(
            out,
            "Witness events ({} to {}): {} since first run · {} last 30 days · {} last 90 days",
            date_part(first),
            date_part(last),
            value.witness_events_total,
            value.witness_events_last_30_days,
            value.witness_events_last_90_days
        );
    } else {
        let _ = writeln!(
            out,
            "Witness events: none recorded for this repository yet."
        );
    }

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
    let witness_section = witness_card_section(value);
    let save_section = save_time_card_section(&value.save_time);

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
{witness_section}{save_section}    <footer>Counts only — no repository details. Recorded and rendered locally by Anvil.</footer>
  </main>
</body>
</html>
"#,
        since = date_part(since),
        as_of = date_part(as_of),
    ))
}

/// The witness-events card section: stat tiles under the stream's own
/// window, or the honest empty line when the chain holds no events.
fn witness_card_section(value: &CumulativeValue) -> String {
    if !value.witness_has_evidence() {
        return "    <h2>Witness events</h2>\n    <p class=\"empty\">None recorded for this repository yet.</p>\n".to_string();
    }
    let first = value.witness_first_event.as_deref().unwrap_or_default();
    let last = value.witness_last_event.as_deref().unwrap_or_default();
    let mut section = format!(
        "    <h2>Witness events <span class=\"window\">{} to {}</span></h2>\n    <div class=\"stats\">\n",
        date_part(first),
        date_part(last)
    );
    push_stat(
        &mut section,
        value.witness_events_total,
        "witness events since first run",
    );
    push_stat(
        &mut section,
        value.witness_events_last_30_days,
        "witness events, last 30 days",
    );
    push_stat(
        &mut section,
        value.witness_events_last_90_days,
        "witness events, last 90 days",
    );
    section.push_str("    </div>\n");
    section
}

/// The save-time card section: stat tiles under the retained window,
/// or the honest empty line when the sidecar holds no rows.
fn save_time_card_section(save: &super::cumulative::SaveTimeCounts) -> String {
    if !save.has_evidence() {
        return "    <h2>Save-time protection</h2>\n    <p class=\"empty\">No recorded events in the retained window yet.</p>\n".to_string();
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::insights::cumulative::SaveTimeCounts;

    /// Fixed aggregate backing the golden card. Any change to the
    /// card's markup or content must be reviewed against the pinned
    /// fixture (see [`card_matches_golden_fixture`]).
    fn golden_value() -> CumulativeValue {
        CumulativeValue {
            since: Some("2026-01-05T08:00:00Z".to_string()),
            as_of: Some("2026-07-08T12:00:00Z".to_string()),
            witness_first_event: Some("2026-01-05T08:00:00Z".to_string()),
            witness_last_event: Some("2026-07-01T10:00:00Z".to_string()),
            witness_events_total: 412,
            witness_events_last_30_days: 58,
            witness_events_last_90_days: 190,
            save_time: SaveTimeCounts {
                window_start: Some("2026-07-05T10:00:00Z".to_string()),
                window_end: Some("2026-07-08T12:00:00Z".to_string()),
                evaluations_observed: 120,
                risky_writes_flagged: 14,
                writes_blocked: 3,
                secret_findings_caught: 2,
                fences_engaged: 1,
            },
        }
    }

    /// Golden pin of the entire share card. Byte-stability is proven
    /// by the determinism test in `commands::insights`; this compare
    /// closes the self-containment guard — any new tag, attribute, or
    /// asset reference shows up as a reviewed fixture diff.
    ///
    /// To update after an intentional render change:
    /// `UPDATE_SCORECARD_GOLDEN=1 cargo test -p eddacraft-anvil \
    ///  insights::scorecard` then re-run without the env var.
    #[test]
    fn card_matches_golden_fixture() {
        let card = render_html_card(&golden_value()).expect("golden value has evidence");
        if std::env::var_os("UPDATE_SCORECARD_GOLDEN").is_some() {
            std::fs::write(
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/insights/testdata/scorecard-golden.html"
                ),
                &card,
            )
            .expect("write golden fixture");
            // The compiled-in fixture is stale on the updating run;
            // the next (normal) run performs the real compare.
            return;
        }
        let golden = include_str!("testdata/scorecard-golden.html");
        assert_eq!(
            card, golden,
            "share card diverged from the pinned golden fixture; if the \
             change is intentional, regenerate with UPDATE_SCORECARD_GOLDEN=1"
        );
    }
}
