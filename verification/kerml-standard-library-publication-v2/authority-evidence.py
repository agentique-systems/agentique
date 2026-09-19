"""Freeze exact authority and classify the reproduced obligations, without changing it."""
from pathlib import Path
import hashlib
import json
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).resolve().parent


def load(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def write(name, value):
    if "--check" in sys.argv:
        assert load(OUT / name) == value, f"Stale {name}"
        return
    with (OUT / name).open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(value, stream, indent=2, ensure_ascii=False)
        stream.write("\n")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


metamodel_path = ROOT / "standards/generated/kerml-1.0/metamodel.json"
metamodel = load(metamodel_path)["metamodel"]
properties = metamodel["properties"]
classes = metamodel["classifiers"]
association = "Kernel-Connectors-A_participantFeature_Association"
interaction = "Kernel-Interactions-A_participantFeature_Interaction"
participant_keys = [association, interaction]
property_keys = [end for key in participant_keys for end in classes[key]["member_ends"]]
property_keys += ["Core-Types-Type-ownedFeature", "Core-Types-Type-endFeature",
                  "Kernel-Associations-Association-associationEnd"]
rule_classes = ["Core-Types-Type", "Core-Types-Conjugation", "Core-Features-Feature",
                "Core-Features-FeatureChaining", "Core-Features-Redefinition",
                "Root-Namespaces-Namespace", "Kernel-Associations-Association",
                "Kernel-Interactions-Interaction", "Kernel-Functions-Expression",
                "Kernel-Expressions-FeatureReferenceExpression", "Kernel-Expressions-FeatureChainExpression"]
retained = {key: [r for r in classes[key]["retained"]
                  if r["tag"] in ("ownedOperation", "ownedRule")]
            for key in rule_classes}
assert not retained["Kernel-Interactions-Interaction"]
mentions = []
for key, classifier in classes.items():
    for item in classifier["retained"]:
        if item["tag"] in ("ownedOperation", "ownedRule") and "participantFeature" in json.dumps(item):
            mentions.append([key, item["attributes"].get("name")])
assert not mentions, mentions
write("retained-authority.json", {
    "source": metamodel["source"], "scope": "Exact retained operations and constraints; reading evidence, not implementation coverage",
    "classes": retained, "participantFeature_rule_mentions": mentions,
    "associations": {key: classes[key] for key in participant_keys},
    "properties": {key: properties[key] for key in property_keys},
})
write("authority-conflict.json", {
    "format": "agentique-authority-conflict/1", "id": "KLPV2-F-001", "category": "F",
    "status": "unresolved-authority-conflict", "blocks_gates": [0, 3, 8],
    "pinned_authorities": [
        {"path": "KerML.pdf", "sha256": digest(ROOT / "KerML.pdf"),
         "clauses": ["7.4.5", "8.3.3.1.10 Type", "8.3.4.4.2 Association", "8.3.4.9.4 Interaction"],
         "finding": "Association has derived associationEnd redefining endFeature. Interaction adds no attributes, operations or constraints. participantFeature is absent."},
        {"path": "standards/normative/kerml-1.0/KerML.xmi", "sha256": digest(ROOT / "standards/normative/kerml-1.0/KerML.xmi"),
         "finding": "Two association-owned participantFeature navigations exist. Interaction end is non-derived, unordered, 2..*; opposite is 1..1. Association end is derived, ordered, 2..*, subsetting ownedFeature.",
         "properties": {key: properties[key]["entity"]["source_range"] for key in property_keys}},
    ],
    "issue": {"url": "https://issues.omg.org/issues/KERML11-81", "status_at_inspection": "open",
              "inspected": "2026-09-19", "reported_baseline": "KerML 1.0b4",
              "capture": "KERML11-81.html", "capture_sha256": digest(OUT / "KERML11-81.html"),
              "finding": "Reported as obsolete XMI associations absent from the specification document; deletion proposed, with no adopted resolution listed.",
              "authority_limit": "An open issue is corroborating evidence, not an adopted amendment or authorization to implement KerML 1.1."},
    "competing_interpretations": [
        {"interpretation": "Populate participantFeature with effective association ends",
         "problem": "No retained derivation equates these properties. Reusing one inherited Feature in two Interactions violates the pinned inverse upper bound 1. Subsetting ownedFeature also conflicts with inherited-only features."},
        {"interpretation": "Retire the two spurious associations from required runtime validation",
         "problem": "Changes the pinned runtime contract and requested strict Snapshot acceptance. Open issue provides no adopted normative resolution; forbidden without an explicit authority decision."},
        {"interpretation": "Copy inherited ends or add fresh participants",
         "problem": "Violates semantic identity and has no normative rule justifying new features."},
        {"interpretation": "Treat Interaction participantFeature as an overlay-derived associationEnd alias",
         "problem": "Changes non-derived metadata, ignores inverse bounds, and invents a mapping not supplied by KerML 1.0."},
    ],
    "witness": {"test": "crates/kerml-semantics/tests/participant_contract.rs",
                "result": "authority-witness-3/results.json",
                "finding": "Two arbitrary Interactions with ordinary specialization share the two original inherited ends. Missing participant links fail strict apply; adding links for both types fails even preview at the inverse 1..1 bound."},
    "resolution_required": "An adopted normative correction, or an explicit project authority decision changing the metamodel interpretation and this milestone's participant acceptance requirement.",
    "kernel_or_pins_modified": False, "publication_accepted": False,
})

report = load(OUT / "gate-0-obligations/obligations.json")
quality = load(OUT / "gate-0-baseline/library-quality.json")
rules = {
    "effective_features": ["Type::inheritableMemberships", "Type::inheritedMemberships", "Type::nonPrivateMemberships", "Type::removeRedefinedFeatures", "deriveTypeFeatureMembership", "deriveTypeFeature"],
    "redefinition": ["KerML 1.0 8.2.3.5.1", "Type::removeRedefinedFeatures", "checkFeatureEndRedefinition", "checkFeatureParameterRedefinition", "checkFeatureResultRedefinition"],
    "expression": ["KerML 1.0 8.2.5.8", "validateExpressionResultParameterMembership", "checkFeatureResultRedefinition", "FeatureReferenceExpression", "FeatureChainExpression"],
}
rows = []
for group in ("structural_obligations", "references", "semantic_queries"):
    for index, record in enumerate(report[group]):
        codes = sorted({d["code"] for d in record.get("result", {}).get("diagnostics", [])})
        participant = record.get("property") == "participantFeature"
        explicit_redefinition = record.get("property") == "redefinedFeature"
        categories = set()
        normative = set()
        if participant:
            categories.add("F")
            normative.add("KLPV2-F-001: no normative participantFeature derivation; do not equate it with associationEnd")
        if explicit_redefinition or "KQ_CONSTRUCTION_OBLIGATION" in codes or "KQ_EVIDENCE" in codes:
            categories.update(["C", "D"])
            normative.update(rules["redefinition"])
        if "KQ_UNSUPPORTED_INHERITANCE" in codes or "KQ_NONFEATURE_MEMBERSHIP" in codes:
            categories.add("C")
            normative.update(rules["effective_features"])
        if "KQ_RESULT_ARITY" in codes:
            categories.add("A")
            normative.update(rules["expression"])
        assert categories, (group, index, record)
        rows.append({
            "id": f"{group}/{index}", "report_pointer": f"gate-0-obligations/obligations.json#/{group}/{index}",
            "document": record["document"], "source_range": record["source_range"],
            "syntax_production": record["syntax_production"], "element": record["element"],
            "metaclass": record["metaclass"], "required_property": record.get("property"),
            "query": record.get("query"), "structure_origin": record["structure_origin"],
            "categories": sorted(categories), "normative_rules": sorted(normative),
            "diagnostic_codes": codes,
            "status": "authority-conflict" if participant else "ordinary-implementation-obligation-not-discharged",
            "evidence": "Exact candidates, completeness, positive and search dependencies remain at report_pointer; categories identify the required rule families, not established resolution.",
        })
documents = quality["documents"]
write("completion-matrix.json", {
    "format": "agentique-kerml-library-completion/2", "base": "ad6203bbac091342ed098a89ee8915595e404e57",
    "branch": "semantics/kerml-standard-library-publication-v2",
    "library_set": quality["library_set"], "rule_version": report["rule_version"],
    "status": "blocked-at-gate-0-by-KLPV2-F-001",
    "counts": {key: sum(d[key] for d in documents) for key in [
        "canonical_element_count", "canonical_relationship_count", "reference_count", "unresolved_count",
        "ambiguous_count", "incomplete_reference_count", "mismatched_endpoint_count", "query_count",
        "incomplete_query_count", "invalid_query_count", "unevaluated_expression_count", "recovery_count"]},
    "mandatory_lower_bound_obligations": len(report["structural_obligations"]),
    "obligations": rows,
    "state_expression_inspection": report["state_expression_inspection"],
    "state_expression_disposition": "Historical invalid result is retained in v1 resolution-obligations-6.json. Fresh bound-context result is explicitly recorded, including complete results; absence from lower-bound counts is not used to hide it.",
    "gates": [{"gate": gate, "status": "blocked-authority-conflict" if gate in (0,3,8) else "not-entered", "reason": "KLPV2-F-001"} for gate in range(17)],
    "publication_accepted": False,
})
print(json.dumps({"classified_obligations": len(rows), "authority_conflict": "KLPV2-F-001"}))
