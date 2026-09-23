# Medium exact-evidence gate review

Reviewed integration `bf02cf8` without changing the running publication executable.
The prior medium report SHA-256 was independently read and matches the gate pin:
`9aa5332692a443a1bc8cb1a761a7207714f83116e816a1e7a6d0201dfaad3288`.

The gate requires 13 complete byte-exact documents, 695 Complete references,
zero obligations/conflicts, converged Complete final-predicate producers, and
14,791 closed applicable pairs / 417,786 closed requirements. Historical identity,
aggregate graph/proof/search, certificate, context and registry digests must match.
Both new runs must enable the executable's full certificate rebuild comparator.
The resumed run must restore more total than completed invocations, skip rounds,
and commit new checkpoints; replaying only finished invocations cannot pass.

## Selected contribution evidence

The semantic graph digest predates selected ordered-reference contributions and
does not encode their individual proof/search payloads. Producer transport hashes
bind read keys, and mandatory-reference query observation hashes bind results,
so neither fills that gap. Synthetic checkpoint tests compare this evidence,
but actual medium equivalence also needs it.

`frontier_artifact_evidence.py` now authenticates the final journal using each
report's independently emitted pin, the referenced ZIP bytes, and the actual
decoded state/graph stream hashes. It requires a converged final state and binds
its graph/context/certificate receipt identities to the report. It resolves each
selected contribution's proof/search indexes to content hashes, then compares
canonical `(element, property, target, position, proof, searches)` evidence.
Intern-pool numbering does not participate in equality. Only proof/search hashes
are retained; irrelevant scheduler rows are skipped in bounded streaming buffers.

The historical report has no such checkpoint archive. Its existing pinned graph
and certificate digests remain required. The additional selected-contribution
comparison applies to uninterrupted versus resumed medium runs. This limitation
is explicit in the generated gate result; no historical evidence is fabricated.

No checkpoint or this Python comparison establishes accepted publication authority.
The strict Rust acceptance operation still follows the successful medium gate.

## Actual focused verification

Working directory: `agentique-frontier-checkpoint`.

| Command | Exit | Output |
| --- | --- | --- |
| `python -m unittest discover -s verification/scripts -p test_frontier_artifact_evidence.py` | 0 | 5 tests, 0 failures, 0.620 seconds, OK |
| `python -m unittest discover -s verification/scripts -p 'test_*frontier*.py'` | 0 | 9 tests, 0 failures, 0.601 seconds, OK |

Regressions cover independent proof/search pool numbering, changed selected
proof/search content, journal/ZIP/decoded state/decoded graph tampering, stale
state/report bindings, unfinished final state, invalid pool indexes, duplicate
contributions, trailing graph rows, and escaped strings spanning stream chunks.
No Rust build or new corpus run was performed for this change.

## Real checkpoint format smoke check

An existing completed scheduler invocation from the running medium session was
read without replaying producers. This is a format/authentication smoke check,
not the final medium equivalence gate: its minimal report bindings were obtained
from that same checkpoint and were not claimed as independent acceptance evidence.

The initial smoke command exited 1 (`ValueError: invalid graph row`) because the
reader omitted the archive's `"Overlay"` unit variant. The reader and synthetic
fixture now include it. The corrected command below exited 0, authenticated a
31,991,877-byte state and 81,025,714-byte graph, and observed 2,568 selected
contributions. Elapsed wall time was 12.047 seconds, including two state reads.

```powershell
@'
import hashlib, json, pathlib, sys, time, zipfile
sys.path.insert(0, 'verification/scripts')
from frontier_artifact_evidence import StateReader, checkpoint_evidence
root = pathlib.Path('C:/Users/phili/github/agentique-systems/agentique/verification/generated/final-language-acceptance/medium-frontiers')
path = root / 'journal-48825c5d990b4c53a945eea0267b299abf9733329208764e564f86c71d938070.json'
raw = path.read_bytes()
entry = json.loads(raw)['entries'][-1]
started = time.monotonic()
with zipfile.ZipFile(root / (bytes(entry['archive_sha256']).hex() + '.zip')) as zipped:
    with zipped.open('state.json') as source:
        reader = StateReader(source)
        state = reader.fields({'converged'}, {'certificate': {'receipt'}})
        reader.whitespace()
        assert not reader.peek()
receipt = state['certificate']['receipt']
report = {'checkpoint_session': {'latest': {'journal': str(path), 'sha256': hashlib.sha256(raw).hexdigest()}},
          'producer_closure': {k: receipt[v] for k,v in [('model_digest','model_digest'),('context_contract_digest','context_contract_digest'),('producer_registry_digest','registry_digest'),('digest','digest')]}}
print('Schema/authentication smoke only:', checkpoint_evidence(report), flush=True)
print('Elapsed seconds:', round(time.monotonic()-started,3), flush=True)
'@ | python -
```

Returned selected-reference evidence digest:
`7f4fcc97ec4c23c95ba202be5f0588f75f0a5d984141e284bbda11dbb67bfbc9`.
The 9-test command above was rerun after this correction: exit 0, 9 tests in
0.599 seconds, OK.
