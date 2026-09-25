"""Pure bookkeeping guards; no source inventory, runtime or language queries."""
import copy
import unittest
from native_alpha_freshness_plan import REVIEWED, propose


class FreshnessPlanTests(unittest.TestCase):
    def setUp(self):
        self.expected = {
            "publication_identity": {"semantic_digest": [42]},
            "receipt_sha256": "fixed-receipt",
            "bindings_sha256": "fixed-bindings",
            "inputs": {source: before for source, (before, _) in REVIEWED.items()},
        }
        self.expected["inputs"]["other-source"] = "unchanged"

    def actual(self, sources):
        result = copy.deepcopy(self.expected)
        for source in sources:
            result["inputs"][source] = REVIEWED[source][1]
        return result

    def test_mount_then_audit_changes_only_remaining_two_entries(self):
        mounted, first = propose(self.expected, self.actual(list(REVIEWED)[:1]), "mount")
        final, second = propose(mounted, self.actual(REVIEWED), "mount-and-audit")
        self.assertEqual(len(first), 1)
        self.assertEqual(len(second), 2)
        self.assertEqual(final, self.actual(REVIEWED))
        self.assertEqual(self.expected["inputs"][next(iter(REVIEWED))], next(iter(REVIEWED.values()))[0])

    def test_combined_requires_exact_reviewed_source_not_any_current_hash(self):
        actual = self.actual(REVIEWED)
        actual["inputs"][next(iter(REVIEWED))] = "different-current-source"
        with self.assertRaisesRegex(ValueError, "differs from reviewed"):
            propose(self.expected, actual, "mount-and-audit")

    def test_unreviewed_existing_fingerprint_is_refused(self):
        expected = copy.deepcopy(self.expected)
        expected["inputs"][next(iter(REVIEWED))] = "unexpected-old-pin"
        with self.assertRaisesRegex(ValueError, "unreviewed existing"):
            propose(expected, self.actual(REVIEWED), "mount-and-audit")

    def test_unlisted_input_changes_and_authority_changes_are_refused(self):
        for label in ("source", "addition", "removal", "receipt", "bindings", "identity"):
            with self.subTest(label=label):
                actual = self.actual(REVIEWED)
                if label == "source":
                    actual["inputs"]["other-source"] = "changed"
                elif label == "addition":
                    actual["inputs"]["unexpected"] = "new"
                elif label == "removal":
                    del actual["inputs"]["other-source"]
                elif label == "identity":
                    actual["publication_identity"]["semantic_digest"] = [43]
                else:
                    actual[label + "_sha256"] = "changed"
                with self.assertRaisesRegex(ValueError, "unreviewed input population"):
                    propose(self.expected, actual, "mount-and-audit")

    def test_mount_phase_cannot_silently_accept_audit_changes(self):
        with self.assertRaisesRegex(ValueError, "unreviewed input population"):
            propose(self.expected, self.actual(REVIEWED), "mount")


if __name__ == "__main__":
    unittest.main()
