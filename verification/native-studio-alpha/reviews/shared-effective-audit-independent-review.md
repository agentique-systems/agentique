# Independent review: reuse the existing effective-usages audit answer

Reviewed commit: `0cbc54a0954845bef3aaf4247ed611472289f52c`, compared with its
parent `edb8775` (the separately held mount-sharing lineage). Reviewer:
runtime/semantic authority stream, independently of the implementation agent.

**Source review passes; adoption remains conditional.** No authority widening,
subject omission, evidence truncation or diagnostic reordering was found in the
concrete patch. The runtime parity test, cold reconstruction oracle and wall-time
comparison have not run for this commit. This review is not an acceptance receipt
for the optimization or for its parent mount-sharing changes.

## Boundary examined

The old authored source loop first called the strict audit dispatcher for each
sorted batch of 32 local subjects, then called `effective_usages` a second time
for every Definition or Usage. It retained the second answer only when its
completeness was not Complete. The new callback observes the first answer by
immutable borrow at that same query's original dispatcher position.

The following remain unchanged in the reviewed diff:

- Canonical local population selection, exclusion of the accepted dependency,
  subject sorting, batching and per-batch immutable context equality check.
- Definition/Usage dispatch guard, all six family lists, other query calls and
  their order, and `audit_typed_answer` itself.
- Complete-query family counts; diagnostic expansion from composed KerML,
  SysML, support-query, support-name and observation evidence; pending-rule
  diagnostics and ordered findings.
- Source origin lookup for each capability, followed by the full report's
  EffectiveAudit diagnostics after all capabilities have been collected.
- Normal standard-publication callers. They retain the existing
  `audit_sysml_population` entry point with a private no-op observer. No caller
  receipt, alternate trust context, relaxed publication method or public
  acceptance bypass is introduced.

The authored observer clones the **whole** non-Complete `SysmlQueryResult`,
including context, private additional completeness, original values, pending
implications, diagnostics, rejected/filtered targets, supporting answers and
observations. The ordinary audit then consumes the unchanged original answer.
No cross-revision cache or inferred negative result replaces a query.

## Qualification concerns and test coverage

There is no source-level blocking finding. Two qualifications remain material:

1. **Required runtime parity:** the code relies on repeated evaluation over an
   immutable query context returning the same full answer even after later
   queries populate memoization. The new ignored test restores the actual
   accepted pair through ordinary facades and compares the old two-pass flow
   with callback delivery on the same graph. It compares callback subject order,
   complete derived Debug of every answer (including private completeness), all
   report counts/findings, complete capability diagnostics and the actual stored
   diagnostic sequence. No revision normalization masks differences. It has
   not yet executed.
2. **Unmeasured resource tradeoff:** an incomplete or invalid answer and its
   clone briefly coexist. Fewer query calls alone do not prove reduced peak
   memory, especially for unsuccessful Working graphs. The implementation note
   explicitly preserves this limitation and claims no measured speedup yet.

The ordinary and variation source fixtures test real Complete/Incomplete
contexts. Invalid-kind and missing-identity probes pass actual public query
answers through the private delivery helper only; they do not broaden the
production Definition/Usage subject guard. Tests create no mock accepted
context. The standard dispatcher's shared implementation means test equality
alone cannot prove its family lists were unchanged; this independent source
diff review supplies that separate check.

The additional CreatePartUsage oracle counters are observations of the existing
final closure invocation. Certificate time is already inside closure wall time;
the new output labels it accordingly and does not add it to the total.

## Freshness and retained evidence

Independently recomputed LF-normalized source digests match the held proposal:

| File | SHA-256 |
| --- | --- |
| `crates/kerml-text/src/source_inputs.rs` | `f15171c15dd27c2e61c05c3dfffc426b1714b14cc09e7c8716bbb046d1f30128` |
| `crates/kerml-text/src/sysml/publication.rs` | `682f079a4e2c6c412b897d159f93f3b495f7fd8d69a675aec11f8ae6df9fbef7` |
| `crates/kerml-text/src/sysml/publication_audit_tests.rs` | `d3a8686765bcac3542bafe8fad0dd13bd25462d82a54ee1f5d8cca64455de70c` |

Only the first two are additional interpretation-source changes. The earlier
source-checkpoint fingerprint remains a separate held change. No accepted
publication receipt, source artifact, binding, profile, rule identity, cache
installation or freshness record was changed by this patch or review.

Inspected the retained [format/diff command record](../checks/shared-effective-audit-source-format.json):
both commands exited 0, elapsed 3.803 seconds; it explicitly records no builds
and no standards consumers. The independent review itself used only read-only
source/diff/hash inspection and did not rerun semantic consumers.

Before any reviewed source-fingerprint update, require the recorded actual
runtime parity command plus the independent cold CreatePartUsage reconstruction
oracle and measured phase/whole-process timing and peak memory. Preserve the
pre-sharing baseline and distinguish these two held optimizations in the report.
