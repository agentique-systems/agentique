"""Run one frontend gate and record actual wall time and process peak working set.

Requires psutil in the verification environment; it is not a build dependency.
Raw output and measurements belong under ignored verification/generated.
"""
import json
import hashlib
import pathlib
import subprocess
import sys
import time

import psutil

output = pathlib.Path(sys.argv[1])
command = sys.argv[2:]
output.parent.mkdir(parents=True, exist_ok=True)
started = time.perf_counter()
source_commit = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
executable_digest = None
if pathlib.Path(command[0]).is_file():
    with pathlib.Path(command[0]).open("rb") as executable:
        executable_digest = hashlib.file_digest(executable, "sha256").hexdigest()
peak = 0
sampled_tree_peak = 0
with output.with_suffix(".log").open("w", encoding="utf-8") as log:
    process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
    while process.poll() is None:
        try:
            children = [psutil.Process(process.pid)] + psutil.Process(process.pid).children(recursive=True)
            resident = 0
            for child in children:
                try:
                    memory = child.memory_info()
                    peak = max(peak, getattr(memory, "peak_wset", memory.rss))
                    resident += memory.rss
                except psutil.Error:
                    pass
            sampled_tree_peak = max(sampled_tree_peak, resident)
        except psutil.Error:
            pass
        time.sleep(0.25)
result = {
    "source_commit_at_start": source_commit,
    "executable_sha256": executable_digest,
    "command": command,
    "exit_code": process.returncode,
    "wall_seconds": time.perf_counter() - started,
    "maximum_process_peak_working_set_bytes": peak,
    "sampled_process_tree_peak_resident_bytes": sampled_tree_peak,
    "sample_interval_seconds": 0.25,
}
output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
lines = output.with_suffix(".log").read_text(encoding="utf-8").splitlines()
print("\n".join(line[:2000] for line in lines[-100:]))
print(json.dumps(result))
sys.exit(process.returncode)
