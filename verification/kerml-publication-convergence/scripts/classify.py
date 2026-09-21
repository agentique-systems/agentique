"""Reproduce the reviewed relevance inventory; no network or issue acquisition."""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / "verification/kerml-publication-convergence"
SOURCE = ROOT / "verification/kerml-semantic-closure-v10/structural-coverage.json"

# Explicit exceptions to structural production/derivation. The retained inventory
# supplies exact formal bodies; this file records the review, not an OCL engine.
NONSTRUCTURAL_DERIVED = set("""
deriveElementDocumentation deriveElementOwnedAnnotation deriveElementTextualRepresentation
deriveMembershipMemberElementId
""".split())
EXECUTION = set("""
deriveExpressionIsModelLevelEvaluable deriveLiteralExpressionIsModelLevelEvaluable
validateElementFilterMembershipConditionIsModelLevelEvaluable
""".split())
CRITICAL_VALIDATORS = set("""
validateAssociationEndTypes validateBindingConnectorIsBinary
validateConstructorExpressionNoDuplicateFeatureRedefinition
validateCrossSubsettingCrossedFeature validateEndFeatureMembershipIsEnd
validateExpressionResultExpressionMembership validateExpressionResultParameterMembership
validateFeatureChainingFeatureConformance validateFeatureChainingFeatureNotOne
validateFeatureChainingFeaturesNotSelf validateFeatureOwnedCrossSubsetting
validateFeatureOwnedReferenceSubsetting validateFeatureReferenceExpressionReferentIsFeature
validateFeatureReferenceExpressionResult validateFlowEndIsEnd validateFlowEndNestedFeature
validateFlowEndOwningType validateFlowPayloadFeature validateFunctionResultExpressionMembership
validateFunctionResultParameterMembership validateInstantiationExpressionInstantiatedType
validateInstantiationExpressionResult validateInvocationExpressionInstantiatedType
validateInvocationExpressionNoDuplicateParameterRedefinition
validateInvocationExpressionParameterRedefinition validateMetadataAccessExpressionReferencedElement
validateMetadataFeatureMetaclass validateMultiplicityRangeBounds
validateNamespaceDistinguishibility validateParameterMembershipOwningType
validateParameterMembershipParameterDirection validateRedefinitionFeaturingTypes
validateResultExpressionMembershipOwningType validateReturnParameterMembershipOwningType
validateSubsettingFeaturingTypes validateTypeAtMostOneConjugator validateTypeOwnedMultiplicity
validateFeatureChainExpressionConformance validateFeatureCrossFeatureSpecialization
validateFeatureCrossFeatureType validateClassifierMultiplicityDomain
validateFeatureEndMultiplicity validateFeatureIsVariable
""".split())


def build():
    blockers = json.loads((ROOT / "standards/kerml-1.0-operational-authority-blockers.json").read_text())
    authority = {}
    for b in blockers["blockers"]:
        for rule in b["formal"]:
            authority.setdefault(rule["name"], []).append(b["id"])
    rows = []
    rationales = {}
    for old in json.loads(SOURCE.read_text())["constraints"]:
        name = old["name"]
        if name in authority:
            relevance = "AuthorityBlocked"
            reason = "Exact graph impact and alternatives are reviewed per witness in authority-blockers.json."
        elif name in EXECUTION:
            relevance = "ExecutionDependent"
            reason = "Determines model-level evaluability; does not choose canonical structural identities or endpoints. Filter population still requires a separate structural applicability proof."
        elif name in NONSTRUCTURAL_DERIVED:
            relevance = "ValidatorOnly"
            reason = "Projects documentary annotations, text, or an existing member ID; no structural identity, lookup, typing or featuring fact is selected. Canonical source records remain retained."
        elif name.startswith(("check", "derive")) or name in CRITICAL_VALIDATORS:
            relevance = "PublicationCritical"
            reason = "Determines or constrains essential relationships, endpoint identity, membership, inheritance, typing, featuring, or structural bounds. Producer/query closure must be proved independently of validator coverage."
        else:
            relevance = "ValidatorOnly"
            reason = "Judges flags, category compatibility, value restrictions or conformance of an already determined graph; does not synthesize or select canonical facts. A concrete unusable graph contradiction remains separately blocking."
        if name == "checkMetadataFeatureSemanticSpecialization":
            reason = "Evaluation can select a canonical specialization target. The structural output remains critical; evaluation dependence is not permission to publish an unknown target."
        if name == "validateMultiplicityRangeBoundResultTypes":
            reason = "Checks Integer/nonnegative values of preserved bound expressions; structural bound identities and order are covered separately by deriveMultiplicityRange* and validateMultiplicityRangeBounds."
        if name == "validateElementIsImpliedIncluded":
            reason = "Checks the model's completeness assertion. Deferred on a partial staging overlay; explicit strict checking remains available. No source flag is rewritten. Publication producer closure is independently critical."
        rationale = next((key for key, value in rationales.items() if value == reason), None)
        if rationale is None:
            rationale = str(len(rationales) + 1)
            rationales[rationale] = reason
        rows.append({"rule": name, "external_id": old["external_id"], "relevance": relevance,
                     "rationale": rationale, "authority_conflicts": authority.get(name, [])})
    return {"format": "agentique-publication-relevance/1",
            "authority": "standards/normative/kerml-1.0/KerML.xmi",
            "inventory": SOURCE.relative_to(ROOT).as_posix(),
            "inventory_sha256": hashlib.sha256(SOURCE.read_bytes()).hexdigest(),
            "scope": "Rule relevance, not implementation or publication acceptance. No NotApplicableToCorpus claims without evidence.",
            "rationales": rationales, "constraints": rows}


if __name__ == "__main__":
    data = build()
    path = OUT / "publication-critical-coverage.json"
    if "--check" in sys.argv:
        assert json.loads(path.read_text()) == data, "relevance inventory differs"
        import xml.etree.ElementTree as ET
        formal = {e.get("{http://www.omg.org/spec/XMI/20161101}id")
                  for e in ET.parse(ROOT / data["authority"]).iter()
                  if e.tag == "ownedRule" and e.get("name", "").startswith(("check", "derive", "validate"))}
        assert formal == {r["external_id"] for r in data["constraints"]}, "inventory does not cover exact pinned XMI"
        assert len(data["constraints"]) == len(formal), "duplicate rule"
        print(f"Reviewed relevance inventory matches all {len(formal)} pinned rules")
    else:
        path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
