#!/usr/bin/env bash
# rust-analyzer LSP coexistence runner — ADOPT-006.
#
# Spawns rust-analyzer, drives an initialize/shutdown cycle against the
# rust fixture, and exits 0 on a clean exchange. Exit 200 signals
# "binary not on PATH" so the harness records a skip.

set -euo pipefail

case "${1:-}" in
  --print-fixture)
    echo "fixtures/rust"
    exit 0
    ;;
  --run-against)
    shift
    target_dir="${1:?--run-against requires a directory}"
    ;;
  *)
    echo "usage: $0 (--print-fixture | --run-against <dir>)" >&2
    exit 2
    ;;
esac

if ! command -v rust-analyzer >/dev/null 2>&1; then
  exit 200
fi

# Minimal JSON-RPC `initialize` + `initialized` + `shutdown` + `exit`.
# Drives rust-analyzer just far enough that it opens the workspace.
python3 - "${target_dir}" <<'PY'
import json
import os
import subprocess
import sys
import time

target_dir = os.path.abspath(sys.argv[1])
root_uri = "file://" + target_dir

def frame(payload: dict) -> bytes:
    body = json.dumps(payload).encode("utf-8")
    return b"Content-Length: " + str(len(body)).encode() + b"\r\n\r\n" + body

initialize = frame({
    "jsonrpc": "2.0", "id": 1, "method": "initialize",
    "params": {
        "processId": os.getpid(),
        "rootUri": root_uri,
        "capabilities": {},
        "workspaceFolders": [{"uri": root_uri, "name": "fixture"}],
    },
})
initialized = frame({"jsonrpc": "2.0", "method": "initialized", "params": {}})
shutdown = frame({"jsonrpc": "2.0", "id": 2, "method": "shutdown"})
exit_msg = frame({"jsonrpc": "2.0", "method": "exit"})

proc = subprocess.Popen(
    ["rust-analyzer"],
    cwd=target_dir,
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
)
try:
    proc.stdin.write(initialize + initialized)
    proc.stdin.flush()
    time.sleep(2.0)
    proc.stdin.write(shutdown + exit_msg)
    proc.stdin.flush()
    try:
        rc = proc.wait(timeout=20)
    except subprocess.TimeoutExpired:
        proc.kill()
        sys.stderr.write("rust-analyzer did not exit within 20s\n")
        sys.exit(1)
finally:
    if proc.poll() is None:
        proc.kill()

sys.exit(rc if rc == 0 else 1)
PY
