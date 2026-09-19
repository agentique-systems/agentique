"""Append-only command evidence, including real output and exit status."""
import datetime
import json
import os
from pathlib import Path
import subprocess
import sys
import time

out = Path(__file__).resolve().parent / sys.argv[1]
out.mkdir()
command = sys.argv[2:]
started = datetime.datetime.now(datetime.timezone.utc).isoformat()
before = time.monotonic()
with (out / 'output.txt').open('wb') as log:
    result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT,
                            env={**os.environ, 'CARGO_NET_OFFLINE': 'true', 'RUSTDOCFLAGS': '-D warnings'})
record = dict(command=command, started_at=started, duration_seconds=time.monotonic() - before,
              exit_code=result.returncode, output='output.txt')
(out / 'result.json').write_text(json.dumps(record, indent=2) + '\n')
print(json.dumps(record))
sys.exit(result.returncode)
