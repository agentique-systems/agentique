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
executable = pathlib.Path(command[0])
if not executable.is_absolute():
    executable = root / executable
def executable_digest():
    if executable.suffix.lower() == ".exe" and executable.is_file():
        with executable.open("rb") as stream:
            return hashlib.file_digest(stream, "sha256").hexdigest()
    return None

executable_before = executable_digest()
started = time.monotonic()
utc = datetime.datetime.now(datetime.timezone.utc).isoformat()
return_code = None
launch_error = None
with output_path.open("w", encoding="utf-8") as output:
    try:
        process = subprocess.run(command, cwd=root, stdout=output, stderr=subprocess.STDOUT,
                                 shell=command[0] == "npm")
        return_code = process.returncode
    except OSError as error:
        launch_error = f"{type(error).__name__}: {error}"
        output.write(f"Process did not start: {launch_error}\n")
output = output_path.read_bytes()
executable_after = executable_digest()
record = {
    "command": command,
    "cwd": str(root),
    "started_utc": utc,
    "elapsed_seconds": round(time.monotonic() - started, 3),
    "exit_code": return_code,
    "output": str(output_path.relative_to(root)).replace("\\", "/"),
    "output_sha256": hashlib.sha256(output).hexdigest(),
    "build_environment": {key: os.environ[key] for key in (
        "CARGO_PROFILE_DEV_DEBUG", "CARGO_PROFILE_TEST_DEBUG", "CARGO_INCREMENTAL", "CARGO_BUILD_JOBS"
    ) if key in os.environ},
}
if launch_error is not None:
    record["launch_error"] = launch_error
if executable_before is not None:
    record["executable"] = {
        "path": str(executable.resolve()),
        "sha256_before": executable_before,
        "sha256_after": executable_after,
        "unchanged_during_run": executable_before == executable_after,
    }
(directory / f"{name}.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
print(json.dumps(record))
print(output.decode("utf-8", errors="replace")[-4500:])
raise SystemExit(1 if launch_error else return_code or (2 if executable_before != executable_after else 0))
