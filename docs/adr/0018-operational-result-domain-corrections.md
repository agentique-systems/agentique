# ADR 0018: Operational result-domain corrections

Status: Five-rule correction authorized; design recorded. Operational v6 implementation and publication withheld at the independent Gate 1 authority conflict KLCV8-F-001.

## Context

The v8 task authorizes a reviewed operational correction for exactly the five
constraints named by **KERML11-145**. The issue remains open. This authorization
is project policy, not a formally adopted OMG correction, and removes
KLCV7-F-001 as an unauthorized authority stop.

The [five-rule matrix](../../verification/kerml-semantic-closure-v8/authority-matrix.json)
retains each external ID, literal published body, prose, domain contradiction,
related constraints, exact pinned source witnesses, current pilot behavior and
reference XMI behavior. The current pilot constructs a chain for valuation but
still uses raw nested results for the other relevant implementations. Its graph
is corroborating evidence, not a uniform replacement specification.

## Authorized design boundary

A contextual result is a **distinct Feature** whose ordered chaining Features
are the Expression and its canonical result. Its domain follows the first
chaining Feature; its terminal identity remains the original result. Implied
identity must depend on rule/profile, Expression identity, result identity and
output role. Names are not identity inputs. Provenance must explain the reviewed
KERML11-145 rule, never label the implied chain as authored StandardLibrary source.

| Constraint | Authorized correction direction |
| --- | --- |
| `checkFeatureValuationSpecialization` | Preserve the undirected/all-implied-specialization antecedent. Subset the contextual value-expression result. Preserve the separate default/initial conditions of `checkFeatureValueBindingConnector`. |
| `checkExpressionResultBindingConnector` | Relate the outer canonical result to the nested contextual result. Preserve FeatureMembership ownership by the outer Expression and prove the required featuring-domain compatibility. |
| `checkFunctionResultBindingConnector` | Relate the canonical Function result to the contextual result of its result Expression. Preserve Function FeatureMembership ownership, inherited result identity, and Function featuring domain. |
| `checkIndexExpressionResultSpecialization` | Within the published conditional branch, subset the contextual first-argument result. The pilot's separate Array-to-Collection guard change belongs to KERML11-69 and is not authorized here. |
| `checkSelectExpressionResultSpecialization` | Preserve the argument-existence condition; subset the contextual first-argument result. No filtering execution is introduced. |

The future v6 profile must extend v5 without modifying historical manifests or
the default SourceProject profile. These design directions are not an implemented
manifest, seven-profile test result, or accepted publication. In particular, they
do not authorize changing `checkFeatureReferenceExpressionBindingConnector`.

## Independent preflight stop

The mandatory up-front sweep cross-references all 258 rules against 410 retained
official tracker records. It exposes [KLCV8-F-001 / KERML11-8](../kerml-feature-reference-binding-authority-conflict.md)
on the exact pinned `ControlFunctions::'.'` declaration. This is the **inner**
FeatureReferenceExpression connector, distinct from the outer Function connector
affected by KERML11-145.

The independently read graph includes result-to-referent subsetting, positional
result/end redefinitions, base specializations and the complete source chain.
The raw result is featured by the nested Expression; the referent is featured by
the Function. Neither domain specializes the other. No prescribed common domain
exists. The pilot's plain OwningMembership does not repair this contradiction.
An independent proof constructs the authorized outer contextual-result graph and
shows that the inner connector still fails. An arbitrary-name canonical fixture
also checks complete ownership, specialization, subsetting, chain and redefinition
answers under all six implemented profiles.

Repairing this inner connector requires a sixth rule-family correction or another
material semantic change. None is authorized. Accordingly, the preflight stops
before registering an unimplemented v6 profile or exposing an accepted library.
Ordinary unresolved references, incomplete answers, Triggers inference, chain and
positional expansion, and formal coverage remain implementation obligations;
they are not additional stop reasons or execution deferrals.

## Consequences

Published and operational v1–v5 remain unchanged. No accepted Snapshot, binding
regeneration, `LoadedKermlStandardLibraries`, or authored accepted-library
integration is issued. The full existing verification matrix is rerun, with
actual exits and the scope of unrun v6 acceptance gates recorded in
[v8 verification](../../verification/kerml-semantic-closure-v8/README.md).
Both structural runtime gates remain distinct from semantic publication and raw
metamodel authoring conformance. SysML semantics and execution remain out of scope.
