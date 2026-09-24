# Scoped corpus witnesses

The publication example's `--audit-only` path now records `corpus_witnesses`
(`agq.systems-corpus-witnesses/v1`) and requires its findings to be empty in
addition to the unchanged strict effective audit. This is read-only preflight
evidence, never a publication artifact. The helper uses the candidate's current
construction overlay, accepted KerML dependency and scheduler certificate, with
the same context constructor as `audit_effective_population`.

Document scope selects assertions: SysML.sysml contributes five enumeration
definitions and thirteen literals; VerificationCases.sysml contributes two
definitions and eight literals; Interfaces.sysml selects BinaryInterface and
binaryInterfaces; Flows.sysml selects three FlowUsage parameter/end cases and
two explicit ConnectionUsage/HappensDuring cases. Definitions, literals, canonical
membership and typing carriers are inspected generically. Structural fixture IDs
are pinned source identities, not declaration-name semantic exceptions.

The exact structural IDs were cross-checked against the authenticated retained
record projection used for the prior diagnosis. Parameter IDs also match the
real evaluator's `real-structural-query-closure.json`. Original relationship IDs
and endpoints are asserted for the two explicit HappensDuring typings. The
helper does not restore a historical frontier or create closure evidence.

Local command results (PowerShell, isolated witness branch):

- `rustfmt --edition 2024 crates/kerml-text/examples/support/systems_corpus_witnesses.rs`
  — exit 0, no output.
- `git diff --check` — exit 0, no output.
- `cargo fmt --all -- --check` — exit 0, no output.

Compilation and real-corpus validation are deliberately coordinated in the lead's
warm release target. They are not claimed by this source-only verification note;
the resulting scoped report must contain the actual witness dispositions.
