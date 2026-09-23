# Modeling workspace phase 1 preparation

MODELING WORKSPACE PHASE 1 REMAINS GATED

The [design and acceptance matrix](../../../docs/modeling-workspace-phase1-design.md)
and [ADR 0024](../../../docs/adr/0024-gen2-modeling-workspace.md) select an additive
`agq-modeling-workspace`. No production crate is added by this preparatory work.
The [stability contract](../../../docs/adr/0026-language-foundation-stability-contract.md)
remains proposed until the integration readiness record adopts it.

Prepared decisions cover immutable document/revision ownership, transitive
accepted Systems/KerML sharing, construction-backed Working revisions, checked
Validated handles, explicit query availability, atomic edit publication and
revision-specific diagnostics. The matrix covers mixed documents, three retained
edits, identity reconciliation, remove/repair, syntax recovery, stale/invalid
edits, closure invalidation, shared standards, trusted dependency restoration,
programmatic equivalence and 100-document parallel reads.

These are design/test obligations, not executed workspace acceptance tests.
Existing frontend strict-snapshot requirements must be addressed explicitly after
the gate. SourceProject currently rebuilds authored dependencies; no incremental
workspace performance or accepted Systems integration is claimed here.

The subsequent [frontend boundary review](../../../docs/modeling-workspace-frontend-boundary.md)
inspects the prepared accepted-publication SourceProject path. It specifies an
additive immutable input/result carrier, obligation-backed query facade, recovery
and removal handling, identity history and 100-document read/scaling assertions.
Its concrete input fixture has real-frontend parser preflight tests; results are
recorded in the `frontend-boundary.json` source-ledger range of [commands.json](commands.json). Those tests do not instantiate a workspace
or establish semantic acceptance. No production workspace has been integrated.

## Working-state acceptance preparation

Reviewed the strict accepted frontend at `695e67e`. `SourceProject::apply` still
rejects recovered production inputs, and `lower_accepted_source` still requires
strict promotion and supplies empty pending-scope sets. The additive Working
frontend and workspace remain necessary; neither is implemented by these tests.

Three held tests in `crates/modeling-workspace/tests/working_states.rs` require:

- targeted recovery/repair preserves a disjoint declaration's reconciled syntax
  and authored identity, while later explicit deletion/re-addition retires it;
- a failed mandatory reference retains its current document/source origin and
  incomplete effective query evidence, with no stale old endpoint;
- Parsed but unsupported variation remains Incomplete and cannot validate even
  when the actual producer status and whole-graph certificate are Complete.

The separate two-test `workspace_working_inputs` preflight passes against the
real Operational v2 parser (0.05 seconds). It confirms complete and malformed
source classifications, exact source roundtrip, disjoint syntax identity across
recovery and repair, and fresh identity for a new document. It does not establish
canonical lowering, workspace acceptance or publication success.

Targeted parser Clippy and both workspace/held-file formatting pass. The requested
held integration command exits 101 because its crate manifest does not exist;
the three semantic tests remain uncompiled and unexecuted. No manifest, Cargo
member, production code, accepted cache load or publication run was added.
The `working-states-commands.json` source-ledger range of [commands.json](commands.json) records commands, exits, hashes and source identity.
