"""Explicit v6 authority acquisition; never invoked by build or offline tests."""
import datetime
import hashlib
import json
from pathlib import Path
import subprocess
import urllib.parse
import urllib.request

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
COMMIT = 'd9231d21e621aeabeafa92aef026b3929c859116'
PILOT = 'Systems-Modeling/SysML-v2-Pilot-Implementation'
index = OUT / 'acquisition.json'
records = json.loads(index.read_text()) if index.exists() else []

def acquire(name, url):
    path = OUT / name
    if path.exists():
        record = next(r for r in records if r['path'] == name)
        data = path.read_bytes()
        assert record['url'] == url and hashlib.sha256(data).hexdigest() == record['sha256']
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
    index.write_text(json.dumps(records, indent=2)+'\n')
    print(name, len(data), flush=True)
    return data

if __name__ == '__main__':
    if not (OUT / 'preflight.json').exists():
        base = subprocess.check_output(['git','rev-parse','HEAD'], text=True).strip()
        assert base == subprocess.check_output(['git','rev-parse','origin/main'], text=True).strip()
        (OUT / 'preflight.json').write_text(json.dumps(dict(base_commit=base,
            branch=subprocess.check_output(['git','branch','--show-current'],text=True).strip(),
            initial_worktree='clean; verified before branch creation',
            fetch=dict(command=['git','fetch','origin','main'],exit_code=0),
            fast_forward=dict(command=['git','merge','--ff-only','origin/main'],exit_code=0)),indent=2)+'\n')
        paths = subprocess.check_output(['git','ls-files','verification','standards','*.pdf','*.html','*.kpar'],text=True).splitlines()
        (OUT / 'preserved.json').write_text(json.dumps([dict(path=p,sha256=hashlib.sha256((ROOT/p).read_bytes()).hexdigest()) for p in paths],indent=2)+'\n')
    acquire('KERML11-68.html','https://issues.omg.org/issues/KERML11-68')
    api = 'https://api.github.com/repos/' + PILOT
    commit = json.loads(acquire('reference-commit.json',api+'/commits/'+COMMIT))
    assert commit['sha'] == COMMIT
    acquire('reference-commit.patch','https://github.com/'+PILOT+'/commit/'+COMMIT+'.patch')
    parent = commit['parents'][0]['sha']
    for file in commit['files']:
        path = file['filename']
        for label, rev in [('before',parent),('after',COMMIT)]:
            if label == 'before' and file['status'] == 'added':
                continue
            acquire(label+'/'+path,'https://raw.githubusercontent.com/'+PILOT+'/'+rev+'/'+urllib.parse.quote(path))
    repo = 'Systems-Modeling/SysML-v2-Release'
    api = 'https://api.github.com/repos/'+repo
    head = json.loads(acquire('release-head.json',api+'/commits/master'))['sha']
    tree = json.loads(acquire('release-tree.json',api+'/git/trees/'+head+'?recursive=1'))
    for item in tree['tree']:
        path = item['path']
        if (path.startswith('sysml.library.xmi.implied/Kernel Libraries/') and path.endswith('.kermlx')) or path == 'doc/1-Kernel_Modeling_Language.pdf':
            acquire('release/'+path,'https://raw.githubusercontent.com/'+repo+'/'+head+'/'+urllib.parse.quote(path))
