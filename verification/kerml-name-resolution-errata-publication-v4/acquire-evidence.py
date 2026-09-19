"""Explicit maintenance acquisition, never a build or verification dependency."""
import datetime
import hashlib
import json
from pathlib import Path
import subprocess
import urllib.parse
import urllib.request

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
INDEX = OUT / 'acquisition.json'
records = json.loads(INDEX.read_text()) if INDEX.exists() else []


def acquire(name, url):
    path = OUT / name
    if path.exists():
        record = next(r for r in records if r['path'] == name)
        data = path.read_bytes()
        assert hashlib.sha256(data).hexdigest() == record['sha256']
        assert record['url'] == url
        return data
    request = urllib.request.Request(url, headers={'User-Agent': 'Agentique-authority-review'})
    with urllib.request.urlopen(request, timeout=45) as response:
        data = response.read()
        record = dict(path=name, url=url, resolved_url=response.url,
                      retrieved_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
                      sha256=hashlib.sha256(data).hexdigest(), bytes=len(data))
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    records.append(record)
    INDEX.write_text(json.dumps(records, indent=2) + '\n')
    print(name, len(data), flush=True)
    return data


if __name__ == '__main__':
    preflight = OUT / 'preflight.json'
    if not preflight.exists():
        preflight.write_text(json.dumps(dict(
            base_commit=subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
            branch=subprocess.check_output(['git', 'branch', '--show-current'], text=True).strip(),
            initial_worktree='clean; checked before creating branch and evidence',
            fetch=dict(command='git fetch origin main', exit_code=0),
            captured_at=datetime.datetime.now(datetime.timezone.utc).isoformat()), indent=2) + '\n')
        protected = subprocess.check_output(['git', 'ls-files', 'verification', 'standards/normative',
                                             '*.pdf', '*.html', '*.kpar'], text=True).splitlines()
        (OUT / 'preserved.json').write_text(json.dumps([
            dict(path=p, sha256=hashlib.sha256((ROOT/p).read_bytes()).hexdigest())
            for p in protected], indent=2) + '\n')
    acquire('KERML11-140.html', 'https://issues.omg.org/issues/KERML11-140')
    for short, repo in [('release', 'SysML-v2-Release'), ('pilot', 'SysML-v2-Pilot-Implementation')]:
        api = 'https://api.github.com/repos/Systems-Modeling/' + repo
        head = json.loads(acquire(short + '-head.json', api + '/commits/master'))['sha']
        tree = json.loads(acquire(short + '-tree.json', api + '/git/trees/' + head + '?recursive=1'))
        acquire(short + '-releases.json', api + '/releases?per_page=30')
        paths = [r['path'] for r in tree['tree'] if r['type'] == 'blob']
        selected = [p for p in paths if (
            ('FeatureReferencingPerformances' in p or 'Observation' in p or 'Spatial' in p)
            and p.endswith(('.kerml', '.kermlx')))
            or ('scoping' in p.lower() and p.endswith(('.java', '.xtend')))
            or ('redefin' in p.lower() and p.endswith(('.java', '.xtend', '.kerml', '.sysml')))
            or p.endswith(('ElementUtil.java', 'TypeUtil.java', 'FeatureUtil.java', 'NamespaceUtil.java',
                           '1-Kernel_Modeling_Language.pdf', 'Release_Notes.md', 'ReleaseNotes.md'))]
        for p in selected:
            acquire(short + '/' + p, 'https://raw.githubusercontent.com/Systems-Modeling/' + repo + '/'
                    + head + '/' + urllib.parse.quote(p))
    witness = ROOT / 'crates/kerml-semantics/tests/imports.rs'
    if not (OUT / 'published-witness-imports.rs').exists():
        (OUT / 'published-witness-imports.rs').write_bytes(witness.read_bytes())
    print('Captured', len(records), 'remote artifacts')
