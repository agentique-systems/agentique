"""Check new local documentation links, frozen prefixes, and evidence syntax."""
import ast
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote, urlsplit

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
BASE = '59abd9801d59de025bf7e346464a32848047fd80'
paths = ['verification/kerml-semantic-closure-v10/README.md',
         'docs/adr/0020-operational-owned-cross-feature-correction.md',
         'docs/kerml-standard-library-foundation-review.md',
         'docs/operational-kerml-profile.md']
rows = []
for relative in paths:
    path = ROOT / relative
    raw = path.read_bytes()
    text = raw.decode('utf8')
    prefix_unchanged = None
    if relative in paths[2:]:
        original = subprocess.check_output(['git', 'cat-file', 'blob', f'{BASE}:{relative}'], cwd=ROOT)
        assert raw.startswith(original), (relative, 'historical documentation prefix changed')
        prefix_unchanged = True
        text = raw[len(original):].decode('utf8')
    checked = []
    for target in re.findall(r'\[[^\]]*\]\(([^)]+)\)', text):
        target = target.strip('<>')
        parsed = urlsplit(target)
        if parsed.scheme or not parsed.path:
            continue
        resolved = (path.parent / unquote(parsed.path)).resolve()
        assert resolved.exists(), (relative, target)
        checked.append(target)
    rows.append(dict(path=relative, sha256=hashlib.sha256(raw).hexdigest(),
                     historical_prefix_unchanged=prefix_unchanged, checked_local_links=checked))
scripts = sorted(OUT.glob('*.py'))
for path in scripts:
    ast.parse(path.read_text(encoding='utf8'), filename=str(path))
report = dict(format='agentique-v10-document-verification/1', documents=rows,
              python_scripts_parsed=[p.name for p in scripts], missing_local_links=[],
              evidence_syntax_valid=True)
encoded = json.dumps(report, indent=2) + '\n'
path = OUT / 'document-check.json'
if '--check' in sys.argv:
    assert path.read_text(encoding='utf8') == encoded
else:
    path.write_text(encoded, encoding='utf8', newline='\n')
print(f'Checked {sum(len(r["checked_local_links"]) for r in rows)} new local documentation links, both unchanged historical prefixes, and {len(scripts)} Python scripts.')
