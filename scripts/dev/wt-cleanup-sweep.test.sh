#!/usr/bin/env bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
helper="${repo_root}/scripts/dev/wt-cleanup-sweep.sh"
helper_py="${repo_root}/scripts/dev/wt-cleanup-sweep.py"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
origin="$tmp/origin.git"
repo="$tmp/repo"

git init -q --bare "$origin"
git init -q -b main "$repo"
git -C "$repo" config user.email test@example.invalid
git -C "$repo" config user.name 'Test User'
printf 'root\n' > "$repo/root.txt"
git -C "$repo" add root.txt
git -C "$repo" commit -q -m 'initial'
git -C "$repo" remote add origin "$origin"
git -C "$repo" push -q -u origin main

# Merged, clean, upstream deleted: the only eligible candidate.
git -C "$repo" worktree add -q "$tmp/wt merged space" -b feat/merged
printf 'merged\n' > "$tmp/wt merged space/merged.txt"
git -C "$tmp/wt merged space" add merged.txt
git -C "$tmp/wt merged space" commit -q -m 'merged branch'
git -C "$tmp/wt merged space" push -q -u origin feat/merged
git -C "$repo" merge -q --no-ff feat/merged -m 'merge feat/merged'
git -C "$repo" push -q origin main
git -C "$repo" push -q origin --delete feat/merged

# Unmerged but remote gone: must not be eligible.
git -C "$repo" worktree add -q "$tmp/wt-unmerged" -b feat/unmerged origin/main
printf 'unmerged\n' > "$tmp/wt-unmerged/unmerged.txt"
git -C "$tmp/wt-unmerged" add unmerged.txt
git -C "$tmp/wt-unmerged" commit -q -m 'unmerged branch'
git -C "$tmp/wt-unmerged" push -q -u origin feat/unmerged
git -C "$repo" push -q origin --delete feat/unmerged

# Merged but dirty: must not be eligible.
git -C "$repo" worktree add -q "$tmp/wt-dirty" -b feat/dirty origin/main
printf 'dirty\n' > "$tmp/wt-dirty/dirty.txt"
git -C "$tmp/wt-dirty" add dirty.txt
git -C "$tmp/wt-dirty" commit -q -m 'dirty branch'
git -C "$tmp/wt-dirty" push -q -u origin feat/dirty
git -C "$repo" merge -q --no-ff feat/dirty -m 'merge feat/dirty'
git -C "$repo" push -q origin main
git -C "$repo" push -q origin --delete feat/dirty
printf 'local edit\n' >> "$tmp/wt-dirty/dirty.txt"

# Detached worktree: manual only.
git -C "$repo" worktree add -q --detach "$tmp/wt-detached" origin/main

git -C "$repo" fetch -q --prune origin main

dry_json="$tmp/dry.json"
(cd "$repo" && "$helper" --dry-run --json) > "$dry_json"
python3 - "$dry_json" <<'PY'
import json, sys
entries=json.load(open(sys.argv[1]))
by_branch={e.get('branch'): e for e in entries}
assert by_branch['feat/merged']['eligible'], by_branch['feat/merged']
for branch in ['feat/unmerged', 'feat/dirty']:
    assert not by_branch[branch]['eligible'], by_branch[branch]
assert by_branch['main']['eligible'] is False, by_branch['main']
assert any(e['detached'] and not e['eligible'] for e in entries), entries
eligible=[e['branch'] for e in entries if e['eligible']]
assert eligible == ['feat/merged'], eligible
PY

log="$tmp/wt.log"
fake_wt="$tmp/wt"
cat > "$fake_wt" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "$WT_LOG"
for arg in "$@"; do
  case "$arg" in
    -f|--force|-D|--force-delete|--no-hooks|-y|--yes)
      echo "forbidden argument: $arg" >&2
      exit 44
      ;;
  esac
done
branch="${@: -1}"
path=$(git -C "$WT_TEST_REPO" worktree list --porcelain | awk -v branch="refs/heads/${branch}" '
  /^worktree / { path=substr($0, 10) }
  /^branch / && substr($0, 8) == branch { print path; found=1 }
  END { if (!found) exit 1 }
')
git -C "$WT_TEST_REPO" worktree remove "$path"
git -C "$WT_TEST_REPO" branch -d "$branch" >/dev/null
printf '{"removed":"%s"}\n' "$branch"
FAKE
chmod +x "$fake_wt"
WT_BIN="$fake_wt" WT_LOG="$log" WT_TEST_REPO="$repo" bash -c 'cd "$0" && "$1" --apply --confirm-for-test feat/merged' "$repo" "$helper" > "$tmp/apply.out"

grep -Fq 'remove --foreground --format json feat/merged' "$log"
if grep -Eq -- '(^| )(-f|--force|-D|--force-delete|--no-hooks|-y|--yes)( |$)' "$log"; then
  echo "forbidden wt flag used" >&2
  cat "$log" >&2
  exit 1
fi
[ ! -d "$tmp/wt merged space" ]
[ -d "$tmp/wt-unmerged" ]
[ -d "$tmp/wt-dirty" ]
[ -d "$tmp/wt-detached" ]
git -C "$repo" rev-parse --verify feat/unmerged >/dev/null
git -C "$repo" rev-parse --verify feat/dirty >/dev/null
if git -C "$repo" rev-parse --verify feat/merged >/dev/null 2>&1; then
  echo "feat/merged branch should have been deleted by fake wt" >&2
  exit 1
fi

# Static guard: the helper itself must not contain destructive bypasses.
if grep -Eq -- 'wt remove.*(-f|--force|-D|--force-delete|--no-hooks|-y|--yes)|git branch -D|rm -rf' "$helper_py"; then
  echo "helper contains forbidden destructive cleanup primitive" >&2
  exit 1
fi
