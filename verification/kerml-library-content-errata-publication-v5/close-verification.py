"""Final read-only checks and exact changed implementation/review identities."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
result = subprocess.run(['python', str(OUT / 'preservation.py')], check=True,
                        capture_output=True, text=True)
print(result.stdout, end='')
preservation = json.loads(result.stdout)
branch = subprocess.check_output(['git', 'branch', '--show-current'], text=True).strip()
assert branch == 'semantics/kerml-library-content-errata-publication-v5'
diff = subprocess.run(['git', 'diff', '--check'], capture_output=True, text=True)
print('git diff --check:', diff.returncode)
assert diff.returncode == 0, diff.stdout + diff.stderr
checked_links = []
documents = [OUT / 'README.md', ROOT / 'docs/adr/0015-operational-standard-library-corrections.md',
             ROOT / 'docs/kerml-feature-chain-end-authority-conflict.md',
             ROOT / 'docs/kerml-standard-library-foundation-review.md']
for path in documents:
    text = path.read_text(encoding='utf8')
    if path.name == 'kerml-standard-library-foundation-review.md':
        text = text.split('## V5', 1)[1]
    for target in re.findall(r'\]\(([^)]+)\)', text):
        if target.startswith(('http:', 'https:', '#')):
            continue
        resolved = (path.parent / target.split('#', 1)[0]).resolve()
        # This runner's own result is written immediately after it exits.
        if resolved == OUT / 'closing-checks/result.json':
            continue
        assert resolved.is_file(), (path, target)
        checked_links.append(dict(source=str(path.relative_to(ROOT)), target=target))
paths = set(subprocess.check_output(['git', 'diff', '--name-only'], text=True).splitlines())
paths.update(subprocess.check_output(['git', 'ls-files', '--others', '--exclude-standard',
                                     'crates', 'docs', 'standards'], text=True).splitlines())
paths.update(str(p.relative_to(ROOT)).replace('\\', '/') for p in OUT.glob('*.py'))
paths.add(str((OUT / 'README.md').relative_to(ROOT)).replace('\\', '/'))
files = [dict(path=p, sha256=hashlib.sha256((ROOT / p).read_bytes()).hexdigest(),
              bytes=(ROOT / p).stat().st_size) for p in sorted(paths)]
report = dict(branch=branch, base_commit=subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
    preservation=preservation, git_diff_check_exit_code=diff.returncode,
    checked_review_links=checked_links, implementation_and_review_files=files,
    accepted_semantic_publication=False, stop='KLCV5-F-001 / KERML11-68')
with (OUT / 'closing-verification.json').open('x', encoding='utf8') as stream:
    json.dump(report, stream, indent=2)
    stream.write('\n')
print(json.dumps(dict(branch=branch, checked_review_links=len(checked_links),
                     implementation_and_review_files=len(files), accepted_semantic_publication=False)))
