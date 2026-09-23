# Final Systems report and generated-artifact gate

Prepared while the sole full publication attempt was running. No Rust build,
producer replay, full-corpus run, authority activation or standard source edit
was performed. There was no existing final G/H artifact validator; the medium
gate and checkpoint reader were retained and reused.

`systems_publication_gate.py` rejects missing or failed final report gates:
21 exact parsed/constructed documents, zero recovery/construction gaps and
kernel obligations, 1,327 Complete mandatory references with every failure
category zero, converged Complete strict producers, a fully closed certificate,
no authority conflicts or publication findings, all 13 capability families,
identity/provenance coverage, 69 distinct expected bindings and a successful
binding stale check. Construction observations cannot substitute for the strict
`accepted_publication` audit.

The original Systems KPAR and all 21 source byte hashes are checked against the
checked-in library manifest. Document/source-revision IDs are independently
recomputed; KerML Operational v9 and SysML Operational v2 interpretation manifests
are pinned. The generated receipt, binding manifest, cache inventory and every
decoded entry hash/size must match. Source-map and binding provenance must agree
with the exact original document revisions and UTF-8 ranges.

The existing frontier reader now factors out its unchanged streaming graph
scanner. In addition to the existing pins and selected contribution digest it
exposes the entire post-header graph payload hash and complete certificate
receipt digest. The final strict cache must exactly match its authenticated
final scheduler checkpoint on both, plus selected proof/search support. Distinct
cache/frontier headers are expected; the graph payload must be identical. This
adds no comparison between medium construction and full strict phases.

This checker emits verification evidence only. It does not issue publication,
install a trusted receipt, change Operational defaults, or replace Rust trusted
restoration and its semantic/dependency validation. Output is restricted to
verification `generated` or `summaries` paths and may not overwrite its inputs.
A failed validation overwrites an older verification output with an explicit
failure and exits nonzero.

Run after the full process has finished writing all four artifacts:

```powershell
python verification/scripts/systems_publication_gate.py --report verification/generated/final-language-acceptance/systems-full/report.json --output verification/summaries/final-language-acceptance/systems-publication-artifacts.json
```

The report directory must also contain `accepted-publication.json`,
`standard-bindings.json`, and `canonical.publication.zip`; the report's final
checkpoint journal and referenced ZIP must remain available.

## Actual focused verification

Worktree: `agentique-frontier-checkpoint`. Commands used Python only.

| Command | Exit | Actual output |
| --- | --- | --- |
| `python -m unittest discover -s verification/scripts -p 'test_*frontier*.py'` | 0 | 9 tests, 0.726 seconds, OK. Existing medium and frontier guard behavior retained. |
| `python -m unittest discover -s verification/scripts -p test_systems_publication_gate.py` | 0 | 4 tests, 0.138 seconds, OK. Includes real pinned source inventory, incomplete/forged report rejection, cache corruption and re-pinned graph mismatch controls. |
| `git diff --check` | 0 | Only Git's CRLF-to-LF informational warning for the existing helper; no whitespace error. |

The final corpus validator has not yet run: the sole full publication attempt
was still active when this gate was prepared. Its future exit and evidence must
be recorded separately; these focused tests do not establish Systems acceptance.
