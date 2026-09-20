"""Explicit, append-only authority acquisition. Never a build dependency."""
import datetime
import hashlib
import json
from pathlib import Path
import subprocess
import urllib.parse
import urllib.request

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[1]
PILOT = 'Systems-Modeling/SysML-v2-Pilot-Implementation'

def acquire(name, url):
    index = OUT / 'acquisition.json'
    records = json.loads(index.read_text()) if index.exists() else []
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
    index.write_text(json.dumps(records, indent=2)+'\n', encoding='utf8')
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
    for issue in [205,206,207,145,182,210]:
        acquire(f'issues/KERML11-{issue}.html',f'https://issues.omg.org/issues/KERML11-{issue}')
    api = 'https://api.github.com/repos/' + PILOT
    commit = json.loads(acquire('pilot-head.json',api+'/commits/master'))
    head = commit['sha']
    acquire('pilot-root-tree.json',api+'/git/trees/'+commit['commit']['tree']['sha']+'?recursive=1')
    tree = json.loads(acquire('pilot-tree.json',api+'/git/trees/'+head+'?recursive=1'))
    for item in tree['tree']:
        path = item['path']
        if (path.endswith(('ImplicitGeneralizationMap.java','FeatureAdapter.java','StepAdapter.java','FunctionAdapter.java','ExpressionAdapter.java','BehaviorAdapter.java','TypeAdapter.java','KerMLValidator.xtend'))
            or ('test' in path.lower() and path.endswith('.kerml.xt'))
            or ('test' in path.lower() and path.endswith(('.kerml','.xtend','.kerml.xt')) and any(s in path.lower() for s in ['feature','step','constraint','specialization','portion','occurrence','performance']))):
            acquire('pilot/'+path,'https://raw.githubusercontent.com/'+PILOT+'/'+head+'/'+urllib.parse.quote(path))
