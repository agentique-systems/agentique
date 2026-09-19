# KerML standard-library publication v2

Base: `ad6203bbac091342ed098a89ee8915595e404e57`, fetched `origin/main`;
initial worktree clean. Branch:
`semantics/kerml-standard-library-publication-v2`.

Result: **KERML STANDARD LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED**.

Gate 0 found a genuine authority conflict, **KLPV2-F-001**. The pinned XMI's
required Interaction participant association is identified as spurious by open
OMG issue [KERML11-81](https://issues.omg.org/issues/KERML11-81). Equating it with
effective association ends violates its inverse `1..1` bound when Features are
inherited by multiple Interactions. The published PDF and retained rules do not
define that equation. See the [detailed analysis](../../docs/kerml-standard-library-publication-authority-conflict.md).

No production semantics, kernel invariants, descriptor metadata, original
libraries or historical evidence were changed. The new executable work is an
obligation reporter and a synthetic regression witness. No accepted-publication
facade is exposed. Gate 1–16 completion is not claimed.

## Evidence

* `gate-0-baseline/`: unchanged full library quality/refinement command, actual
  exit 1. Reproduces ten structural lower-bound obligations, five unresolved
  names and 57 incomplete reference answers.
* `gate-0-obligations/`: fresh detailed report, including source ranges,
  production identities, canonical IDs, metaclasses, candidates, completeness,
  diagnostics, positive/search dependencies and a failed ordinary Snapshot apply.
  Dependency dictionary indexes keep repeated evidence compact. These are
  report labels, not a semantic identity digest.
* `completion-matrix.json`: obligation rule-family classifications and gate
  dispositions. Ordinary implementation gaps remain open; category F identifies
  only the conflicting participant requirement.
* `authority-conflict.json`, `retained-authority.json`: exact pinned authority,
  source ranges, retained operations/constraints and competing interpretations.
* `KERML11-81.html`: explicit read-only capture of the open OMG issue, not a
  replacement normative artifact or permission to implement KerML 1.1.
* `kerml-authority-extract.txt`: local text extraction of the unchanged supplied
  PDF, with its SHA-256 and physical page numbers.
* `authority-witness-1/` and `authority-witness-2/`: retained development failures.
  `authority-witness-3/`: passing arbitrary-ID witness of the contradictory
  inverse bound. Passing the witness means the invalid transaction is rejected.
* `final-1/`: complete workspace/application, generated-descriptor, both runtime,
  strict Rustdoc and preservation checks, with actual outputs and exit codes.
* `strict-1/`: separate strict metamodel conformance commands; both exit 1.
* `evidence-1/` and `review-1/`: authority classification generation, deterministic
  currentness checks, PDF extraction verification, standards integrity and diff
  checks; all exit 0.

All **19** commands in `final-1/results.json` exit 0, including both complete
structural runtime checks and strict Rustdoc for all eight generation-2 public
crates. Preservation verifies **780** previously tracked files. The quality gate
still exits **1**, and no validation success is inferred from the successful
execution of the obligation reporter or expected-failure witness.

The current `incomingTransitionTrigger` lookup is explicitly included even though
it is complete in the fresh baseline. The older invalid report under bootstrap v1
is preserved. This avoids silently dropping historical expression evidence or
claiming a new v2 expression fix.

## Reproduction

Run from the repository root. Every run needs a fresh evidence label:

```text
node verification/kerml-standard-library-publication-v2/run.mjs quality <fresh-label>
node verification/kerml-standard-library-publication-v2/run.mjs obligations <fresh-label>
node verification/kerml-standard-library-publication-v2/run.mjs witness <fresh-label>
node verification/kerml-standard-library-publication-v2/run.mjs final <fresh-label>
node verification/kerml-standard-library-publication-v2/run.mjs strict <fresh-label>
```

All language construction and verification use pinned offline inputs.
`authority-evidence.py` produces the matrix from the named frozen Gate 0 runs.
`preservation.py` compares authority, generation-1 obligations and all historical
verification evidence against the exact base; original artifact bytes are checked
exactly, while Git text allows line-ending normalization.
