"""Offline evidence-contract tests. No native process or accepted assets needed."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location(
    "native_report", Path(__file__).with_name("summarize_native_performance.py"))
report = importlib.util.module_from_spec(spec)
spec.loader.exec_module(report)


class EvidenceContracts(unittest.TestCase):
    def test_failed_completed_run_retains_missing_stages_and_metrics(self):
        value = {
            "format": "agentique-native-real-acceptance/1", "outcome": "failed",
            "passed": False, "failure": "stopped before validation", "metrics": None,
            "assertions": [{"name": report.STAGES["prepare"], "passed": True,
                            "elapsed_ms": 143240, "since_start_ms": 496058}],
            "preparation_responsiveness": {"elapsed_ms": 140216},
        }
        summary = report.journey(value)
        self.assertEqual(summary["input_format"], value["format"])
        self.assertEqual(summary["input_version"], 1)
        self.assertIsNone(summary["engineering_evidence"])
        self.assertFalse(summary["passed"])
        self.assertIsNone(summary["semantic_stage_ui_steps"]["validate"])
        self.assertIsNone(summary["semantic_stage_ui_steps"]["commit"])
        self.assertIsNone(summary["final_metrics"])
        self.assertEqual(summary["semantic_stage_ui_steps"]["prepare"]["elapsed_ms"], 143240)
        self.assertEqual(summary["preparation_observation"]["elapsed_ms"], 140216)

    def test_v2_wrapper_core_steps_keep_stage_timings_and_new_evidence_separate(self):
        # Bookkeeping-only fixture; no semantic or native acceptance is asserted.
        names = [
            "open authenticated Agentique with Validated semantic closure",
            "select the actual NativeStudio definition",
            "enter NativeStudio to inspect its scene and bounded agent architecture",
            "return to the composed Studio architecture",
            "select the actual composed core definition",
            "enter the core through native keyboard input",
            "select the core before nested part creation",
            "enter the core before nested part creation",
            report.STAGES["prepare"], report.STAGES["validate"],
        ]
        evidence = {"studio_composition": [{"id": "fixture-studio-ownership", "revision_id": "fixture-r2"}],
                    "native_studio_composition": [], "native_studio_inspector": None,
                    "studio_requirement_subjects": [{"id": "fixture-subject-typing"}]}
        value = {
            "format": "agentique-native-real-acceptance/2", "scenario": "real",
            "outcome": "failed", "passed": False, "restart_verified": False,
            "project": "fixture-project-v2", "previous_report_digest": None,
            "resumed_baseline": False, "engineering_evidence": evidence,
            "assertions": [{"name": name, "passed": index < len(names) - 1,
                            "elapsed_ms": index + 10, "since_start_ms": (index + 1) * 100,
                            "after": {"binding": {"project": "fixture-project-v2", "revision": "fixture-r2"}}}
                           for index, name in enumerate(names)],
        }
        original = json.dumps(value, sort_keys=True)
        summary = report.journey(value)
        self.assertEqual(summary["input_format"], value["format"])
        self.assertEqual(summary["input_version"], 2)
        self.assertFalse(summary["passed"])
        self.assertFalse(summary["restart_verified"])
        self.assertFalse(summary["resumed_baseline"])
        self.assertIsNone(summary["previous_report_digest"])
        self.assertEqual([step["name"] for step in summary["steps"]], names)
        self.assertEqual(summary["first_asserted_validated_view_since_runner_start_ms"], 100)
        self.assertEqual(summary["semantic_stage_ui_steps"]["prepare"]["elapsed_ms"], 18)
        self.assertFalse(summary["semantic_stage_ui_steps"]["validate"]["passed"])
        self.assertIsNone(summary["semantic_stage_ui_steps"]["commit"])
        self.assertEqual(summary["engineering_evidence"], evidence)
        self.assertNotIn("requirement_subject_path", summary["engineering_evidence"])
        self.assertEqual(json.dumps(value, sort_keys=True), original)

    def test_unknown_versions_are_refused_and_legacy_metadata_is_not_promoted(self):
        legacy = {"format": "agentique-native-real-acceptance/1", "outcome": "failed",
                  "engineering_evidence": {"requirement_subject_path": []}}
        summary = report.journey(legacy)
        self.assertEqual(summary["engineering_evidence"], {"requirement_subject_path": []})
        self.assertNotIn("studio_composition", summary["engineering_evidence"])
        for unknown in [None, "agentique-native-real-acceptance/0", "agentique-native-real-acceptance/3",
                        "agentique-native-real-acceptance/2.1", "agentique-native-real-acceptance/02"]:
            with self.subTest(unknown=unknown), self.assertRaises(ValueError):
                report.journey({**legacy, "format": unknown})

    def test_incomplete_reports_or_ambiguous_stages_are_refused(self):
        with self.assertRaises(ValueError):
            report.completed_process({"exit_code": None})
        with self.assertRaises(ValueError):
            report.journey({"format": "agentique-native-real-acceptance/1", "outcome": "running"})
        with self.assertRaises(ValueError):
            report.journey({"format": "agentique-native-real-acceptance/1", "outcome": "failed",
                            "assertions": [{"name": report.STAGES["prepare"]}] * 2})
        self.assertEqual(report.completed_process({"exit_code": 2})["exit_code"], 2)

    def test_low_count_p95_is_suppressed_but_missing_measurements_are_not_zero(self):
        value = {"pan": {"window_samples": 1, "total_samples": 1, "p95": 9, "median": 9},
                 "zoom": {"window_samples": 120, "total_samples": 120, "p95": 12},
                 "gpu_timestamp_ms": None, "presentation_ms": None}
        summary = report.sampled(value)
        self.assertIsNone(summary["pan"]["p95"])
        self.assertEqual(summary["pan"]["median"], 9)
        self.assertEqual(summary["zoom"]["p95"], 12)
        self.assertIsNone(summary["gpu_timestamp_ms"])
        self.assertIsNone(summary["presentation_ms"])
        self.assertEqual(value["pan"]["p95"], 9)

    def test_profiles_keep_exact_bindings_hit_miss_and_failure_separate(self):
        base = {"format": "agentique-studio-inspector-cache/1", "project": "p",
                "revision": "r1", "operation": "revision_reader_inspect",
                "cache": "hit", "outcome": "ok", "elapsed_ms": 0.1}
        records = [base, {**base, "elapsed_ms": 0.3},
                   {**base, "cache": "miss", "elapsed_ms": 2000},
                   {**base, "revision": "r2", "elapsed_ms": 0.2},
                   {**base, "outcome": "error", "elapsed_ms": 10},
                   {**base, "format": "agentique-studio-projection-cache/1",
                    "operation": "platform_project", "elapsed_ms": 0.4}]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "process.log"
            path.write_bytes(b"native start\n" + b"\n".join(json.dumps(record).encode()
                            for record in records) + b"\n" + json.dumps(base).encode())
            inputs = report.Inputs()
            summary = report.logs(inputs, [path])
            self.assertEqual(len(summary["records"]), 6)
            self.assertEqual(len(summary["summaries"]), 5)
            self.assertEqual(summary["ignored_unterminated_lines"], 1)
            hit = next(group for group in summary["summaries"] if group["observations"] == 2)
            self.assertEqual(hit["median_ms"], 0.2)
            self.assertNotIn("p95", hit)
            self.assertEqual(summary["records"][0]["source_sha256"], inputs.artifacts[0]["sha256"])

    def test_bad_completed_profile_duration_is_not_silently_dropped(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "process.log"
            path.write_text(json.dumps({"format": report.PROFILE, "total_ms": -1}) + "\n")
            with self.assertRaises(ValueError):
                report.logs(report.Inputs(), [path])

    def test_image_hash_mismatch_and_cross_revision_capture_stay_explicit(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "01-system-world.json"
            path.with_suffix(".png").write_bytes(b"observed image")
            path.write_text(json.dumps({"fixture": None, "image_digest": "wrong",
                            "projection": {"revision_id": "r1", "nodes": [1], "edges": []},
                            "state": {"scene_revision": "r2"}, "metrics": None}))
            captures = report.gallery(report.Inputs(), directory)
            self.assertFalse(captures[0]["image_matches_sidecar"])
            self.assertFalse(captures[0]["scene_matches_projection_revision"])
            self.assertEqual(captures[0]["projection_nodes"], 1)
            self.assertIsNone(captures[0]["metrics"])


if __name__ == "__main__":
    unittest.main()
