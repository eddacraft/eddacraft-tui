#!/usr/bin/env python3
"""Ref-resolution tests for the vendored-gitleaks refresh script.

`--upgrade` used to take `object.sha` from `git/ref/tags/<tag>` as the commit.
That is true for a lightweight tag and false for an annotated one, where the
SHA names a tag object. These tests pin the peel: inspect the object type,
walk nested tag objects to a commit, and fail loudly when the chain is unbound
or the object is not a commit.
"""

from __future__ import annotations

import json
import os
import stat
import subprocess
import tempfile
import tomllib
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("refresh-gitleaks-ruleset.sh")

LIGHT_COMMIT = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
ANN_TAG = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
ANN_COMMIT = "cccccccccccccccccccccccccccccccccccccccc"
NEST_TAG_A = "dddddddddddddddddddddddddddddddddddddddd"
NEST_TAG_B = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
NEST_COMMIT = "ffffffffffffffffffffffffffffffffffffffff"
TREE_OBJ = "1111111111111111111111111111111111111111"
UNBOUND_A = "2222222222222222222222222222222222222222"
UNBOUND_B = "3333333333333333333333333333333333333333"

# Documented bound in refresh-gitleaks-ruleset.sh — a tag chain that still has
# not reached a commit after this many peels is treated as unbound.
MAX_TAG_PEEL = 8

UPSTREAM_TOML = r"""[[rules]]
id = "postman-api-token"
description = "Postman API token"
regex = '''\b(PMAK-[a-zA-Z0-9]{24}-[a-zA-Z0-9]{34})\b'''
"""
UPSTREAM_LICENCE = "MIT License\n"

GRAPH = {
    "refs": {
        "v-light": {"type": "commit", "sha": LIGHT_COMMIT},
        "v-ann": {"type": "tag", "sha": ANN_TAG},
        "v-nested": {"type": "tag", "sha": NEST_TAG_A},
        "v-tree": {"type": "tree", "sha": TREE_OBJ},
        "v-unbound": {"type": "tag", "sha": UNBOUND_A},
    },
    "tags": {
        ANN_TAG: {"type": "commit", "sha": ANN_COMMIT},
        NEST_TAG_A: {"type": "tag", "sha": NEST_TAG_B},
        NEST_TAG_B: {"type": "commit", "sha": NEST_COMMIT},
        UNBOUND_A: {"type": "tag", "sha": UNBOUND_B},
        UNBOUND_B: {"type": "tag", "sha": UNBOUND_A},
    },
}

FAKE_CURL = r"""#!/usr/bin/env python3
import json, os, sys
from pathlib import Path

graph = json.loads(os.environ["FAKE_GITLEAKS_GRAPH"])
log_path = Path(os.environ["FAKE_CURL_LOG"])
body_dir = Path(os.environ["FAKE_CURL_BODIES"])

args = sys.argv[1:]
out = None
url = None
i = 0
while i < len(args):
    arg = args[i]
    if arg in ("-o", "--output"):
        out = args[i + 1]
        i += 2
        continue
    if arg.startswith("-"):
        i += 1
        continue
    url = arg
    i += 1

if not url:
    sys.stderr.write("fake curl: no URL\n")
    sys.exit(2)

with log_path.open("a", encoding="utf-8") as handle:
    handle.write(url + "\n")


def write(body: bytes, status: int = 0) -> None:
    if out:
        Path(out).write_bytes(body)
    else:
        sys.stdout.buffer.write(body)
    raise SystemExit(status)


if "/git/ref/tags/" in url:
    tag = url.rsplit("/", 1)[-1]
    ref = graph["refs"].get(tag)
    if ref is None:
        sys.stderr.write(f"fake curl: unknown tag ref {tag}\n")
        write(b"{}", 22)
    payload = {"object": {"type": ref["type"], "sha": ref["sha"]}}
    write(json.dumps(payload).encode())

if "/git/tags/" in url:
    sha = url.rsplit("/", 1)[-1]
    tag_obj = graph["tags"].get(sha)
    if tag_obj is None:
        sys.stderr.write(f"fake curl: unknown tag object {sha}\n")
        write(b"{}", 22)
    payload = {"object": {"type": tag_obj["type"], "sha": tag_obj["sha"]}}
    write(json.dumps(payload).encode())

if "raw.githubusercontent.com" in url:
    parts = url.split("/")
    rev = parts[5]
    path = "/".join(parts[6:])
    key = f"{rev}/{path}"
    body_file = body_dir / key.replace("/", "__")
    if not body_file.is_file():
        sys.stderr.write(f"fake curl: no raw body for {key}\n")
        write(b"", 22)
    write(body_file.read_bytes())

sys.stderr.write(f"fake curl: unhandled URL {url}\n")
sys.exit(2)
"""


def _logged_urls(log_path: Path) -> list[str]:
    if not log_path.is_file():
        return []
    return [line for line in log_path.read_text(encoding="utf-8").splitlines() if line]


class FakeGithubHarness:
    """Temporary PATH with a curl that serves a canned GitHub tag graph."""

    def __init__(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        root = Path(self._tmp.name)
        self.bin = root / "bin"
        self.bin.mkdir()
        self.log = root / "curl.log"
        self.bodies = root / "bodies"
        self.bodies.mkdir()
        self.vendor = root / "vendor"
        self.vendor.mkdir()
        curl = self.bin / "curl"
        curl.write_text(FAKE_CURL, encoding="utf-8")
        curl.chmod(curl.stat().st_mode | stat.S_IEXEC)
        self.env = os.environ.copy()
        self.env["PATH"] = f"{self.bin}:{self.env.get('PATH', '')}"
        self.env["FAKE_GITLEAKS_GRAPH"] = json.dumps(GRAPH)
        self.env["FAKE_CURL_LOG"] = str(self.log)
        self.env["FAKE_CURL_BODIES"] = str(self.bodies)
        self.env["ANVIL_GITLEAKS_VENDOR_DIR"] = str(self.vendor)
        self.env["ANVIL_SKIP_CALIBRATE"] = "1"

    def close(self) -> None:
        self._tmp.cleanup()

    def put_raw(self, rev: str, path: str, body: str) -> None:
        target = self.bodies / f"{rev}__{path.replace('/', '__')}"
        target.write_text(body, encoding="utf-8")

    def seed_vendor(self) -> None:
        (self.vendor / "PIN.toml").write_text(
            "\n".join(
                [
                    'repo = "https://github.com/gitleaks/gitleaks"',
                    'tag = "v0.0.0"',
                    'commit = "0000000000000000000000000000000000000000"',
                    'source_path = "config/gitleaks.toml"',
                    'sha256 = "0000000000000000000000000000000000000000000000000000000000000000"',
                    'licence = "MIT"',
                    'licence_path = "LICENSE"',
                    "",
                ]
            ),
            encoding="utf-8",
        )
        (self.vendor / "tier1-rules.txt").write_text("postman-api-token\n", encoding="utf-8")
        (self.vendor / "tier1.json").write_text("{}\n", encoding="utf-8")
        (self.vendor / "PROVENANCE.md").write_text("# stub\n", encoding="utf-8")
        (self.vendor / "LICENSE").write_text("stub\n", encoding="utf-8")


class RefreshRefResolutionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.harness = FakeGithubHarness()

    def tearDown(self) -> None:
        self.harness.close()

    def resolve(self, tag: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                "bash",
                "-c",
                'source "$1"; resolve_tag_to_commit "gitleaks/gitleaks" "$2"',
                "bash",
                str(SCRIPT),
                tag,
            ],
            env=self.harness.env,
            capture_output=True,
            text=True,
            check=False,
        )

    def test_lightweight_tag_is_the_commit_without_peeling(self) -> None:
        result = self.resolve("v-light")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), LIGHT_COMMIT)
        urls = _logged_urls(self.harness.log)
        self.assertEqual(len(urls), 1)
        self.assertIn("/git/ref/tags/v-light", urls[0])
        self.assertFalse(any("/git/tags/" in url for url in urls))

    def test_annotated_tag_peels_to_the_commit(self) -> None:
        result = self.resolve("v-ann")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), ANN_COMMIT)
        self.assertNotEqual(result.stdout.strip(), ANN_TAG)
        urls = _logged_urls(self.harness.log)
        self.assertTrue(any("/git/ref/tags/v-ann" in url for url in urls))
        self.assertTrue(any(url.endswith(f"/git/tags/{ANN_TAG}") for url in urls))

    def test_nested_tag_peels_to_the_commit(self) -> None:
        result = self.resolve("v-nested")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), NEST_COMMIT)
        urls = _logged_urls(self.harness.log)
        self.assertTrue(any(url.endswith(f"/git/tags/{NEST_TAG_A}") for url in urls))
        self.assertTrue(any(url.endswith(f"/git/tags/{NEST_TAG_B}") for url in urls))

    def test_non_commit_object_fails_loudly(self) -> None:
        result = self.resolve("v-tree")
        self.assertNotEqual(result.returncode, 0)
        err = result.stderr
        self.assertIn("not a commit", err)
        self.assertIn("tree", err)
        self.assertIn("v-tree", err)

    def test_unbound_tag_chain_fails_loudly(self) -> None:
        result = self.resolve("v-unbound")
        self.assertNotEqual(result.returncode, 0)
        err = result.stderr
        self.assertIn("v-unbound", err)
        self.assertTrue(
            "peel" in err.lower() or "chain" in err.lower(),
            f"expected a peel/chain error, got: {err}",
        )
        self.assertIn(str(MAX_TAG_PEEL), err)
        tag_fetches = [
            url for url in _logged_urls(self.harness.log) if "/git/tags/" in url
        ]
        self.assertEqual(len(tag_fetches), MAX_TAG_PEEL)


class RefreshUpgradeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.harness = FakeGithubHarness()
        self.harness.seed_vendor()
        for rev in (LIGHT_COMMIT, ANN_COMMIT):
            self.harness.put_raw(rev, "config/gitleaks.toml", UPSTREAM_TOML)
            self.harness.put_raw(rev, "LICENSE", UPSTREAM_LICENCE)

    def tearDown(self) -> None:
        self.harness.close()

    def upgrade(self, tag: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["bash", str(SCRIPT), "--upgrade", tag, "--no-calibrate"],
            env=self.harness.env,
            capture_output=True,
            text=True,
            check=False,
        )

    def pin(self) -> dict[str, str]:
        return tomllib.loads((self.harness.vendor / "PIN.toml").read_text(encoding="utf-8"))

    def test_lightweight_tag_upgrade_still_pins_the_commit(self) -> None:
        result = self.upgrade("v-light")
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        pin = self.pin()
        self.assertEqual(pin["tag"], "v-light")
        self.assertEqual(pin["commit"], LIGHT_COMMIT)
        urls = _logged_urls(self.harness.log)
        self.assertTrue(
            any(f"/{LIGHT_COMMIT}/config/gitleaks.toml" in url for url in urls),
            urls,
        )
        self.assertFalse(any("/git/tags/" in url for url in urls))
        self.assertIn("postman-api-token", (self.harness.vendor / "tier1.json").read_text())

    def test_annotated_tag_upgrade_pins_peeled_commit_not_tag_object(self) -> None:
        result = self.upgrade("v-ann")
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        pin = self.pin()
        self.assertEqual(pin["tag"], "v-ann")
        self.assertEqual(pin["commit"], ANN_COMMIT)
        self.assertNotEqual(pin["commit"], ANN_TAG)
        urls = _logged_urls(self.harness.log)
        self.assertTrue(any(f"/{ANN_COMMIT}/config/gitleaks.toml" in url for url in urls), urls)
        self.assertFalse(any(f"/{ANN_TAG}/" in url for url in urls))


if __name__ == "__main__":
    unittest.main()
