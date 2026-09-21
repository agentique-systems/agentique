"""Run a verification command; keep raw output only in ignored storage."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]


def source_identity(summary):
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT).decode().strip()
    digest = hashlib.sha256(subprocess.check_output(["git", "diff", "HEAD", "--binary", "--", ".", ":(exclude)" + summary], cwd=ROOT, stderr=subprocess.DEVNULL))
    paths = subprocess.check_output(
        ["git", "ls-files", "--others", "--exclude-standard", "-z"], cwd=ROOT
    ).decode().split("\0")
    for name in sorted(filter(None, paths)):
        if name == summary:
            continue
        digest.update(name.encode())
        digest.update((ROOT / name).read_bytes())
    return commit, digest.hexdigest()


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--name", required=True)
    parser.add_argument("--summary", default="verification/kerml-publication-convergence/summary.json")
    parser.add_argument("--env", action="append", default=[], help="Explicit NAME=value override, recorded in the summary")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        parser.error("a command is required")
    output = ROOT / "verification/generated" / Path(args.summary).parent.name
    output.mkdir(parents=True, exist_ok=True)
    commit, digest = source_identity(Path(args.summary).as_posix())
    executable = shutil.which(command[0]) or command[0]
    overrides = dict(item.split("=", 1) for item in args.env)
    version = subprocess.run([executable, "--version"], cwd=ROOT, capture_output=True, text=True)
    start = time.monotonic()
    log = output / (args.name + ".log")
    attempt = 1
    while log.exists():
        attempt += 1
        log = output / f"{args.name}-{attempt}.log"
    with log.open("w", encoding="utf-8") as stream:
        result = subprocess.run([executable, *command[1:]], cwd=ROOT, stdout=stream, stderr=subprocess.STDOUT,
                                env={**os.environ, **overrides})
    summary = ROOT / args.summary
    summary.parent.mkdir(parents=True, exist_ok=True)
    record = {
        "name": args.name, "command": command, "exit_code": result.returncode,
        "tool_version": (version.stdout or version.stderr).strip().splitlines()[0],
        "source_commit": commit, "working_changes_sha256": digest,
        "output_sha256": hashlib.sha256(log.read_bytes()).hexdigest(),
        "duration_seconds": round(time.monotonic() - start, 2),
        "output": log.relative_to(ROOT).as_posix(),
        "environment_overrides": overrides,
    }
    # Independent checks may finish concurrently; serialize only the small summary write.
    lock = output / "summary.lock"
    while True:
        try:
            descriptor = os.open(lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
            break
        except FileExistsError:
            time.sleep(0.05)
    try:
        data = json.loads(summary.read_text()) if summary.exists() else {"format": "agentique-verification-summary/1", "commands": []}
        data["commands"].append(record)
        summary.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    finally:
        os.close(descriptor)
        lock.unlink()
    print(f"{args.name}: exit {result.returncode}; {log.relative_to(ROOT)}", flush=True)
    raise SystemExit(result.returncode)


if __name__ == "__main__":
    main()
