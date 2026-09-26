"""Adversarial bookkeeping fixtures only; these never stand in for semantic proof."""
import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import freshness_proposal as gate


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.review = gate.read_review()

    def put(self, name, value):
        file = self.root / name
        file.parent.mkdir(parents=True, exist_ok=True)
        data = json.dumps(value).encode()
        file.write_bytes(data)
        return {"path": name, "sha256": gate.digest(data)}

    def oracle(self):
        observations = {name: "a" * 64 for name in [
            "source-checkpoint", "semantic-fingerprint", "semantic-closure", "diagnostics", "strict-audit",
            "strict-audit-subjects", "element/1", "occurrence/1", "query/1/effective_usages", "reference/1"]}
        command = {"validated": True, "negative_reuse_cases": 16, "canonical_elements": 1,
                   "authored_documents": 1, "owner_and_ancestor_identity_preserved": True,
                   "same_authenticated_mount_observed": True, "command_work": {"semantic_cache_used": False}}
        cold = {"validated": True, "full_work": {"semantic_cache_used": False, "documents_reparsed": 1}}
        result = {**command, **cold, "format": "agentique-create-part-performance/4", "exact_equivalence": True,
                  "independent_cold_mount_observed": True, "exact_observation_count": len(observations)}
        build = {"format": "agentique-native-acceptance-build/1", "component": "oracle", "outcome": "passed",
                 "source_commit": "b" * 40, "binaries": [{"sha256": "c" * 64}],
                 "commands": [{"command": ["cargo", "build"], "exit_code": 0}],
                 "source_files": {"crates/modeling-agent/tests/create_part_performance.rs": "d" * 64}}
        invocation = {"source_commit": build["source_commit"], "outcome": "passed", "exit_code": 0,
                      "executable_sha256": "c" * 64, "oracle_harness_sha256": "d" * 64,
                      "runtime_sha256": gate.BUNDLE_SHA256,
                      "command": ["test.exe", gate.TEST_NAME, "--exact", "--ignored", "--test-threads=1"]}
        values = {"build": build, "invocation": invocation, "result": result, "command_metrics": command,
                  "cold_metrics": cold, "command_observations": observations, "cold_observations": copy.deepcopy(observations)}
        return values

    def oracle_refs(self, values):
        return {key: self.put("oracle/" + key + ".json", value) for key, value in values.items()}

    def runtime(self):
        expected = self.review["accepted_identity"]
        manifest = {"format": "agq-accepted-publication-bundle/1", "identity": gate.BUNDLE_ID}
        for family, profile, publication in [
            ("kerml", "agentique-kerml-1.0-operational/9", expected["accepted_kerml_digest"]),
            ("systems", "agentique-sysml-2.0-operational/3", expected["publication_digest"]),
        ]:
            manifest[family] = {"file": family + ".cache", "bytes": 10, "sha256": "e" * 64,
                                "publication": {"profile": profile, "publication_digest": publication}}
        timings = dict.fromkeys(["transport_verify_ms", "sources_verify_ms", "kerml_restore_ms", "systems_restore_ms"], 1)
        outputs = {"verify": timings, "install": {"directory": "isolated/store", "manifest": manifest, "timings": timings},
                   "status": {"origin": "installed"}}
        qualification = {"format": "agq-runtime-distribution-qualification/1", "passed": True,
                         "expected_sha256": gate.BUNDLE_SHA256, "bundle_sha256": gate.BUNDLE_SHA256,
                         "bundle_identity": gate.BUNDLE_ID, "installer_sha256": "f" * 64,
                         "installer": "agq-publications", "installed_directory": "isolated/store", "commands": []}
        for name, output in outputs.items():
            stdout = self.put("runtime/" + name + ".stdout.json", output)
            stderr = self.put("runtime/" + name + ".stderr.txt", "Ready")
            qualification["commands"].append({"command": ["agq-publications", name], "exit_code": 0,
                                              "stdout": Path(stdout["path"]).name, "stdout_sha256": stdout["sha256"],
                                              "stderr": Path(stderr["path"]).name, "stderr_sha256": stderr["sha256"]})
        return {"transport": self.put("runtime/transport.json", {"sha256": gate.BUNDLE_SHA256, "source_commit": "b" * 40}),
                "qualification": self.put("runtime/qualification.json", qualification)}

    def test_complete_oracle_checks_maps_not_only_success_flag(self):
        refs = self.oracle_refs(self.oracle())
        commits = []
        result = gate.verify_oracle(gate.Artifacts(self.root), refs, commits.append)
        self.assertEqual(commits, ["b" * 40])
        self.assertEqual(result["observations"], 10)

    def test_equal_counts_cannot_hide_different_proof_or_missing_population(self):
        for action in ["change-proof", "missing-population", "missing-family"]:
            with self.subTest(action=action):
                values = self.oracle()
                if action == "change-proof":
                    values["cold_observations"]["strict-audit"] = "0" * 64
                elif action == "missing-population":
                    del values["cold_observations"]["occurrence/1"]
                else:
                    del values["cold_observations"]["query/1/effective_usages"]
                    del values["command_observations"]["query/1/effective_usages"]
                    values["result"]["exact_observation_count"] -= 1
                with self.assertRaises(ValueError):
                    gate.verify_oracle(gate.Artifacts(self.root), self.oracle_refs(values), lambda _: None)

    def test_failed_stale_cached_or_unvalidated_oracle_is_refused(self):
        mutations = [
            lambda v: v["invocation"].update(exit_code=101),
            lambda v: v["invocation"].update(source_commit="0" * 40),
            lambda v: v["invocation"].update(executable_sha256="0" * 64),
            lambda v: v["invocation"].update(runtime_sha256="0" * 64),
            lambda v: v["cold_metrics"].update(validated=False),
            lambda v: v["cold_metrics"]["full_work"].update(semantic_cache_used=True),
            lambda v: v["result"].update(exact_equivalence=False),
            lambda v: v["result"].update(canonical_elements=2),
            lambda v: v["command_metrics"].update(negative_reuse_cases=15),
        ]
        for index, mutate in enumerate(mutations):
            with self.subTest(index=index):
                values = self.oracle()
                mutate(values)
                with self.assertRaises(ValueError):
                    gate.verify_oracle(gate.Artifacts(self.root), self.oracle_refs(values), lambda _: None)

    def test_runtime_requires_hash_bound_ordinary_success_and_accepted_identity(self):
        refs = self.runtime()
        self.assertEqual(gate.verify_runtime(gate.Artifacts(self.root), refs, self.review, lambda _: None)["bundle_sha256"], gate.BUNDLE_SHA256)
        for action in ["failed", "transport", "output-tamper", "authority"]:
            with self.subTest(action=action):
                refs = self.runtime()
                record = gate.decode((self.root / refs["qualification"]["path"]).read_bytes())
                if action == "failed":
                    record["commands"][0]["exit_code"] = 1
                elif action == "transport":
                    record["bundle_sha256"] = "0" * 64
                elif action == "output-tamper":
                    (self.root / "runtime/verify.stdout.json").write_text("{}")
                else:
                    output = gate.decode((self.root / "runtime/install.stdout.json").read_bytes())
                    output["manifest"]["systems"]["publication"]["publication_digest"] = "0" * 64
                    record["commands"][1]["stdout_sha256"] = self.put("runtime/install.stdout.json", output)["sha256"]
                refs["qualification"] = self.put("runtime/qualification.json", record)
                with self.assertRaises(ValueError):
                    gate.verify_runtime(gate.Artifacts(self.root), refs, self.review, lambda _: None)

    def test_artifact_substitution_and_path_escape_are_refused(self):
        reference = self.put("proof.json", {})
        artifacts = gate.Artifacts(self.root)
        artifacts.json(reference)
        (self.root / "proof.json").write_text("changed")
        with self.assertRaisesRegex(ValueError, "changed"):
            artifacts.unchanged()
        with self.assertRaisesRegex(ValueError, "hash mismatch"):
            artifacts.json(reference)
        with self.assertRaisesRegex(ValueError, "relative"):
            artifacts.bytes({"path": str(self.root / "proof.json"), "sha256": "0" * 64})
        with self.assertRaisesRegex(ValueError, "Duplicate"):
            gate.decode(b'{"passed":false,"passed":true}')

    def test_source_binding_failure_is_not_ignored(self):
        refs = self.oracle_refs(self.oracle())
        def changed_source(_):
            raise ValueError("source changed")
        with self.assertRaisesRegex(ValueError, "source changed"):
            gate.verify_oracle(gate.Artifacts(self.root), refs, changed_source)

    def test_git_source_binding_rejects_new_runtime_input_and_dirty_source(self):
        def git(*args):
            subprocess.run(["git", *args], cwd=self.root, check=True, capture_output=True)
        git("init")
        git("config", "user.name", "Bookkeeping test")
        git("config", "user.email", "test@example.invalid")
        source = self.root / "crates/kernel/src"
        source.mkdir(parents=True)
        (source / "lib.rs").write_text("// unchanged\n")
        git("add", ".")
        git("commit", "-m", "fixture")
        commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=self.root, text=True).strip()
        gate.sources_match(self.root, commit)
        (source / "new.rs").write_text("// newly compiled module\n")
        with self.assertRaisesRegex(ValueError, "Untracked"):
            gate.sources_match(self.root, commit)
        (source / "new.rs").unlink()
        (source / "lib.rs").write_text("// changed\n")
        with self.assertRaisesRegex(ValueError, "differs"):
            gate.sources_match(self.root, commit)

    def test_only_exact_reviewed_test_delta_with_retained_gate_can_follow_proof(self):
        repository = Path(__file__).resolve().parents[2]
        before = subprocess.check_output(["git", "show",
            "a6e1f41d2b549a8be10a79dfa3ec786c090b65b3:" + gate.REVIEWED_TEST_PATH], cwd=repository)
        def git(*args):
            subprocess.run(["git", *args], cwd=self.root, check=True, capture_output=True)
        git("init")
        git("config", "user.name", "Bookkeeping test")
        git("config", "user.email", "test@example.invalid")
        source = self.root / gate.REVIEWED_TEST_PATH
        source.parent.mkdir(parents=True)
        source.write_bytes(before)
        git("add", ".")
        git("commit", "-m", "reviewed before fixture")
        commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=self.root, text=True).strip()
        after = (repository / gate.REVIEWED_TEST_PATH).read_bytes()
        source.write_bytes(after)
        for file in [gate.REVIEWED_TEST_RECEIPT, gate.REVIEWED_TEST_OUTPUT]:
            target = self.root / file
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes((repository / file).read_bytes())
        exceptions = gate.sources_match(self.root, commit)
        self.assertEqual(len(exceptions), 1)
        self.assertEqual(exceptions[0]["after"], gate.REVIEWED_TEST_AFTER)
        source.write_bytes(after + b"\n// unreviewed test addition\n")
        with self.assertRaisesRegex(ValueError, "exact reviewed"):
            gate.sources_match(self.root, commit)
        source.write_bytes(after)
        (self.root / gate.REVIEWED_TEST_OUTPUT).write_text("8 passed; 0 failed; 0 ignored")
        with self.assertRaisesRegex(ValueError, "evidence changed"):
            gate.sources_match(self.root, commit)
        with self.assertRaisesRegex(ValueError, "Unreviewed"):
            gate.reviewed_test_change(self.root, commit, "crates/kerml-semantics/tests/unit/other.rs")

    def test_frozen_scope_rejects_every_unreviewed_change(self):
        expected = {"publication_identity": {"fixed": True}, "inputs": {"Cargo.lock": "keep"}}
        actual = copy.deepcopy(expected)
        for item in self.review["changes"]:
            if item["before"] is not None:
                expected["inputs"][item["file"]] = item["before"]
            actual["inputs"][item["file"]] = item["after"]
        self.assertEqual(gate.propose(expected, actual, self.review), actual)
        first = self.review["changes"][0]["file"]
        for action in ["missing-new", "unexpected-new", "source-drift", "authority", "lock", "removal"]:
            changed = copy.deepcopy(actual)
            if action == "missing-new":
                del changed["inputs"][first]
            elif action == "unexpected-new":
                changed["inputs"]["crates/kernel/src/unreviewed.rs"] = "0" * 64
            elif action == "source-drift":
                changed["inputs"][first] = "0" * 64
            elif action == "authority":
                changed["publication_identity"]["fixed"] = False
            elif action == "lock":
                changed["inputs"]["Cargo.lock"] = "new pin"
            else:
                del changed["inputs"]["Cargo.lock"]
            with self.subTest(action=action), self.assertRaises(ValueError):
                gate.propose(expected, changed, self.review)

    def test_no_output_on_missing_proof_or_existing_proposal(self):
        output = self.root / "already"
        output.mkdir()
        (output / "proposal.patch").write_text("retained")
        with self.assertRaisesRegex(ValueError, "overwrite"):
            gate.prepare(self.root, self.root / "missing-proof.json", "0" * 64, output)
        self.assertEqual((output / "proposal.patch").read_text(), "retained")
        with self.assertRaises(FileNotFoundError):
            gate.verify_proofs(self.root / "missing-proof.json", "0" * 64, self.root, self.review)

    def test_patch_emission_preserves_original_ledger_and_refuses_source_race(self):
        # Exercise output mechanics with mocked qualification only. This is not
        # a successful semantic proof and emits solely into a temporary folder.
        repository = Path(__file__).resolve().parents[2]
        original = (repository / gate.MANIFEST).read_bytes()
        ledger = self.root / gate.MANIFEST
        ledger.parent.mkdir(parents=True)
        ledger.write_bytes(original)
        expected = gate.decode(original)
        actual = copy.deepcopy(expected)
        for entry in self.review["changes"]:
            actual["inputs"][entry["file"]] = entry["after"]
        observed = {"actual": actual, "lockCompatibility": {"status": "fixture-only"}}
        proof_file = self.root / "fixture-proof.json"
        proof_file.write_text("{}")
        summary = {"oracle": {"source_commit": "b" * 40}, "runtime": {"source_commit": "b" * 40}}
        artifacts = gate.Artifacts(self.root)
        with patch.object(gate, "verify_proofs", return_value=(artifacts, summary)), \
             patch.object(gate, "sources_match"), patch.object(gate, "capture", return_value=observed):
            gate.prepare(self.root, proof_file, "0" * 64, self.root / "proposal")
        self.assertEqual(ledger.read_bytes(), original)
        result = gate.decode((self.root / "proposal/proposal.json").read_bytes())
        self.assertEqual(len(result["changes"]), 23)
        self.assertFalse(result["grants_publication_authority"])
        patch_text = (self.root / "proposal/proposal.patch").read_text()
        self.assertIn("+++ b/" + gate.MANIFEST, patch_text)
        self.assertNotIn('+  "publication_identity"', patch_text)
        changed = copy.deepcopy(observed)
        changed["actual"]["inputs"]["unreviewed"] = "0" * 64
        with patch.object(gate, "verify_proofs", return_value=(artifacts, summary)), \
             patch.object(gate, "capture", side_effect=[observed, changed]), \
             self.assertRaisesRegex(ValueError, "changed during"):
            gate.prepare(self.root, proof_file, "0" * 64, self.root / "race")
        self.assertFalse((self.root / "race").exists())
        self.assertEqual(ledger.read_bytes(), original)

    def test_proof_set_hash_and_explicit_review_are_required(self):
        refs = self.oracle_refs(self.oracle())
        runtime = self.runtime()
        proof = {"format": "agentique-alpha-freshness-proof-set/1", "reviewer": "test reviewer",
                 "review_note": "Synthetic bookkeeping only", "oracle": refs, "runtime": runtime}
        ref = self.put("proof-set.json", proof)
        with patch.object(gate, "sources_match"):
            artifacts, summary = gate.verify_proofs(self.root / ref["path"], ref["sha256"], self.root, self.review)
            artifacts.unchanged()
            self.assertEqual(summary["proof_set_sha256"], ref["sha256"])
            with self.assertRaisesRegex(ValueError, "hash mismatch"):
                gate.verify_proofs(self.root / ref["path"], "0" * 64, self.root, self.review)
            proof["reviewer"] = None
            ref = self.put("proof-set.json", proof)
            with self.assertRaisesRegex(ValueError, "independent proof review"):
                gate.verify_proofs(self.root / ref["path"], ref["sha256"], self.root, self.review)


if __name__ == "__main__":
    unittest.main()
