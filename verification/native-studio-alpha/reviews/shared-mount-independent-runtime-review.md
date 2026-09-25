# Independent review of guarded accepted-mount sharing

Reviewed commit: `089fde4a4e408ff41500edee2ca8b7c6db1d0343`.

Scope: read-only source review of the exact held diff, its
`shared-mount-reconstruction.md` contract, ADR 0026, existing source compilation,
accepted dependency mounting, declared identity restoration, workspace validation,
and the proposed cold oracle. No build, test, accepted-runtime consumer, source
mutation or freshness recapture was performed by this reviewer.

Judgment: the immutable dependency-sharing change fits ADR 0026's implementation
freedom in principle. I found no new path that manufactures an accepted facade,
inherits local closure/validation, substitutes a weaker producer registry, or
mutates the predecessor. Actual cold-oracle equivalence and measured performance
remain unrun at this review checkpoint. Source review is not acceptance.

## Ranked findings

### P2 — the advertised exact oracle omits public result identity fields

In `crates/modeling-agent/tests/create_part_performance.rs:69`, the reference
comparison checks relationship, specific element, name, origin and resolution,
but omits `ReferenceAssertion.kind`, `alias()` and `visibility()`. At line 85,
the SysML comparison checks the embedded KerML answer and projected evidence but
does not compare `SysmlQueryResult.context`.

The surrounding graph/fingerprint checks are strong, and this is not evidence
that mount sharing currently changes one of those fields. It is a concrete gap
in the claimed exact oracle: a changed interpretation-context or reference
classification could escape these particular assertions while the underlying
graph and selected answer still match.

Requested fix before using the oracle as exact equivalence evidence: compare
reference kind, alias and visibility, and compare the complete SysML context
identity after normalizing only its nested fresh KerML revision label, as the
existing KerML comparison already does. Keep every other context field exact.
No production semantic behavior needs to change for this fix.

### P2 — document the checkpoint-history trust precondition of the public API

`SourceIdentityCheckpoint::restore_sharing_dependency` (source checkpoint line
139) and `ProjectRevisionCheckpoint::restore_sharing_dependency` (workspace
checkpoint line 105) check publication identity, project/root identity and the
claimed direct parent. They do not prove that all supplied identity reservations,
retired IDs and source-ledger entries descend from the predecessor. At source
checkpoint line 269, identity history is restored against a new snapshot of the
standard dependency, not compared with `predecessor.history`.

That distinction matters because checkpoints are public mutable data. Removing a
predecessor's retired local IDs from a caller-supplied checkpoint is not rejected
by the new predecessor checks themselves. The ordinary cold restoration boundary
already relies on trustworthy checkpoint identity data, so this is not a newly
demonstrated service regression or an accepted-publication bypass.

The intended service path remains appropriately bounded: `source_identity.rs`
clones the actual predecessor checkpoint, edits the service-proven syntax arena,
and retains existing continuity postchecks before a prepared candidate is issued.
The new method must not be described as authenticating arbitrary external
checkpoint lineage merely because its parent UUID matches.

Requested fix: explicitly document that callers must authenticate/prove checkpoint
lineage through the service/repository boundary; exact parent matching authorizes
mount reuse, not arbitrary identity-history replacement. If the public method is
instead intended to enforce lineage on untrusted checkpoint data, add monotonic
reservation/retirement checks and a retained-ID rollback rejection case before
claiming that stronger contract. Keep that broader enforcement change separate
from this storage optimization. The current service-owned call does not require
inventing a new source reconciliation rule.

## Boundaries independently traced

- `AcceptedSourceDependency` is private and constructed from an accepted
  `CanonicalSysmlSystemsLibrary`. Its `ProducerClosedDependency` has private
  immutable overlay, context and certificate fields. The new method obtains the
  facade from the predecessor itself; callers cannot supply a competing facade.
- `SourceInputs::apply` already shares this accepted dependency across ordinary
  source edits. Sharing it during a proven full reconstruction is consistent with
  that existing design, rather than a new standard/local evidence boundary.
- The new path checks the same checkpoint format, accepted KerML and Systems
  digests, exact source population and bytes, duplicate document/path/syntax
  identities and restored syntax shape as the cold path. It additionally checks
  project/root and outer predecessor identities.
- Both branches rebuild fresh document state with default parse limits and end
  at `compile_with_history(None, restored_history, false, None)`. No predecessor
  local graph, lowering cache, scheduler frontier, certificate or validation
  result enters compilation. The immutable standard certificate retains its
  existing boundary; local producer/search obligations are rebuilt.
- Workspace restoration returns `WorkingProjectRevision`. Existing validation
  and repository CAS remain required. The new pointer comparison exists under
  the verification feature only and is not a semantic acceptance identity.
- The proposed oracle uses the unchanged cold checkpoint path, which creates a
  distinct mount, rather than `full_rebuild` over already shared inputs. It checks
  exact checkpoints, canonical records and occurrences, semantic/context/closure
  fingerprints, reference evidence, the selected effective query, validation,
  full document reparsing and the absence of semantic-cache reuse.

## ADR 0026 and the single freshness input

Yes: one reviewed change to `source_checkpoint.rs` can legitimately fit ADR 0026.
The relevant contract is preservation of observable identity, order, provenance,
results, completeness and evidence; it is not a ban on changing a file whose hash
is recorded. Immutable accepted dependencies are expressly shared, and pointer
identity may be observed for storage tests without becoming semantic authority.

I independently inspected the commit's changed language-file population: only
`crates/kerml-text/src/source_checkpoint.rs` changes among the frozen kernel,
KerML and SysML language crates. Its normalized SHA-256 is:

```text
Recorded: eb86d74ead07e68664081b131c3dbd152877fdb12c1374d3ba2175764a57aa0e
Reviewed: 1b77839611609533c329b800843aca3e97530b1c8e011f8c42c3ddd593261e07
```

This does not authorize immediate freshness recapture. Preserve the baseline
executable measurement first, address the oracle omissions, run the exact cold
equivalence/negative/validation gates, then independently review the one
fingerprint replacement. Assert that every other input, full publication
identity, original accepted receipt, binding manifest, profile/rule identity and
transport pin remains unchanged. Any further source edit changes the reviewed
hash and requires its own explicit diff review; do not reuse the hash above.

## Acceptance and performance limits

Compile, focused Clippy, required Rustdoc and actual real-runtime cold equivalence
are still necessary. Existing source-only restart and predecessor immutability
gates should remain intact. Report command and cold restore wall time separately:
the former includes source proof and service postchecks, while the latter creates
its own authenticated mount. Their residual difference is not a direct mount
timing. This change can remove repeated mount authentication; it does not establish
incremental producer/closure work elimination. Whole-process peak memory while
both graphs remain live is not a per-edit allocation measurement.

No P0/P1 implementation defect was found in the intended service path. Approval
remains conditional on the concrete qualification above; no tests were run here.
