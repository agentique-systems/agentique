"""Selected proof/search equality is independent of archive pool numbering."""
import copy
import hashlib
import io
import json
from pathlib import Path
import tempfile
import unittest
import zipfile

from frontier_artifact_evidence import StateReader, checkpoint_evidence


def encoded(value):
    return json.dumps(value, separators=(",", ":")).encode()


def sha(raw):
    return list(hashlib.sha256(raw).digest())


class ArtifactTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.cert = dict(model_digest=[1] * 32, producer_registry_digest=[2] * 32,
                         context_contract_digest=[3] * 32, digest=[4] * 32)
        receipt = dict(model_digest=[1] * 32, registry_digest=[2] * 32,
                       context_contract_digest=[3] * 32, digest=[4] * 32)
        self.state = dict(converged=True, model_digest=[1] * 32, context_contract=[3] * 32,
                          dependencies=[["ignored", 'escaped \\" {} []'] for _ in range(3000)],
                          certificate=dict(receipt=receipt, atoms=[], rows=[]),
                          evaluation_rows=[])
        self.proof = {"rule": "selected-rule", "dependencies": ["selected-source"]}
        self.search = [{"Element": "selected-provider"}]
        self.row = dict(element="a", property="b", target="c", position=0, proof=0, searches=0)

    def report(self, *, reverse=False, proof=None, search=None, state=None,
               entry_overrides=None, graph_suffix=b"", duplicate=False):
        selected_proof = self.proof if proof is None else proof
        selected_search = self.search if search is None else search
        proofs = [selected_proof, {"rule": "unrelated", "dependencies": []}]
        searches = [selected_search, []]
        row = copy.deepcopy(self.row)
        if reverse:
            proofs.reverse()
            searches.reverse()
            row.update(proof=1, searches=1)
        entries = ([{"Proof": value} for value in proofs]
                   + [{"Search": value} for value in searches]
                   + [{"ReferenceContribution": row}])
        if duplicate:
            entries.append({"ReferenceContribution": row})
        graph = b"".join(encoded(value) + b"\n" for value in entries + ["End"]) + graph_suffix
        state_bytes = encoded(self.state if state is None else state)
        output = io.BytesIO()
        with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            archive.writestr("state.json", state_bytes)
            archive.writestr("graph.jsonl", graph)
        archive_bytes = output.getvalue()
        archive_pin = hashlib.sha256(archive_bytes).hexdigest()
        (self.root / f"{archive_pin}.zip").write_bytes(archive_bytes)
        entry = dict(input_identity=[5] * 32, archive_sha256=sha(archive_bytes),
                     state_sha256=sha(state_bytes), graph_sha256=sha(graph))
        entry.update(entry_overrides or {})
        journal = encoded(dict(format="agq-unaccepted-publication-frontier/1",
                               source_identity=[6] * 32, entries=[entry]))
        pin = hashlib.sha256(journal).hexdigest()
        path = self.root / f"journal-{pin}.json"
        path.write_bytes(journal)
        return dict(producer_closure=self.cert, checkpoint_session=dict(
            latest=dict(journal=str(path), sha256=pin)))

    def test_pool_numbering_does_not_change_selected_evidence(self):
        a = checkpoint_evidence(self.report())
        b = checkpoint_evidence(self.report(reverse=True))
        self.assertNotEqual(a["archive_sha256"], b["archive_sha256"])
        self.assertEqual(a["selected_reference_digest"], b["selected_reference_digest"])
        self.assertEqual(a["contributions"], 1)

    def test_selected_proof_and_search_changes_remain_visible(self):
        before = checkpoint_evidence(self.report())["selected_reference_digest"]
        for changes in (dict(proof={"rule": "different", "dependencies": []}),
                        dict(search=[{"Element": "different"}])):
            self.assertNotEqual(before, checkpoint_evidence(self.report(**changes))[
                "selected_reference_digest"])

    def test_byte_authentication_and_state_binding_fail_closed(self):
        for key in ("state_sha256", "graph_sha256"):
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, "hash"):
                checkpoint_evidence(self.report(entry_overrides={key: [0] * 32}))
        for key, value in (("converged", False), ("model_digest", [8] * 32),
                           ("context_contract", [8] * 32)):
            state = copy.deepcopy(self.state)
            state[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                checkpoint_evidence(self.report(state=state))
        state = copy.deepcopy(self.state)
        state["certificate"]["receipt"]["registry_digest"] = [8] * 32
        with self.assertRaisesRegex(ValueError, "receipt binding"):
            checkpoint_evidence(self.report(state=state))
        report = self.report()
        Path(report["checkpoint_session"]["latest"]["journal"]).write_bytes(b"{}")
        with self.assertRaisesRegex(ValueError, "journal hash"):
            checkpoint_evidence(report)
        report = self.report()
        pin = checkpoint_evidence(report)["archive_sha256"]
        (self.root / f"{pin}.zip").write_bytes(b"changed")
        with self.assertRaisesRegex(ValueError, "archive hash"):
            checkpoint_evidence(report)

    def test_invalid_graph_pool_duplicate_and_trailing_rows_reject(self):
        self.row["proof"] = 10
        with self.assertRaisesRegex(ValueError, "pool index"):
            checkpoint_evidence(self.report())
        self.row["proof"] = 0
        with self.assertRaisesRegex(ValueError, "duplicate"):
            checkpoint_evidence(self.report(duplicate=True))
        with self.assertRaisesRegex(ValueError, "after End"):
            checkpoint_evidence(self.report(graph_suffix=b"{}\n"))

    def test_streamed_skips_handle_chunk_boundaries_and_escaped_strings(self):
        class SmallChunks(io.BytesIO):
            def read(self, length):
                return super().read(min(length, 7))
        raw = encoded(dict(ignored=[{"key": '\\"}[]\\\\"'}] * 100,
                           selected=dict(a=[1, 2, 3])))
        reader = StateReader(SmallChunks(raw))
        self.assertEqual(reader.fields({"selected"}), {"selected": {"a": [1, 2, 3]}})
        reader.whitespace()
        self.assertFalse(reader.peek())
        self.assertEqual(reader.digest.hexdigest(), hashlib.sha256(raw).hexdigest())


if __name__ == "__main__":
    unittest.main()
