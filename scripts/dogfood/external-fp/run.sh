#!/usr/bin/env bash
#
# External-codebase false-positive dogfood runner.
#
# Clones the pinned corpus (corpus.json) at fixed commits, runs `anvil check`
# over each repo in two passes (default catalogue + --include-opt-in), and dumps
# raw JSON + a per-repo summary into the output directory. The judgement step
# (TP/FP classification) is done by classify.py against these artifacts.
#
# This is the repeatable layer on top of the in-repo internal dogfood test
# (crates/anvil-checks-ast/tests/dogfood.rs, RSTLAN-008): that test guards Anvil
# against itself; this harness guards the rule catalogues against diverse,
# idiomatic external code (council §16.5 #9 FP bar).
#
# Usage:
#   ANVIL_BIN=/path/to/anvil ./run.sh <rust|langts|all>
#
# Env:
#   ANVIL_BIN      (required) path to a built `anvil` binary
#   EXT_FP_WORK    clone cache dir            (default: /tmp/anvil-ext-fp)
#   EXT_FP_OUT     results dir                (default: $EXT_FP_WORK/out)
#
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CORPUS="$HERE/corpus.json"
GROUP="${1:?usage: ANVIL_BIN=... run.sh <rust|langts|all>}"
: "${ANVIL_BIN:?set ANVIL_BIN to a built anvil binary}"
WORK="${EXT_FP_WORK:-/tmp/anvil-ext-fp}"
OUT="${EXT_FP_OUT:-$WORK/out}"
mkdir -p "$WORK" "$OUT"

groups=()
case "$GROUP" in
  all) groups=(rust langts) ;;
  rust|langts) groups=("$GROUP") ;;
  *) echo "unknown group: $GROUP (want rust|langts|all)" >&2; exit 2 ;;
esac

# Emit "group<TAB>name<TAB>url<TAB>sha" lines for the requested group(s).
manifest_rows() {
  python3 - "$CORPUS" "$@" <<'PY'
import json, sys
corpus = json.load(open(sys.argv[1]))
for g in sys.argv[2:]:
    for r in corpus["groups"][g]["repos"]:
        print(f'{g}\t{r["name"]}\t{r["url"]}\t{r["sha"]}')
PY
}

clone_pinned() {  # url sha dest
  local url="$1" sha="$2" dest="$3"
  if [ -e "$dest/.git" ] && [ "$(git -C "$dest" rev-parse HEAD 2>/dev/null)" = "$sha" ]; then
    echo "  cached at $sha"
    return
  fi
  rm -rf "$dest"
  git init -q "$dest"
  git -C "$dest" remote add origin "$url"
  # GitHub allows fetching a specific commit; --depth 1 keeps it cheap.
  git -C "$dest" fetch -q --depth 1 origin "$sha"
  git -C "$dest" checkout -q FETCH_HEAD
  echo "  fetched $sha"
}

while IFS=$'\t' read -r group name url sha; do
  [ -n "${name:-}" ] || continue
  echo "== [$group] $name =="
  dest="$WORK/$name"
  clone_pinned "$url" "$sha" "$dest"

  # check returns rc=1 when findings exist; don't let set -e abort.
  ( cd "$dest" && "$ANVIL_BIN" check --all --json --no-tui ) \
      > "$OUT/$name.default.json" 2> "$OUT/$name.default.err" || true
  ( cd "$dest" && "$ANVIL_BIN" check --all --include-opt-in --json --no-tui ) \
      > "$OUT/$name.optin.json" 2> "$OUT/$name.optin.err" || true
  # architecture validate needs .anvil/architecture.yaml; on a bare external
  # repo it reports "no architecture.yaml" — captured as N/A, not a finding.
  ( cd "$dest" && "$ANVIL_BIN" architecture validate --json --no-tui ) \
      > "$OUT/$name.arch.json" 2> "$OUT/$name.arch.err" || true

  python3 - "$OUT/$name.default.json" "$OUT/$name.default.err" "$name" <<'PY'
import json, sys, collections, pathlib
out, err, name = sys.argv[1], sys.argv[2], sys.argv[3]
try:
    d = json.load(open(out))
except Exception as e:
    print(f"  !! could not parse {out}: {e}"); sys.exit(0)
warns = d.get("warnings", [])
by = collections.Counter(w.get("id") for w in warns)
errtext = pathlib.Path(err).read_text(errors="replace") if pathlib.Path(err).exists() else ""
panics = sum(errtext.count(s) for s in ("panicked", "RUST_BACKTRACE", "thread '"))
print(f"  files scanned: {len(d.get('files', []))}  findings: {len(warns)}  panics(stderr): {panics}")
for rid, n in sorted(by.items()):
    print(f"    {rid}: {n}")
PY
done < <(manifest_rows "${groups[@]}")

echo
echo "raw artifacts + summaries in: $OUT"
echo "next: python3 $HERE/classify.py <rust|langts|all>   # builds the TP/FP worksheet"
