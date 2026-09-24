"""Read pinned local authority; no download and no build-time acquisition."""
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
XMI = ROOT / "standards/normative/sysml-2.0/SysML.xmi"
CLASSES = {
    "Usage", "OccurrenceUsage", "ItemUsage", "PartUsage", "ConnectionUsage",
    "AttributeUsage", "PortUsage", "Definition", "EnumerationDefinition",
    "EnumerationUsage", "VariantMembership",
}
rows = []
for element in ET.parse(XMI).getroot().iter("packagedElement"):
    if element.get("name") not in CLASSES:
        continue
    selected = []
    for child in element:
        name = child.get("name", "")
        if child.tag == "ownedAttribute" and any(
            term in name.lower() for term in ("definition", "variation", "variant")
        ):
            selected.append({
                "property": name,
                "attributes": child.attrib,
                "contract": [
                    {"kind": item.tag, **item.attrib}
                    for item in child
                    if item.tag in {
                        "redefinedProperty", "subsettedProperty", "type",
                        "defaultValue", "lowerValue", "upperValue",
                    }
                ],
            })
        if child.tag in {"ownedRule", "ownedOperation"} and any(
            term in name for term in ("Definition", "Variation", "Variant", "namingFeature")
        ):
            selected.append({
                "rule_or_operation": name,
                "formal_bodies": [
                    item.get("body") for item in child.iter()
                    if item.get("body") and item.tag != "ownedComment"
                ],
            })
    rows.append({"class": element.get("name"), "contracts": selected})

print(json.dumps({
    "xmi": str(XMI.relative_to(ROOT)),
    "xmi_sha256": hashlib.sha256(XMI.read_bytes()).hexdigest(),
    "classes": sorted(rows, key=lambda row: row["class"]),
}, indent=2))
