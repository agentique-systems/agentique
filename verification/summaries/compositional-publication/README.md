# Compositional publication

Base: clean fetched `origin/main`, `068f7b0995825e84ffb04cb9937437ba15fd5ebb`.
Branch: `foundation/compositional-publication-and-modeling-workspace`.
[ADR 0027](../../../docs/adr/0027-compositional-semantic-publication.md) was
committed before implementation. Independent planner, certificate and equivalence
work used isolated `agq-comp-plan`, `agq-comp-cert` and `agq-comp-audit` worktrees.
Long-path checkout initially failed; enabling repository-local Git long-path
support allowed clean isolated checkouts. No existing worktree was reset.

## Acceptance boundary

Compositional closure is **incomplete**. Dependency planning and read-only
component audits do not seal strata, issue a composite publication certificate,
authenticate resumable checkpoints or establish the medium equivalence gate.
No new full Systems producer run is permitted by these diagnostic results.

The existing certificate conservatively opens every non-typing requirement
affected by an unfinished producer across the local graph. Its source comment
states that precise footprints for those query families remain unimplemented.
Current producer write scopes alone cannot replace that read-side proof.
Inverse/global negative searches and future subjects are additional explicit
dependencies. An immutable earlier Element record is not proof that a later
local relationship cannot change an incoming search.

The implementation therefore retains these guards in its SCC plan and requires
the existing scheduler certificate's actual local pairs, requirements and read
evidence in its component audit. Neither a caller-provided partition nor matching
counts can promote it to acceptance. The strengthened equivalence fixture checks
logical canonical content and evidence, not traversal counters.

Nine planner tests cover SCC ordering, linear/diamond/cyclic dependencies,
canonical typing/subsetting carriers, fresh writers, unknown and negative reads,
input permutation identity and authenticated external providers. In particular,
an immutable mount or an advertised digest without closure authority cannot hide
an open provider. Six actual scheduler component-audit tests distinguish a truly
unsafe model-wide writer from the conservative guard on a disjoint subject-scoped
writer: the latter leaves every earlier local pair closed but its membership
requirement open. A zero-output negative search can change captured read evidence
without changing the flat receipt, so a new component proof must bind those reads.

The exact comparator's five mutation controls reject equal-count changes to
proofs, searches, failures and requirements. Existing full-scan/worklist fixtures
continue to pass with this stronger comparison. These tests do not execute a
compositional scheduler. The [independent boundary review](audit-boundary.md)
also records the ordered-contribution evidence that checkpoint restore must
preserve; a graph archive alone is insufficient for the proposed new proof.

## Publication and readiness

`SYSML SYSTEMS LIBRARY CANONICAL PUBLICATION INCOMPLETE`

The last completed full **construction** audit on the base recorded 1,327/1,327
reference assertions passing and zero kernel obligations. That remains historical
construction evidence, not a newly issued strict publication certificate. The
last strict attempt did not finish. Accepted Systems bindings and trusted receipt
remain unavailable; accepted KerML Operational v9 is unchanged.

`AGENTIQUE LANGUAGE FOUNDATION NOT YET STABLE`

ADR 0026 remains proposed. Agentique self-model and effective SysML acceptance
against an accepted Systems publication remain unrun. Full SysML conformance is
not an extra readiness gate.

`MODELING WORKSPACE PHASE 1 REMAINS GATED`

No production modeling workspace crate or platform integrations are introduced.
Gen1 release obligations and original authority/library bytes are unchanged.

## Reproduction and remaining proof

Actual verification commands, source identities, output hashes and exit codes
are recorded in `commands.json`; raw output stays under ignored
`verification/generated/compositional-publication/` and the established watchdog
storage. Focused agent ledgers are consolidated into that file with their source
metadata. Every copied raw output was checked against its recorded SHA-256;
PowerShell's UTF-16 certificate-workstream captures retain their original bytes.

The bounded corpus planning test requires `AGQ_ACCEPTED_KERML_CACHE` pointing to
the pinned accepted cache and `AGQ_COMPONENT_PLAN_REPORT` pointing to ignored
generated output. It parses and constructs original sources, refines declared
references and plans dependencies without invoking any producer scheduler. Its
construction/reference observations do not replace the earlier completed
construction audit or the final publication audit.

Before a component can seal, the remaining work is to establish complete bounded
requirement/provider footprints (including zero-output and future-writer cases),
bind them to private component issuance, and integrate immutable stratum storage
and authenticated checkpoint restore. Then execute exact three-strategy synthetic
equivalence and medium monolithic/compositional equivalence. Only a passing medium
result permits component Systems closure and the final global read-only audit.
