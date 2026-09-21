# Focused workstream checks

Source work ran in isolated worktrees against the fetched base. The table records
successful focused checks; diagnostic failures are described below and in
[commands.json](commands.json). Raw output stays in ignored generated storage.
Compilation used the shared target directory and at most two Rust workers.

| Integrated work | Actual command | Result |
| --- | --- | --- |
| Kernel `7e5f9e1`, `cc5a542` | `cargo test -p agq-kernel --locked --offline` | Passed; includes derived occurrences, immutable dependencies, scale fixture and Rustdoc |
| Kernel | `cargo clippy -p agq-kernel --all-targets --locked --offline -- -D warnings` | Passed |
| Kernel | `cargo fmt -p agq-kernel` | Passed |
| Semantic digest `a8c38ea` | `cargo test -p agq-kerml-semantics --locked --offline context_digest --lib` | Six tests passed in 1.02 s; 60,000 premises encoded once across 20,000 outputs, 19,999 proof reuses |
| Semantic digest | `cargo test -p agq-kerml-semantics --locked --offline --test foundation --test profiles` | 29 foundation and two profile tests passed |
| Semantic digest | `cargo clippy -p agq-kerml-semantics --lib --locked --offline -- -D warnings` | Passed |
| Semantic digest | `cargo fmt -p agq-kerml-semantics` | Passed |
| Reference refinement `3039f8f` | `cargo test --locked --offline -p agq-kerml-text --lib library::refinement::tests -- --nocapture` | Four tests passed in 1.24 s; five-round cached/full-scan equivalence, negative search, occurrence retarget/removal and pending context changes |
| Reference refinement | `cargo clippy --locked --offline -p agq-kerml-text --lib --tests -- -D warnings` | Passed |
| Compact invalidation `38ac48b` | `cargo test -p agq-kerml-semantics --locked --offline --lib read_dependencies::tests` | Two tests passed in 0.19 s, including 262,144 full/compact invalidation comparisons |
| Compact invalidation | `cargo test -p agq-kerml-text --locked --offline --lib library::refinement::tests` | Four tests passed in 1.30 s |
| Compact invalidation | `cargo clippy -p agq-kerml-semantics -p agq-kerml-text --lib --locked --offline -- -D warnings` | Passed |
| Slice dependency closure `fa81149` | `cargo test --locked --offline -p agq-kerml-text --test publication_slice_dependencies -- --nocapture` | Two tests passed in 0.44 s; reproduces old omitted-owner false positive and verifies canonical Function result binding after closure |

Kernel scale covers 60,000 declared subjects, 30,000 derived Elements and 20,000
derived AssociationOccurrences over three frontiers, with 110,002 dependency
edges considered. The separately monitored native invocation is recorded in
commands.json; the early Python-launcher resource sample is superseded.

These focused passes do not establish whole-library publication. The first
semantic 60k-subject scale attempt was stopped at 4 GiB approximately 53 seconds
into the test, after 273 seconds of compilation. That failure blocked a corpus
attempt and led to investigation of irrelevant incoming-relationship premises.

An isolated release probe for the source-endpoint fix was stopped during
descriptor compilation at the lead's request: exit 124, 108.125 seconds,
1,778.1 MiB peak private memory, no semantic test execution. Subsequent release
scale compilation stays in the main worktree to avoid repeated path-dependent
descriptor optimization. Its four focused endpoint tests had passed in 0.48 s;
the worklist suite passed ten tests in 19.38 s, including the 512-subject
independent full-scan comparison. Two explicit scale gates were ignored.

Final read-only cleanup review checked 505 local Markdown links and 161 retained
scripts with 894 existing literal data references. No deleted input remained a
literal script target; all eight explicitly retained raw proof inputs matched the
base bytes. An initial link-audit exit 1 identified 22 broken relative links in
three frozen historical documents; a follow-up exit 0 established that all 22
already existed in byte-identical base copies. No cleanup-created link failure
was found. Normative PDFs/HTML/KPAR/ZIP/XMI, locks, operational errata and
generation-1 coverage/traceability had an empty diff against the fetched base.
Six raw-output ignore probes were ignored, while curated summaries stay trackable.

The later kernel incoming-property index (`519f03c`), together with allocation-free
registry lookup and deterministic metaclass iteration, passed
`cargo test -p agq-kernel --locked --offline`: **82 unit/integration tests and one
doctest**, exit 0. This is the exact count from the current log; it supersedes
informal earlier counts. `cargo fmt -p agq-kernel -- --check` and `git diff --check`
also exited 0. Tests include indexed/full-filter equivalence for every reference
carrier and a 4,096-relationship fan-out fixture.

The semantic source-role index (`acc1572`) passed these exact commands with
`CARGO_BUILD_JOBS=2` and the shared main target directory:

- `cargo test --locked --offline -p agq-kerml-semantics --lib relationship_sources -- --nocapture`: exit 0, one descriptor-extension test.
- `cargo test --locked --offline -p agq-kerml-semantics --lib producer_worklist_tests`: exit 0, ten passed and two explicitly ignored scale gates.
- `cargo test --locked --offline -p agq-kerml-semantics --test relationship_population`: exit 0, four tests.
- `cargo clippy --locked --offline -p agq-kerml-semantics --all-targets -- -D warnings`: initial exit 101 for a needless test struct update; removing that redundant update produced exit 0.
- `cargo fmt --all -- --check` and `git diff --check`: exit 0.

Raw logs use `source-role-index{,-worklist,-population,-clippy,-clippy-fixed}.log`
under ignored generated storage. A final independent read-only scheduler review
found no concrete invalidation, applicability or frontier-order defect; it did
not replace executable equivalence fixtures.

An attempted removal of inactive ignored build-cache directories was rejected by
automatic approval review as “blocked by policy”, before command execution. The
directories were retained. No alternate deletion mechanism was attempted.

The v8 producer-phase fix (`43ac3a8`, integrated as `90877b3`) passed
`cargo test --locked --offline -p agq-kerml-semantics`: exit 0, 163 passed and two
ignored scale gates across 32 result summaries. The exact all-target Clippy
command, `cargo clippy --locked --offline -p agq-kerml-semantics --all-targets -- -D warnings`,
also exited 0, as did formatting and diff checks. Raw observations are
`semantics-context-strata-package.log` and `semantics-context-strata-clippy.log`.

Two new reference-binding fixtures cover changing contexts caused by valuation
and invocation specialization. Their worklist runs span four orders and three
batch sizes, comparing full graph/provenance/query/digest results to independent
reference closure and requiring exactly the final context binding. The focused
pair passed in 11.91 seconds. A separate negative fixture proves that a failed
deferred binding and an exhausted structural-only stage budget cannot establish
publication completion.

The first candidate fixture failed with result arity after inheriting a second
return Feature. Explicit result redefinition corrected that incidental defect;
the valid fixture then reproduced the intended TypeFeaturing collision (exit
101, 0.49 seconds) before the producer-phase fix. Those diagnostic failures are
not semantic acceptance passes.

Empty-frontier reuse (`dc942d0`, `afc21cb`, `c9efbc1`) retains the immutable overlay
only after validating every proposed record. The independent reference still
materializes the frontier. All six pending write classes count as changes,
including failed computations and search-only metadata. Retained scheduler
subject/read-key edges are now counted, and empty reverse-index buckets removed.

- `cargo test -p agq-kernel --test derivations --locked --offline`: exit 0, 17 tests, 0.01 seconds in the test body.
- `cargo test --locked --offline -p agq-kerml-semantics --lib producer_worklist -- --nocapture`: exit 0, 14 passed and two explicitly ignored, 12.35 seconds.
- `cargo clippy --locked --offline -p agq-kerml-semantics --all-targets -- -D warnings`: initial exit 101 for `items_after_test_module`; moving the module produced exit 0, 3.05 seconds.
- Formatting and diff checks: exit 0.

The semantic commands used two build jobs, no debug symbols and no incremental
cache; the kernel command disabled incremental caching. Raw observations remain
in the workers' ignored generated directories. These checks include unchanged
proof metrics on reuse and replacement/pruning of retained read edges.

Structural-search interning (`ff734cf`, `be043fa`) preserves the borrowed public
search APIs while sharing immutable sets across facts and overlay revisions.
`cargo test -p agq-kernel --locked --offline` exited 0: 90 unit/integration tests
and one doctest. The three shared-search tests took 0.27 seconds. Their scale
fixture has 12,000 facts with 6,000 identical negative-search keys: 72 million
logical entries retain exactly one 6,000-entry set. The next owned frontier
retains that same allocation; exact unions, caller isolation, failed-computation
metadata and immutable snapshot sharing are checked. Kernel all-target Clippy
exited 0 in 3.47 seconds. Commands used two jobs, no symbols and no incremental
cache; raw logs remain under the worker's ignored `overnight-shared-searches/`.

Package-anchor scope correction (`0b69c3e`) passed five dependency-slice tests:
`cargo test --locked --offline -p agq-kerml-text --test publication_slice_dependencies -- --nocapture`,
exit 0, 0.37 seconds. Targeted example/test Clippy exited 0 in 5.10 seconds.
Formatting was corrected after an initial check failure; final formatting and
diff checks exited 0. Explicit selected Packages still expand, concrete Feature
anchors still bring their owning Function, missing anchors fail, and the final
read boundary remains mandatory. Whole Packages used only as binding lookup
anchors no longer unconditionally schedule all their members.

Producer-side search sharing (`79a9a84`, integrated as `f8a5c18`) passed
`cargo test --locked --offline -p agq-kerml-semantics`: exit 0, 167 passed and
two explicitly ignored scale gates across 32 result summaries. The first build
exited 101 for an unqualified import in a nested test initializer; correcting
that import preceded the successful rerun. All-target semantic Clippy exited 0
in 9.70 seconds; formatting and diff checks exited 0. The commands used two jobs,
no debug symbols and no incremental cache. Raw logs are
`shared-search-semantics-package{,-fixed}.log` and
`shared-search-semantics-clippy.log` under ignored generated storage.

A separate read-only review found no lost search keys, dangling allocation-cache
keys, snapshot mutation or allocation-dependent digest. The review does not
replace the exact graph/search/digest equivalence fixtures above.

The symbolic-bound fixture (`74bf6de`, integrated as `2fdefad`) passed
`cargo test --locked --offline -p agq-kerml-semantics --test publication_v8_bounds -- --nocapture`:
exit 0, two tests, 1.12 seconds. An initial build exited 101 for unavailable test
helper functions; local equivalents fixed it before the passing rerun. Targeted
Clippy, formatting and diff checks exited 0. Raw logs are
`multiplicity-reference-context-regression{,-fixed}.log` and
`multiplicity-reference-context-clippy.log`. A subsequent read-only review
confirmed the current authority contract permits no lexical-context fallback and
requested explicit final-overlay diagnostic assertions, added by the lead.
