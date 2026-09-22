# Medium Systems dependency preflight

Use **13 documents**:
`Actions,Attributes,Calculations,Connections,Constraints,Flows,Interfaces,Items,Parts,Ports,Requirements,States,Views`.

The proposed 12-document scope omitted an implicit dependency. The original
`Interfaces.sysml` declares `private calc def excludingOnce`; the generic
`checkCalculationDefinitionSpecialization` producer requires
`Calculations::Calculation`. Its source need not explicitly import that base.
`Calculations.sysml` adds only Actions and accepted KerML Performances dependencies.

Read-only inspection used the pinned files under
`standards/libraries/Systems-Library/Systems Library`, stripping block/line
comments and collecting `\b([A-Za-z_][A-Za-z_0-9]*)\s*::` prefixes that match
Systems document names. The resulting direct cross-document edges are:

| Document | Explicit Systems dependencies |
| --- | --- |
| Actions | Flows |
| Attributes | none |
| Calculations | Actions |
| Connections | Actions, Parts |
| Constraints | none |
| Flows | Actions |
| Interfaces | Connections, Ports |
| Items | Constraints, Parts |
| Parts | Actions, Items, Ports, States |
| Ports | none |
| Requirements | Actions, Attributes, Constraints, Interfaces, Parts |
| States | Actions |
| Views | Parts, Requirements |

The present declaration/usage families and their generated metaclass ancestors
were compared with `BASE_RULES` and the exact role paths in
`crates/sysml-semantics/src/{producers,bindings}.rs`. Their Systems target packages
are Actions, Calculations, Connections, Constraints, Flows, Interfaces, Items,
Parts, Ports, Requirements, States and Views. Contextual owned/composite,
subaction, transition, state and Operational v2 Viewpoint rules introduce no
additional Systems package. KerML targets resolve against the separately
accepted immutable KerML publication.

This is bounded static dependency preflight, not reference completeness,
producer closure, publication acceptance or language-readiness evidence. No
medium/full corpus run was started. The lead owns those gates after Actions.
