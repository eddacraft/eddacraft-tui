#!/usr/bin/env bash
# gctx-goal-verify.sh — mechanical verification for GV2-032 + GCTX-021/022/023.
#
# Runs the goal/plan.md verification steps, tees full output to SCRATCH, and
# fails unless primary observables appear in the logs.
#
# Usage:
#   SCRATCH=/tmp/grok-goal-702d0af3ed1f/implementer ./scripts/gctx-goal-verify.sh
#
# Exit codes:
#   0 — all gates passed
#   1 — a test or observable grep failed

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

SCRATCH="${SCRATCH:-${TMPDIR:-/tmp}/gctx-goal-verify}"
mkdir -p "$SCRATCH"

log() {
  echo "gctx-goal-verify: $*" >&2
}

capture() {
  local out="$1"
  shift
  # Redirect to a file (not tee) — tee can SIGPIPE cargo when stdout has no reader.
  "$@" >"$out" 2>&1
  cat "$out"
}

run_twice() {
  local name="$1"
  shift
  local out1="$SCRATCH/${name}-run1.log"
  local out2="$SCRATCH/${name}-run2.log"
  log "running (1/2): $*"
  capture "$out1" "$@"
  log "running (2/2): $*"
  capture "$out2" "$@"
}

require_grep() {
  local file="$1"
  local pattern="$2"
  local label="$3"
  if ! grep -qE "$pattern" "$file"; then
    log "FAIL: $label not found in $file (pattern: $pattern)"
    exit 1
  fi
  log "ok: $label in $(basename "$file")"
}

log "scratch=$SCRATCH"

# Step 1 — GV2-032 span population + related graph-cache tests
run_twice gv2-032-tests \
  cargo test -p eddacraft-anvil-graph-cache -- span_population gv2_032 snapshot_no_leak

cat "$SCRATCH/gv2-032-tests-run1.log" "$SCRATCH/gv2-032-tests-run2.log" \
  >"$SCRATCH/gv2-032-tests.log"
require_grep "$SCRATCH/gv2-032-tests.log" 'span_population.*ok' 'span_population pass'
require_grep "$SCRATCH/gv2-032-tests.log" 'snapshot_no_leak.*ok' 'snapshot no-leak pass'
require_grep "$SCRATCH/gv2-032-tests.log" 'gv2_032.*ok' 'gv2_032 fixture pass'

# Step 2 — GCTX snippet egress paths (CE-1/2/3/7)
log "running: gctx snippet / egress unit tests"
capture "$SCRATCH/gctx-snippet-tests.log" \
  cargo test -p eddacraft-anvil-gctx-egress -- \
    project_snippet ce_1 ce1 ce-1 redact stale secret path
capture "$SCRATCH/gctx-snippet-types.log" \
  cargo test -p eddacraft-anvil-gctx-types -- snippet
cat "$SCRATCH/gctx-snippet-types.log" >> "$SCRATCH/gctx-snippet-tests.log"

require_grep "$SCRATCH/gctx-snippet-tests.log" 'project_snippet_is_identity_only_without_capability_ce1.*ok' 'CE-1 identity-only'
require_grep "$SCRATCH/gctx-snippet-tests.log" 'project_snippet_returns_text_when_fresh_and_capability_asserted.*ok' 'CE-1 gated text'
require_grep "$SCRATCH/gctx-snippet-tests.log" 'project_snippet_withholds_text_when_stale_ce7.*ok' 'CE-7 stale withhold'

# Step 3 — GCTX-022 slicer
log "running: gctx slicer tests"
capture "$SCRATCH/gctx-slice-tests.log" \
  cargo test -p eddacraft-anvil-gctx-egress -- slice budget byte ceiling determin

require_grep "$SCRATCH/gctx-slice-tests.log" 'test result: ok' 'slicer tests green'

# Step 4 — symbol_context integration (socket + MCP), twice each
run_twice gctx-symbol-context-wired \
  cargo test -p eddacraft-anvil-intercept --test gctx_symbol_context_wired

run_twice gctx-symbol-context-mcp \
  cargo test -p eddacraft-anvil --test gctx_symbol_context_integration

# Merge integration evidence into the plan-mandated log name
cat "$SCRATCH/gctx-symbol-context-wired-run1.log" \
    "$SCRATCH/gctx-symbol-context-wired-run2.log" \
    "$SCRATCH/gctx-symbol-context-mcp-run1.log" \
    "$SCRATCH/gctx-symbol-context-mcp-run2.log" \
  > "$SCRATCH/gctx-symbol-context.log"

require_grep "$SCRATCH/gctx-symbol-context.log" \
  'symbol_context_identity_only_without_snippet_egress.*ok' \
  'socket CE-1 identity-only'
require_grep "$SCRATCH/gctx-symbol-context.log" \
  'symbol_context_emits_text_with_egress_and_capability.*ok' \
  'socket gated text'
require_grep "$SCRATCH/gctx-symbol-context.log" \
  'mcp_symbol_context_identity_only_without_snippet_egress.*ok' \
  'MCP CE-1 identity-only'
require_grep "$SCRATCH/gctx-symbol-context.log" \
  'mcp_symbol_context_emits_text_with_egress_and_capability.*ok' \
  'MCP gated text'
require_grep "$SCRATCH/gctx-symbol-context.log" \
  'symbol_context_not_ready_on_cold' \
  'warming/cold degradation'
require_grep "$ROOT/crates/anvil-intercept/tests/gctx_symbol_context_wired.rs" \
  'redaction_summary' \
  'redaction_summary assertion in wired integration tests'
require_grep "$ROOT/crates/anvil-cli/tests/gctx_symbol_context_integration.rs" \
  'redaction_summary' \
  'redaction_summary assertion in MCP integration tests'

# Step 5 — targeted workspace evidence (gctx surfaces; avoids unrelated crate flakes)
log "running: targeted gctx/graph-cache crate tests"
capture "$SCRATCH/workspace-tests.log" \
  cargo test -p eddacraft-anvil-graph-cache \
             -p eddacraft-anvil-gctx-types \
             -p eddacraft-anvil-gctx-egress \
             -p eddacraft-anvil-intercept \
             -p eddacraft-anvil
require_grep "$SCRATCH/workspace-tests.log" 'test result: ok' 'targeted gctx crates green'

# Full egress crate run (supplementary)
capture "$SCRATCH/gctx-egress-tests.log" cargo test -p eddacraft-anvil-gctx-egress

log "all gates passed — logs in $SCRATCH"