"""Close the evidence record without changing tracked historical evidence."""
import datetime
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT).decode('utf-8')


def record(path, base):
    return dict(path=path.relative_to(base).as_posix(), bytes=path.stat().st_size,
                sha256=hashlib.sha256(path.read_bytes()).hexdigest())


names = git('ls-files', '--cached', '--others', '--exclude-standard').splitlines()
inputs = sorted({name for name in names if name.startswith(('crates/', 'tools/metamodel-gen/', 'docs/'))
                 or name in ['.gitattributes', 'Cargo.toml', 'Cargo.lock', 'package.json', 'package-lock.json',
                             'standards/v2-coverage.json', 'standards/kerml-standard-bindings.json',
                             'standards/kerml-1.0-operational-errata.json', 'standards/kerml-1.0-operational-errata-v2.json']})
state = dict(format='agentique-source-verification-state/1',
             captured_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
             branch=git('branch', '--show-current').strip(), head=git('rev-parse', 'HEAD').strip(),
             status=git('status', '--short').splitlines(), source_files=[record(ROOT/name,ROOT) for name in inputs])
with (HERE/'source-state.json').open('x',encoding='utf-8') as stream:
    json.dump(state,stream,indent=2)
    stream.write('\n')
files = sorted(p for p in HERE.rglob('*') if p.is_file()
               and p.name != 'evidence-inventory.json' and 'closure-1' not in p.relative_to(HERE).parts)
inventory = dict(format='agentique-v4-evidence-inventory/1',
                 exclusions=['This inventory itself', 'Active closure-1 command log and results'],
                 files=[record(p,HERE) for p in files])
with (HERE/'evidence-inventory.json').open('x',encoding='utf-8') as stream:
    json.dump(inventory,stream,indent=2)
    stream.write('\n')
print(json.dumps(dict(source_files=len(inputs), evidence_files=len(files), branch=state['branch'], head=state['head'])))
