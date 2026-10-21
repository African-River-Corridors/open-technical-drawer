"""Copy the Drawing Format Standard into standard/ and pin it.

    python3 tools/vendor_standard.py --from <clone of technical_drawings_for_agents> [--commit <sha>]
    python3 tools/vendor_standard.py --check

The copy holds the clone's docs/standard/ folder and, under fonts/, the standard's font with its
licence. PIN.json records the commit of the standard's repository and one SHA-256 over the copy:
the SHA-256 of the text made of one line per file, "<file sha256>  <relative path>\n", sorted by path,
PIN.json left out. The Rust test `vendored_copy_is_unchanged` computes the same hash.

Updating the copy is a deliberate change: run this script, read the diff, and say why in the PR.
Standard library only.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path

DEST = Path(__file__).resolve().parent.parent / "standard"
FONT_SRC = Path("src/technical_drawings_for_agents/data/fonts")
REPO = "African-River-Corridors/technical_drawings_for_agents"


def copy_hash(root: Path) -> tuple[str, int]:
    lines = []
    for path in sorted((p for p in root.rglob("*") if p.is_file() and p.name != "PIN.json"), key=lambda p: p.relative_to(root).as_posix()):
        rel = path.relative_to(root).as_posix()
        lines.append(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {rel}\n")
    return hashlib.sha256("".join(lines).encode()).hexdigest(), len(lines)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--from", dest="src")
    ap.add_argument("--commit")
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    if args.check:
        pin = json.loads((DEST / "PIN.json").read_text())
        digest, count = copy_hash(DEST)
        ok = digest == pin["sha256"] and count == pin["files"]
        print("ok" if ok else f"CHANGED: {digest} ({count} files) != {pin['sha256']} ({pin['files']})")
        return 0 if ok else 1
    if not args.src:
        ap.error("--from is required")
    src = Path(args.src).resolve()
    commit = args.commit or subprocess.run(["git", "-C", str(src), "rev-parse", "HEAD"], capture_output=True, text=True, check=True).stdout.strip()
    dirty = subprocess.run(["git", "-C", str(src), "status", "--porcelain", "docs/standard", str(FONT_SRC)], capture_output=True, text=True, check=True).stdout
    if dirty.strip():
        print("the clone has uncommitted changes in the standard; refusing", file=sys.stderr)
        return 1
    if DEST.exists():
        shutil.rmtree(DEST)
    shutil.copytree(src / "docs" / "standard", DEST)
    shutil.copytree(src / FONT_SRC, DEST / "fonts")
    digest, count = copy_hash(DEST)
    pin = {"repository": REPO, "commit": commit, "files": count, "sha256": digest}
    (DEST / "PIN.json").write_text(json.dumps(pin, indent=2) + "\n")
    print(json.dumps(pin, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
