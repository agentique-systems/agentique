# Indexed assignment closure regression

Source base: `9e65a77d161be65e9d6c6ada3e3eb01cd598342e`. Commands, source/diff
identities, log hashes, timings and exits are in `index-usage-commands.json`.

The pinned `Actions.sysml:518` construction test passes (0.78 s), without a
standard cache or producer replay. Assignment input `98c30374` is a ReferenceUsage
owning FeatureValue `d32765d8`, which owns IndexExpression `f81c5d7e`. The Index
result is a constructed declared **plain Feature** (`7be8d317`) under an implied
ReturnParameterMembership. The nested `seq` / `index` FeatureReferenceExpressions
own syntax EmptyFeature **ReferenceUsage** results. These are distinct identities.

The genuine three-layer scheduler fixture preserves those classes and ownership,
the closed `BaseFunctions::'#'` signature, and Action→Occurrence /
assignmentActions→AssignmentAction→Action ancestry with target(in) and
replacementValues(inout). It reproduces the corpus's four incomplete pairs in
22.19 s: assignment input FeatureValue and MayTimeVary, and both nested
FeatureReferenceExpression producers. Diagnostics match producer typing closure,
missing value context and variable snapshot context. No production fix yet.

Two fixture corrections precede that result: the operator slot uses its canonical
IndexExpression alias, and the standard `'#'` Function must exist before dependency
closure. A prior direct-Index-result ReferenceUsage variant reproduced a separate
argument-population overread; it is retained as a broader control and is not
presented as the exact corpus shape. With skeletal Action anchors the class-correct
fixture passed because the input owner lacked the required Occurrence ancestry;
that run is not acceptance evidence for the corrected fixture. Initial diagnostic
test compile errors (FactKey iteration and borrowed lookup key) were corrected
without changing assertions. All attempts remain in the command ledger.
