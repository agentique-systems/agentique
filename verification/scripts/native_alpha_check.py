"""Execute one alpha gate, retaining actual output, timing and exit status."""
import datetime
import hashlib
import json
import os
import pathlib
import subprocess
import sys
import time

sys.stdout.reconfigure(encoding="utf-8", errors="replace")
root = pathlib.Path(__file__).resolve().parents[2]
name, *command = sys.argv[1:]
if command and command[0] == "--":
    command.pop(0)
directory = root / "verification/native-studio-alpha/checks"
directory.mkdir(parents=True, exist_ok=True)
output_path = directory / f"{name}.txt"
started = time.monotonic()
utc = datetime.datetime.now(datetime.timezone.utc).isoformat()
with output_path.open("w", encoding="utf-8") as output:
    process = subprocess.run(command, cwd=root, stdout=output, stderr=subprocess.STDOUT,
                             shell=command[0] == "npm")
output = output_path.read_bytes()
record = {
    "command": command,
    "cwd": str(root),
    "started_utc": utc,
    "elapsed_seconds": round(time.monotonic() - started, 3),
    "exit_code": process.returncode,
    "output": str(output_path.relative_to(root)).replace("\\", "/"),
    "output_sha256": hashlib.sha256(output).hexdigest(),
    "build_environment": {key: os.environ[key] for key in (
        "CARGO_PROFILE_DEV_DEBUG", "CARGO_PROFILE_TEST_DEBUG", "CARGO_INCREMENTAL", "CARGO_BUILD_JOBS"
    ) if key in os.environ},
}
(directory / f"{name}.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
print(json.dumps(record))
print(output.decode("utf-8", errors="replace")[-4500:])
raise SystemExit(process.returncode)
