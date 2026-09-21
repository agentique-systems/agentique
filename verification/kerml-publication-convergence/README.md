# KerML publication convergence

Canonical publication is **incomplete**. KerML conformance coverage is **incomplete**.
Incomplete validator coverage is not the reason publication is refused.

Work started from clean fetched `main` at
`952c03bce889514e7720c002127749706d13232f` on
`foundation/kerml-publication-convergence`. No new authority correction is applied;
Operational v7 and all original library/authority bytes remain unchanged.

## Architectural correction

[ADR 0021](../../docs/adr/0021-verification-evidence-policy.md) replaces routine
command-capture commits with ignored generated output and one command summary.
Historical evidence is retained. The ordinary standards checker and verification
runner now put their raw reports/logs in ignored storage.

[ADR 0022](../../docs/adr/0022-kerml-publication-vs-conformance.md) defines strict
Snapshots, canonical semantic publications and independent conformance reports.
The production query context distinguishes a partial derivation overlay. Local
validation reports its deferred implied-inclusion assertion explicitly; the strict
check remains callable and source flags remain unchanged. This removes the
architectural cause of the historical staged implied-inclusion failures without
claiming that missing producer families are complete.

All pinned formal rules have a publication-relevance classification in
[publication-critical-coverage.json](publication-critical-coverage.json). Its
script verifies exact inventory equality against the pinned XMI. Classification
is separate from validator implementation status and producer/query completion.
For example, a specialization obligation can be satisfied by the existing library
base query even when its independent conformance validator is unimplemented.
Publication-critical rows are not a requirement to write one validator per row.
No rule is declared inapplicable to the corpus without evidence. The narrower
[derived/producer review](publication-critical-derived.json) retains open
structural obligations rather than demanding every derive operation be implemented.

## Authority and ordinary semantics

[The authority reassessment](authority-blockers.json) distinguishes:

- KERML11-2: retain the end/cross identities and report the failed end-count
  validation. The disagreement alone does not make graph construction ambiguous.
- KERML11-4: retain the formal multiplicity domain and report its conflicting
  validation predicate. Bound structure/accessibility still require closure.
- SelfLink: the required featuring domain differs between a Cartesian product
  and the opposite-end type. This remains publication blocking.
- KERML11-75: published prose and OCL select different imported Membership
  identities. This remains publication blocking, independently of validator breadth.

The local verifier checks exact retained source bytes/ranges and the import
witnesses. There is no issue-tracker crawl or negative applicability sweep.

`StandardRole::OccurrenceSnapshots` now supplies the validated canonical role
used by variable featuring. `is_featuring_type` implements the general owner/
snapshot-redefinition predicate by identity even when the Feature is renamed.
Eligibility does not invent a domain relationship; missing contextual snapshot
domains remain incomplete until their producer is established.
The private fixture-heavy resolution regression moved from the production module
to `crates/kerml-semantics/tests/unit/`. Public phase/coverage regressions live in
`tests/publication_contract.rs`.

[The binding review](standard-binding-review.json) records the remaining initial
value and Array role migrations. No accepted binding manifest is regenerated.

## Measured scope and remaining work

The historical full expanded baseline is
`verification/kerml-semantic-closure-v10/full-v7-complete.json`. Its counts are
historical observations, not hardcoded acceptance thresholds or new measurements.
A duplicate baseline run was deliberately stopped during refinement after the
retained command record showed that the prior complete run took over five hours.
Its nonzero exit is recorded as cancellation, not a failed semantic check.

The current `--source-only` audit strictly applies the source graph, checks all
mandatory source reference assertions, and runs the audit's implemented validation
and query families against an empty partial overlay. It does not claim expanded
producer closure. Separate publication and conformance commands consume that
measured output. Raw JSON and logs stay in `verification/generated/`.
This draft audit completed before the final removal of speculative snapshot-domain
inference. The final Rust checks cover that conservative change. The audit counts
are scoped observations, not a final expanded-publication measurement.

Publication still requires complete mandatory reference evidence, Triggers
parameter/member suppression, essential typing/featuring, cross-feature domains,
the remaining required derived facts and producers, and complete identity/source
mapping proof. The publication facade, accepted bindings and authored dependency
integration remain gated on those obligations. SysML semantic work must not start.
Generation-1 release requirements and traceability are unchanged.

## Reproduction

`summary.json` records actual command vectors, exit statuses, tool versions, source
commit and working-input digests. Successful raw logs are not retained in Git.

```powershell
python verification/kerml-publication-convergence/scripts/classify.py --check
python verification/kerml-publication-convergence/scripts/authority.py
cargo run --release --locked --offline -p agq-kerml-text --example library_publication_audit -- --source-only --output=verification/generated/kerml-publication-convergence/source-corpus.json
python verification/kerml-publication-convergence/scripts/report.py publication --audit verification/generated/kerml-publication-convergence/source-corpus.json
python verification/kerml-publication-convergence/scripts/report.py conformance --audit verification/generated/kerml-publication-convergence/source-corpus.json
```

Use an unused output filename for the audit. The publication gate is expected to
exit 1 while these graph obligations remain open. The conformance command exits 0
when it has generated its report; that exit does not mean conformance success.
`verification/scripts/run.py --name NAME -- COMMAND ...` records any verification
command with raw output in ignored storage. Strict Rustdoc records its explicit
`RUSTDOCFLAGS=-D warnings` environment override in the same summary.
