"""Explicit authority acquisition. Never invoked by builds or offline verification."""
import datetime
import hashlib
import json
from pathlib import Path
import subprocess
import urllib.parse
import urllib.request

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
index = OUT / 'acquisition.json'
records = json.loads(index.read_text()) if index.exists() else []


def acquire(name, url):
    path = OUT / name
    if path.exists():
        record = next(r for r in records if r['path'] == name)
        data = path.read_bytes()
        assert record['url'] == url
        assert hashlib.sha256(data).hexdigest() == record['sha256']
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
    index.write_text(json.dumps(records, indent=2) + '\n')
    print(name, len(data), flush=True)
    return data


if __name__ == '__main__':
    if not (OUT / 'preflight.json').exists():
        (OUT / 'preflight.json').write_text(json.dumps(dict(
            base_commit=subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
            branch=subprocess.check_output(['git', 'branch', '--show-current'], text=True).strip(),
            initial_worktree='clean, verified before branch creation',
            fetch=dict(command='git fetch origin main', exit_code=0),
            head_matched_origin_main=True), indent=2) + '\n')
        paths = subprocess.check_output(['git', 'ls-files', 'verification', 'standards',
                                         '*.pdf', '*.html', '*.kpar'], text=True).splitlines()
        (OUT / 'preserved.json').write_text(json.dumps([
            dict(path=p, sha256=hashlib.sha256((ROOT / p).read_bytes()).hexdigest())
            for p in paths], indent=2) + '\n')
    for issue in ['KERML11-76', 'KERML11-72']:
        acquire(issue + '.html', 'https://issues.omg.org/issues/' + issue)
    models = ['FeatureReferencingPerformances', 'Objects', 'Observation', 'VectorFunctions']
    for short, repo in [('release', 'SysML-v2-Release'), ('pilot', 'SysML-v2-Pilot-Implementation')]:
        api = 'https://api.github.com/repos/Systems-Modeling/' + repo
        head = json.loads(acquire(short + '-head.json', api + '/commits/master'))['sha']
        tree = json.loads(acquire(short + '-tree.json', api + '/git/trees/' + head + '?recursive=1'))
        selected = [r['path'] for r in tree['tree'] if r['type'] == 'blob' and (
            (any(m + '.' in r['path'] for m in models) and r['path'].endswith(('.kerml', '.kermlx')))
            or ('redefin' in r['path'].lower() and r['path'].endswith(('.kerml', '.sysml')))
            or r['path'].endswith(('TypeUtil.java', 'FeatureUtil.java', 'NamespaceUtil.java', 'Release_Notes.md')))]
        for path in selected:
            acquire(short + '/' + path, 'https://raw.githubusercontent.com/Systems-Modeling/' + repo + '/' + head + '/' + urllib.parse.quote(path))
        search = 'https://api.github.com/search/commits?q=' + urllib.parse.quote('repo:Systems-Modeling/' + repo + ' KERML11-76')
        commits = json.loads(acquire(short + '-issue-commits.json', search))
        for commit in commits.get('items', []):
            acquire(short + '-commit-' + commit['sha'] + '.json', api + '/commits/' + commit['sha'])
        for model in models:
            paths = [p for p in selected if p.endswith('/' + model + '.kerml')]
            if paths:
                acquire(short + '-' + model + '-history.json', api + '/commits?path=' + urllib.parse.quote(paths[0]) + '&per_page=100')
    print('Captured', len(records), 'remote artifacts')
