# Positive owned-Feature witness for compatibility

The actual `FeatureReferenceExpression` and `VariableFeaturing` descriptors
reproduce a causal cycle: compatibility is already false because a Feature owns
another Feature, but its whole membership search imports a pending child's
snapshot writer. The regression failed with reader state `Pending` on
`0ddafbc` and passed with the selected witness in `cda94f4`.

The KerML `Feature::isCompatibleWith` specialization branch and its evidence are
unchanged. Only the subsequent conjunction requiring both owned-Feature
populations to be empty has a positive counterexample path. A canonical owned
FeatureMembership and its current unique Feature endpoint prove nonemptiness.
Original ownership keeps its declared-slot guard; derived ownership retains its
selected contribution and creation proof, or the full aggregate proof when
precise contribution metadata is unavailable. No witness implies no absence
claim and falls back to the existing exhaustive queries.

The causal, truth-table, and reconstruction controls pass: specialization still
wins, either operand can prove nonemptiness, empty/unknown cases preserve their
original behavior, and removing or retargeting the selected carrier reopens its
reader. Separate proof controls cover native/archived derived evidence and
independent broad reads in both merge orders and evaluator modes. Commands,
outputs and fixture setup corrections are retained in
`compatible-witness-commands.json`. All five focused tests passed (1.75 s),
package all-target Clippy passed with warnings denied, and workspace formatting
passed; each final command exited 0.

The full compound authored fixture remains separately gated: its first run after
this fix still reported one incomplete MayTimeVary pair. This bounded proof
correction does not establish Systems publication or readiness acceptance.
