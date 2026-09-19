"""Read every tracked v4 evidence artifact without rewriting historical evidence."""
import collections
import hashlib
import json
from pathlib import Path
import subprocess
import xml.etree.ElementTree as ET

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
paths = subprocess.check_output(['git', 'ls-files', 'verification/kerml-name-resolution-errata-publication-v4'], text=True).splitlines()
rows = []
commands = []
for name in paths:
    path = ROOT / name
    data = path.read_bytes()
    row = dict(path=name, bytes=len(data), sha256=hashlib.sha256(data).hexdigest())
    if path.suffix == '.json':
        value = json.loads(data)
        row['shape'] = list(value) if isinstance(value, dict) else dict(length=len(value))
        if path.name == 'results.json':
            commands.extend(dict(evidence=name, **r) for r in value)
    elif path.suffix == '.kermlx':
        root = ET.fromstring(data)
        row['xml_elements'] = sum(1 for _ in root.iter())
    elif path.suffix not in ['.pdf', '.png', '.zip']:
        try:
            text = data.decode('utf-8')
            row['lines'] = len(text.splitlines())
            row['failure_lines'] = [s for s in text.splitlines() if any(t in s.lower() for t in ['error:', 'failed', 'panic', 'incomplete', 'blocked'])][:30]
        except UnicodeDecodeError:
            row['binary'] = True
    rows.append(row)
report = dict(files=rows, commands=commands)
(OUT / 'historical-evidence-inspection.json').write_text(json.dumps(report, indent=2) + '\n')
print('Read', len(rows), 'v4 artifacts;', sum(r['bytes'] for r in rows), 'bytes')
print('Command outcomes:', collections.Counter(r['exitCode'] for r in commands))
for r in commands:
    if r['exitCode'] != 0:
        print(r['evidence'], r['command'], r['exitCode'])
