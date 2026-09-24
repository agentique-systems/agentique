# Source command and candidate integrity review

The independent review found and fixed two issues in the agent/service boundary:

1. Revalidating a retained Validated candidate previously regenerated its durable
   manifest (including creation time) with the same operation ID. A commit whose
   acknowledgement was lost could therefore no longer replay after another
   Validate request. `PreparedChanges::validate` now returns the exact retained
   request for an already Validated candidate. It preserves metadata/cache bytes,
   project revision identity and the original CAS expectation.
2. A selected definition's declared qualified name can be shadowed in the
   selected owner's namespace. After compiling a typed CreatePartUsage command,
   the agent now locates the inserted declaration by its exact source range,
   syntax identity and current canonical source origin, then verifies its
   authored FeatureTyping target set equals the selected ElementId. A different
   or unresolved target rejects the proposal. Endpoint properties are resolved
   through the registry, preserving FeatureTyping property redefinitions.

Bounded tests execute without accepted publication caches. They check insertion
inside the selected PartDefinition in a package (retaining its sibling), source
range identification of the created PartUsage, and rejection of a valid canonical
typing to a different definition. They use real syntax and the real Gen2 kernel
registry. Six agent tests and seven service unit tests pass; one unrelated legacy
cache characterization remains ignored. Focused Clippy passes. Commands, outputs
and exit codes are in `agent-review-commands.json`.

The lost-acknowledgement revalidate/recommit scenario requires a real prepared
semantic revision. Its accepted-cache integration regression belongs to the
Studio vertical; the unit results above do not claim that unavailable runtime
gate passed.

The same review reported candidate rejection/commit races, retained completed
candidates consuming all proposal capacity, and a check-then-prepare capacity
race to the integration lead, who owns the host lifecycle corrections. It also
reported empty/equal configured operator and machine capability tokens. Host
verification and final disposition belong to the integrated Studio evidence.

The subsequent full-path review found that `publish(None)` could nevertheless
reuse colliding records already in its base. The integration lead identified
this gap in the initial review. Element and occurrence reuse now explicitly
require an incremental reconstruction base; `None` unconditionally creates and
links, preserving the original kernel collision rejection. A new regression
checks no-cache and reuse-disabled paths reject an existing identity without
mutating the base. All four focused reconstruction oracles pass after the fix.

Publication constructors supply no lowering cache or prior authored strict
snapshot, and therefore retain that full construction path. New work counters
are not source checkpoint or semantic-cache identity inputs. No standard bytes,
descriptors, profile versions or query contracts changed. The Cargo.lock diff
adds only three platform packages; existing versions/dependencies are unchanged.
This source review and the recorded regressions support refreshing the
non-authoritative interpretation-input freshness ledger. They do not authorize
changing accepted receipts, bindings, profile/publication identities, or claiming
a fresh accepted-corpus publication run.
