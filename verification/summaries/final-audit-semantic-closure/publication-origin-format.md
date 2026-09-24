# Issued source-origin transport correction

Source baseline: `4ac9b8e58695ad837fed6643b0cedfac05d15635`.
The independent Python artifact verifier expected `{start, end}`, while
`ByteRange::serialize` in `crates/kernel/src/provenance.rs` serializes the tuple
`(self.start, self.end)`. The actual issued binding manifest therefore contains
`[start, end]`. The retained Item binding has `[779, 3501]` in the original
`Systems Library/Items.sysml` document.

The verifier now requires exactly a two-element JSON array. Endpoint integer
validation, source revision and syntax identity checks, bounds, UTF-8 decoding,
document/source digests, binding provenance and the full graph/certificate/cache
checks are unchanged. Object-shaped ranges are rejected; this does not add an
alternate transport format or weaken publication acceptance.

The regression fixture is the exact first binding from the issued manifest,
independently checked for equality below. Tests load the original pinned KPAR and
recompute source/document/revision identities, accept this emitted range, and
reject malformed arrays, the old object shape, noninteger/Boolean/negative or
reversed endpoints, out-of-bounds ranges, changed revisions, invalid syntax IDs,
unknown documents and split UTF-8 sequences. The existing complete synthetic
cache/receipt corruption regression now uses the authoritative range shape too.

## Actual checks

| Command | Exit | Result |
| --- | --- | --- |
| `python -m unittest discover -s verification/scripts -p test_systems_publication_gate.py` | 0 | 7 tests passed, 0.420 s test time; 0.844 s subprocess wall time |
| Python stdin script below | 0 | 69 binding origins checked against 21 pinned documents; retained fixture equals issued binding |
| `git diff --check` | 0 | Empty output |

The unit-test combined output is retained locally as
`verification/generated/final-closure-projections/publication-origin-format-tests.log`,
SHA-256 `d45ae6b60b33c4ed2a529bed0f822c71741ab8a94b4c27345d9084d7228ef4b2`.
The binding-origin output is
`verification/generated/final-closure-projections/issued-binding-origins.json`.
The inspected report SHA-256 is
`ebd7cdc9478363d94a02d5a916dbbef312202ea64b9380dcb9808975a4e19b7c`;
the issued binding manifest SHA-256 is
`dbd794bba9ebdfdea5af433945f6bb5d0475f0f05ad1d229cd004f5b57045789`.

Executed from the isolated worktree, with this literal script piped to `python -`:

```python
import hashlib,json,pathlib,sys
sys.path.insert(0,'verification/scripts')
import systems_publication_gate as gate
root=pathlib.Path('C:/Users/phili/github/agentique-systems/agentique')
directory=root/'verification/generated/final-audit-semantic-closure/accepted'
report_raw=(directory/'report.json').read_bytes()
bindings_raw=(directory/'standard-bindings.json').read_bytes()
report,bindings=json.loads(report_raw),json.loads(bindings_raw)
_,_,library,docs=gate.source_pins(root,report)
for anchor in bindings['bindings']:
    doc=gate.source_origin(anchor['source'],docs)
    gate.require(anchor['library']==library and anchor['source_path']==doc['path'] and anchor['source_sha256']==doc['sha256'],'binding provenance')
fixture=json.loads(pathlib.Path('verification/fixtures/final-audit-semantic-closure/emitted-binding-source.json').read_bytes())
gate.require(fixture['binding']==bindings['bindings'][0],'retained fixture equals exact issued binding')
gate.require(fixture['evidence']['report_sha256']==hashlib.sha256(report_raw).hexdigest(),'fixture report hash')
gate.require(fixture['evidence']['bindings_sha256']==hashlib.sha256(bindings_raw).hexdigest(),'fixture bindings hash')
result=dict(binding_origins_checked=len(bindings['bindings']),source_documents=len(docs),all_original_source_ranges_valid=True,retained_fixture_equals_issued_binding=True,full_artifact_gate_run=False,publication_authority=False,report_sha256=hashlib.sha256(report_raw).hexdigest(),bindings_sha256=hashlib.sha256(bindings_raw).hexdigest())
raw=(json.dumps(result,sort_keys=True,indent=2)+'\n').encode()
pathlib.Path('verification/generated/final-closure-projections/issued-binding-origins.json').write_bytes(raw)
print(raw.decode(),end='')
```

No Rust code, finalizer, profile default, receipt catalogue or issued artifact was
edited. No heavy build or full artifact/cache scan ran in this bounded task. The
integration owner must rerun the unchanged full artifact gate with this decoding
correction; these focused checks do not grant publication authority or establish
trusted restoration or phase M readiness.
