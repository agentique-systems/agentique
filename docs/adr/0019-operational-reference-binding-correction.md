# ADR 0019: Operational reference-binding correction

Date: 2026-09-20

Status: Accepted for the explicitly authorized v6 interpretation. Library semantic
publication remains blocked by the separate KLCV9-F-001 authority conflict.

## Authority and scope

The v9 task authorizes `checkFeatureReferenceExpressionBindingConnector` and only
the connector-featuring consequences needed for its implied binding. KERML11-8
is still open. The [authority record](../../verification/kerml-semantic-closure-v9/reference-binding-authority.json)
retains the exact pinned formal bodies, default-context derivation, source bytes,
current reference graph, pilot construction and validator, preliminary material,
and independent domain truth table. Pilot behavior and TODOs are corroboration,
not normative authority.

V6 also implements the five separately authorized KERML11-145 producers from
[ADR 0018](0018-operational-result-domain-corrections.md). Its registration consists
of a [profile manifest](../../standards/kerml-1.0-operational-profile-v6.json), a
[result-domain manifest](../../standards/kerml-1.0-operational-result-domain-errata-v6.json),
and a [reference-binding manifest](../../standards/kerml-1.0-operational-reference-binding-errata-v6.json).
These are separate content identities. No v1–v5 manifest is rewritten. The default
`OPERATIONAL` stays v2.

## Semantic decision

An implied binding has a typed `ImpliedBindingRole`. `FeatureReferenceResult`
identifies the connector required by the FeatureReferenceExpression rule. Its
profile-qualified RuleId participates in deterministic derivation keys, canonical
record provenance, validation explanations and search dependencies. An authored
connector, an unclassified implied connector, or another producer role cannot
claim this exception solely because its owner is an expression.

The connector retains the exact referent and raw result identities, in that order.
The result must be owned by the same expression via ReturnParameterMembership.
Only that endpoint may satisfy featuring conformance through the expression
context. The referent still has to pass the ordinary check. Extra endpoints,
reversed endpoint roles, the wrong result owner, and a different semantic producer
do not qualify. No variable flag, specialization, source declaration, or raw
result identity is changed to obtain conformance.

The context query first tries the ordinary common featuring-context calculation.
If it establishes no common context, v6 selects the unique nearest direct or
indirect featuring context of the expression in which the referent is valid.
Multiple nearest contexts and missing evidence remain Incomplete. ID order cannot
select a winner. The independent `ControlFunctions::'.'` witness selects the outer
Function, agreeing with the reference XMI for that inner binding.

FeatureChainExpression was reviewed separately. Its prescribed result chain and
source/target redefinitions do not themselves mandate this raw reference/result
binding. The pilot's broader validator conditional does not establish equivalent
authority. Direct FeatureChainExpression bindings are excluded. A nested
FeatureReferenceExpression retains its own reviewed role.

## Contextual results

The reusable primitive creates a distinct Feature with an ordered two-link chain
`[expression, raw_result]`. Its identity includes the producer rule, profile,
subject and output inputs. The chain's domain follows the expression, and its
terminal contributes ordinary supertype/subsetting semantics. Repeated production
over the same immutable input produces the same IDs without duplicate outputs.

The five producers retain their own antecedents: undirected valuation with only
implied specializations; Expression result binding; Function result binding;
Index result specialization with the published Array guard; and Select result
specialization. The KERML11-69 Collection guard is not adopted. No filtering or
other runtime evaluation is implemented.

The kernel supports validated monotone extension of ordered reference properties
in a derived overlay. The original Snapshot and its ordering remain unchanged.
Every extension retains declared evidence and the added derived records. Invalid
storage kinds, duplicates and missing evidence fail atomically. This facility has
no language-specific logic.

SemanticContext includes both correction digests, the aggregate v6 identity,
rule-set version, descriptor graph, immutable model identity and library pins.
Overlays produced for a different profile are rejected. Query completion and a
valid kernel overlay are not semantic acceptance operations.

## Verification and independent stop

The arbitrary-name matrices exercise all seven profiles, all five result-domain
rules, asymmetric reference endpoint qualification, authored and other-role
connectors, ownership, inheritance, missing/ambiguous inputs and nesting. The
inner reference correction and outer Function contextual binding are tested
independently. These are producer/query tests, not a claim that every structural
constraint on the complete corpus is implemented.

The applicability review independently reproduces
[KLCV9-F-001 / KERML11-1](../kerml-cross-feature-authority-conflict.md) in two exact
Occurrences end-value declarations. Published cross-feature selection and the
value-member featuring requirements impose incompatible exact domains. The
contradiction persists after the authorized result/reference corrections and is
outside their scope. No cross-feature exclusion is added to v6.

Full corpus closure, strict semantic publication, accepted bindings, the accepted
library facade, and authored consumption remain unissued at this stop. The
ordinary unresolved-reference, incomplete-query, Triggers, feature-chain,
positional and coverage work remains implementation work. None is reclassified
as execution or used as an independent stop reason. Actual commands and exits are
in [v9 verification](../../verification/kerml-semantic-closure-v9/README.md).
