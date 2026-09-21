"""Explicit authority acquisition; never invoked by an offline build or test."""
import datetime
import hashlib
import json
from pathlib import Path
import subprocess
import urllib.request

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]


def acquire(name, url):
    index = OUT / 'acquisition.json'
    rows = json.loads(index.read_text()) if index.exists() else []
    path = OUT / name
    if path.exists():
        row = next(r for r in rows if r['path'] == name)
        assert row['url'] == url
        assert hashlib.sha256(path.read_bytes()).hexdigest() == row['sha256']
        return path.read_bytes()
    request = urllib.request.Request(url, headers={'User-Agent': 'Agentique-authority-review'})
    with urllib.request.urlopen(request, timeout=45) as response:
        data = response.read()
        row = dict(path=name, url=url, resolved_url=response.url,
                   retrieved_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
                   sha256=hashlib.sha256(data).hexdigest(), bytes=len(data))
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    rows.append(row)
    index.write_text(json.dumps(rows, indent=2)+'\n', encoding='utf8', newline='\n')
    print(name, len(data), flush=True)
    return data


if __name__ == '__main__':
    if not (OUT / 'preserved.json').exists():
        paths = subprocess.check_output(['git', 'ls-files', 'verification', 'standards', '*.pdf', '*.html', '*.kpar'], text=True).splitlines()
        rows = [dict(path=p, sha256=hashlib.sha256((ROOT/p).read_bytes()).hexdigest()) for p in paths]
        (OUT / 'preserved.json').write_text(json.dumps(rows, indent=2)+'\n', encoding='utf8', newline='\n')
    acquire('issues/KerML-all.html', 'https://issues.omg.org/issues/spec/KerML?view=ALL')
    acquire('issues/KERML11-145.html', 'https://issues.omg.org/issues/KERML11-145')
    acquire('issues/KERML11-8.html', 'https://issues.omg.org/issues/KERML11-8')
    acquire('issues/KERML11-1.html', 'https://issues.omg.org/issues/KERML11-1')
    repo = 'Systems-Modeling/SysML-v2-Pilot-Implementation'
    commit = '553cf8205c19241c9127ab264f8372f5b58d3895'
    change = json.loads(acquire('resolution/commit.json', f'https://api.github.com/repos/{repo}/commits/{commit}'))
    acquire('resolution/commit.patch', f'https://github.com/{repo}/commit/{commit}.patch')
    acquire('resolution/git-commit.json', f'https://api.github.com/repos/{repo}/git/commits/{commit}')
    parent = change['parents'][0]['sha']
    for row in change['files']:
        for label, revision in [('before', parent), ('after', commit)]:
            if row['status'] == 'added' and label == 'before':
                continue
            path = row['filename']
            acquire(f'resolution/{label}/{path}', f'https://raw.githubusercontent.com/{repo}/{revision}/{path}')
    head = json.loads(acquire('pilot-head.json', f'https://api.github.com/repos/{repo}/commits/master'))['sha']
    for name in ['FeatureAdapter', 'ExpressionAdapter', 'FunctionAdapter', 'TypeAdapter', 'IndexExpressionAdapter', 'SelectExpressionAdapter', 'FeatureReferenceExpressionAdapter']:
        path = f'org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/{name}.java'
        acquire(f'pilot/{name}.java', f'https://raw.githubusercontent.com/{repo}/{head}/{path}')
    for name, path in [
        ('KerMLValidator.xtend', 'org.omg.kerml.xtext/src/org/omg/kerml/xtext/validation/KerMLValidator.xtend'),
        ('FeatureChainExpressionAdapter.java', 'org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/FeatureChainExpressionAdapter.java'),
        ('FeatureUtil.java', 'org.omg.sysml.logic/src/main/java/org/omg/sysml/util/FeatureUtil.java'),
    ]:
        acquire(f'pilot/{name}', f'https://raw.githubusercontent.com/{repo}/{head}/{path}')
    release = json.loads(acquire('release-head.json', 'https://api.github.com/repos/Systems-Modeling/SysML-v2-Release/commits/master'))['sha']
    acquire('release-tree.json', f'https://api.github.com/repos/Systems-Modeling/SysML-v2-Release/git/trees/{release}?recursive=1')
    acquire('preliminary/KerML.pdf', f'https://raw.githubusercontent.com/Systems-Modeling/SysML-v2-Release/{release}/doc/1-Kernel_Modeling_Language.pdf')
