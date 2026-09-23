# Component certificate boundary audit

This change implements a read-only audit of existing scheduler-issued proof. It
does **not** issue component/composite closure certificates, establish partition
equivalence, or advance Systems publication acceptance.

`ProducerClosureCertificate::audit_component` binds an independently expected
registry and exact semantic context before inspecting a proposed subject set.
It reports pending/incomplete local evaluations, open local requirements, missing
subjects and missing captured reads. It preserves the existing certificate's
global future-writer and provider analysis; selecting local subjects never masks
an unfinished external producer or declares it immutable.

The audit digest binds the source certificate and exact local captured read
content under a versioned encoding. It covers zero-output negative searches,
which need not appear in canonical output proofs. Interning, allocation identity
and evaluation order do not change the digest. Passing the audit remains only a
necessary condition: future-writer exclusion, complete dependency/provider
boundaries, graph-delta identity and mandatory references require further proof.

## Demonstrated obstruction

A real scheduler fixture has two Classifier subjects and a negative member search.
Both producer callbacks on subject 1 return Complete. Subject 2 has an unfinished
producer permitted to write model-wide Membership effects. The scheduler retains
subject 1's reader as Pending and its EffectiveMembership requirement open. A
proposed component containing only subject 1 fails the audit. Local callback
success cannot bypass the later writer's effect contract.

A control uses the same pending Membership producer with `Subject` scope and
scoped fresh ownership. Subject 2 is disjoint from the earlier negative search on
subject 1. The precise causal analysis now closes both subject 1 evaluations and
every applicable pair. Nevertheless EffectiveMembership remains open, with only
open-requirement audit findings. This isolates the existing conservative global
requirement-footprint guard from the genuinely unsafe model-wide writer case;
it is a proof-architecture limitation, not evidence of a semantic cycle between
these two subjects.

Separately, two complete runs with no derived output and identical flat closure
receipts read different negative populations. Their audit read-evidence digests
differ. The current compact receipt deliberately omits optional scheduler reads;
trusted restoration preserves its established acceptance authority but cannot
supply those absent reads to prove a new component boundary.

The current issuer conservatively applies unfinished relevant effects globally
for all requirements except EffectiveTyping. Sound partial issuance therefore
requires the missing exact dependency/effect boundary proof or joint closure of
the affected populations. Neither matching counts nor hashing a provisional DAG
provides it. No proposed component is sealed by this change.

## Verification

The six actual scheduler tests pass, including invalid split rejection, its
disjoint-scope control, zero-output evidence identity, legal scheduler order
permutations, restoration evidence limits, and wrong graph/registry/population rejection. Package Clippy
for library/tests with warnings denied and workspace formatting pass.

[Commands and actual outputs](certificate-audit-commands.json) retain the initial
fixture compilation error and zero-test filter attempt as well as successful
checks. Low-artifact settings use one build job, no incremental compilation and
no dev/test debug data. No corpus closure was run in this workstream.
