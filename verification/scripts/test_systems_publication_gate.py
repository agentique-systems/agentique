"""Fail-closed final gate and generated-cache corruption regressions."""
import copy
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import systems_publication_gate as gate


def report_fixture():
    checked = {family: 1 for family in gate.FAMILIES}
    checked.update(Syntax=21, CanonicalLowering=21, StandardBindings=69)
    documents = [dict(path=f"fixture/{i}.sysml", document=str(gate.uuid.UUID(int=i + 1)),
        sha256="00" * 32, profile=gate.PROFILE, parsed=True, byte_exact=True,
        recovery_count=0, production_count=1, construction_gap=None) for i in range(21)]
    certificate = dict(fully_closed=True, incomplete_pairs=0, applicable_pairs=100,
        closed_pairs=100, required_requirements=200, closed_requirements=200)
    certificate.update({field: [1] * 32 for field in (*gate.CERT_FIELDS.values(),
                       "semantic_closure_digest", "revalidation_digest")})
    return dict(format="agq-sysml-systems-publication-audit/1", scope=None,
        publication_attempted=True, publication_accepted=True, reference_audit_scope="accepted_publication",
        sysml_profile=gate.PROFILE, construction_complete=True, systems_documents_parsed=21,
        systems_documents_constructed=21, systems_documents_byte_exact=21,
        kernel_obligations=0, kernel_obligation_details=[], authority_conflicts=[], kerml_producers_replayed=False,
        mandatory_references=dict(total=1327, failures=[], counts=dict(complete=1327, unresolved=0,
            incomplete=0, ambiguous=0, invalid=0, endpoint_mismatch=0)),
        publication_producers=dict(converged=True, completeness="Complete"), producer_closure=certificate,
        publication_digest=[1] * 32, semantic_digest=[1] * 32, accepted_kerml_digest=[1] * 32,
        publication_gate=dict(findings=[], mandatory_references=1327, complete_references=1327, checked=checked),
        accepted_bindings=69, bindings_stale_check=True, checkpoint_session=dict(accepted_authority=False),
        documents=documents)


class GateTests(unittest.TestCase):
    def test_strict_complete_report_passes(self):
        gate.accept_report(report_fixture())

    def test_pinned_source_archive_and_document_identities(self):
        root = Path(__file__).resolve().parents[2]
        manifest = json.loads((root / "standards/normative/sysml-2.0/library-set.json").read_bytes())
        systems, = [a for a in manifest["artifacts"] if a["specification"] == "SysML"]
        library = "6c418b74-7704-5af0-bce2-28b92196cb15"
        kerml = json.loads((root / "standards/kerml-accepted-publication.json").read_bytes())
        identity = kerml["complete_overlay"]["identity"]
        report = dict(accepted_kerml_profile=identity["operational_profile"],
            accepted_kerml_digest=identity["semantic_digest"], documents=[
                dict(path=e["path"], sha256=e["sha256"], document=gate.stable_id([
                    "agentique-library-document/1", library, e["path"], e["sha256"]]))
                for e in systems["entries"] if e["path"].endswith(".sysml")])
        actual = gate.source_pins(root, report)
        self.assertEqual(actual[2], library)
        self.assertEqual(len(actual[3]), 21)
        report["documents"][0]["sha256"] = "00" * 32
        with self.assertRaisesRegex(ValueError, "source bytes"):
            gate.source_pins(root, report)

    def test_equal_counts_cannot_mask_missing_or_failed_acceptance(self):
        mutations = [
            ("scope", []), ("publication_accepted", False), ("reference_audit_scope", "construction"),
            ("publication_attempted", False), ("kernel_obligations", False), ("kernel_obligations", 1),
            ("bindings_stale_check", False), ("authority_conflicts", ["conflict"]),
            ("systems_documents_byte_exact", 20), ("accepted_bindings", 68),
        ]
        for key, value in mutations:
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                report = report_fixture()
                report[key] = value
                gate.accept_report(report)
        for object_name, key, value in [
            ("publication_producers", "converged", False),
            ("publication_producers", "completeness", "Incomplete"),
            ("producer_closure", "fully_closed", False),
            ("producer_closure", "closed_pairs", 99),
            ("producer_closure", "closed_requirements", 199),
            ("producer_closure", "model_digest", [2] * 32),
            ("publication_gate", "findings", ["Identity(provenance)"]),
            ("publication_gate", "checked", {}),
        ]:
            with self.subTest(key=key), self.assertRaises(ValueError):
                report = report_fixture()
                report[object_name][key] = value
                gate.accept_report(report)
        for field in ("unresolved", "incomplete", "ambiguous", "invalid", "endpoint_mismatch"):
            with self.subTest(field=field), self.assertRaises(ValueError):
                report = report_fixture()
                report["mandatory_references"]["counts"][field] = 1
                gate.accept_report(report)
        with self.assertRaises(KeyError):
            report = report_fixture()
            del report["publication_gate"]
            gate.accept_report(report)

    def test_cache_and_receipt_corruption_fail_closed(self):
        root = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            report = report_fixture()
            report["exported_cache"] = str(directory / "canonical.publication.zip")
            report_path = directory / "report.json"
            report_path.write_bytes(gate.encoded(report))
            doc = report["documents"][0]
            library = str(gate.uuid.UUID(int=100))
            origin = dict(document=doc["document"], revision=str(gate.uuid.UUID(int=200)),
                          syntax_node=str(gate.uuid.UUID(int=300)), range=dict(start=0, end=1))
            docs = {doc["document"]: dict(path=doc["path"], sha256=doc["sha256"], raw=b"x", revision=origin["revision"])}
            identity = dict(publication_digest=[1] * 32, semantic_digest=[1] * 32, accepted_kerml_digest=[1] * 32,
                systems_kpar="22" * 32, systems_source_content_set=[0] * 32, operational_profile=gate.PROFILE,
                rule_set="agq-sysml-query/5", producer_registry_digest=[1] * 32, producer_closure_digest=[1] * 32,
                producer_context_contract_digest=[1] * 32, dependency_contract_digest=[1] * 32, combined_descriptor_graph=[1] * 32)
            for field, path in (("grammar_compatibility_manifest", "standards/grammar/sysml-2.0-operational-v1.json"),
                                ("semantic_correction_manifest", "standards/sysml-2.0-operational-semantic-v2.json")):
                identity[field] = list(bytes.fromhex(gate.sha((root / path).read_bytes().replace(b"\r\n", b"\n"))))
            bindings = copy.deepcopy(identity)
            bindings.update(format="agq-sysml-accepted-bindings/1", source_content_set="sha256:" + "00" * 32,
                systems_library=library, accepted_systems_digest=bindings.pop("publication_digest"),
                bindings=[dict(role=role, element=str(gate.uuid.UUID(int=i + 500)), metaclass=library,
                    expected_metaclass=library, source=origin, library=library, visibility="public",
                    qualified_path=["Fixture", role], source_path=doc["path"], source_sha256=doc["sha256"])
                    for i, role in enumerate(sorted(gate.ROLES))])
            (directory / "standard-bindings.json").write_bytes(gate.encoded(bindings))
            graph = gate.encoded({"DependentHeader": {"format": "agq-kernel-dependent-evidence-archive/1"}}) + b'\n"Overlay"\n"End"\n'
            graph_data = gate.graph_evidence(io.BytesIO(graph))
            facade = dict(format="agq-sysml-publication-facade/1", source_content_set=bindings["source_content_set"],
                mandatory_references=1327, complete_references=1327, checked=report["publication_gate"]["checked"],
                documents=report["documents"], roots=[library], source_map=[[{"Element": library}, origin]]
                    + [[{"Element": a["element"]}, origin] for a in bindings["bindings"]])
            closure = {a: report["producer_closure"][b] for a, b in gate.CERT_FIELDS.items()}
            closure["format"] = "agq-producer-closure-certificate/2"
            graph_data["certificate_receipt_digest"] = gate.sha(gate.encoded(closure))
            entries = {"facade.json": gate.encoded(facade), "closure.json": gate.encoded(closure), "kernel.jsonl": graph}
            receipt = dict(format="agq-sysml-accepted-publication/1", status="accepted", identity=identity,
                source_content_set=bindings["source_content_set"], binding_manifest_sha256=list(bytes.fromhex(gate.sha(gate.encoded(bindings)))),
                entries={name: dict(bytes=len(raw), sha256=list(bytes.fromhex(gate.sha(raw)))) for name, raw in entries.items()})
            receipt_path = directory / "accepted-publication.json"
            receipt_path.write_bytes(gate.encoded(receipt))

            def write_cache(payload):
                with zipfile.ZipFile(directory / "canonical.publication.zip", "w") as archive:
                    for name, raw in payload.items():
                        archive.writestr(name, raw)

            with patch.object(gate, "source_pins", return_value=(bindings["source_content_set"], "22" * 32, library, docs)), \
                 patch.object(gate, "checkpoint_evidence", return_value=graph_data):
                write_cache(entries)
                self.assertTrue(gate.validate(report_path, root)["passed"])
                for name in entries:
                    changed = dict(entries)
                    changed[name] += b" "
                    write_cache(changed)
                    with self.subTest(name=name), self.assertRaises(ValueError):
                        gate.validate(report_path, root)
                # Re-pinning changed cache bytes cannot hide disagreement with
                # the independently authenticated completed scheduler graph.
                changed = dict(entries)
                changed["kernel.jsonl"] = graph.replace(b'"End"', b'{"Record":{"id":"changed"}}\n"End"')
                receipt["entries"]["kernel.jsonl"] = dict(bytes=len(changed["kernel.jsonl"]),
                    sha256=list(bytes.fromhex(gate.sha(changed["kernel.jsonl"]))))
                receipt_path.write_bytes(gate.encoded(receipt))
                write_cache(changed)
                with self.assertRaisesRegex(ValueError, "cache/checkpoint graph evidence"):
                    gate.validate(report_path, root)
                receipt["entries"]["kernel.jsonl"] = dict(bytes=len(graph), sha256=list(bytes.fromhex(gate.sha(graph))))
                write_cache(entries)
                receipt["identity"]["producer_context_contract_digest"] = [8] * 32
                receipt_path.write_bytes(gate.encoded(receipt))
                with self.assertRaisesRegex(ValueError, "receipt certificate"):
                    gate.validate(report_path, root)


if __name__ == "__main__":
    unittest.main()
