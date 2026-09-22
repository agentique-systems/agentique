# SysML Systems Library publication

**SYSML SYSTEMS LIBRARY CANONICAL PUBLICATION INCOMPLETE.** The publication
facade is implemented, but no Systems publication has been accepted. The combined
candidate run finished with 1,315/1,327 mandatory references Complete, twelve
Incomplete and two kernel construction obligations. Three additional canonical
target decisions await the user; they are separate from the remaining producer
closure findings.
SysML conformance coverage remains incomplete and is tracked separately; complete
conformance is not a Systems publication gate.

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

SysML naming derivations cover performed actions, requirement constraints,
variants and transition payload parameters. Mandatory reference lookup excludes
only names that depend on the endpoint currently being resolved, preserving
declared names and aliases. In composed language contexts, a cyclic end-order
query completes only when every participant explicitly covers every inherited
position; uncovered cycles remain incomplete. KerML-only ordering behavior stays
unchanged.

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

The final construction worklist converged with all scalar predicates enabled,
but remains Incomplete: five negative owner-typing proofs, three variable
featuring domains, three value-binding contexts and eight applicable missing
formal-target obligations. The twelve incomplete references are in Actions.sysml.
Ten have matching selected endpoints; `accepter` and `acceptedMessage` in
`AcceptAction` remain unbound. All other reference failure counters are zero.
The Flow end-order and Views viewRendering references now resolve. No unresolved
lowering family is classified as an authority conflict.

The actual publication facade rejected this candidate at construction/authority
preflight. Consequently, strict publication closure, complete binding acceptance,
final capability/provenance auditing and corpus digest determinism are not
established. Phase 2 authored Systems dependencies and richer effective APIs have
not started. Focused scheduler order/batch/partition regressions remain useful
implementation checks, not an accepted corpus certificate.

The run completed in 1,670.796 seconds (27m 51s), peaking at 5,999.1 MiB private
memory. Neither the 30-minute nor 6-GiB watchdog fired. Its exit code is 1 because
publication was rejected. The narrow remaining runtime margin still warrants
attention when closure work resumes; the retained evidence makes no performance
acceptance claim for an unpublished candidate.

Earlier runs exposed a scalar ownership proof cycle, circular reference naming
and cyclic positional ordering. Each received an implementation fix and focused
regressions; the final run passed those frontiers. Failed and deliberately stopped
attempts remain distinguished in the command summary.

Integrated verification passes: workspace formatting, Clippy with warnings
denied, workspace tests, strict workspace Rustdoc, both grammar stale checks,
metamodel stale checks, both language runtime gates, frontend check/build/tests,
and standards validation. The final workspace tests and Rustdoc include the last
shared-query changes. Browser tests were not run: this generation-2 language work
does not affect the operational browser/frontend integration. These passing
checks do not override the failed publication acceptance run.

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
  observed corpus outcomes, remaining endpoint/producer findings, and capabilities
  that could not yet be certified.
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
