import hashlib
import json
from pathlib import Path
here = Path(__file__).resolve().parent
root = here.parents[1]
records = json.loads((here/'preserved.json').read_bytes())
failures = [r['path'] for r in records if hashlib.sha256((root/r['path']).read_bytes()).hexdigest() != r['sha256']]
print(json.dumps(dict(checked=len(records), failures=failures)))
raise SystemExit(bool(failures))
