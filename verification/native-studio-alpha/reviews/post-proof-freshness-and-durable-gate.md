# Post-proof freshness plan and durable gate review

Reviewed against integration `bcf354a`, after the mount commits and call-local
query reuse, before held audit `0cbc54a`. The mount oracle subsequently passed;
its [independent result review](mount-only-result-review.md) retains the limits.
No pins, receipts, profiles, models or accepted runtime assets were changed here.

## Scope and interactions

Exact Git-byte `git apply --check` of the audit production diff passed against
the current integration tree. (A PowerShell text pipeline first failed because
it translated the patch stream; it was replaced with a binary subprocess pipe.)
The integrated mount source has the reviewed normalized SHA-256
`8465c80bf7707c540399265d0447f5b0d44dba7730ff4ec3818a6f5e62010368`.
The other two inputs still have their original hashes. After the held audit
patch, only these additional reviewed hashes are allowed:

| Source | Expected after held audit |
| --- | --- |
| `crates/kerml-text/src/source_inputs.rs` | `f15171c15dd27c2e61c05c3dfffc426b1714b14cc09e7c8716bbb046d1f30128` |
| `crates/kerml-text/src/sysml/publication.rs` | `682f079a4e2c6c412b897d159f93f3b495f7fd8d69a675aec11f8ae6df9fbef7` |

Mount sharing affects authenticated dependency construction. The audit observer
changes delivery of one already evaluated immutable answer; it retains the exact
Definition-or-Usage guard, ordered batches, full family dispatcher and final
diagnostic ordering. Standard publication still uses the same dispatcher with a
no-op observer. Query reuse affects modeling-view calls after revision binding;
it does not enter authored compilation or the publication audit. These scopes
are separate, but neither source review nor the successful mount gate replaces
the audit's exact same-context parity and combined cold reconstruction gate.

The new audit `_tests.rs` file is excluded by the existing interpretation
inventory. modeling-view and Studio reader sources are outside that inventory.
The view oracle's added Cargo.lock test edges require the existing accepted
language dependency-closure proof; **do not update the Cargo.lock input pin**.

## Narrow proposal tool

`verification/scripts/native_alpha_freshness_plan.py` calls the existing source
inventory, receipt/binding validation and language-lock compatibility code. It
requires the exact reviewed old/new source hashes, checks every other inventory
entry and authority field for equality, then emits a fresh `proposal.patch` and
`proposal.json`. It never writes the publication manifest. Unknown source
changes, source population changes, metadata changes or lock incompatibility
fail closed. Existing proposal directories cannot be overwritten.

Five pure bookkeeping regression tests passed (exit 0); retained evidence is
`checks/freshness-plan-unit.*`. A source-only read of current integration produced
`checks/mount-freshness-plan/`: exactly one proposed entry, unchanged authority
and successful language-lock compatibility. This is **proposal evidence**, not
permission to apply before the relevant proof/review gates.

After the required proof and independent review, regenerate at the exact tree
being approved. Mount-only uses `--phase mount`; all three reviewed changes use
`--phase mount-and-audit`. If the mount pin was already reviewed and updated,
the latter emits only the remaining two edits.

```powershell
$freshnessPlan = 'verification/generated/native-studio-alpha/freshness-reviewed'
python verification/scripts/native_alpha_freshness_plan.py --root . --phase mount-and-audit --output $freshnessPlan
if ($LASTEXITCODE -ne 0) { throw 'Freshness scope differs from reviewed sources' }
# Inspect the proposal and required successful proof receipts before applying.
git apply --check "$freshnessPlan/proposal.patch"
if ($LASTEXITCODE -ne 0) { throw 'Proposal no longer matches this ledger' }
git apply "$freshnessPlan/proposal.patch"
if ($LASTEXITCODE -ne 0) { throw 'Proposal application failed' }
git diff -- standards/sysml-publication-inputs.json
npm run standards:check
if ($LASTEXITCODE -ne 0) { throw 'Standards freshness/authority checks failed' }
```

The patch does not touch the ledger's publication identity, accepted receipts,
bindings, transport pins, authority registry, normative artifacts or profiles.
Retain the actual standards-check output and reviewed diff. Do not use blanket
`--capture`. The proposal hash authenticates its before/after normalized ledger
bytes; rerun the tool if source integration or conflict resolution changes them.

## Remaining audit qualification

Use the exact serial build/run helpers and names in
[held-reconstruction-qualification.md](held-reconstruction-qualification.md):

1. Integrate `0cbc54a` separately after the query gate; retain its existing source
   review. Do not update fingerprints merely to make a check green.
2. Run `sysml::publication::authored_audit_parity_tests::authored_audit_observer_matches_two_pass_queries_and_diagnostics`
   in the agq-kerml-text lib test. It uses ordinary accepted facades, complete and
   incomplete actual source populations, helper-only invalid/missing probes,
   exact full answers/context, audit reports and stored diagnostic order.
3. Run `create_part_command_matches_full_self_model_reconstruction` again with
   both optimizations. Compare its exact equivalence and complete measured
   profiles with the mount-only receipt. Certificate microseconds/counters are
   included in closure and are not additional phase durations.
4. Run the real platform durable gate and normal formatting, Clippy, Rustdoc and
   remaining repository checks. Then perform the independent freshness review.

## Real platform durable gate: scope and time budget

`native_in_process_self_model_candidate_commit_and_restore` uses a new temporary
database. It retains all prior obligations; the reader-cache checks are additive.
Its source-controlled sequence contains:

- Three ordinary accepted-runtime restorations: initial open, normal restart,
  then authenticated runtime load for the first deliberately source-only restart.
- Initial empty project plus two real seed commits: architecture, then agent
  fabric. Both authored seed commits use ordinary validated source reconstruction.
- Three successful Create Part preparations: original, competing CAS candidate,
  and a typed nested part after source-only restart. Each is explicitly validated.
- A referenced-definition rename which must fail after reconstruction/binding
  checks; a sibling-collision rename which should fail before reconstruction;
  and one successful rename with validation/commit.
- Two independent source-only restores, each after deleting only the exact
  disposable semantic-cache blob from this isolated test DB. Durable sources,
  checkpoint, manifest and accepted standard caches remain intact.
- Repeated commit identity, refusal to commit before validation, durable CAS
  conflict/cancellation, unchanged predecessor projections, preserved owner and
  ancestor IDs, renamed child ownership, exact source-restored projections,
  Validated restoration and final durable head assertions.
- Real Inspector repeat-cache equality including serialization and caller-copy
  isolation; derived Explain, dependency view and revision diff remain covered.

The final rename restore reuses the already authenticated standard publication
but constructs a fresh service with no authored revisions in memory. It still
asserts `DurableSource` and actual Validated restoration. The normal reopen is
allowed to use its authenticated semantic cache; it does not substitute for
either source-only assertion.

Budget **roughly 25–40 minutes**, not a timeout guarantee or a new measurement.
This estimate follows three approximately 72 s runtime restorations, two seed
reconstructions, four successful command reconstructions, one expensive refused
rename, two roughly 126 s source-only restores, validation and projections.
Seed sizes and command types differ from the measured Create Part case. The
gate also requests a graph including standards, so its repeated full DTOs may
cost more time/memory than focused product views. Preparation calls sometimes
include repeated validation/serialization; do not add nested timers or infer
missing phase durations. Record the actual whole-process result separately.

This test is a service/platform durability gate, not a substitute for native
input, screenshot, responsiveness or separate-process operator acceptance.
