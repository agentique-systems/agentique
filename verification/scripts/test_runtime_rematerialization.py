"""Reject transport recovery unless original accepted payload bytes are reproduced."""
import copy
import hashlib
import importlib.util
import json
import pathlib
import tempfile
import unittest
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("restore_transport", ROOT / "tools/restore-accepted-kerml-transport.py")
TOOL = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(TOOL)
COMPARE_SPEC = importlib.util.spec_from_file_location(
    "compare_systems", ROOT / "tools/runtime-recovery/compare_systems_contract.py")
COMPARE = importlib.util.module_from_spec(COMPARE_SPEC)
COMPARE_SPEC.loader.exec_module(COMPARE)


class SystemsContractComparison(unittest.TestCase):
    def setUp(self):
        self.accepted = {
            "identity": {"semantic_digest": [1] * 32},
            "status": "accepted",
            "entries": {name: {"bytes": 5, "sha256": [2] * 32}
                        for name in ("closure.json", "facade.json", "kernel.jsonl")},
        }
        self.generated = copy.deepcopy(self.accepted)
        self.bindings = {"anchors": [{"id": "canonical"}]}

    def compare(self, new_bindings=None):
        return COMPARE.compare_contract(self.accepted, self.generated, self.bindings,
                                        self.bindings if new_bindings is None else new_bindings)

    def test_transport_difference_does_not_authorize_runtime(self):
        self.generated["entries"]["kernel.jsonl"]["sha256"][0] ^= 1
        result = self.compare()
        self.assertTrue(result["semantic_contract_equal"])
        self.assertTrue(result["transport_entry_schema_equal"])
        self.assertFalse(result["original_transport_entries_equal"])
        self.assertFalse(result["runtime_accepted"])
        self.assertTrue(result["ordinary_facade_authentication_required"])

    def test_semantic_additions_omissions_and_changes_fail(self):
        for mode in ("add", "omit", "change"):
            self.generated = copy.deepcopy(self.accepted)
            if mode == "add":
                self.generated["unreviewed_authority"] = None
            elif mode == "omit":
                del self.generated["identity"]
            else:
                self.generated["identity"]["semantic_digest"][0] ^= 1
            self.assertFalse(self.compare()["semantic_contract_equal"], mode)

    def test_complete_binding_manifest_must_match(self):
        self.assertFalse(self.compare({"anchors": []})["accepted_bindings_equal"])

    def test_transport_receipt_shape_stays_exact(self):
        for mutation in (None, {}, {"attacker": {"bytes": 5, "sha256": [2] * 32}}):
            self.generated["entries"] = mutation
            self.assertFalse(self.compare()["transport_entry_schema_equal"])
        self.generated = copy.deepcopy(self.accepted)
        self.generated["entries"]["kernel.jsonl"]["sha256"][0] = True
        self.assertFalse(self.compare()["transport_entry_schema_equal"])


class ExactTransportRecovery(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.temp.name)
        self.addCleanup(self.temp.cleanup)
        self.cache = self.root / "regenerated.zip"
        self.output = self.root / "restored.zip"
        self.old = "5751331e-1f43-44a6-b18b-3816bd9df051"
        self.new = "00000000-0000-4000-8000-000000000001"
        self.metadata = b'{"fixture":"not a semantic publication"}'
        self.graph = ('{"Snapshot":{"revision":"' + self.old + '","used_ids":["element"]}}\n{"Record":{"name":"part"}}\n').encode()
        self.authority = {
            "format": "fixture-only", "binding_manifest_sha256": [1] * 32,
            "facade_metadata_bytes": len(self.metadata),
            "facade_metadata_sha256": list(hashlib.sha256(self.metadata).digest()),
            "complete_overlay": {
                "graph_bytes": len(self.graph),
                "graph_sha256": list(hashlib.sha256(self.graph).digest()),
                "identity": {"revision": self.old, "semantic_digest": [2] * 32, "rule_set": "fixture"},
            },
        }
        self.regenerated = self.graph.replace(self.old.encode(), self.new.encode())
        self.candidate = copy.deepcopy(self.authority)
        self.candidate["complete_overlay"]["identity"]["revision"] = self.new
        self.write_cache(self.regenerated)

    def write_cache(self, graph, extra=False):
        self.candidate["complete_overlay"]["graph_sha256"] = list(hashlib.sha256(graph).digest())
        self.candidate["complete_overlay"]["graph_bytes"] = len(graph)
        with zipfile.ZipFile(self.cache, "w") as archive:
            archive.writestr("facade.json", self.metadata)
            archive.writestr("kernel.jsonl", graph)
            if extra:
                archive.writestr("replacement-authority.json", b'{}')

    def restore(self):
        return TOOL.restore(self.cache, self.candidate, self.authority, self.output)

    def test_only_snapshot_label_is_restored_and_all_payload_bytes_match(self):
        original = copy.deepcopy(self.authority)
        result = self.restore()
        self.assertTrue(result["facade_authentication_required"])
        self.assertFalse(result["authority_receipt_changed"])
        self.assertEqual(self.authority, original)
        with zipfile.ZipFile(self.output) as archive:
            self.assertEqual(archive.read("kernel.jsonl"), self.graph)
            self.assertEqual(archive.read("facade.json"), self.metadata)

    def test_semantic_mismatch_cannot_be_labeled_transport(self):
        self.candidate["complete_overlay"]["identity"]["semantic_digest"][0] ^= 1
        with self.assertRaisesRegex(ValueError, "semantic/capability"):
            self.restore()
        self.assertFalse(self.output.exists())

    def test_canonical_record_mutation_cannot_be_promoted(self):
        self.write_cache(self.regenerated.replace(b'"part"', b'"fake"'))
        with self.assertRaisesRegex(ValueError, "not reproduced"):
            self.restore()
        self.assertFalse(self.output.exists())

    def test_input_hash_forgery_fails_before_promotion(self):
        self.candidate["complete_overlay"]["graph_sha256"][0] ^= 1
        with self.assertRaisesRegex(ValueError, "input bytes"):
            self.restore()
        self.assertFalse(self.output.exists())

    def test_package_cannot_supply_replacement_authority(self):
        self.write_cache(self.regenerated, extra=True)
        with self.assertRaisesRegex(ValueError, "Unexpected"):
            self.restore()
        self.assertFalse(self.output.exists())

    def test_existing_output_is_never_overwritten(self):
        self.output.write_bytes(b"retained")
        with self.assertRaisesRegex(ValueError, "already exists"):
            self.restore()
        self.assertEqual(self.output.read_bytes(), b"retained")

    def test_revision_label_elsewhere_is_not_rewritten(self):
        line = ('{"Record":{"name":"' + self.new + '"}}\n').encode()
        restored, changed = TOOL.restore_snapshot_label(line, self.new, self.old)
        self.assertFalse(changed)
        self.assertEqual(restored, line)


if __name__ == "__main__":
    unittest.main()
