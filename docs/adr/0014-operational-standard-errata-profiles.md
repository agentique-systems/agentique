# ADR 0014: Explicit operational standard errata profiles

Status: accepted by Agentique project authority, 2026-09-19.

KerML 1.0 remains the normative target. The received XMI, PDF, KPAR archives,
descriptor provenance and historical evidence remain unchanged. The published
profile (`omg-kerml-1.0-published/1`) preserves the complete raw descriptor graph,
including the KERML11-81 contradiction. `descriptors()` and `registry()` retain
their published meaning. `published_descriptors()` makes that meaning explicit.

The separately identified `agentique-kerml-1.0-operational/1` profile applies the
reviewed manifest in `standards/kerml-1.0-operational-errata.json`. This is an
Agentique operational interpretation informed by official OMG evidence. It is
not a claim of a final adopted KerML 1.0 erratum. The official issue remains open.
The 2026-05 release notes incorporate the issue in preliminary revisions; those
revisions corroborate the review and do not replace our normative baseline.

An errata review requires an exact defect identified by an official issue, an
unambiguous correction, exact source-qualified descriptor identities, an exact
artifact hash, reviewed reference/ownership closure, and retained evidence. The
KERML11-81 transform deletes only the two reviewed associations and four ends
they exclusively own. It never supplies participant facts. All surviving
descriptors and their provenance are identical to the published set. Independent
XML verification checks the deletion closure and incoming references. Ordinary
kernel registration checks the resulting graph without repairing dependencies.

`BaselineProfile` is a first-class language-layer authority choice. Explicit
profile descriptor/registry factories distinguish both graphs. The transform
checks the complete input against the reviewed source graph before cloning and
deleting. Changes to source hashes, versions, identities, descriptors or their
provenance fail closed. The patch cannot carry into KerML 1.1 or another 1.0
artifact. The generic kernel has no knowledge of this issue or its disposition.

Generation-2 library publication and authored KerML construction select the
operational profile explicitly. Legacy query constructors continue to accept
published registries through their published default; semantic options carry the
profile when binding operational models. Context identity includes the profile,
reviewed manifest digest, effective graph digest, original artifact identity,
library pins, rule-set version, bindings and revision-bound semantic evidence.
Published and operational results cannot share a context key.

A future operational revision requires a new profile identity, reviewed manifest
and implemented transform. Changes in profile identity invalidate results even
if library bytes are unchanged. Repository commits, API metadata, transformations
and execution compilation must carry the same identity when those systems are
implemented. They must reject incompatible contexts or perform an explicit
migration; this ADR does not implement those systems.

This is not source rewriting: the raw graph remains independently queryable,
the effective graph is a distinct value, and the manifest explains every removed
descriptor. Strict source/conformance reports continue to use and label the
published profile. Successful operational structural publication does not claim
complete executable semantics or erase published metamodel findings.
