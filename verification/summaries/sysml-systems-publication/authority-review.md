# Exact Systems Library authority review

Three unresolved target conflicts are present in the pinned final SysML 2.0 PDF,
its XMI and the original Systems KPAR. Each has an actual corpus declaration
that exercises the rule. These rules determine canonical specialization
endpoints: they are relevant to producer closure, not merely unimplemented
validation coverage. No additional correction is authorized by this review.

The [decision manifest](authority-decision.json) pins the authority hashes, exact
XMI bodies, PDF pages, source entry hashes, byte ranges and antecedents. PDF page
numbers below are one-based; printed page numbers are 32 lower.

| Finding | Formal rule target | Other pinned authority and source |
| --- | --- | --- |
| SYSML20-PUB-001: `checkViewpointDefinitionSpecialization` | `Views::Viewpoint`, §8.3.26.8, PDF 420–421; XMI agrees, including adjacent prose | Table 31, PDF 428; §8.4.22.3, PDF 490; and the catalog, §9.2.19.2.11, PDF 543–544, identify `Views::ViewpointCheck`. `Views.sysml:52` declares it as a ViewpointDefinition. |
| SYSML20-PUB-002: `checkViewpointUsageSpecialization` | `Views::viewpoints`, §8.3.26.9, PDF 421; XMI and Table 32, PDF 432, agree | §8.4.22.4, PDF 490, and §9.2.19.2.12, PDF 544, identify `Views::viewpointChecks`. `Views.sysml:30` declares `viewpointSatisfactions`, and line 107 declares `viewpointChecks`, both ViewpointUsages. |
| SYSML20-PUB-003: `checkConnectionDefinitionBinarySpecialization` | `Connections::BinaryConnections`, §8.3.13.3, PDF 330; XMI formal body agrees | Adjacent PDF/XMI prose, Table 31, PDF 428, and §8.4.9.1 prose, PDF 444, use `Connections::BinaryConnection`. `Connections.sysml:35` declares it with exactly two written owned ends at lines 43–44. Allocation also declares two owned ends at lines 19–20. |

The Viewpoint rules are unconditional for their metaclasses. The binary rule has
the exact condition `ownedEndFeature->size() = 2`; an effective or inherited end
count must not replace that antecedent. Allocation specializes ConnectionDefinition
through its metaclass, so its two declared ends provide another source witness.

§8.4.1, PDF 427, identifies Subclassification as the implied relationship for
Definition specialization constraints and Subsetting for Usage specialization
constraints. It also requires suppression of redundant implied relationships.
Consequently, choosing `ViewpointCheck` instead of `Viewpoint`, or another name in
the table above, changes the required canonical endpoint. An existing typing or
specialization to a differently named declaration does not establish satisfaction
of the unresolved formal target.

A whole-word scan of all 21 exact source texts finds no `Viewpoint` or
`BinaryConnections` occurrence. The only `viewpoints` occurrence is descriptive
comment text at `Views.sysml:4`. There is no source alias under any missing name.
This source observation is separate from a canonical namespace lookup. The
focused declared-graph test described below now supplies exact owned-path
witnesses; full accepted-dependency candidate closure remains pending. Neither
the source scan nor this test makes an accepted-publication claim.

## Declared canonical witnesses

The focused test constructs all 21 exact documents together in an unpublished
ConstructionView on an empty canonical graph with the full combined registry.
Its unresolved reference obligations remain visible. It deliberately uses no
accepted KerML dependency, reference refinement or producer closure.

Every step of the owned path requires `owned_relationships` and `member` to be
Complete, including their canonical ownership-property dependencies. It reads
declared names and restricts traversal to OwningMemberships, following the exact
declaration contract used by StandardSysmlBindings. It also checks uniqueness,
public visibility, exact metaclasses, StandardLibrary origin, and the pinned
DocumentId, SourceRevisionId, SyntaxNodeId and ByteRange for each subject and
membership. Imports, inherited membership and global namespace completeness do
not participate in this test.

| Formal target absent on the complete declared owned path | Actual canonical declaration | ElementId |
| --- | --- | --- |
| `Views::Viewpoint` | `Views::ViewpointCheck` — ViewpointDefinition | `8c8a3d17-aa0d-50dc-bb3b-b5b41b330b5d` |
| `Views::viewpoints` | `Views::viewpointChecks` — ViewpointUsage | `29b6d0ae-8a25-55d0-ac30-9d153e62ddba` |
| `Connections::BinaryConnections` | `Connections::BinaryConnection` — ConnectionDefinition | `6f256a75-d031-5390-90d0-24ba7666497b` |

BinaryConnection directly owns exactly two FeatureMembership members with a
computed declared `Feature::isEnd = true`: `source`
(`6c7986f9-6e5b-59f0-83e6-51e1fb686d81`) and `target`
(`66864d33-a47d-5da9-9da6-41b7aa317ee1`), both ReferenceUsages. This witnesses the
rule's declared owned-end antecedent without substituting an inherited or
effective end count. The additional actual ViewpointUsage
`Views::View::viewpointSatisfactions` has canonical identity
`92713464-c2df-5d85-9a2c-8d9641ef3714`.

The test passed in 5.01 seconds; package formatting and Clippy with warnings
denied also passed. Reproduce it with:

```text
cargo test --locked --offline -p agq-kerml-text --lib exact_systems_authority_targets_have_complete_declared_owned_path_witnesses -- --nocapture
```

The output's `AUTHORITY_DECLARED_WITNESS` record contains the observed IDs,
provenance and path outcomes. These are declared canonical witnesses, not a
claim that all mandatory references or the combined semantic worklist close.

Some neighboring examples have additional spelling inconsistencies: the
ViewpointDefinition example on PDF 490 spells its package `Viewpoints`, and the
binary example on PDF 444 repeats `BinaryConnections` despite the adjacent
singular prose. These examples strengthen the need to preserve the distinct
authority statements; they do not authorize source rewriting or aliases.

## Decisions already authorized

Operational v1 has exactly the four textual compatibility decisions recorded in
the [grammar manifest](../../../standards/grammar/sysml-2.0-operational-v1.json):
AllocationDefinition dispatch; Case return members; the two reviewed end-usage
prefix changes; and optional `assert`/`not` in SatisfyRequirementUsage. The strict
Published grammar remains selectable and reproduces its eight retained failures.

The milestone separately authorizes only the Item composite-target correction
from `Items::Item::subitem` to `Items::Item::subitems`, retaining the entire formal
antecedent and actual canonical identity. The formal body is on PDF 320;
Table 32 on PDF 429 and `Items.sysml:110` use the plural declaration. Published
semantics retain the singular target. The user cited OMG SYSML21-46 as additional
rationale; this local review does not adopt any preliminary specification.

For AttributeUsage, the authorized target remains `Base::dataValues`. The formal
rule and adjacent prose on PDF 311, §8.4.3.2 on PDF 436, and the pinned
`Attributes::attributeValues` alias all agree. Table 32's `Attributes::attributes`
on PDF 429 is recorded as a summary inconsistency. It does not establish a
canonical graph contradiction or authorize another declaration.

The three new findings are distinct from those authorized choices. They retain
their formal targets, with missing lookup or producer obligations reported
explicitly. No fallback target, synthetic alias, source edit, or silent producer
waiver is permitted by this review. Accepted KerML Operational v9 is unchanged.

## Local reproduction

From the repository root, execute this Python block with `python -`. It reads
only the pinned authority and manifest; `pypdf` is a review-time dependency, not
a build or runtime dependency. The actual result is recorded in the manifest.
Raw extraction remains ignored under `verification/generated/`.

```python
import hashlib
import json
import re
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET
from pypdf import PdfReader

path = Path("verification/summaries/sysml-systems-publication/authority-decision.json")
review = json.loads(path.read_text(encoding="utf-8"))
for artifact in review["authority_inputs"]:
    data = Path(artifact["path"]).read_bytes()
    assert len(data) == artifact["bytes"]
    assert hashlib.sha256(data).hexdigest() == artifact["sha256"]

xml = ET.parse("standards/normative/sysml-2.0/SysML.xmi").getroot()
rules = {n.attrib["name"]: n for n in xml.iter()
         if n.tag == "ownedRule" and "name" in n.attrib}
pdf = PdfReader("SysML.pdf")
formal_pages = {
    "checkViewpointDefinitionSpecialization": 421,
    "checkViewpointUsageSpecialization": 421,
    "checkConnectionDefinitionBinarySpecialization": 330,
    "checkItemUsageSubitemSpecialization": 320,
    "checkAttributeUsageSpecialization": 311,
}
entries = review["authorized_decisions"] + review["remaining_findings"]
with zipfile.ZipFile("standards/artifacts/Systems-Library.kpar") as archive:
    sources = {n: archive.read(n) for n in archive.namelist() if n.endswith(".sysml")}
    assert len(sources) == 21
    witnesses = 0
    for entry in entries:
        if "rule" in entry:
            rule = entry["rule"]
            node = rules[rule["name"]]
            assert node.attrib["{http://www.omg.org/spec/XMI/20161101}id"] == rule["xmi_id"]
            assert node.find("specification").attrib["body"] == rule["formal_body"]
            body = re.sub(r"\s+", "", rule["formal_body"])
            page = pdf.pages[formal_pages[rule["name"]] - 1].extract_text()
            assert body in re.sub(r"\s+", "", page), rule["name"]
        for witness in entry.get("source_witnesses", []):
            source = sources[witness["path"]]
            assert hashlib.sha256(source).hexdigest() == witness["source_sha256"]
            start, end = witness["byte_range"]
            assert source[start:end].decode() == witness["text"]
            assert source[:start].count(b"\n") + 1 == witness["line"]
            witnesses += 1
    for name, count in [("Viewpoint", 0), ("BinaryConnections", 0), ("viewpoints", 1)]:
        hits = [(path, line) for path, source in sources.items()
                for line in source.decode().splitlines()
                if re.search(r"\b" + name + r"\b", line)]
        assert len(hits) == count, (name, hits)
        if name == "viewpoints":
            assert hits[0][0] == "Systems Library/Views.sysml"
            assert hits[0][1].lstrip().startswith("*")
for page, target in [(428, "Views::ViewpointCheck"), (490, "Views::viewpointChecks"),
                     (428, "Connections::BinaryConnection"), (429, "Attributes::attributes"),
                     (436, "Base::dataValues")]:
    assert re.search(re.escape(target) + r"(?!\w)", pdf.pages[page - 1].extract_text())
print(f"Verified 3 authority hashes, 5 formal PDF/XMI rules, {witnesses} source witnesses, "
      "and the 21-document name observations; canonical graph evidence remains separate.")
```
