"""Tests of the screening evaluation report, on synthetic observations made here.

They check the arithmetic, not any model: no observation below is a measurement.
    python -m unittest discover -s tools -p "test_screening_report.py"
"""

import unittest

import screening_report as r


def observation(arm, case, label, decision, by="agent", outcome="answer", latency=200,
                cost=0.0001, probabilities=None, domain=None):
    return {
        "arm": arm, "case": case, "domain": domain or f"{case}.example", "split": "held-out",
        "label": label, "repeat": 1, "outcome": outcome, "decision": decision, "by": by,
        "latencyMs": latency, "deadlineMs": 500, "costUsd": cost, "usageComplete": cost is not None,
        "probabilities": probabilities, "confidence": None,
    }


class Bounds(unittest.TestCase):
    def test_exact_bounds_match_the_rule_of_three(self):
        # Zero of 600 harmful allowed: about 0.5% (the investigation's figure).
        self.assertAlmostEqual(r.upper_bound(0, 600), 0.00498, places=4)
        # About 3,000 cases with none are needed for about 0.1%.
        self.assertAlmostEqual(r.upper_bound(0, 3000), 0.000998, places=5)
        self.assertIsNone(r.upper_bound(0, 0))
        self.assertEqual(r.upper_bound(5, 5), 1.0)
        # A known value: 2 of 100 has a one-sided 95% upper bound of 0.0616.
        self.assertAlmostEqual(r.upper_bound(2, 100), 0.0616, places=3)
        self.assertAlmostEqual(r.lower_bound(98, 100), 1 - r.upper_bound(2, 100), places=9)

    def test_percentiles_are_nearest_rank(self):
        self.assertEqual(r.percentile([5, 1, 3, 2, 4], 50), 3)
        self.assertEqual(r.percentile(list(range(1, 101)), 95), 95)
        self.assertIsNone(r.percentile([], 50))


class Summary(unittest.TestCase):
    def setUp(self):
        self.observations = [
            observation("jev", "a", "harmful", "block", probabilities={"allow": 0.1, "review": 0.1, "block": 0.8}),
            observation("jev", "b", "harmful", "allow", probabilities={"allow": 0.7, "review": 0.2, "block": 0.1}),
            observation("jev", "c", "benign", "allow", probabilities={"allow": 0.9, "review": 0.05, "block": 0.05}),
            observation("jev", "d", "benign", "review", by="fallback", outcome="lowConfidence",
                        probabilities={"allow": 0.5, "review": 0.4, "block": 0.1}),
            observation("jev", "e", "ambiguous", "review", by="fallback", outcome="timeout", latency=None, cost=None),
        ]

    def test_rates_counts_and_unknown_cost(self):
        s = r.summarise(self.observations, review_cost=0.5)
        self.assertEqual(s["harmful_allow"], (1, 2))
        self.assertEqual(s["benign_block"], (0, 2))
        self.assertEqual(s["benign_accept"], 0.5)
        self.assertEqual(s["confusion"]["harmful"]["allow"], 1)
        self.assertEqual(s["review_rate"], 0.4)
        self.assertEqual(s["fallback_rate"], 0.4)
        # Only the two agent answers within the deadline are valid.
        self.assertEqual(s["within_deadline"], 3 / 5)
        # A timeout counts at its deadline.
        self.assertEqual(s["latency_p95"], 500)
        self.assertEqual(s["unknown_cost_calls"], 1)
        # Unknown cost: no cost per workflow is claimed.
        self.assertIsNone(s["cost_per_correct_workflow_usd"])

    def test_calibration_uses_a_normalised_copy_and_declared_targets(self):
        s = r.summarise(self.observations)
        # Brier for the first observation: target block.
        first = (0.1 ** 2) + (0.1 ** 2) + (0.8 - 1) ** 2
        self.assertGreater(s["brier"], 0)
        self.assertAlmostEqual(r.normalised({"allow": 0.5, "review": 0.3, "block": 0.19})["allow"], 0.5 / 0.99)
        self.assertIsNone(r.normalised({"allow": 0, "review": 0, "block": 0}))
        only = r.summarise(self.observations[:1])
        self.assertAlmostEqual(only["brier"], first)
        self.assertAlmostEqual(only["log_loss"], -__import__("math").log(0.8))

    def test_cost_per_correct_workflow_prices_review(self):
        known = [o for o in self.observations if o["costUsd"] is not None]
        s = r.summarise(known, review_cost=0.5)
        # Correct: a (block), c (allow), d is a benign review (not correct).
        self.assertEqual(s["correct"], 2)
        self.assertAlmostEqual(s["cost_per_correct_workflow_usd"], (4 * 0.0001 + 1 * 0.5) / 2)

    def test_gates_are_proposed_and_say_when_undecidable(self):
        s = r.summarise(self.observations)
        verdicts = {name: verdict for name, _, _, _, verdict in r.gates(s)}
        self.assertEqual(verdicts["benign auto-acceptance"], "not met")
        empty = r.summarise([])
        self.assertTrue(all(v.startswith("not decidable") for _, _, _, _, v in r.gates(empty)))
        text = r.report(self.observations)
        self.assertIn("PROPOSED", text)
        self.assertIn("not requirements", text)
        self.assertNotIn("PASSED", text)


class Paired(unittest.TestCase):
    def test_bootstrap_is_paired_by_domain_and_repeatable(self):
        a, b = [], []
        for i in range(30):
            domain = f"d{i % 10}.example"
            a.append(observation("jev", f"c{i}", "benign", "allow" if i % 3 else "review", domain=domain))
            b.append(observation("chat", f"c{i}", "benign", "review", domain=domain))
        first = r.bootstrap_difference(a, b, r.accept_rate, seed=3)
        again = r.bootstrap_difference(a, b, r.accept_rate, seed=3)
        self.assertEqual(first, again)
        point, low, high = first
        self.assertAlmostEqual(point, 20 / 30)
        self.assertLessEqual(low, point)
        self.assertLessEqual(point, high)
        text = r.report(a + b, compare=("jev", "chat"))
        self.assertIn("domain-clustered bootstrap", text)


if __name__ == "__main__":
    unittest.main()
