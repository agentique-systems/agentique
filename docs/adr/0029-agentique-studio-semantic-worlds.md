# ADR 0029: Agentique Studio semantic worlds and reviewed agent proposals

Status: implemented boundary; accepted-publication product runtime acceptance pending.

Agentique Studio is a Generation 2 operator surface at `/studio`. The legacy
Generation 1 Console remains at `/expert` on its existing host. Their semantic
engines, release obligations and persistence contracts remain separate.

```text
Studio (React presentation)
    -> agq-studio (loopback HTTP host, access control, metadata)
        -> agq-modeling-view -> agq-modeling-workspace -> language/kernel
        -> agq-modeling-agent -> modeling-service + view
        -> agq-modeling-service -> repository -> workspace
        -> agq-modeling-sqlite -> repository
```

`ViewDefinition` is versioned platform metadata: selection, bounded context,
relationship families and presentation omissions. It is not a modeled SysML
View/Viewpoint. The SQLite sidecar stores definitions, never semantic node copies.
Every projection, source location, inspector and explanation binds an immutable
project revision. Layout, camera, selection, visibility and semantic detail are
presentation state. Removing something from a view never deletes a model element.

System World begins with canonical parts, ownership and typing; Graph World
exposes local relationships and deliberately expanded adjacent standards.
Requirements selects actual requirement/verification metaclasses and their
available semantic neighborhood. Absent satisfaction or verification facts are
not invented. Inspect preserves effective-query completeness and dependencies;
Why projects bounded existing canonical provenance with explicitly marked proof
arrows. Connector spokes retain their canonical connector and endpoint order.

History reads durable branch heads and parent relationships. A diff explicitly
names both revisions; removed objects are inspected against the earlier binding.
No viewport action reconstructs source or validates a project. Reconstruction and
validation are explicit model-work operations with bounded host concurrency.

The initial Agent Fabric is a set of contracts, not an autonomous framework:
`AgentContext`, `AgentIntent`, `AgentPolicy`, `AgentProposal`, `ModelCommand` and
`AgentCandidate`. Default machine authority is Read + Propose. Validate and Commit
are independent host-granted capabilities. A deterministic decision provider
demonstrates intent-to-view routing; Choice, Score and Boolean questions remain
provider neutral. Mock answers do not assert calibrated confidence. No provider
network transport or behavior execution has been introduced.

Only `CreatePartUsage` currently maps to a reviewed source edit. It resolves a
local authored part by canonical ID, checks the exact source/syntax binding,
constructs a lossless text edit, and delegates reconstruction to the existing
workspace. Other typed commands fail explicitly. A candidate starts Working,
can be inspected and diffed, and must pass the unchanged Phase1V1 contract before
operator commit. Validation retains the candidate identity. Durable CAS and
operation receipts control branch movement and exact retry; no UI state grants
commit authority. Rejection serializes with candidate work and cannot undo an
already committed or ambiguously acknowledged operation.

Candidates are bounded process-local review handles. They are not durable drafts;
restart discards uncommitted work. Completed handles have bounded retry retention,
while repository operation receipts remain durable. The host is a local,
single-operator application, not a multi-user identity provider. Distributed
deployments need an authenticating gateway and durable agent-job policy.

Agentique's future agents are ordinary concepts in `AgentFabric.sysml`, not new
SysML semantics. Execution IR, modeled-system execution, live model providers,
automatic merge and autonomous commits remain unimplemented.

Runtime acceptance requires existing authenticated KerML Operational v9 and
Systems Operational v3 caches. Startup never acquires or republishes standards,
and missing inputs cannot be substituted with a sample graph or a Validated badge.
