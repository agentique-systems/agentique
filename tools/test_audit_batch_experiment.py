import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    "audit_batch", Path(__file__).with_name("audit-batch-experiment.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class ExperimentEvidenceTests(unittest.TestCase):
    def observations(self):
        return {"source-checkpoint": "source", "strict-audit": "audit",
                "semantic-closure": "closure", "query/1/effective_names": "full-proof"}

    def test_exact_query_proof_change_fails_even_when_graph_and_audit_match(self):
        baseline = self.observations()
        changed = dict(baseline, **{"query/1/effective_names": "different-proof"})
        result = runner.compare_observations(baseline, changed)
        self.assertFalse(result["exact_equivalence"])
        self.assertEqual(result["different"], ["query/1/effective_names"])

    def test_missing_and_extra_observations_fail(self):
        baseline = self.observations()
        changed = dict(baseline)
        del changed["query/1/effective_names"]
        changed["query/2/effective_names"] = "full-proof"
        result = runner.compare_observations(baseline, changed)
        self.assertFalse(result["exact_equivalence"])
        self.assertEqual(len(result["missing"]), 1)
        self.assertEqual(len(result["extra"]), 1)

    def test_empty_or_count_only_baseline_cannot_pass(self):
        for baseline in ({}, {"element-count": 76153}, {"strict-audit": "audit"}):
            with self.assertRaises(ValueError):
                runner.compare_observations(baseline, baseline)

    def test_equal_full_observations_pass(self):
        result = runner.compare_observations(self.observations(), self.observations())
        self.assertTrue(result["exact_equivalence"])
        self.assertEqual(result["query_observation_count"], 1)

    def test_larger_batch_requires_success_and_measured_memory_headroom(self):
        mib = 1024 ** 2
        previous = {"exit_code": 0, "terminated_reason": None,
                    "peak_rss_bytes": 4000 * mib,
                    "minimum_host_available_bytes": 2500 * mib}
        self.assertTrue(runner.larger_batch_is_safe(previous, 10000*mib, 8000*mib, 512*mib)[0])
        for changed in (dict(previous, exit_code=1),
                        dict(previous, terminated_reason="timeout"),
                        dict(previous, peak_rss_bytes=7500*mib),
                        dict(previous, minimum_host_available_bytes=800*mib)):
            self.assertFalse(runner.larger_batch_is_safe(changed, 10000*mib, 8000*mib, 512*mib)[0])


if __name__ == "__main__":
    unittest.main()
