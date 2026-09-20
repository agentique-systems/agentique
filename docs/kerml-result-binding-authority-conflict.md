# KLCV7-F-001: Function result-binding ownership and domain

**KERML OPERATIONAL LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED**

This is an independently reproduced structural conflict in the exact pinned
`Kernel Function Library/ControlFunctions.kerml`, associated with open
[KERML11-145](https://issues.omg.org/issues/KERML11-145). No correction is applied.
It is separate from the six authorized v5 target corrections.

## Exact witness

The function `ControlFunctions::'.'` ends with the expression `chain`, after
declaring `private feature chain chains source.target;`. The grammar establishes
a ResultExpressionMembership containing a FeatureReferenceExpression. That
expression has its own result Feature, distinct from the Function's result.

The [independent packet](../verification/kerml-semantic-closure-v7/result-binding-authority-conflict.json)
retains exact pinned source bytes/range and ZIP hash, formal XMI clauses and hash,
the reference release and XMI IDs, complete Function specialization closure,
owning memberships, featuring types, and identity predicates. The independent
script uses ZIP/XML reads, not Agentique lowering, queries or an OCL interpreter.
[A six-profile canonical witness](../crates/kerml-semantics/tests/result_binding_authority_v7.rs)
also establishes the ownership, specialization and distinct result/chain identities
with arbitrary names.

## Formal requirements

`checkFunctionResultBindingConnector` requires a BindingConnector selected from
the Function's **ownedFeature**, with related Features including the Function's
result and the nested expression's **raw result identity**. These are identity
tests, not specialization tests.

An ordinary nonvariable Feature owned through a FeatureMembership has its owning
Function as a required featuring type (`checkFeatureFeatureMembershipTypeFeaturing`
and `Feature::isFeaturingType`). `checkConnectorTypeFeaturing` requires each
related Feature to be featured within every featuring type of the connector.

The nested raw result is nonvariable, has no feature chain, and is featured by
its expression. `Feature::isFeaturedWithin(Function)` therefore requires the
Function to be compatible with that expression. The Function is a Classifier,
so `Type::isCompatibleWith` means specialization. The complete independently read
reference supertype closure excludes the nested expression. The domain test fails.
Adding another featuring type cannot repair a universal `forAll` test. An empty
connector domain also fails the raw result's `isFeaturedWithin(null)` test.

## Independent reference comparison and semantic choice

The frozen reference XMI keeps both raw results as related Features, but owns the
binding through **plain OwningMembership** and features it by the **nested
expression**, not the Function. This avoids the Function-domain obligation but
does not satisfy the formal `ownedFeature` selection. The packet does **not**
assert that this actual reference connector fails its own featuring check; the
failed domain predicate is the one imposed by the published ownedFeature
interpretation.

Alternatively, chaining through the expression to its result repairs the domain
but supplies a different Feature identity, failing the literal inclusion of the
raw result. Making the Function specialize its nested expression, changing flags,
or relocating the result would change canonical meaning without an authorized
inference rule. These are materially different semantic choices.

Missing Agentique connectors, implied ordering, end flags, bindings or inherited
membership inference do not make distinct owned identities equal. This witness
uses no runtime value evaluation. It does not rely on treating specialization
through a chain as unequal to specialization of its terminal Feature: that alone
would not prove a conflict for `checkFeatureValuationSpecialization`.

## Disposition

All five stop conditions are met: the pinned publication exercises the rule;
independent source/XMI and canonical witnesses reproduce it; the conflict persists
after the required ownership/identity facts are supplied; no existing correction
changes these requirements; and resolving it requires an ownership/domain or
identity decision beyond KERML11-205/206/207.

KERML11-145 remains unauthorized. Adjacent issues 182 and 210 are retained as
future authority metadata, with no correction or additional conflict claimed.
The 258-rule coverage gate remains closed to publication. Ordinary structural
work is retained as mandatory, without reclassifying it as execution or authority.
No accepted Snapshot, binding set or `LoadedKermlStandardLibraries` is issued.

Reproduce offline:

```text
python -B -X utf8 verification/kerml-semantic-closure-v7/result-binding-authority.py --check
cargo test --locked --offline -p agq-kerml-semantics --test result_binding_authority_v7 -- --nocapture
```
