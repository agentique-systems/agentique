# Enumeration canonical construction

Authority is the pinned SysML 2.0 corpus, with no new operational interpretation.
SysML.pdf SHA-256 is
`46e6c0476a6f1f34f367d57e039d56659bff75e41d2e4b3d37ca4cadea84a83a`;
standards/normative/sysml-2.0/SysML.xmi SHA-256 is
`caa65d54f56798bf7582d173f7567e1eea37a49c45984f8bd7df145011cf8c6f`.
Original bytes are unchanged.

| Formal construct | Exact pinned disposition | Implementation consequence |
| --- | --- | --- |
| EnumerationDefinition::isVariation | Nonderived Boolean redefinition of Definition::isVariation; default true; validateEnumerationDefinitionIsVariation requires true | The enum textual constructor sets the effective stored property to true, preserving its source origin. Ordinary Definition retains its false default. |
| validateDefinitionVariationIsAbstract | isVariation implies isAbstract | The enum textual constructor also sets isAbstract true, since its grammar has no abstract modifier. |
| EnumerationDefinition::enumeratedValue | Redefines Definition::variant | The literal's VariantMembership remains the canonical membership, without conversion into FeatureMembership. |
| EnumerationUsage::enumerationDefinition | Derived, exactly one EnumerationDefinition; redefines AttributeUsage::attributeDefinition | Its type comes from canonical FeatureTyping/specialization and the normal effective type query. No owner is injected into a query. |
| Usage::definition | Derived Classifier population redefining Feature::type | Canonical FeatureTyping is visible through existing KerML type semantics. |
| Usage::isVariation | Nonderived Boolean with false default | A literal does not become a variation merely because it is a variant. |
| VariantMembership | OwningMembership whose ownedVariantUsage redefines ownedMemberElement; owning namespace must be a variation Definition or Usage | Owner discovery follows owning_relationship then owning_related_element. Feature::owningType intentionally excludes this membership class. |
| checkUsageVariationDefinitionSpecialization | A Usage with owningVariationDefinition must specialize that Definition directly or indirectly | One generic producer proposes FeatureTyping from the Usage to its owning variation Definition. It observes membership, owner, variation flag and any existing specialization witness. |

The normative rule is the enumeration-owner typing rule in this case. Its source
is a Feature and its target a Classifier, so FeatureTyping is the exact canonical
carrier. Subsetting would have the wrong target domain. A second enumeration-only
edge or query exception is unnecessary. Existing direct or indirect specialization
discharges the rule with its witness and generates no redundant relationship.

Relevant PDF sections are 8.2.2.8 (PDF page 205, printed page 173),
8.3.6.3 Usage (the rule on PDF page 304, printed page 272),
8.3.8.2–3 (PDF pages 312–313, printed pages 280–281), and 8.4.2.3
(PDF page 435, printed page 403). Corresponding XMI IDs start with
`Systems-Enumerations-EnumerationDefinition`, `Systems-Enumerations-EnumerationUsage`,
`Systems-DefinitionAndUsage-VariantMembership`, and
`Systems-DefinitionAndUsage-Usage-checkUsageVariationDefinitionSpecialization`.

Current pilot source was independently inspected at
[`5cca16d846016e62bb1e54e0e50e675254a022ef`](https://github.com/Systems-Modeling/SysML-v2-Pilot-Implementation/tree/5cca16d846016e62bb1e54e0e50e675254a022ef).
`EnumerationDefinitionImpl` sets the variation default true. `UsageAdapter::addVariationTyping`
tests VariantMembership and an owning variation Definition, then adds implicit
FeatureTyping; `EnumerationUsage_enumerationDefinition_SettingDelegate` obtains
the first EnumerationDefinition from the normal effective type population.
This is corroborating implementation inspection, not replacement authority or a
claim that the pilot was executed for these enum fixtures. The lead's separate
ConnectionUsage authority review retains its own dynamic reference evidence.

Focused tests cover arbitrary Color/red/green names, a generic AttributeUsage
variant, ordinary membership and nonvariation negative controls, direct and
indirect existing typing, provenance, scheduler-issued closure, and actual
effective Usage type/name APIs. A source corpus fixture independently checks all
seven enum Definitions and all 21 VariantMembership-owned literals in the exact
pinned Systems bytes. Their families are PortionKind, RequirementConstraintKind,
StateSubactionKind, TransitionFeatureKind, TriggerKind, VerdictKind and
VerificationMethodKind.

The new producer descriptor advertises Typing and Specialization on its subject.
The rule registry includes its provenance RuleId. This makes prior converged
checkpoints obsolete without creating an enumeration-specific profile.
The lead owns the composed SysML rule-set identity bump and fresh corpus run.

Actual command arguments, exit codes, tool versions and tested-tree identities
are recorded in commands.json; raw output is retained in ignored
verification/generated/enumeration-closure. Authority extracts and downloaded
corroborating source are retained in ignored verification/generated/final-audit-enumeration.

Final focused status: all five enumeration tests passed (two source construction
tests and three semantic producer/effective API tests). The complete SysML
semantics library suite passed 96 tests in 130.43 seconds. After restricting the
new descriptor's relationship classes to FeatureTyping, the three semantic tests
passed again and the changed Rust files passed rustfmt. Earlier recorded failures
were fixture setup mistakes and the expected pre-integration Variation marker;
they are retained as failed commands, not counted as acceptance.

This proves the focused enum gate. It does not claim fresh Systems slice,
medium-corpus, full scheduler or final-publication acceptance.
