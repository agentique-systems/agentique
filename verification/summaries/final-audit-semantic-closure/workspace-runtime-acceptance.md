# Workspace runtime acceptance

Current disposition: **11 of 12 unique accepted-cache workspace tests have
passed**. All four smaller phase-1 tests passed separately. The recovery-scale
rerun with streamed diagnostic signatures is running; the original allocation
failure remains recorded below. N1 and N2 remain passed, and ADR 0024 remains
proposed until the final runtime result and integration review.

Initial measured record at integration source `65c1bf2`: **7 of 12 unique
accepted-cache workspace tests have passed**. The required phase N1 self-model
dogfooding and N2 validated-scale gates have passed. The malformed-typing test
also passed again with the forked evaluator; that rerun is not an eighth unique
test. Five remaining phase-1 tests are running and are explicitly pending below.
Full workspace acceptance is not yet claimed, and ADR 0024 remains proposed.

The [language-foundation readiness decision](../final-language-acceptance/semantic-closure-readiness.md)
and adopted ADR 0026 remain established. All these workspace gates consume the
actual accepted KerML Operational v9 and Systems Operational v3 publications,
with SysML query contract `agq-sysml-query/6`. The
[accepted artifact identities](accepted-artifact-gate.json),
[compiled Systems receipt](../../../standards/sysml-accepted-publication.json) and
[accepted bindings](../../../standards/sysml-standard-bindings.json) remain
unchanged. No accepted standard producer replay or new KerML acceptance occurred.

## Passed runtime gates

The [command ledger](commands.json) retains exact commands, source and working
change identities, cache paths, execution settings, outputs and exits. These are
explicit ignored-test executions, not results inferred from compilation or the
ordinary workspace suite.

| Gate / ledger entry | Unique tests passed | Exit | Test seconds | Command seconds |
| --- | ---: | ---: | ---: | ---: |
| `workspace-working-state-acceptance` | 5 | 0 | 2341.71 | 2342.64 |
| N1: `workspace-self-model-acceptance` | 1 | 0 | 840.84 | 844.09 |
| N2: `workspace-validated-scale-complete-ports` | 1 | 0 | 1227.48 | 1228.53 |
| `workspace-forked-invalid-typing-acceptance` | 0 additional; 1 rerun | 0 | 308.84 | 309.78 |

N1 opens the five real Agentique self-model documents in `ProjectWorkspace`,
creates Validated revision 1, edits ModelingPlatform and creates Validated
revision 2. The test verifies semantic architecture relationships, retained
authored identities, unchanged old source/query results and shared accepted
standard storage. This is the real workspace gate, separate from the earlier
three-revision `SourceProject` language acceptance fixture.

N2 retains **five Validated revisions, each with 100 documents: 50 KerML and
50 SysML**. Port and specialization/redefinition edits affect groups 025 and
049. Baselines are captured before subsequent edits. Four barrier-synchronized
readers make two passes across all five revisions, comparing query populations,
canonical identities, exact contexts and bounded evidence for groups 000, 025
and 049. Each revision records 72 selected query fact keys and 210 selected query
searches; these are the fixture's selected evidence, not total graph counts.
Original inherited identities, unchanged documents and old revisions remain
intact. The concluding output records:

```text
validated scale: revisions=5 documents_each=100 kerml_each=50 sysml_each=50 parallel_readers=4 read_passes=2
```

Physical storage observers verify the actual accepted backing tables, zero
copied authoritative dependency entries and original standard record identities.
Scheduler observations verify zero accepted producer subjects evaluated during
authored edits. Facade `Arc` equality alone is not the sharing assertion.

The five Working-state tests cover malformed attribute typing, unsupported
variation, temporary syntax recovery followed by explicit deletion, unresolved
reference/provider removal and repair, and unsupported source with exact origins.
The malformed attribute remains canonically typed by a PartDefinition despite
Complete references, strict construction and closed producers. Its actual
effective attribute-definition query is Invalid, and the matching retained audit
blocks validation. Repairing the definition validates a new revision/context
without changing the earlier Invalid graph, answer or report.

The full Working-state suite used the pre-fork audit implementation. The exact
malformed-typing test subsequently passed with the forked evaluator, as did N1
and N2. The [batching correction](../semantic-closure-structural/authored-audit-batching.md)
authenticates a revision once and uses fresh bounded evaluator caches. It retains
the strict effective dispatcher over all local declared and derived subjects,
accepted-subject exclusion and exact report/context matching. No finalizer or
production semantic rule was changed to obtain these runtime results.

## Rejected scale assertion and exact correction

The first validated-scale run, ledger entry
`workspace-validated-scale-acceptance`, exited **101** after **677.42 seconds**
test time (678.38 seconds command time). Three 100-document revisions had
validated before its exact-one-port assertion failed. That run remains recorded
as a failed test and contributes no scale acceptance.

The additional returned ID `7adae769-a6bb-5eb2-a1fb-2fdda0a6fdb2` is the accepted
`Parts::Part::ownedPorts` abstract PortUsage. `effective_ports` returns the full
inherited PortUsage population. The authored `bus` subsets this standard feature;
it does not redefine or suppress it. The corrected fixture therefore requires
exactly the authored `bus` identity and the accepted `Parts::Part::ownedPorts`
binding identity, preserving their canonical owners and rejecting extras. It
does not hardcode the failed run's authored ID or filter the query result.
The same test expectation was corrected in the legacy recovery-scale fixture;
that fixture's runtime result remains pending.

This is a correction to the acceptance fixture's expected population. No
production query, canonical graph rule, profile, standard receipt or finalizer
acceptance gate changed. The successful N2 rerun above independently establishes
the full five-revision/four-reader result after this correction.

## Exact execution evidence

All commands use `cargo test --release --locked --offline -p
agq-modeling-workspace --features verification`. Their remaining arguments are:

| Ledger entry | Exact remaining arguments |
| --- | --- |
| `workspace-working-state-acceptance` | `--test working_states -- --ignored --nocapture --test-threads=1` |
| `workspace-self-model-acceptance` | `--test self_model -- --ignored --nocapture --test-threads=1` |
| `workspace-validated-scale-acceptance` and `workspace-validated-scale-complete-ports` | `--test phase1 hundred_documents_five_validated_revisions_and_four_parallel_readers -- --ignored --exact --nocapture --test-threads=1` |
| `workspace-forked-invalid-typing-acceptance` | `--test working_states closed_malformed_attribute_typing_stays_working_until_repaired -- --ignored --exact --nocapture --test-threads=1` |

The ledger additionally retains `AGENTIQUE_KERML_CACHE` and
`AGENTIQUE_SYSTEMS_CACHE`, the shared target location, disabled incremental/debug
artifacts, two build jobs and disabled release LTO. These execution settings do
not replace semantic assertions.

| Execution | Tested source | Working-change SHA-256 | Output SHA-256 |
| --- | --- | --- | --- |
| Working suite | `5bdbc60a6f6c59fa4fa9b6caa3515fe3584b4a8c` | `8d8850168f1ef9eea9541c43c49e0e03cffb9a894b584f9f2588df93a34b9620` | `4e8f00ed3ffcee61e0e4fdf662ab8f0de6e9bf233150c89937d681f8c972ebca` |
| Workspace self-model | `80dea6dbd6e477cdbe34daddabd4fb0b015a4c1e` | `7f283880bd9e2bbd590a9605d392941a5d87bc60f8ec69ffa3587b55a511302b` | `b35a6440c852af94e7790f41bb541f16f88891caabce624543c4e2203a4e3be5` |
| Rejected scale expectation | `61f2147660110dc5b027db4a784ffb249dd656e6` | `7f283880bd9e2bbd590a9605d392941a5d87bc60f8ec69ffa3587b55a511302b` | `4ddaf179e4f5d1f070d60e3fe9a16124e1d74065cfad9d83e72130fd9bc45db8` |
| Corrected validated scale | `42bcc8974a91688301f8f80f04e3d79f7db024e6` | `24c5bed541a456892bc12d58492d2b7d334e1daa15364acfb95375f3f8086fb9` | `f86d8b22a671107ca8a0db336f672cde6d7487a36651abf8d7a5283fe522aed9` |
| Forked malformed-typing rerun | `2b6f04c1778c2274f4610a396f01476a656c957c` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` | `901f487f5537e27dc9427a67f7a71224514c56bd450a13833306b5f1c0078721` |

Raw logs are retained at
`verification/generated/final-audit-semantic-closure/<ledger-entry>.log`.
The source and working-change fields distinguish tested code from later
documentation commits; the failed and successful scale logs are separate files.

## Remaining five runtime tests

At the initial snapshot, the remaining `phase1` run was in progress. No
completed exit or passing result was asserted for:

- `hundred_documents_five_revisions_and_parallel_borrowed_reads`
- `mixed_documents_and_edits_retain_old_revisions_and_inherited_ids`
- `operational_failures_publish_nothing_and_independent_projects_share_only_standards`
- `recovered_and_unresolved_edits_are_working_then_repair_to_validated`
- `removal_and_readding_a_path_does_not_resurrect_retired_identity`

The first is the recovery-scale sequence: its fourth retained revision has
99 documents and is Working after provider removal; the fifth repairs to 100.
Four readers make eight passes across all five retained revisions. This is
distinct from the now-passed N2 test, where all five revisions are Validated
and retain 100 documents.

## Scope and decision boundary

The integrated workspace is in-memory and provides atomic visibility of exact
source/semantic revisions. Authored semantic reconstruction remains permitted;
shared standards are immutable and are not copied or re-produced on edits.
The [architectural retrospective](core-foundation-retrospective.md) records the
identity, evidence, dependency and validation boundaries. Existing kernel/cache
and ordinary repository checks remain separately recorded in the ledger.

Completing N1 and N2 does not convert the five pending tests into passes. ADR
0024 adoption and a full workspace acceptance statement await those outcomes
and final integration review. ADR 0027 remains proposed research. Persistence,
application/server migration, diagrams, full language conformance, execution and
simulation remain outside this phase; Gen1 obligations remain unchanged.

This record was prepared from existing code, logs and command evidence. No
build, cache load, producer or runtime test was executed to write it.

Documentation checks: `git diff --check` exited 0 with empty output. The command
below exited 0 and printed `7 local links valid`:

```text
python -c "import pathlib,re; p=pathlib.Path('verification/summaries/final-audit-semantic-closure/workspace-runtime-acceptance.md'); links=re.findall(r'\]\(([^)]+)\)',p.read_text()); assert all((p.parent/link.split('#')[0]).is_file() for link in links); print(str(len(links))+' local links valid')"
```

## Resource-interruption update

The subsequent `workspace-phase1-remaining-acceptance` command terminated after
**916.59 seconds of command time**, with exit **3221226505**. It began the first
of the five selected tests,
`hundred_documents_five_revisions_and_parallel_borrowed_reads`, and logged:

```text
memory allocation of 8589934592 bytes failed
```

The log contains no semantic assertion failure, completed test result, or
passing test count. This is a failed command with an unresolved recovery-scale
test, not evidence that any of the five selected tests passed. Allocation
failure alone does not establish the underlying cause or the semantic outcome.
Static review is investigating expansion of diagnostic `Debug` output into a
`String`; that is a hypothesis, not a cause confirmed by a runtime trace.

The command was:

```text
cargo test --release --locked --offline -p agq-modeling-workspace --features verification --test phase1 -- --ignored --skip hundred_documents_five_validated_revisions_and_four_parallel_readers --nocapture --test-threads=1
```

| Evidence field | Recorded value |
| --- | --- |
| Source commit | `65c1bf261512772a7f4b592f3749f6b430556c3d` |
| Working-change SHA-256 | `24c5bed541a456892bc12d58492d2b7d334e1daa15364acfb95375f3f8086fb9` |
| Output SHA-256 | `61d81945fdd3275bae56e56552ed987b8e6bbaf54001cb114b34ce15d3453f49` |
| Raw output | `verification/generated/final-audit-semantic-closure/workspace-phase1-remaining-acceptance.log` |
| Command duration | 916.59 seconds |
| Exit | 3221226505 |

The raw output SHA-256 was independently recomputed and matches the command
ledger. The previous measured records, including the rejected and corrected N2
scale runs, remain intact.

The four smaller tests listed above are now running separately under
`workspace-phase1-four-edit-gates`; the observed log starts with `running 4 tests`
and the mixed-document test. No completed exit or passing result is claimed for
that command here. The recovery-scale test remains pending investigation and a
successful rerun. Thus the current count remains **7 of 12 unique tests passed**,
plus the already-recorded malformed-typing rerun. Required N1 and N2 remain
passed. Full workspace acceptance and ADR 0024 adoption remain pending.

Update verification: `git diff --check` exited 0 with empty output, and the
local-link command above exited 0 with `7 local links valid`. No build, cache
load, producer, or runtime test was run to prepare this update.

## Four lifecycle tests completed

`workspace-phase1-four-edit-gates` exited **0** with **4 passed, 0 failed,
0 ignored**, in **1802.72 seconds test time / 1803.44 seconds command time**.
It explicitly selected the four non-scale ignored `phase1` tests with
`--ignored --skip hundred_documents --nocapture --test-threads=1`. Mixed edits,
operational failures/independent sharing, syntax/unresolved recovery and repair,
and removal/re-addition identity retirement all passed. The command tested
`5c5b09be7ffa00166767a531184353d08d2b4a4b` with an empty working change
(`e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`);
its output SHA-256 is
`2b62ce15e1e9eda5444ee0790b74bff0ac01415e971dc5bf8e9067f6b83ac5da`.
These results establish **11 of 12 unique workspace acceptance tests passed**.

After that executable exited, the diagnostic-streaming fixture was rebuilt.
The two exact evidence-signature regressions passed on the integration checkout
(`workspace-streamed-evidence-signature-regressions`, 0.06 seconds test time).
The final `workspace-recovery-scale-streamed-diagnostics` execution is now
running. It preserves every diagnostic/reference field and all 160 reader
comparisons, while streaming the same Debug bytes into SHA-256 plus byte count.
Stage markers measure each revision, signature and reader pass. This is a
test-memory change, not a production semantic change or a passed scale result.
