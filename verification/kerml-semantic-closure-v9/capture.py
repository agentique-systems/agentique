"""Append-only command capture for v9. No command success implies publication."""
import datetime
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time

OUT = Path(__file__).resolve().parent
label, *command = sys.argv[1:]
directory = OUT / label
directory.mkdir()
root = OUT.parents[1]
paths = subprocess.check_output(['git', 'ls-files', '--cached', '--others', '--exclude-standard', 'crates', 'standards', 'Cargo.toml', 'Cargo.lock'], text=True).splitlines()
(directory / 'input-hashes.json').write_text(json.dumps([
    dict(path=p, sha256=hashlib.sha256((root/p).read_bytes()).hexdigest())
    for p in paths if (root/p).is_file()
], indent=2)+'\n')
started = datetime.datetime.now(datetime.timezone.utc).isoformat()
tick = time.monotonic()
with (directory / 'output.txt').open('wb') as log:
    result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
record = dict(command=command, started=started, duration_seconds=time.monotonic()-tick,
              exit_code=result.returncode, output='output.txt')
(directory / 'result.json').write_text(json.dumps(record, indent=2)+'\n')
print(json.dumps(record), flush=True)
sys.exit(result.returncode)
