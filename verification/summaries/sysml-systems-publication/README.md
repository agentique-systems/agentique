# SysML Systems Library publication

**SYSML SYSTEMS LIBRARY CANONICAL PUBLICATION INCOMPLETE.** The publication
facade is implemented, but no Systems publication has been accepted. The final
combined corpus rerun remains pending. Three additional canonical target
decisions await the user; they are separate from ordinary implementation gaps.
SysML conformance coverage remains incomplete and is a separate gate.

All 21 original Systems Library documents retain their exact bytes. The strict
Published SysML 2.0 grammar reproduces 13 complete parses and eight exact retained
failures. Explicit Operational v1 parses all 21 with zero recovery. Reusable
Definition/Usage family contracts construct all 21 declared documents through
the shared lossless syntax and canonical kernel machinery. Construction coverage
does not establish reference or producer completeness.

The Systems candidate shares the sealed, accepted KerML Operational v9
publication as an immutable dependency. The dependency-driven worklist applies
KerML and SysML producers to new Systems records and their derived facts; it
does not replay producers over accepted KerML library records. An explicit
unpublished construction overlay permits those producers to supply facts needed
by mandatory reference refinement while retaining unresolved obligations.
Scalar derivations, relationship contributions, search dependencies and changed
endpoints participate in the same invalidation contract.

Canonical source identities and StandardLibrary provenance retain DocumentId,
SourceRevisionId, SyntaxNodeId and ByteRange. Reference refinement retains the
declared identities while binding endpoints against the combined namespace.
The shared query context binds the accepted KerML digest, both profiles,
descriptors, grammar and semantic manifests, Systems library identity, bindings
and rule sets. Authored edits cannot rename the immutable library dependency.

`CanonicalSysmlSystemsLibrary::publish` computes its acceptance findings itself.
It rejects pending construction, producer, mandatory reference, capability,
binding, identity or authority findings. Its immutable facade shares the accepted
KerML dependency by `Arc`; constructing the facade API is not evidence that a
candidate passes its gates.

The first full combined closure attempt ended after approximately six minutes
with a scalar ownership proof cycle. The implementation was fixed and a focused
determinism regression added. That failed attempt is retained in the command
summary; the final corpus outcome must come from the rerun, not the regression.

Evidence follows [ADR 0021](../../../docs/adr/0021-verification-evidence-policy.md):

- [Grammar matrix](../sysml-operational-v1/grammar-status.json): every document,
  profile, byte-preservation result, recovery count and production count.
- [Context identity checks](../sysml-operational-v1/context-identity.json): frozen
  interpretation inputs and preservation of historical KerML receipt encoding.
- [Authority decisions](authority-decision.json) and [review](authority-review.md):
  the four authorized syntax decisions, composite Item correction, Attribute
  formal-target precedence and three unresolved canonical target conflicts.
  Declared owned-path witnesses establish the actual identities without aliases.
- [Candidate status](candidate-status.json) and [capability status](capability-status.json):
  final rerun outcomes, to be populated when that run finishes.
- [Command summary](summary.json): actual commands, exit codes, tested source
  identities and resource observations, including failed attempts. Raw logs stay
  under ignored `verification/generated/`.

From the repository root, build and run the audit using an existing accepted
KerML cache. Restoring that cache verifies the sealed dependency; this procedure
does not rebuild its producers. On Windows:

```powershell
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_PROFILE_RELEASE_LTO = 'false'
cargo build --locked --offline --release -p agq-kerml-text --example sysml_systems_publication
python verification/scripts/watchdog.py --name systems-publication --summary verification/summaries/sysml-systems-publication/summary.json --wall-seconds 1800 --private-mib 6144 --min-free-mib 1024 -- target/release/examples/sysml_systems_publication.exe --cache=verification/generated/kerml-v9-publication/canonical.publication.zip --output=verification/generated/sysml-systems-publication/candidate.json
```

The 30-minute and 6-GiB private-memory limits are workflow stops. A nonzero exit,
incomplete report or resource stop does not establish semantic acceptance.
