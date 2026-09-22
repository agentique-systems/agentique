# Actions, variable domains and authority preflight

Tested implementation: `b595959a3145ff85e52d83a76f1bffe28ad6cc1b`, plus the
two-document authority fixture in this commit. Its SHA-256 was
`9d4e550b327608fd26e88f354481c76387ed4c503c4eeca68221a81483de3e17`.
Tools: rustc 1.92.0 (`ded5c06cf`), cargo 1.92.0 (`344c4567c`).

SysML registers each implemented producer rule independently, including both
evaluated AcceptAction alternatives and the separately staged `mayTimeVary`
scalar family. Descriptors declare effects before execution. A negative scalar
premise needs scheduler closure; the current-graph query remains separate.
Positive standard generalizations use their actual antecedents and exact target;
unfinished exhaustive ancestry cannot suppress a proved edge. The regression
retains Incomplete for the unresolved authored specialization and missing target.

The Actions micro fixture uses synthetic immutable standard anchors. It proves
the generic effective-owner negative predicate is Incomplete before closure and
Complete with the certificate, resolves `accepter` and `acceptedMessage`, and
retains the inherited payload's original identity. A second fixture activates
the existing KerML snapshot domain from certified SysML `mayTimeVary`, then
checks the strict FeatureValue binding uses that same domain. Neither fixture
confers a library publication receipt.

All commands used `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, and `CARGO_BUILD_JOBS=2`. Tests used the isolated
`target/foundation-actions` target: concurrent worktree builds against one target
can replace each other's same-package executables. Disk preflight found
51,332,349,952 bytes available before these focused builds. No caches were deleted.

| Command | Exit | Actual result |
| --- | ---: | --- |
| `cargo fmt --all -- --check` | 0 | Clean |
| `cargo clippy -p agq-sysml-semantics --all-targets -- -D warnings` | 0 | Clean |
| `cargo test -p agq-sysml-semantics` | 0 | 48 passed; 0 failed; 10.84 s |
| `cargo test -p agq-kerml-semantics --lib variable -- --nocapture` | 0 | 4 passed; 10.10 s |
| `cargo test -p agq-kerml-semantics --lib feature_values -- --nocapture` | 0 | 1 passed; 15.38 s |
| `cargo test -p agq-sysml-semantics operational_v2_authority -- --nocapture` | 0 | 1 passed; 0.49 s |
| `cargo test -p agq-kerml-text exact_systems_authority_targets_have_complete_declared_owned_path_witnesses -- --nocapture` | 0 | 1 passed; 0.56 s |

The final authority command constructs only exact `Views.sysml` and
`Connections.sysml` declarations under Operational v2, with no producer run.
It retains the three fixed canonical target IDs, exact metaclasses, public
ownership, source provenance and the BinaryConnection two-end antecedent.

The bounded Actions corpus input is
`Actions,Connections,Constraints,Flows,Items,Parts,Ports,States` (eight documents).
Actions/Flows import each other; the Actions StateUsage and Flows ConnectionUsage
add implicit standard targets, whose source dependencies establish the remainder.
The bounded Views/Connections medium input is
`Actions,Attributes,Connections,Constraints,Flows,Interfaces,Items,Parts,Ports,Requirements,States,Views`
(12 documents). These are preflight scope definitions, not closure success claims.
Whole-library publication and authored language readiness remain separate gates.
