# Current Gen2 coverage reconciliation

Reviewed source: `3af2afd2e7daebe2197959d1d996ed611ae2c488`.
This documentation-only change updates the root README's Gen2 layer list and
four current partial capability records in `standards/v2-coverage.json`, plus
the latest completed Systems scope boundary. The four previous records are
preserved as exact JSON values in an explicitly historical snapshot. Existing
historical failures, all other coverage values and Gen1 README content remain
unchanged.

The evidence establishes accepted Systems publication, adopted ADR 0026 and
passed language integration, plus the separately passed workspace self-model
and five-Validated-revision/100-document/four-reader gates. Full workspace
runtime acceptance and ADR 0024 adoption remain pending. No profile, receipt,
accepted publication, producer, semantic code or conformance obligation changes.

No standalone schema or consumer for `agentique-generation-2-coverage/1` was
found in the repository's verification/tool scripts. The check below validates
its existing format and current capability field contract, checks local evidence
targets, and proves exact preservation of unrelated JSON values. It does not
replace the repository's separate standards-artifact integrity gate. No build,
accepted-cache load, producer or runtime test was run for this change.

## Commands and results

All commands ran in the isolated enumeration worktree.

| Check | Exit | Actual output |
| --- | --- | --- |
| Initial inline Python format/preservation/link check | 1 | `Traceback (most recent call last):` / `File "<stdin>", line 32, in <module>` / `AssertionError` |
| Corrected inline Python check | 0 | Five `PASS` lines reproduced below. |
| Final check below, also comparing all unchanged capability records | 0 | The same five `PASS` lines. |
| `git diff --check` | 0 | Empty output. |

The first link check required every README target to be a file with
`Path(x).is_file()`. The existing `verification/screenshots` link correctly
targets a directory. The corrected README check uses `exists()`; all current
capability evidence paths still must be files. No source or link was changed
to resolve this verification-script assumption. The failed check remains
recorded above rather than counted as a pass.

The final exact PowerShell command was:

```powershell
@'
import copy,json,re,subprocess
from pathlib import Path
base='3af2afd2e7daebe2197959d1d996ed611ae2c488'
p=Path('standards/v2-coverage.json')
d=json.loads(p.read_text(encoding='utf-8'))
prior=json.loads(subprocess.check_output(['git','show',base+':standards/v2-coverage.json'],text=True,encoding='utf-8'))
ids={'semantic-queries','textual-syntax','libraries','sysml'}
snapshot=d['historical_capability_snapshot']
assert snapshot['historical'] is True and snapshot['recorded_from_source']==base
assert snapshot['capabilities']==[x for x in prior['capabilities'] if x['id'] in ids]
assert [x for x in d['capabilities'] if x['id'] not in ids]==[x for x in prior['capabilities'] if x['id'] not in ids]
restored=copy.deepcopy(d)
del restored['historical_capability_snapshot']
restored['capabilities']=copy.deepcopy(prior['capabilities'])
restored['sysml_systems_library_publication']['latest_completed_scope']['boundary']=prior['sysml_systems_library_publication']['latest_completed_scope']['boundary']
assert restored==prior, 'unrelated coverage content changed'
assert d['format']=='agentique-generation-2-coverage/1' and d['generation']==2
assert len({x['id'] for x in d['capabilities']})==len(d['capabilities'])
current=[x for x in d['capabilities'] if x['id'] in ids]
assert len(current)==4
for x in current:
 assert set(x)=={'id','status','boundary','evidence','gaps'}
 assert x['status']=='partial' and x['boundary'] and x['gaps']
 assert all(isinstance(y,str) and y for y in x['evidence']+x['gaps'])
 assert all(Path(y).is_file() for y in x['evidence'])
readme=Path('README.md').read_text(encoding='utf-8')
old_readme=subprocess.check_output(['git','show',base+':README.md'],text=True,encoding='utf-8')
assert readme.split('**Generation 2**')[0]==old_readme.split('**Generation 2**')[0]
assert readme.split('Prerequisites:',1)[1]==old_readme.split('Prerequisites:',1)[1]
links=re.findall(r'\]\(([^)]+)\)',readme)
local=[x.split('#')[0] for x in links if not re.match(r'[a-z]+://|#',x)]
assert all(Path(x).exists() for x in local)
print('PASS: four current partial capability records satisfy the retained format and field contract')
print('PASS: four historical capability records equal the baseline JSON values exactly')
print('PASS: all unrelated coverage values and Gen1 README content are unchanged')
print(f'PASS: {sum(len(x["evidence"]) for x in current)} current capability evidence paths and {len(local)} README local links exist')
print('PASS: no acceptance status promoted; full workspace and ADR 0024 remain pending')
'@ | python -
```

Output:

```text
PASS: four current partial capability records satisfy the retained format and field contract
PASS: four historical capability records equal the baseline JSON values exactly
PASS: all unrelated coverage values and Gen1 README content are unchanged
PASS: 30 current capability evidence paths and 14 README local links exist
PASS: no acceptance status promoted; full workspace and ADR 0024 remain pending
```
