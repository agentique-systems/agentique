# Retained enumeration Variation findings

The strict finalizer rejected publication after its effective SysML audit. Of
674 family findings, 514 mention `Variation`: 21 distinct EnumerationUsage
literals and propagation into two ReferenceUsage results. Earlier capability,
provenance, authority and binding audits passed; all 1,327 mandatory references
were Complete. These observations do not establish publication acceptance.

This diagnosis reads the original authenticated round-28 archive
`c42d679d2908df7ba0a864710eda0e4e8c8cb97e70ead2ef5e9c604888bd447d.zip`
from ignored `verification/generated/final-language-acceptance/full-frontiers/`.
It changes neither its graph nor its certificate. The source inspected is
`047807a`. JSONL Record rows were streamed with Python `json`/`zipfile`, retaining
the last overlay record for each identity and resolving descriptor IDs using the
generated KerML/SysML constants. Those read-only commands exited 0. Raw selected
rows are ignored `verification/generated/systems-finalization/enum-structure.json`
and `enum-literal-relations.json` in the lead worktree. No compiler, scheduler or
finalizer was run for this diagnosis.

## Representative canonical evidence

`SysML::Systems::TransitionFeatureKind::trigger` has these retained identities:

| Role | Identity |
| --- | --- |
| EnumerationUsage `trigger` | `12bb4729-06c1-51b0-9974-891634003445` |
| Owning VariantMembership | `491e3d29-d761-53e5-844f-ae19475d60fd` |
| EnumerationDefinition `TransitionFeatureKind` | `59e40ca7-2a9a-55ca-8f79-da5ca6b6ad0b` |
| Literal's sole owned relationship, Subsetting | `36c9c101-b766-5237-b80d-398b2f1d3a55` |
| That relationship's subsetted feature, accepted `DataValues` | `170bd54f-53f0-512b-b16b-2c96e7fe9021` |

All 21 literals have the same relationship shape: one derived Subsetting to
accepted KerML `DataValues`, with no owned typing/specialization to their
EnumerationDefinition. `checkAttributeUsageSpecialization` produces this general
base edge; it does not supply the missing enumeration typing. The literal's
declared name is present and its own `Usage::isVariation` is false.

The subsequent full search did not limit candidates to owned relationships.
Across all 15,420 serialized Record rows (13,624 final local records), there are
exactly 46 canonical slot references to the 21 literals: 21 VariantMembership
owned-member endpoints, 21 Subsetting source endpoints, two Membership aliases
and two ReferenceSubsetting **target** endpoints. There is no additional
FeatureTyping, Specialization or redefined source-role endpoint on any literal.
The two ReferenceSubsettings point from other expression features to `pass` and
`fail`; their direction does not add typing to either literal.

All 22 canonical association occurrences were inspected; none has a literal or
DataValues endpoint. DataValues has only one local membership alias and 57 local
Subsetting **general** endpoints, with no local source-side relationship. Thus
no nonowned local relation supplies an indirect route from DataValues to an
enumeration owner. The full local population also contains zero source-side
specialization/typing/subsetting/redefinition references to any accepted dependency
subject, and every local association occurrence endpoint is local. Thus no outer
relationship extends an intermediate accepted ancestor either. The sealed KerML
dependency cannot itself refer to these new local EnumerationDefinitions.
This covers the incoming source-role search used
by `incoming_source_relationships`/`targets`, in addition to the owned
conjugation, chaining and reference-subsetting paths.

All seven EnumerationDefinition records store false for the **resolved override**
`EnumerationDefinition::isVariation`, property
`5172cf68-e527-5aa1-9e82-725821e72197`. This is not confusion with a superseded
Definition property. `sysml_structural_completion` resolves the effective
descriptor and fills this Boolean with false. The affected definitions are
PortionKind, RequirementConstraintKind, StateSubactionKind, TransitionFeatureKind,
TriggerKind, VerdictKind and VerificationMethodKind.

## Authority and applicability

The unchanged pinned final SysML 2.0 `standards/normative/sysml-2.0/SysML.xmi`
has SHA-256 `caa65d54f56798bf7582d173f7567e1eea37a49c45984f8bd7df145011cf8c6f`,
independently checked with `Get-FileHash` (exit 0). Relevant exact XMI keys are:

- `Systems-Enumerations-EnumerationDefinition-isVariation`: default true;
  EnumerationDefinition is a variation whose enumerated values are its variants.
- `Systems-Enumerations-EnumerationUsage-enumerationDefinition`: exactly one
  EnumerationDefinition types each EnumerationUsage, redefining attributeDefinition.
- `Systems-DefinitionAndUsage-Usage-checkUsageVariationDefinitionSpecialization`:
  a Usage with an owning variation Definition must specialize that Definition.
- `Systems-DefinitionAndUsage-Usage-namingFeature_`: variant naming uses its
  reference-subsetting target when present, otherwise no naming feature.

The incorrect owner flag by itself concerns an unimplemented validator and is
not used to expand publication into full conformance checking. The absent
enumeration typing is different: effective Usage/Attribute typing is a required
publication capability. Returning only general DataValue typing as Complete
would omit the enumeration's required, more specific Definition identity.
`PendingSysmlRule::Variation` currently prevents that unsupported claim.

Naming has a narrower issue. KerML naming already establishes each explicit
literal name; its registered SysML extension also implements the pinned variant
naming-source dispatch. `current_names` still adds a blanket Variation marker
for VariantMembership, making those exact naming answers unnecessarily incomplete.
A naming-only correction can preserve all structural pending implications, but
cannot establish the missing literal typing and cannot accept this publication.
An uncompiled naming-only patch is held under ignored generated storage in the
isolated finalization worktree; it is not integrated.

## Consequence for exact finalization

An enum-metaclass exemption from structural Variation would mask missing typing.
Inserting the owner into a query result without its canonical specialization and
proof would also reinterpret the retained closed graph. Closing that gap requires
corrected source lowering and the corresponding semantic producer coverage,
followed by newly authenticated graph/certificate identities. The existing
certificate closes its registered producers; it does not certify missing
variation implications. Such work is incompatible with exact read-only reuse of
this checkpoint and is not authorized by a query-only acceptance fix.

The checkpoint remains unchanged and unaccepted. No producer replay is authorized
or performed, and no broad Variation exception or validator expansion is adopted.

## Reproducible absence check

`verification/scripts/diagnose_systems_enumeration_frontier.py` authenticates both
the archive and decoded graph hashes, reconstructs the final local record map,
checks every canonical reference regardless of ownership, checks all association
occurrences and rejects any additional unclassified literal/DataValues reference.
Its exit zero establishes this diagnosed absence, not acceptance authority.

Actual command, from the isolated `agentique-platform-finalization` worktree:

```powershell
python verification/scripts/diagnose_systems_enumeration_frontier.py --repository C:/Users/phili/github/agentique-systems/agentique --archive C:/Users/phili/github/agentique-systems/agentique/verification/generated/final-language-acceptance/full-frontiers/c42d679d2908df7ba0a864710eda0e4e8c8cb97e70ead2ef5e9c604888bd447d.zip --archive-sha256 c42d679d2908df7ba0a864710eda0e4e8c8cb97e70ead2ef5e9c604888bd447d --graph-sha256 c056ecf5ea4b168235677763d3b2dd64ebefb15fbc71ca43bcee090865b1608a --output C:/Users/phili/github/agentique-systems/agentique/verification/generated/systems-finalization/enumeration-diagnosis.json
```

Python 3.12.10; exit: **0**. Report: 13,624 local records, 22 association occurrences, 21 literals
missing enumeration-owner typing. Output JSON SHA-256:
`0416b56b12d9fe248bcfc998cc263e1ab10f5d0b9bb3fe494a37f0f01118a1da`.
The output also pins the inspected generated descriptor-name tables and retains
the complete 46-reference inventory and per-literal representative identities.
