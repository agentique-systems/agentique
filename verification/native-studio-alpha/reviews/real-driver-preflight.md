# Real native first-light driver review

Reviewed the actual integration worktree's six `models/agentique/*.sysml` files,
bootstrap sequence, projection defaults, inspector queries and ordinary native
input handlers. The held self-model changes were not used for the source check.

**Fixed driver defect:** bootstrap's parent revision contains the original five
architecture documents; its head adds the disconnected Agent Fabric document.
The original journey asked for a visible change in the default System World
projection, whose bounded ownership/typing neighborhood can omit every added
agent object. The journey now uses Graph World and the ordinary **Show loaded
graph overview** command before comparing with the parent. It requires the exact
parent, matching projection definitions and the actual added `AgentRuntime`
canonical node. The subsequent History selection restores the baseline and
returns to System World before creating a part.

The unchanged checks were traced to these concrete contracts:

- `ModelingPlatform`, `ModelRepository`, `ProjectWorkspace`, `Agentique` and
  `AgentRuntime` are unique authored PartDefinition selectors. Semantic graph
  projections used by the platform gate are not restricted to an architecture
  neighborhood.
- `ModelRepository` specializes `Repository`, which declares
  `repositoryRevisions`. The Inspector checks real effective features, so it can
  inspect this inherited PortUsage without copying the port into its subtype.
- Requirements World clears the previous graph focus before requesting its view;
  the prior repository dependency view therefore does not constrain its seeds.
- The CreatePartUsage owner has a plain authored body and all three insertion
  names are unused. The rename refusal target is `ProjectWorkspace`, which has a
  plain header and existing type references; `ModelRepository` would have been
  an unsuitable refusal test because its specialization header is outside the
  bounded rename contract.
- Bootstrap retains the pending worker request through progress messages. The
  driver does not mistake an ongoing runtime restoration for an empty project.
- Current viewport dragging uses the canvas response to pan regardless of which
  node lies beneath the pointer; the driver's primary-button background pan is
  consistent with that existing input handler.

[Textual source preflight](../checks/real-driver-source-preflight.json) exited 0
and retains the exact hashes of the integration worktree's source bytes. The
script explicitly reports that it performs no parsing, runtime authentication or
semantic validation. [Native formatting](../checks/real-driver-preflight-format.json)
also exited 0. No standards restoration or build was started for this review.

Still requiring actual first light: the chosen default architecture root and its
visual density; an unoccluded hit-testable derived edge; complete inherited port
queries; runtime closure and source-command reconstruction; real candidate
validation/commit; restart equality. This review does not establish those results.
