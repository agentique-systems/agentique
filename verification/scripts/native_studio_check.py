"""Run named checks with durable exit summaries and ignored full command outputs."""
import argparse
import datetime
import json
import pathlib
import subprocess
import sys
import time

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

parser = argparse.ArgumentParser()
parser.add_argument("name")
parser.add_argument("command", nargs=argparse.REMAINDER)
args = parser.parse_args()
root = pathlib.Path(__file__).resolve().parents[2]
logs = root / "verification/generated/native-studio"
logs.mkdir(parents=True, exist_ok=True)
started = time.monotonic()
command = args.command
if command and command[0] == "--":
    command = command[1:]
with (logs / f"{args.name}.txt").open("w", encoding="utf-8") as output:
    process = subprocess.run(command, cwd=root, stdout=output, stderr=subprocess.STDOUT,
                             shell=(command[0] == "npm"))
entry = {"command": command, "exit_code": process.returncode,
         "elapsed_seconds": round(time.monotonic() - started, 3),
         "utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
         "output": f"verification/generated/native-studio/{args.name}.txt"}
summary = root / "verification/summaries/native-studio"
summary.mkdir(parents=True, exist_ok=True)
(summary / f"{args.name}.json").write_text(json.dumps(entry, indent=2) + "\n", encoding="utf-8")
print(json.dumps(entry))
print((logs / f"{args.name}.txt").read_text(encoding="utf-8")[-3500:])
raise SystemExit(process.returncode)
