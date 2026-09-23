# Gen2 modeling workspace Phase 1

Status: reviewed implementation integrated after the measured
[language readiness decision](../final-language-acceptance/semantic-closure-readiness.md).
Workspace acceptance remains pending. The preparation outcomes below retain
their original source commits and do not establish acceptance of this integration.

The earlier [language rejection](../language-foundation-accepted/README.md) is
historical and remains preserved. The reviewed recovery commits are
`427d29b2633d5ac1183e0338df96d4ef4300552e` and
`1ee7be295f43266c7f480b78865f3fd60a0874e9`. They are source inputs for this
integration, not independent acceptance authority.

The prepared implementation exists at actual commit
`e5a65f8b3f6413b3d2094dc88c172cbba6435c3d`, retained by
`foundation/workspace-authenticated-frontier-held` (HEAD `8e16e36`). It was applied
cleanly to inspected main `0c4c0adae19125c5a72b3e3521c7df1c6c6d84f0` in branch
`platform/workspace-integration` as `427d29b`.

The crate supplies ProjectWorkspace, distinct project revision IDs, immutable
Working and checked Validated handles, mixed source edits, native borrowed query
facades and source/element lookup. Additive frontend inputs preserve recovery and
unresolved construction. Kernel history preserves retired identities; shared-base
storage avoids standard table/proof copies. Gen1 remains separate.

The ten prepared accepted-cache tests include the five-document Agentique
self-model edit, explicit effective ModelingPlatform membership changes,
unchanged authored identities, invalid Working repair and the 100-document /
five-revision / four-reader fixture. They inspect actual shared storage and
producer evaluation subjects, not just publication Arc equality. Requested
accepted-cache tests fail if either dependency cache or compiled authority is
missing; they never publish standards or skip a requested gate.

Actual command arguments, exit codes, source/patch identities and output digests
are in [commands.json](commands.json). Raw logs remain ignored under
`verification/generated/modeling-workspace-phase1/`. Initial non-compiling
preflight passed formatting and the generation dependency audit (20 packages,
zero violations; all normal/build/dev/optional/target dependency kinds), with eight
boundary-check regressions. The feature-enabled kernel/frontend/workspace test
compilation passed, including all ten workspace acceptance tests. The full kernel
package passed 139 tests including doctests; the current-input frontend passed
three focused tests. Strict Rustdoc and accepted-cache execution remain pending.

Earlier parser preparation and neutral kernel-oracle outcomes remain in the same
command ledger with their original source commits and failed held-crate command.
Those historical missing-manifest failures do not describe the prepared crate.
The original retained branch preserves its detailed preparation notes; they are
not imported as a second command-log collection for this milestone.

[ADR 0024](../../../docs/adr/0024-gen2-modeling-workspace.md) describes workspace
ownership and validation. It remains proposed until acceptance. The
[Phase 2 roadmap](../../../docs/modeling-platform-phase2-roadmap.md) defines
repository, branch, application/query and Systems Modeling API boundaries only;
none of those services is implemented here.
