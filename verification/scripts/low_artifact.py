"""Run verification with bounded Cargo artifacts and an explicit disk preflight.

Never deletes caches or changes the developer's Cargo profiles. The child keeps
normal test assertions, overflow checks and optimization settings.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

from run import ROOT


def directory_bytes(path):
    return sum(item.stat().st_size for item in path.rglob("*") if item.is_file()) if path.exists() else 0


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--name", required=True)
    parser.add_argument("--summary", default="verification/summaries/producer-closure-certificate/summary.json")
    parser.add_argument("--target", default="target/foundation-integration")
    parser.add_argument("--min-free-mib", type=int, default=4096)
    parser.add_argument("--env", action="append", default=[])
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        parser.error("a verification command is required")
    target = (ROOT / args.target).resolve()
    ancestor = target
    while not ancestor.exists():
        ancestor = ancestor.parent
    free = shutil.disk_usage(ancestor).free
    preflight = {
        "target_directory": str(target), "existing_target_bytes": directory_bytes(target),
        "available_bytes": free, "required_free_bytes": args.min_free_mib * 1024 * 1024,
        "infrastructure_blocked": free < args.min_free_mib * 1024 * 1024,
    }
    output = ROOT / "verification/generated" / Path(args.summary).parent.name
    output.mkdir(parents=True, exist_ok=True)
    (output / f"{args.name}-disk.json").write_text(json.dumps(preflight, indent=2) + "\n")
    print(json.dumps(preflight), flush=True)
    if preflight["infrastructure_blocked"]:
        print("Insufficient disk reserve; no build started and no cache removed.", flush=True)
        return 125
    overrides = {
        "CARGO_INCREMENTAL": "0", "CARGO_PROFILE_DEV_DEBUG": "0",
        "CARGO_PROFILE_TEST_DEBUG": "0", "CARGO_BUILD_JOBS": "2",
        "CARGO_TARGET_DIR": str(target),
    }
    overrides.update(item.split("=", 1) for item in args.env)
    runner = [sys.executable, str(ROOT / "verification/scripts/run.py"),
              "--name", args.name, "--summary", args.summary]
    for name, value in overrides.items():
        runner += ["--env", f"{name}={value}"]
    return subprocess.call([*runner, "--", *command], cwd=ROOT, env=os.environ.copy())


if __name__ == "__main__":
    raise SystemExit(main())
