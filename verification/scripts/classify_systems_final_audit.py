"""Classify strict effective-audit diagnostics without conflating family weights.

Classification is explanatory evidence, never a waiver or publication receipt.
Unknown diagnostics remain Other and fail the classification gate.
"""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import re
import uuid
import zipfile


CATEGORIES = (
    "EnumerationConstruction", "VariationNaming", "TypedProjection",
    "InheritedEndSuppression", "ResultCycle", "EndCycle",
    "ConnectionAuthority", "Other",
)
DIAGNOSTIC = re.compile(
    r'Capability \{ family: (\w+), diagnostic: Diagnostic \{ code: "([^"]+)", '
    r'subject: ElementId\((\d+)\), message: ("(?:[^"\\]|\\.)*") \} \}'
)


def diagnostics(report):
    gate = report.get("effective_sysml_audit", report.get("publication_gate", {}))
    if "findings" not in gate:
        raise ValueError("missing effective audit findings; an absent audit is not clean")
    grouped = defaultdict(Counter)
    for finding in gate["findings"]:
        match = DIAGNOSTIC.fullmatch(finding)
        if match is None:
            raise ValueError(f"unrecognized strict audit finding: {finding}")
        family, code, decimal, message = match.groups()
        grouped[(code, str(uuid.UUID(int=int(decimal))), json.loads(message))][family] += 1
    return [dict(code=code, subject=subject, message=message, families=dict(sorted(families.items())))
            for (code, subject, message), families in sorted(grouped.items())]


def classify(rows, subjects):
    codes = defaultdict(set)
    for row in rows:
        codes[row["subject"]].add(row["code"])
    result = []
    for row in rows:
        code, subject, message = (row[key] for key in ("code", "subject", "message"))
        kind = subjects.get(subject, {}).get("class")
        category = "Other"
        if code == "KQ_RESULT_INHERITANCE_CYCLE":
            category = "ResultCycle"
        elif code == "KQ_END_CYCLE":
            category = "EndCycle"
        elif code == "SQ_ELEMENT_KIND":
            if kind == "ASSOCIATION":
                category = "ConnectionAuthority"
            elif kind == "FEATURE" and any(
                    r["message"].startswith("effective interface ends is Invalid") for r in rows):
                category = "InheritedEndSuppression"
        elif code == "SQ_PUBLICATION_TYPED_QUERY":
            pending = re.findall(r"ElementId\((\d+)\), Variation", message)
            if pending and all(subjects.get(str(uuid.UUID(int=int(value))), {}).get("class")
                               == "ENUMERATION_USAGE" for value in pending):
                category = "VariationNaming" if message.startswith("effective names ") else "EnumerationConstruction"
            elif message.startswith("effective return parameters is Incomplete") and "KQ_RESULT_INHERITANCE_CYCLE" in codes[subject]:
                category = "ResultCycle"
            elif message.startswith("effective parameters is Incomplete") and "KQ_END_CYCLE" in codes[subject]:
                category = "EndCycle"
            elif message.startswith("effective interface ends is Invalid") and kind == "INTERFACE_USAGE":
                category = "InheritedEndSuppression"
            elif re.match(r"(?:current|effective) (?:item|part) definitions is Invalid", message) and kind == "CONNECTION_USAGE":
                category = "TypedProjection"
        result.append({**row, "category": category})
    return result


def summarize(rows):
    return {
        "weighted_findings": sum(sum(row["families"].values()) for row in rows),
        "distinct_diagnostics": len(rows),
        "subjects": len({row["subject"] for row in rows}),
        "categories": {category: {
            "distinct_diagnostics": sum(row["category"] == category for row in rows),
            "weighted_findings": sum(sum(row["families"].values()) for row in rows if row["category"] == category),
        } for category in CATEGORIES},
    }


def dependency_subjects(cache, wanted):
    """Read missing diagnostic targets from the independently pinned KerML graph."""
    root = Path(__file__).resolve().parents[2]
    receipt = json.loads((root / "standards/kerml-accepted-publication.json").read_bytes())
    expected = bytes(receipt["complete_overlay"]["graph_sha256"]).hex()
    names = {}
    for crate in ("kerml", "sysml"):
        text = (root / f"crates/{crate}/src/generated/typed_views.rs").read_text(encoding="utf-8")
        for name, identity in re.findall(r"pub const (\w+):.*?from_u128\(0x([0-9a-f]+)\)", text):
            names[str(uuid.UUID(int=int(identity, 16)))] = name
    digest, found = hashlib.sha256(), {}
    with zipfile.ZipFile(cache) as archive, archive.open("kernel.jsonl") as graph:
        for line in graph:
            digest.update(line)
            if b'"Record"' not in line:
                continue
            record = json.loads(line).get("Record")
            if record is not None and record["id"] in wanted:
                name = next((slot["value"]["Scalar"].get("String") for key, slot in record["slots"]
                             if names.get(key) == "ELEMENT_DECLARED_NAME"), None)
                found[record["id"]] = dict(**{"class": names[record["class"]]}, name=name)
    if digest.hexdigest() != expected:
        raise ValueError("accepted KerML graph does not match independent checked-in receipt")
    return found, expected


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--subjects", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--require-clean", action="store_true")
    parser.add_argument("--dependency-cache", type=Path)
    args = parser.parse_args()
    raw = args.report.read_bytes()
    subject_raw = args.subjects.read_bytes()
    subjects = json.loads(subject_raw)
    if isinstance(subjects, list):
        subjects = {row["id"]: {key: row[key] for key in ("class", "name")} for row in subjects}
    rows = diagnostics(json.loads(raw))
    dependency_digest = None
    if args.dependency_cache:
        found, dependency_digest = dependency_subjects(args.dependency_cache,
                                                       {row["subject"] for row in rows} - subjects.keys())
        subjects.update(found)
    rows = classify(rows, subjects)
    result = dict(format="agq-final-audit-classification/1", publication_authority=False,
                  report_sha256=hashlib.sha256(raw).hexdigest(),
                  subjects_sha256=hashlib.sha256(subject_raw).hexdigest(),
                  accepted_kerml_graph_sha256=dependency_digest,
                  summary=summarize(rows), subjects=subjects, diagnostics=rows)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result["summary"], indent=2))
    if result["summary"]["categories"]["Other"]["distinct_diagnostics"]:
        raise SystemExit(1)
    if args.require_clean and rows:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
