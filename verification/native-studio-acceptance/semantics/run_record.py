"""Run one actual command and retain unmodified output, exit code, and timing."""
import argparse
import datetime
import hashlib
import json
import pathlib
import subprocess
import sys
import time

try:
    import psutil
except ImportError:
    psutil = None

parser = argparse.ArgumentParser()
parser.add_argument("--cwd", required=True)
parser.add_argument("--name", required=True)
parser.add_argument("command", nargs=argparse.REMAINDER)
args = parser.parse_args()
command = args.command[1:] if args.command[:1] == ["--"] else args.command
destination = pathlib.Path(__file__).resolve().parent
started = time.monotonic()
record = {"command": command, "cwd": args.cwd,
          "started_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
          "python": sys.version,
          "source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=args.cwd, text=True).strip(),
          "working_diff_sha256": hashlib.sha256(subprocess.check_output(["git", "diff", "HEAD"], cwd=args.cwd)).hexdigest()}
with (destination / (args.name + ".log")).open("wb") as output:
    process = subprocess.Popen(command, cwd=args.cwd, stdout=output, stderr=subprocess.STDOUT)
    peak_rss = peak_private = 0
    while process.poll() is None:
        if psutil:
            try:
                memory = psutil.Process(process.pid).memory_info()
                peak_rss = max(peak_rss, memory.rss)
                peak_private = max(peak_private, getattr(memory, "private", 0))
            except psutil.Error:
                pass
        time.sleep(1)
record.update(exit_code=process.returncode, elapsed_seconds=round(time.monotonic()-started, 3),
              peak_rss_bytes=peak_rss if psutil else None,
              peak_private_bytes=peak_private if psutil else None,
              memory_scope="direct child process sampled at 1 second; excludes descendant compilers")
log = destination / (args.name + ".log")
with log.open("rb") as stream:
    record["output_sha256"] = hashlib.file_digest(stream, "sha256").hexdigest()
record["raw_output"] = str(log)
(destination / (args.name + ".json")).write_text(json.dumps(record, indent=2), encoding="utf-8")
print(json.dumps(record), flush=True)
raise SystemExit(process.returncode)
