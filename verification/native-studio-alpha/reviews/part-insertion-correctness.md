# CreatePartUsage identity review

Reviewed the service-owned insertion proof and agent command wiring through
`52ce2a6`, independently of the implementation author. This review covers the
bounded append-one-part command; it does not approve generic rename, arbitrary
source edits or incremental semantic reconstruction.

## Ranked findings and changes

1. **P2, fixed: a name anywhere in the selected source body prevented insertion.**
   `part def Platform { part subsystem { part observer; } }` incorrectly refused
   a new direct `observer`. A qualified type token such as `Devices::Sensor`
   likewise prevented a sibling named `Sensor`. The agent now intersects exact
   semantic name lookup with direct owned membership identities. Direct sibling
   names and short names still reject; nested names and unrelated source tokens
   do not. Query incompleteness refuses the command. The service's previous
   reference-target comparison remains intact, so a new shadowing declaration
   cannot silently change the meaning of an existing reference.

2. **P2, covered but execution pending: restart could exercise only the project
   semantic cache.** The real platform gate now additionally removes just the
   exact optional project graph cache blob from its isolated temporary database.
   It authenticates the accepted runtime again and requires
   `RevisionLoadPath::DurableSource`, a Validated revision and an exactly equal
   projection after reconstruction. It then inserts a typed child inside the
   first newly committed part, validates and commits again, and checks original
   ancestors, both new identities, owner relationships, branch head and immutable
   predecessor projection. This covers the semicolon-to-body owner edit after a
   source-only restart. Accepted standard artifacts, source blobs and manifests
   are not altered. The gate has compiled, but has **not run** without the accepted
   runtime pair.

3. **Acceptance limitation: the full-rebuild oracle is not a second identity
   authorization mechanism.** It recompiles the candidate's already reconciled
   source identity arena through the complete ordinary semantic pipeline. Exact
   graph, provenance, occurrence ordering, closure, reference, query and evidence
   comparisons test reconstruction equivalence. Authorization to retain each old
   identity is instead established by the separate service proof and continuity
   checks. Neither test has executed on the accepted pair during this review.

No concrete identity-transfer bypass was found in the reviewed bounded proof.
That is a source-review conclusion, not a substitute for the pending real gate.

## Identity and authority checks inspected

- The caller supplies one text edit and the selected owner, not an identity map
  or checkpoint. The predecessor must be the exact Validated branch head.
- Canonical owner kind, authored document, source revision, source range and
  production kind must agree. Standard elements cannot become editable owners.
- The parser proof requires complete Operational v3 SysML, the exact owner's
  closing brace or semicolon, one plain PartUsage and an optional simple
  qualified type name. Extra declarations, modifications outside the boundary
  and hidden syntax refuse the edit.
- Every previous syntax production maps by exact transformed range and kind;
  used-target and identity checks enforce a bijection. Existing child ordering
  survives. All new productions lie inside the inserted part. The restored
  source arena must independently match its expected complete shape.
- Only the service derives retained IDs. Other documents, accepted publication
  identity and retired identity reservations come from the authenticated
  predecessor checkpoint. Newly restored source origins update to the edited
  document revision rather than retaining obsolete source ranges.
- Full reconstruction verifies all old declared IDs, kinds, owners and declared
  slots. Only the selected owner's ordered owned-relationship collection may
  gain an appended member. Existing references must retain exact names, targets,
  resolutions and completeness. Retired canonical IDs cannot reappear.

The strict single-part grammar intentionally does not support quoted/Unicode
new identifiers, complex type expressions or arbitrary declaration edits. No
generic RenameElement claim follows from this path.

## Verification

All build commands used the existing root `target` directory, with
`CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=0`
and `CARGO_BUILD_JOBS=2` for Rust test/Clippy commands. Exact commands, output,
exit status and elapsed time are retained in the linked receipts.

| Check | Result |
| --- | --- |
| [Agent library regressions](../checks/part-name-scope-fixed-tests.json) | Exit 0; 10 passed, including direct sibling/short-name refusal and nested-name reuse |
| [Initial real platform test compile](../checks/part-source-restart-compile.json) | Exit 0; executable built, real test not run |
| [Final agent/platform all-target Clippy](../checks/part-name-restart-clippy-fixed.json) | Exit 0; includes the final typed nested restart gate |
| [Workspace formatting](../checks/part-name-restart-fmt-fixed.json) | Exit 0 |

The first hand-built query fixture omitted required feature/end and membership
visibility values, so completeness correctly refused it. Those fixture values
were added using the registry's canonical visibility enumeration; completeness
was not weakened. The [initial failure](../checks/part-name-scope-tests.json) and
[diagnostic run](../checks/part-name-scope-diagnostic.json) remain recorded.
Initial [Clippy](../checks/part-name-restart-clippy.json) and
[format](../checks/part-name-restart-fmt.json) failures were in the predecessor
proof's nested conditional/test formatting; merging the integration lead's
`52ce2a6` supplied those fixes before the successful final checks above.

The integration lead owns the complete workspace and product gates. Runtime
first light, source-only durable restart, the second insertion, full semantic
equivalence, validation latency and peak memory remain unmeasured here. This
change makes **no semantic wall-time improvement claim**.
