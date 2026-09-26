"""Acceptance receipt tests; no semantic runtime or native application is mocked."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


spec = importlib.util.spec_from_file_location("run_real_soak", Path(__file__).with_name("run_real_soak.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


def successful_report():
    return {"format": "agentique-native-real-acceptance/1", "scenario": "real-restart",
            "passed": True, "restart_verified": True, "outcome": "passed", "failure": None,
            "assertions": [{"name": "real observed native assertion", "passed": True}],
            "soak": {"elapsed_ms": 600123, "completed_cycles": 2, "navigation_restorations": 2,
                     "cancelled_preparations": 2, "validated_renames": 2, "binding_observations": 250,
                     "graph_reads_completed_during_preparation": 2, "maximum_zoom_anchor_error_world": 0.001}}


class SoakAcceptanceTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name) / "journey.json"

    def assess(self, report=None, samples=2400):
        self.path.write_text(json.dumps(report if report is not None else successful_report()), encoding="utf-8")
        return runner.assess_journey(self.path, 600, samples)

    def test_complete_observations_accept_zero_exit_and_retain_native_status(self):
        result = self.assess()
        self.assertTrue(result["passed"])
        self.assertTrue(result["native_report_passed"])
        self.assertEqual(result["errors"], [])
        self.assertEqual(len(result["report_sha256"]), 64)
        self.assertEqual(runner.acceptance_exit_code(0, False, result), 0)
        self.assertEqual(runner.acceptance_exit_code(7, False, result), 7)
        self.assertEqual(runner.acceptance_exit_code(0, True, result), 2)

    def test_early_window_close_cannot_turn_zero_process_exit_into_acceptance(self):
        report = successful_report()
        report.update(passed=False, restart_verified=False, outcome="running")
        report["soak"]["elapsed_ms"] = 1500
        result = self.assess(report)
        self.assertFalse(result["passed"])
        self.assertFalse(result["native_report_passed"])
        self.assertEqual(result["outcome"], "running")
        self.assertEqual(runner.acceptance_exit_code(0, False, result), 2)

    def test_missing_partial_and_non_object_report_fail_closed(self):
        self.assertFalse(runner.assess_journey(self.path, 600, 2400)["passed"])
        for payload in ('{"passed": true,', '[]', 'null'):
            with self.subTest(payload=payload):
                self.path.write_text(payload, encoding="utf-8")
                result = runner.assess_journey(self.path, 600, 2400)
                self.assertFalse(result["passed"])
                self.assertTrue(result["report_sha256"])
                self.assertTrue(result["errors"])

    def test_no_memory_observation_fails_without_rewriting_native_pass(self):
        result = self.assess(samples=0)
        self.assertFalse(result["passed"])
        self.assertTrue(result["native_report_passed"])
        self.assertIn("No process memory samples were observed", result["errors"])

    def test_missing_short_or_incomplete_cycle_evidence_fails(self):
        for field in ("elapsed_ms", "completed_cycles", "navigation_restorations",
                      "cancelled_preparations", "validated_renames", "binding_observations",
                      "graph_reads_completed_during_preparation"):
            for value in (None, 0, True):
                with self.subTest(field=field, value=value):
                    report = successful_report()
                    report["soak"][field] = value
                    self.assertFalse(self.assess(report)["passed"])
        report = successful_report()
        report["soak"]["navigation_restorations"] = 1
        self.assertFalse(self.assess(report)["passed"])

    def test_failed_missing_or_nonfinite_observations_fail(self):
        for assertions in ([], [{"passed": False}], [None]):
            report = successful_report()
            report["assertions"] = assertions
            self.assertFalse(self.assess(report)["passed"])
        for error in (None, -1, 0.026, float("nan"), float("inf")):
            report = successful_report()
            report["soak"]["maximum_zoom_anchor_error_world"] = error
            self.assertFalse(self.assess(report)["passed"])
        report = successful_report()
        report["failure"] = "Observed native failure"
        self.assertFalse(self.assess(report)["passed"])


if __name__ == "__main__":
    unittest.main()
