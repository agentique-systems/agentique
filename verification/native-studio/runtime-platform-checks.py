"""Focused checks of native in-process boundary and unchanged web bootstrap client."""
import json
import pathlib
import subprocess
import time

root = pathlib.Path(__file__).resolve().parents[2]
commands = [
    ["cargo", "fmt", "--all", "--", "--check"],
    ["cargo", "clippy", "--locked", "--offline", "-p", "agq-studio-platform", "-p", "agq-studio", "--all-targets", "--", "-D", "warnings"],
    ["cargo", "test", "--locked", "--offline", "-p", "agq-studio-platform", "-p", "agq-studio"],
    ["cargo", "doc", "--locked", "--offline", "--no-deps", "-p", "agq-studio-platform"],
]
records = []
for command in commands:
    start = time.monotonic()
    result = subprocess.run(command, cwd=root, capture_output=True, text=True, errors="replace")
    record = dict(command=command, exit_code=result.returncode,
                  elapsed_ms=round((time.monotonic()-start)*1000),
                  output=result.stdout, stderr=result.stderr)
    records.append(record)
    print(json.dumps(record), flush=True)
    (root / "verification/native-studio/runtime-platform-checks.json").write_text(
        json.dumps(records, indent=2), encoding="utf-8")
raise SystemExit(int(any(record["exit_code"] for record in records)))
