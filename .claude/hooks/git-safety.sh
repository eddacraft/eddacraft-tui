#!/bin/bash
# Git Safety Hook - PreToolUse guard for destructive git operations
# Returns "ask" for operations that can destroy untracked files or affect shared state

set -euo pipefail

TOOL_INPUT="${1:-}"

[[ -z "$TOOL_INPUT" ]] && exit 0

# Extract command string from JSON or raw input
cmd=$(echo "$TOOL_INPUT" | sed -n 's/.*"command"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' 2>/dev/null || echo "$TOOL_INPUT")

# --- Untracked file destruction ---
# git clean removes untracked files (the -f flag makes it actually run)
if echo "$cmd" | grep -qE '\bgit\b.*\bclean\b.*-[a-zA-Z]*f'; then
    echo '{"decision": "ask", "reason": "git clean -f deletes untracked files permanently"}' >&2
    exit 0
fi

# git checkout -- . / git restore . discard all working tree changes
if echo "$cmd" | grep -qE '\bgit\b\s+checkout\s+--\s+\.'; then
    echo '{"decision": "ask", "reason": "git checkout -- . discards all uncommitted changes"}' >&2
    exit 0
fi
if echo "$cmd" | grep -qE '\bgit\b\s+restore\s+\.'; then
    echo '{"decision": "ask", "reason": "git restore . discards all uncommitted changes"}' >&2
    exit 0
fi

# git reset --hard throws away uncommitted work
if echo "$cmd" | grep -qE '\bgit\b.*\breset\b.*--hard'; then
    echo '{"decision": "ask", "reason": "git reset --hard discards all uncommitted changes"}' >&2
    exit 0
fi

# git stash --all / --include-untracked can lose untracked files if stash is dropped
if echo "$cmd" | grep -qE '\bgit\b\s+stash\b.*(--all|--include-untracked|-u\b)'; then
    echo '{"decision": "ask", "reason": "git stash with untracked files — stash can be lost if dropped"}' >&2
    exit 0
fi

# --- Worktree / branch removal that may contain untracked files ---
if echo "$cmd" | grep -qE '\bgit\b\s+worktree\s+remove\b'; then
    echo '{"decision": "ask", "reason": "git worktree remove deletes the worktree directory including any untracked files"}' >&2
    exit 0
fi

# git branch -D (force delete) — less dangerous but can lose unmerged work
if echo "$cmd" | grep -qE '\bgit\b\s+branch\s+-D\b'; then
    echo '{"decision": "ask", "reason": "git branch -D force-deletes branch even if unmerged"}' >&2
    exit 0
fi

# --- Shared state: PR merges and force pushes ---
if echo "$cmd" | grep -qE '\bgh\b\s+pr\s+merge\b'; then
    echo '{"decision": "ask", "reason": "Merging a pull request — confirm this is ready"}' >&2
    exit 0
fi

if echo "$cmd" | grep -qE '\bgit\b\s+push\b.*--force'; then
    echo '{"decision": "ask", "reason": "Force push rewrites remote history"}' >&2
    exit 0
fi

if echo "$cmd" | grep -qE '\bgit\b\s+push\b.*\b(main|master)\b'; then
    echo '{"decision": "ask", "reason": "Pushing directly to main/master"}' >&2
    exit 0
fi

exit 0
