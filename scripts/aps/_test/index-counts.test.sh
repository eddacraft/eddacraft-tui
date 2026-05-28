#!/usr/bin/env bash
# Fixture tests for scripts/aps/index-counts.mjs (CIB-022).
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
GEN=(node "$ROOT/scripts/aps/index-counts.mjs")

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/plans/modules" "$tmp/plans/archive/modules"

fail() {
  printf 'index-counts.test.sh: FAIL: %s\n' "$1" >&2
  exit 1
}

# ── A headered module: 2 of 3 items done. ────────────────────────────────
cat >"$tmp/plans/modules/widget.aps.md" <<'EOF'
# Widget

| ID  | Owner | Status      | Progress |
| --- | ----- | ----------- | -------- |
| WID | —     | In Progress | 1/3      |

## Tasks

### WID-001: first

- **Status:** Merged 2026-01-01 via PR #1

### WID-002: second

- **Status:** Done

### WID-003: third

- **Status:** In Progress
EOF

# A headerless module: index count is a curated planned total, NOT item-derived.
cat >"$tmp/plans/modules/planning.aps.md" <<'EOF'
# Planning

| ID  | Owner | Status   |
| --- | ----- | -------- |
| PLAN | —    | Proposed |

### PLAN-001: only one filed so far

- **Status:** Proposed
EOF

# An archived module row must never be touched.
cat >"$tmp/plans/archive/modules/old.aps.md" <<'EOF'
# Old

| ID  | Owner | Status   | Progress |
| --- | ----- | -------- | -------- |
| OLD | —     | Complete | 5/5      |

### OLD-001: x

- **Status:** Complete
EOF

# zeta: both items done (2/2), but its index row is stale (1/2). A text-name
# "N-row" above it links to zeta in its PROSE — the generator must update
# zeta's OWN row, not the N-row (the live-index N1→multilayer-protection-v2
# trap).
cat >"$tmp/plans/modules/zeta.aps.md" <<'EOF'
# Zeta

| ID   | Owner | Status   | Progress |
| ---- | ----- | -------- | -------- |
| ZETA | —     | Complete | 1/2      |

### ZETA-001: a

- **Status:** Merged

### ZETA-002: b

- **Status:** Done
EOF

# gamma: 1 of 2 done. Its index row keeps the count INSIDE prose with no
# dedicated count cell — the generator must update gamma's header but leave the
# prose token alone (the live-index EATEST "(6/38 complete)" shape).
cat >"$tmp/plans/modules/gamma.aps.md" <<'EOF'
# Gamma

| ID    | Owner | Status      | Progress |
| ----- | ----- | ----------- | -------- |
| GAMMA | —     | In Progress | 0/2      |

### GAMMA-001: a

- **Status:** Merged

### GAMMA-002: b

- **Status:** In Progress
EOF

# phased: items live under a `### Phase` group as `#### PHX-NNN` (the MLP2
# shape). The count must derive from the #### items (1 Done of 2), not the
# stale 0/2 header — the regression guard for the #### count-gate fix. The
# `### Phase A` group heading carries no work-item ID, so it must not be
# counted; only the two `#### PHX-NNN` items are.
cat >"$tmp/plans/modules/phased.aps.md" <<'EOF'
# Phased

| ID  | Owner | Status      | Progress |
| --- | ----- | ----------- | -------- |
| PHX | —     | In Progress | 0/2      |

## Tasks

### Phase A — groundwork

#### PHX-001: first

- **Status:** Done

#### PHX-002: second

- **Status:** In Progress
EOF

# Index: WID header is stale (1/3); a DIFFERENT row's prose links to widget
# (the TUIR-style hijack trap); the N9 text-name row links to zeta in prose
# (the N1 trap); zeta's own row is stale (1/2); gamma carries its count in
# prose only; PLAN is a curated headerless total; OLD is archived.
cat >"$tmp/plans/index.aps.md" <<'EOF'
# Index

| Module | Scope | Status | Progress |
| --- | --- | --- | --- |
| [decoy](./modules/decoy.aps.md) | DEC | In Progress | 0/9 — see [widget](./modules/widget.aps.md) |
| [widget](./modules/widget.aps.md) | WID | In Progress | 1/3 (WID-001 PR #1) |
| N9 — legacy zeta | Complete | 9/9 | folded into [zeta](./modules/zeta.aps.md) |
| [zeta](./modules/zeta.aps.md) | ZETA | Complete | 1/2 |
| [gamma](./modules/gamma.aps.md) | GAMMA | scaffolding (0/2 wired) | In Progress |
| [phased](./modules/phased.aps.md) | PHX | In Progress | 0/2 |
| [planning](./modules/planning.aps.md) | PLAN | Proposed | 1/8 (8 planned) |
| [old](./archive/modules/old.aps.md) | OLD | Complete | 5/5 |
EOF

# ── 1. --check detects the stale WID count (2/3 expected, 1/3 on disk). ───
if "${GEN[@]}" --root "$tmp" --check >/tmp/ic-check.out 2>&1; then
  fail "--check should exit non-zero on stale WID count"
fi
grep -q 'WID' /tmp/ic-check.out || fail "--check output should name WID"

# ── 2. write mode fixes header + index, leaves prose/planning/archived alone. ─
"${GEN[@]}" --root "$tmp" >/dev/null 2>&1 || fail "write mode should succeed"

grep -qE '^\| WID +\| +— +\| In Progress +\| 2/3 ' "$tmp/plans/modules/widget.aps.md" \
  || fail "module header not updated to 2/3"

grep -q '| WID | In Progress | 2/3 (WID-001 PR #1) |' "$tmp/plans/index.aps.md" \
  || fail "index WID count not updated to 2/3, or prose not preserved"

# Prose link hijack trap: the decoy row must keep its own 0/9.
grep -q '| DEC | In Progress | 0/9 — see' "$tmp/plans/index.aps.md" \
  || fail "decoy row (prose links to widget) was wrongly rewritten"

# N-row trap: zeta's OWN row must update to 2/2; the N9 row whose prose links to
# zeta must keep its own 9/9.
grep -qF '| [zeta](./modules/zeta.aps.md) | ZETA | Complete | 2/2 |' "$tmp/plans/index.aps.md" \
  || fail "zeta's own index row not updated to 2/2"
grep -q '| N9 — legacy zeta | Complete | 9/9 | folded into' "$tmp/plans/index.aps.md" \
  || fail "N9 text-name row (prose links to zeta) was wrongly rewritten"

# Prose-embedded count: gamma's header updates to 1/2, but its index prose
# token (no dedicated count cell) is left alone.
grep -qE '^\| GAMMA +\| +— +\| In Progress +\| 1/2 ' "$tmp/plans/modules/gamma.aps.md" \
  || fail "gamma header not updated to 1/2"
grep -q '| GAMMA | scaffolding (0/2 wired) | In Progress |' "$tmp/plans/index.aps.md" \
  || fail "gamma prose-embedded count was wrongly rewritten"

# Fourth-level items: `#### PHX-NNN` under a `### Phase` group must be counted
# (1 Done of 2), proving the count gate no longer skips #### task headings.
grep -qE '^\| PHX +\| +— +\| In Progress +\| 1/2 ' "$tmp/plans/modules/phased.aps.md" \
  || fail "phased module #### items not counted (header not updated to 1/2)"
grep -qF '| [phased](./modules/phased.aps.md) | PHX | In Progress | 1/2 |' "$tmp/plans/index.aps.md" \
  || fail "phased index row not updated to 1/2 from #### items"

# Headerless module: curated 1/8 must be untouched (item count would be 1/1).
grep -q '| PLAN | Proposed | 1/8 (8 planned) |' "$tmp/plans/index.aps.md" \
  || fail "headerless planning row was wrongly rewritten"

# Archived row untouched.
grep -q '| OLD | Complete | 5/5 |' "$tmp/plans/index.aps.md" \
  || fail "archived OLD row was touched"

# ── 3. --check is now clean (idempotent). ────────────────────────────────
"${GEN[@]}" --root "$tmp" --check >/dev/null 2>&1 || fail "--check should be clean after --write"

# ── 4. write mode is idempotent (second run changes nothing). ────────────
before="$(cat "$tmp/plans/index.aps.md" "$tmp/plans/modules/widget.aps.md")"
"${GEN[@]}" --root "$tmp" >/dev/null 2>&1 || fail "second write run should succeed"
after="$(cat "$tmp/plans/index.aps.md" "$tmp/plans/modules/widget.aps.md")"
[ "$before" = "$after" ] || fail "write mode is not idempotent"

printf 'index-counts.test.sh: ok\n'
