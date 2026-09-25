# Interrupted proposal: preserve the failed journey and prove the durable head

**Source judgment: an explicit retry can reuse the unchanged durable baseline,
provided it independently checks repository state and repeats ordinary semantic
restoration.** The old pre-proposal `--resume-report` guard correctly rejects
`real-run03`; its recorded preparation attempt must not be erased or relabeled.

## Why a failed preparation is distinguishable from a failed commit

`StudioPlatform::propose` stores its review handle in the process-local candidate
map. `ModelingService::prepare_changes` constructs `PreparedChanges` without
writing the repository. Only `commit_prepared` calls `commit_revision`.
The SQLite adapter stores candidate blobs, revision, head compare-and-set and
operation receipt within one FULL-synchronous transaction. A lost acknowledgement
can therefore leave durable state even when the UI has no success response.
`CommitUnresolved` is deliberately a different state from an interrupted prepare.

The schema has no durable working-candidate table. Preparation alone cannot
register a durable revision. This is a source-contract observation, not proof
that every failed application process stopped before a commit; check the actual
repository and report together.

## Independent read-only observation after real-run03

The retained [audit script](interrupted-proposal-storage-audit.py) opens the exact
reported database with SQLite `mode=ro`, enables `query_only`, and takes one
consistent read transaction. It does not load an accepted runtime or change any
repository bytes. The [initial audit](interrupted-proposal-storage-audit.json)
and [expanded column-binding audit](interrupted-proposal-storage-audit-v2.json)
retain actual observed values. The second audit additionally checks that indexed
project, branch, revision and receipt identity columns equal their hashed payloads.

Observed evidence:

- Main remains `8dcfb224-55e6-4d0d-ac3a-84fcfffb9a46`, and its entire parsed
  manifest equals the failed report's retained Validated baseline.
- Exactly three retained revisions are the baseline's complete ancestry:
  initial `753357f6-3a43-464c-bf14-cf90f48156c1`, architecture
  `a0ce9ecb-728a-40ce-87b5-88f8cbc84105`, and that baseline. There is no additional
  stored candidate, including one left behind by a later branch rewind.
- Exactly three operation receipts refer to those three revisions. Both retained
  branch heads remain within that ancestry.
- All eleven stored blob addresses match the actual SHA-256 of their bytes.
  Mandatory source/checkpoint addresses are present. Current Agentique model
  source bytes equal the baseline document digests; the proposed
  `alphaStudioObserver` name is absent from those bytes.
- SQLite integrity and foreign-key checks pass. All observations are from the
  same read snapshot; the failed report's actual bytes are unchanged afterward.

These checks establish observed storage identity. They do **not** authenticate a
closure certificate, validate a model, qualify responsiveness or make the failed
journey pass. The audit is a point-in-time observation; a later launcher must
recheck the repository rather than trusting this JSON as perpetual authority.

## Minimal recovery contract

1. Use a distinct explicit interrupted-proposal recovery input. Digest the latest
   failed report, preserve its failure and chain that exact digest in the new
   report. Do not silently reuse an earlier pre-proposal report.
2. Reject a predecessor that records committed identity, a commit attempt,
   `CommitUnresolved`, or progression into commit. For the immediate retry,
   scope the exception narrowly to interruption during the first prepare; do not
   broaden the existing pre-proposal resume path.
3. Before submitting another proposal, compare the actual project/branch head,
   complete manifest, accepted publication bindings, source/checkpoint addresses
   and retained revision/operation inventory with the expected baseline. Refuse
   any mismatch. Do not delete candidate rows, remove receipts, move a branch or
   rewrite source to make the check pass.
4. Open through ordinary accepted runtime authentication and
   `ModelingService::resolve`. Its receipt checks, reconstructed graph/context/
   closure comparisons and `working.validate()` still determine Validated.
   Re-run the actual baseline/source checks and the entire operator journey.
5. Preserve normal optimistic concurrency at eventual commit. A head change
   after the recovery observation remains a genuine CAS conflict, never implicit
   permission to overwrite someone else's work.

No production recovery implementation or new runtime consumer was introduced by
this review. The actual audit commands and command outputs are retained in
[command evidence](interrupted-proposal-storage-commands.json).
