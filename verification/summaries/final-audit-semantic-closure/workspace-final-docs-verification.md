# Final workspace documentation verification

Base: `cf12614d292d398348357103f3e6d42dc39ca7dd`.
Nine assigned current documents now record all twelve passed accepted-cache
workspace tests, bounded in-memory Phase 1 adoption of ADR 0024, and closure of
the architectural retrospective. Historical reviews, the eleven-test snapshot
and failed attempts remain intact. ADR 0026 stays adopted and ADR 0027 stays
proposed. No runtime, build, accepted-cache load or producer was run for this
documentation change.

The command ledger, runtime record, milestone README, root README, coverage and
language-readiness appendix remain owned by the integration lead and are not
edited here. Runtime test durations describe observed runs and introduce no
performance threshold or edit-cost guarantee.

## Existing final runtime evidence

The final root command ledger and raw log were read. This exact Python check
ran from the root workspace, exited 0, and independently authenticated the log
and all distinct reader/pass completion markers:

```powershell
@'
from pathlib import Path
import hashlib,re
p=Path('verification/generated/final-audit-semantic-closure/workspace-recovery-scale-streamed-diagnostics.log')
b=p.read_bytes(); text=b.decode('utf-8')
print('sha256='+hashlib.sha256(b).hexdigest())
readers=re.findall(r'^recovery scale reader=(\d+) pass=(\d+): complete$',text,re.M)
assert len(readers)==32 and len(set(readers))==32
assert set(readers)=={(str(r),str(p)) for r in range(4) for p in range(8)}
print('PASS: 32 distinct reader/pass completions cover four readers and eight passes')
for line in text.splitlines():
 if line.startswith('recovery scale r4:') or line.startswith('recovery scale r5:') or line.startswith('test result:'):
  print(line)
'@ | python -
```

Output:

```text
sha256=4dbf578837a2d3f53e91ed3d4c8f2b61e1900626b08ec717aff2c46adf77ae7e
PASS: 32 distinct reader/pass completions cover four readers and eight passes
recovery scale r4: removing provider
recovery scale r4: apply complete; checking Working state
recovery scale r4: signature begin; documents=99 diagnostics=33612 references=102
recovery scale r4: signature complete; diagnostic_debug_bytes=34631669193 reference_debug_bytes=594549739 elapsed=58.184028s
recovery scale r4: projections begin
recovery scale r4: projections complete; elapsed=8.4805148s
recovery scale r5: restoring provider
recovery scale r5: apply complete; checking validation
recovery scale r5: signature begin; documents=100 diagnostics=0 references=102
recovery scale r5: signature complete; diagnostic_debug_bytes=2 reference_debug_bytes=595669713 elapsed=3.6474651s
recovery scale r5: projections begin
recovery scale r5: projections complete; elapsed=8.2099284s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 2346.29s
```

The computed SHA-256 matches the existing ledger. The retrospective retains
the exact runtime command, source, empty working-change identity and duration.
The 160 revision comparisons follow the actual fixture's nested four-reader,
eight-pass, five-revision loop; they are not inferred from command exit alone.

## Documentation checks

`git diff --check` exited 0 with empty output. This exact check ran in the
isolated worktree before staging and exited 0:

```powershell
@'
import re,subprocess
from pathlib import Path
base='cf12614d292d398348357103f3e6d42dc39ca7dd'
files=subprocess.check_output(['git','diff','--name-only'],text=True).splitlines()
assert len(files)==9
links=0
for name in files:
 p=Path(name); text=p.read_text(encoding='utf-8')
 for target in re.findall(r'\]\(([^)]+)\)',text):
  if re.match(r'[a-z]+://',target): continue
  path,_,anchor=target.partition('#')
  destination=p.parent/path if path else p
  assert destination.exists(),(name,target)
  if anchor=='final-architectural-disposition':
   assert '## Final architectural disposition' in destination.read_text(encoding='utf-8')
  if path: links+=1

def old(path): return subprocess.check_output(['git','show',base+':'+path],text=True,encoding='utf-8')
def new(path): return Path(path).read_text(encoding='utf-8')
def tail(path,mark): assert old(path).split(mark,1)[1]==new(path).split(mark,1)[1],path
def span(path,start,end): assert old(path).split(start,1)[1].split(end,1)[0]==new(path).split(start,1)[1].split(end,1)[0],path

tail('docs/gen1-gen2-architecture-audit.md','## Historical source audit')
tail('docs/architecture.md','## Generation 1: integrated v0.1 application')
tail('docs/modeling-workspace-frontend-boundary.md','Read-only inspection at `b49afe5`')
span('docs/modeling-workspace-frontend-boundary.md','## Historical reuse audit and design requirements','## Shared dependency storage review')
tail('docs/modeling-workspace-phase1-design.md','### Minimum shared-storage implementation')
span('docs/modeling-workspace-phase1-design.md','## Ownership and identity','## Acceptance contract')
p='docs/adr/0024-gen2-modeling-workspace.md'
assert old(p).split('## Context and reuse audit',1)[1].split('## Proposed decision',1)[0]==new(p).split('## Context and reuse audit',1)[1].split('## Decision',1)[0]
p='verification/summaries/final-audit-semantic-closure/core-foundation-retrospective.md'
assert new(p).split('Reviewed production source:',1)[1].startswith(old(p).split('Reviewed production source:',1)[1])
assert new('docs/adr/0024-gen2-modeling-workspace.md').splitlines()[2].startswith('Status: adopted for the bounded')
for p in ['docs/adr/0026-language-foundation-stability-contract.md','docs/adr/0027-compositional-semantic-publication.md']:
 assert old(p)==new(p)
print(f'PASS: {links} local documentation links exist across nine assigned files; final-disposition anchors resolve')
print('PASS: eight historical/Gen1 body comparisons, including the complete eleven-test snapshot')
print('PASS: ADR 0024 adopts bounded in-memory Phase 1; ADR 0026 and ADR 0027 are unchanged')
'@ | python -
```

Output:

```text
PASS: 140 local documentation links exist across nine assigned files; final-disposition anchors resolve
PASS: eight historical/Gen1 body comparisons, including the complete eleven-test snapshot
PASS: ADR 0024 adopts bounded in-memory Phase 1; ADR 0026 and ADR 0027 are unchanged
```
