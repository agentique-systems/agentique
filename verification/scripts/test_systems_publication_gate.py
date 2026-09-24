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


def report_fixture(profile=gate.PROFILE):
    checked = {family: 1 for family in gate.FAMILIES}
    checked.update(Syntax=21, CanonicalLowering=21, StandardBindings=69)
    documents = [dict(path=f"fixture/{i}.sysml", document=str(gate.uuid.UUID(int=i + 1)),
        sha256="00" * 32, profile=profile, parsed=True, byte_exact=True,
        recovery_count=0, production_count=1, construction_gap=None) for i in range(21)]
    certificate = dict(fully_closed=True, incomplete_pairs=0, applicable_pairs=100,
        closed_pairs=100, required_requirements=200, closed_requirements=200)
    certificate.update({field: [1] * 32 for field in (*gate.CERT_FIELDS.values(),
                       "semantic_closure_digest", "revalidation_digest")})
    return dict(format="agq-sysml-systems-publication-audit/1", scope=None,
        publication_attempted=True, publication_accepted=True, reference_audit_scope="accepted_publication",
        sysml_profile=profile, construction_complete=True, systems_documents_parsed=21,
        systems_documents_constructed=21, systems_documents_byte_exact=21,
        kernel_obligations=0, kernel_obligation_details=[], authority_conflicts=[], kerml_producers_replayed=False,
        mandatory_references=dict(total=1327, failures=[], counts=dict(complete=1327, unresolved=0,
            incomplete=0, ambiguous=0, invalid=0, endpoint_mismatch=0)),
        publication_producers=dict(converged=True, completeness="Complete"), producer_closure=certificate,
        publication_digest=[1] * 32, semantic_digest=[1] * 32, accepted_kerml_digest=[1] * 32,
        publication_gate=dict(findings=[], mandatory_references=1327, complete_references=1327, checked=checked),
        accepted_bindings=69, bindings_stale_check=True, checkpoint_session=dict(accepted_authority=False),
        candidate_cache_restored=True, candidate_restore_producers_replayed=False,
        artifact_promotion="atomic_directory_rename", documents=documents)


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

    def test_v3_report_requires_one_explicit_supported_profile(self):
        for profile in gate.PROFILES:
            report = report_fixture(profile)
            gate.accept_report(report)
            report["documents"][0]["profile"] = "mismatched-profile"
            with self.assertRaisesRegex(ValueError, "document status"):
                gate.accept_report(report)
        report = report_fixture("agentique-sysml-2.0-operational/4")
        with self.assertRaisesRegex(ValueError, "profile/construction"):
            gate.accept_report(report)

    def test_equal_counts_cannot_mask_missing_or_failed_acceptance(self):
        mutations = [
            ("scope", []), ("publication_accepted", False), ("reference_audit_scope", "construction"),
            ("publication_attempted", False), ("kernel_obligations", False), ("kernel_obligations", 1),
            ("bindings_stale_check", False), ("authority_conflicts", ["conflict"]),
            ("systems_documents_byte_exact", 20), ("accepted_bindings", 68),
            ("candidate_cache_restored", False), ("candidate_restore_producers_replayed", True),
            ("artifact_promotion", "individual_file_renames"),
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
        for profile, (rule_sets, _) in gate.PROFILES.items():
            for rule_set in rule_sets:
                with self.subTest(profile=profile, rule_set=rule_set):
                    self.check_cache_and_receipt_corruption(profile, rule_set)

    def check_cache_and_receipt_corruption(self, profile, rule_set):
        root = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            report = report_fixture(profile)
            report["exported_cache"] = str(directory / "canonical.publication.zip")
            report_path = directory / "report.json"
            report_path.write_bytes(gate.encoded(report))
            doc = report["documents"][0]
            library = str(gate.uuid.UUID(int=100))
            origin = dict(document=doc["document"], revision=str(gate.uuid.UUID(int=200)),
                          syntax_node=str(gate.uuid.UUID(int=300)), range=dict(start=0, end=1))
            docs = {doc["document"]: dict(path=doc["path"], sha256=doc["sha256"], raw=b"x", revision=origin["revision"])}
            identity = dict(publication_digest=[1] * 32, semantic_digest=[1] * 32, accepted_kerml_digest=[1] * 32,
                systems_kpar="22" * 32, systems_source_content_set=[0] * 32, operational_profile=profile,
                rule_set=rule_set, producer_registry_digest=[1] * 32, producer_closure_digest=[1] * 32,
                producer_context_contract_digest=[1] * 32, dependency_contract_digest=[1] * 32, combined_descriptor_graph=[1] * 32)
            for field, path in (("grammar_compatibility_manifest", "standards/grammar/sysml-2.0-operational-v1.json"),
                                ("semantic_correction_manifest", gate.PROFILES[profile][1])):
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
                # Updating both documents and their digest cannot authorize an
                # unsupported profile/rule-set combination. The independent
                # certificate and graph checks remain required below.
                for unsupported in ("agq-sysml-query/4", "agq-sysml-query/7", "agq-sysml-query/5"):
                    if unsupported in gate.PROFILES[profile][0]:
                        continue
                    changed_receipt, changed_bindings = copy.deepcopy(receipt), copy.deepcopy(bindings)
                    changed_receipt["identity"]["rule_set"] = unsupported
                    changed_bindings["rule_set"] = unsupported
                    changed_receipt["binding_manifest_sha256"] = list(bytes.fromhex(gate.sha(gate.encoded(changed_bindings))))
                    receipt_path.write_bytes(gate.encoded(changed_receipt))
                    (directory / "standard-bindings.json").write_bytes(gate.encoded(changed_bindings))
                    with self.subTest(unsupported=unsupported), self.assertRaisesRegex(ValueError, "interpretation profile"):
                        gate.validate(report_path, root)
                receipt_path.write_bytes(gate.encoded(receipt))
                (directory / "standard-bindings.json").write_bytes(gate.encoded(bindings))
                # Consistently altered report/document profiles cannot re-label
                # a receipt issued under a different explicit interpretation.
                other = next(p for p in gate.PROFILES if p != profile)
                changed_report = report_fixture(other)
                changed_report["exported_cache"] = report["exported_cache"]
                report_path.write_bytes(gate.encoded(changed_report))
                with self.assertRaisesRegex(ValueError, "interpretation profile"):
                    gate.validate(report_path, root)
                report_path.write_bytes(gate.encoded(report))
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
