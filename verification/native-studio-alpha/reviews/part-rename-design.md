# Bounded Part rename

`ModelCommand::RenameElement { element, name }` now prepares a real source-backed
Working candidate through `ModelingService::prepare_part_rename`. It does not
write presentation labels or fabricate a fixture revision. The operator must
still validate, review and commit the exact candidate through the existing
authority checks and durable branch compare-and-swap.

## Supported scope

The target must be an authored PartDefinition or PartUsage on the exact current
Validated branch head. Its source header must have this shape:

```sysml
part [def] Name [ : Qualified::Type ] { /* existing body */ }
// or the equivalent semicolon body
```

Both old and new names are plain ASCII identifiers of at most 120 characters.
Whitespace/ordinary line comments between header tokens are retained byte for byte. The body
can contain other supported declarations; rename does not change its contents.
Short names, quoted/Unicode names, modifiers, specialization/redefinition,
multiplicity and other complex headers are explicitly refused. Reserved words,
source injection, no-op names and duplicate direct sibling names also refuse.

References are not rewritten. The service refuses a rename that breaks or changes
any existing reference resolution. It also refuses other effective graph changes,
including changes to implied relationships. Initial native UI scope can therefore
be described as **Rename Part**, with these restrictions explained by source
command refusals. Successful preparation is the final capability decision for a
specific new name; an authored Part icon alone does not guarantee it.

## Identity proof and semantic boundary

The public service request accepts intent and the durable revision binding only.
It accepts neither caller edits nor checkpoints nor identity mappings. The service
checks canonical source provenance and derives the exact declared-name token edit.

After parsing the edited source, every old production must map uniquely by kind
and transformed byte range. Production count and child ownership/order must be
identical: zero additions, zero removals. Restoring the service-derived IDs must
pass the parser's independent complete arena shape checker. Outside that single
name token, source bytes remain identical.

`source_identity` extracts the insertion command's internal checkpoint restoration
and continuity checks. The insertion policy and receipt remain
`agentique-source-identity/part-insertion/1`; its ownership append exception is
unchanged. Rename has a separate `agentique-source-identity/part-rename/1` receipt
and one exact slot exception: the selected element's canonical
`ELEMENT_DECLARED_NAME`, checked against both old and requested values.

Full ordinary reconstruction must retain all old declared IDs, kinds, owners,
other slot values and exact Complete reference targets. Rename also requires an
unchanged canonical record/occurrence population and unchanged retired identity
reservations. It requires actual complete producer closure. Effective graph IDs,
kinds, slot values, relationship endpoints and endpoint ordering must remain
identical except for that one declared-name value. Source origins, evidence and
revision labels are reconstructed normally and are not copied from old results.

The existing naming implementation calculates effective names and namespace names
through semantic queries; alias membership names are separate authored values.
This makes an unreferenced plain Part a plausible usable scope. If actual accepted
runtime data has materialized derived naming consequences, the strict guard will
refuse them. It is not weakened merely to make the first example succeed.

No standard source, producer rule, accepted identity or receipt was changed.
There is no incremental reconstruction or semantic latency improvement claim.

## Qualification

The added proof tests cover repeated longer/shorter renames, complete ID-set retention,
unchanged surrounding source, typed and nested declarations, comments, and
unsupported or injected input. An isolated kernel comparison test exercises the
guard against extra records, unrelated name changes and non-name slot changes;
it is not a semantic fixture acceptance claim.

| Check | Observed result |
| --- | --- |
| [Service and agent library tests](../checks/part-rename-focused-tests-fixed.json) | Exit 0; service 15 passed / 1 unrelated cache benchmark ignored; agent 10 passed |
| [Service, agent and platform all-target Clippy](../checks/part-rename-clippy.json) | Exit 0; also compiles the extended real gate |
| [Workspace formatting](../checks/part-rename-fmt-final.json) | Exit 0 |

Rust builds used the existing integration worktree's `target`, disabled debug
information/incremental compilation, and one build job. Receipts retain exact
commands, environment, elapsed time, exit code and output hashes. The original
[test failure](../checks/part-rename-focused-tests.json) and
[diagnostic run](../checks/part-rename-proof-diagnostic.json) exposed a test fixture
using a block documentation token between `part` and its name, which the pinned
grammar does not accept. The fixture now uses an ordinary line comment; neither
parser support nor proof completeness was weakened.

The real platform gate additionally attempts to rename ProjectWorkspace, which
has live references, and requires refusal; checks duplicate sibling refusal;
renames the newly inserted parent Part while retaining its typed nested child;
validates/commits; and reconstructs the durable renamed revision from source after
removing only its optional project graph cache blob in the isolated test database.
The final new service reuses the immutable standard publication object after its
earlier runtime authentication, with no project revisions retained in memory.

That real gate is **not executed yet**: the accepted runtime pair is unavailable.
Compiled code and source-only proof tests do not establish real rename acceptance.
