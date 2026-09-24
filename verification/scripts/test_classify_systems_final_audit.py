import json
from pathlib import Path
import unittest

from classify_systems_final_audit import classify, diagnostics, summarize


FIXTURE = Path(__file__).resolve().parents[1] / "fixtures/final-audit-semantic-closure/prior-classification.json"


class FinalAuditClassification(unittest.TestCase):
    def test_retained_diagnostics_partition_without_losing_weight(self):
        fixture = json.loads(FIXTURE.read_bytes())
        rows = classify(fixture["diagnostics"], fixture["subjects"])
        self.assertEqual(rows, fixture["diagnostics"])
        summary = summarize(rows)
        self.assertEqual((summary["weighted_findings"], summary["distinct_diagnostics"], summary["subjects"]),
                         (674, 238, 50))
        self.assertEqual(summary["categories"]["Other"]["distinct_diagnostics"], 0)

    def test_unfamiliar_and_unexplained_diagnostics_remain_other(self):
        rows = [dict(code=code, subject="unseen", message=message, families={"DefinitionUsage": 1})
                for code, message in [("NEW_CODE", "new defect"),
                                      ("SQ_PUBLICATION_TYPED_QUERY", "effective return parameters is Incomplete; pending implications: {}"),
                                      ("SQ_ELEMENT_KIND", "Canonical element has the wrong SysML metaclass")]]
        self.assertEqual([row["category"] for row in classify(rows, {})], ["Other"] * 3)

    def test_missing_audit_is_not_zero_findings(self):
        with self.assertRaises(ValueError):
            diagnostics({"publication_accepted": False})
        self.assertEqual(diagnostics({"effective_sysml_audit": {"findings": []}}), [])
        with self.assertRaises(ValueError):
            diagnostics({"publication_gate": {"findings": ["Identity(closure missing)"]}})


if __name__ == "__main__":
    unittest.main()
