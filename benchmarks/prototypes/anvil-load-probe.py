#!/usr/bin/env python3
"""Anvil watch CPU load probe — prototype for the tipping-point bench.

Reproduces the field "expensive CPU" report by exercising what the
watch_resource_budget bench does NOT: real file churn, the default `check`
action (a per-save `anvil check` subprocess), the whole process tree
(parent utime+stime + reaped-children cutime+cstime), and a ramp of N
concurrent "agents" (independent watch pipelines) to find saturation.

Linux only: CPU sampling reads /proc. The harness exits early on other
platforms. Cleans up spawned `anvil watch` agents and the synthetic temp
repo on normal exit AND on Ctrl-C / SIGTERM.

Usage:
  anvil-load-probe.py [--bin PATH] [--files N] [--agents "1,2,4,8"]
                      [--settle S] [--measure S] [--churn-ms MS] [--repo DIR]
Manual single cell:
  anvil-load-probe.py --agents 4 --action check   # or: --action none (control)

First run (2026-05-30, anvil 0.7.2-beta, 16-core box, 800 files, 200ms churn):
  agents  action   machine%   cores
       1   check      43.5%    6.96
       2   check      78.8%   12.61
       3   check      86.0%   13.75
       4   check      88.1%   14.09
       4    none       0.2%    0.03   <- control: watch loop alone is ~free
  => one agent eats ~7 of 16 cores; ~2 agents saturate. The per-save
     `anvil check --all` subprocess is ~100% of the cost. This is the seed
     for RLB-001 (see plans/modules/resource-load-benchmarking.aps.md, GH #2156).
"""
import argparse, atexit, os, shutil, signal, subprocess, sys, tempfile, threading, time

CLK = os.sysconf("SC_CLK_TCK")
NCPU = os.cpu_count() or 1

# Live resources so a Ctrl-C / SIGTERM between cells (or mid-build, before a
# cell's own teardown runs) never orphans `anvil watch` agents or leaks the
# synthetic temp repo. run_cell registers its procs/churns here and clears them
# on normal completion; _cleanup is idempotent.
_LIVE = {"procs": [], "churns": [], "repo": None, "created_tmp": False}

def _cleanup():
    for c in _LIVE["churns"]:
        c.stop.set()
    for p in _LIVE["procs"]:
        try:
            p.send_signal(signal.SIGINT)
        except OSError:
            # Already exited / reaped — nothing to signal.
            pass
    for p in _LIVE["procs"]:
        try:
            p.wait(timeout=5)
        except (subprocess.TimeoutExpired, OSError):
            try:
                p.kill()
                p.wait(timeout=5)  # reap so we don't leave a zombie
            except (OSError, subprocess.TimeoutExpired):
                # Best-effort: process gone or unkillable; do not block cleanup.
                pass
    _LIVE["procs"].clear()
    _LIVE["churns"].clear()
    if _LIVE["created_tmp"] and _LIVE["repo"]:
        shutil.rmtree(_LIVE["repo"], ignore_errors=True)
        _LIVE["repo"] = None

def _signal_cleanup(_signum, _frame):
    _cleanup()
    raise SystemExit(130)

def read_total_cpu():
    with open("/proc/stat") as f:
        parts = f.readline().split()[1:]
    return sum(int(x) for x in parts)

def read_proc_cpu(pid):
    """utime+stime+cutime+cstime (jiffies) for pid; 0 if gone."""
    try:
        with open(f"/proc/{pid}/stat") as f:
            data = f.read()
    except OSError:
        return 0
    rp = data.rfind(")")
    toks = data[rp + 2:].split()
    # fields (1-indexed orig): utime=14 stime=15 cutime=16 cstime=17
    # after split on last ')', state is field3 -> toks[0]; field N -> toks[N-3]
    return sum(int(toks[i]) for i in (11, 12, 13, 14))

def read_rss_mib(pid):
    try:
        with open(f"/proc/{pid}/status") as f:
            for line in f:
                if line.startswith("VmRSS:"):
                    return int(line.split()[1]) / 1024.0
    except OSError:
        # pid exited mid-sample (transient check child); count it as 0 RSS.
        pass
    return 0.0

def make_repo(root, n_files):
    # Caller guarantees `root` is an empty dir we own (never a user's tree).
    src = os.path.join(root, "src")
    for i in range(n_files):
        d = os.path.join(src, f"mod{i // 50}")
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, f"f{i}.ts"), "w") as f:
            f.write(f"export const v{i} = {i};\n")
            f.write(f"export function fn{i}(x: number): number {{ return x + {i}; }}\n")
            for j in range(20):
                f.write(f"export const a{i}_{j} = {i * j};\n")
    return src

class Churn(threading.Thread):
    def __init__(self, path, interval):
        super().__init__(daemon=True)
        self.path, self.interval, self.stop = path, interval, threading.Event()
    def run(self):
        n = 0
        while not self.stop.is_set():
            n += 1
            try:
                with open(self.path, "w") as f:
                    f.write(f"export const churn{n} = {n};\n")
                    f.write(f"export function edit{n}(x:number){{return x*{n};}}\n")
            except OSError:
                # Watcher may hold/rotate the file mid-write; skip this tick.
                pass
            self.stop.wait(self.interval)

def run_cell(binp, src_root, agents, action, settle, measure, churn_ms):
    env = {**os.environ, "ANVIL_DEV": "1", "ANVIL_DISABLE_UPDATE_HINT": "1"}
    args = [binp, "--json", "--no-tui", "watch", "--all", "--debounce=100"]
    if action != "check":
        args += ["--action", action]
    procs, churns = [], []
    # Publish to the live registry as we spawn so an interrupt mid-cell still
    # reaps every agent (the lists are shared references, updated in place).
    _LIVE["procs"] = procs
    _LIVE["churns"] = churns
    for k in range(agents):
        p = subprocess.Popen(args, cwd=os.path.dirname(src_root), env=env,
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        procs.append(p)
    time.sleep(settle)  # let cold scan + any initial check finish
    # one churn writer per agent (distinct file each)
    for k in range(agents):
        cf = os.path.join(src_root, f"agent{k}.ts")
        c = Churn(cf, churn_ms / 1000.0); c.start(); churns.append(c)
    # measure window
    t0_proc = sum(read_proc_cpu(p.pid) for p in procs)
    t0_tot = read_total_cpu()
    peak_rss = 0.0
    deadline = time.time() + measure
    while time.time() < deadline:
        peak_rss = max(peak_rss, sum(read_rss_mib(p.pid) for p in procs))
        time.sleep(0.25)
    t1_proc = sum(read_proc_cpu(p.pid) for p in procs)
    t1_tot = read_total_cpu()
    for c in churns: c.stop.set()
    for p in procs:
        p.send_signal(signal.SIGINT)
    for p in procs:
        try:
            p.wait(timeout=5)
        except subprocess.TimeoutExpired:
            p.kill()
            try:
                p.wait(timeout=5)  # reap the killed child; avoid a zombie
            except subprocess.TimeoutExpired:
                pass
    # Cell torn down cleanly — drop it from the live registry.
    _LIVE["procs"] = []
    _LIVE["churns"] = []
    dp, dt = (t1_proc - t0_proc), (t1_tot - t0_tot)
    machine_pct = 100.0 * dp / dt if dt else 0.0
    cores = NCPU * dp / dt if dt else 0.0
    return {"agents": agents, "action": action, "machine_pct": machine_pct,
            "cores": cores, "peak_rss_parent_mib": peak_rss}

def main():
    # CPU sampling reads /proc; bail clearly on non-Linux BEFORE spawning
    # anything (so we never leave orphaned agents on an unsupported platform).
    # Cross-platform coverage is tracked separately as RLB-006.
    if not os.path.exists("/proc/stat"):
        print("# anvil-load-probe requires Linux /proc for CPU sampling; "
              f"unsupported platform {sys.platform} (RLB-006 tracks portability)",
              file=sys.stderr)
        sys.exit(3)
    # Reap agents + temp repo on Ctrl-C / SIGTERM / normal exit.
    atexit.register(_cleanup)
    signal.signal(signal.SIGINT, _signal_cleanup)
    signal.signal(signal.SIGTERM, _signal_cleanup)

    ap = argparse.ArgumentParser()
    # Default to `anvil` on PATH; override with --bin or the ANVIL_BIN env var.
    ap.add_argument("--bin", default=os.environ.get("ANVIL_BIN", "anvil"))
    ap.add_argument("--files", type=int, default=1500)
    ap.add_argument("--agents", default="1,2,4,8")
    ap.add_argument("--action", default=None, help="single-cell action override")
    ap.add_argument("--settle", type=float, default=4.0)
    ap.add_argument("--measure", type=float, default=12.0)
    ap.add_argument("--churn-ms", type=int, default=200)
    ap.add_argument("--repo", default=None,
                    help="empty/new dir to build the synthetic repo in; "
                         "kept after the run. Default: a temp dir we create + remove.")
    a = ap.parse_args()
    # Only ever delete a directory we created. A user-supplied --repo must be
    # empty (or absent) so we never clobber an existing checkout.
    created_tmp = a.repo is None
    if created_tmp:
        repo = tempfile.mkdtemp(prefix="anvil-load-")
        # Hand the temp repo to the cleanup path so an interrupt removes it.
        _LIVE["repo"] = repo
        _LIVE["created_tmp"] = True
    else:
        repo = a.repo
        if os.path.isdir(repo) and os.listdir(repo):
            print(f"# refusing non-empty --repo {repo} (would clobber); "
                  f"pass an empty or new path", file=sys.stderr)
            sys.exit(2)
        os.makedirs(repo, exist_ok=True)
    print(f"# bin={a.bin}\n# files={a.files} ncpu={NCPU} churn={a.churn_ms}ms "
          f"settle={a.settle}s measure={a.measure}s", flush=True)
    print(f"# building {a.files}-file repo at {repo} ...", flush=True)
    src = make_repo(repo, a.files)
    print(f"{'agents':>6} {'action':>7} {'machine%':>9} {'cores':>7} {'RSS(parent)MiB':>15}", flush=True)
    levels = [int(x) for x in a.agents.split(",")]
    cells = []
    if a.action:  # single-cell manual mode
        cells = [(n, a.action) for n in levels]
    else:  # ramp: check at every level + a none control at the top level
        cells = [(n, "check") for n in levels] + [(max(levels), "none")]
    for agents, action in cells:
        r = run_cell(a.bin, src, agents, action, a.settle, a.measure, a.churn_ms)
        print(f"{r['agents']:>6} {r['action']:>7} {r['machine_pct']:>8.1f}% "
              f"{r['cores']:>7.2f} {r['peak_rss_parent_mib']:>15.1f}", flush=True)
    if created_tmp:
        shutil.rmtree(repo, ignore_errors=True)
    else:
        print(f"# left synthetic repo at {repo} (user-provided --repo)", flush=True)
    print("# done", flush=True)

if __name__ == "__main__":
    main()
