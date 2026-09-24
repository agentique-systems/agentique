"""Audit timing is available only for a complete, internally consistent journal."""
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from uuid import UUID

from profile_systems_effective_audit import BATCH, EFFECTIVE, FORMAT, profile


def encode(events):
    return b"".join((json.dumps(event) + "\n").encode() for event in events)


def fixture(workers=2, enclosing=True):
    events = []
    tick = 0

    def event(stage, phase, duration, **fields):
        nonlocal tick
        tick += 100
        events.append(dict(format=FORMAT, publication_authority=False, stage=stage, event=phase,
                           elapsed_micros=duration, run_elapsed_micros=tick, findings=0, **fields))

    if enclosing:
        event(EFFECTIVE, "begin", 0)
    for window in range(0, 3, workers):
        for phase in ("begin", "end"):
            for index in range(window, min(window + workers, 3)):
                # Batch 1 runs faster, but joins are written in semantic order.
                event(BATCH, phase, [90, 10, 50][index] if phase == "end" else None,
                      batch_index=index, workers=workers, total_subjects=19, subjects=[8, 8, 3][index],
                      first_subject=str(UUID(int=8 * index + 1)), last_subject=str(UUID(int=min(8 * index + 8, 19))))
    if enclosing:
        event(EFFECTIVE, "end", 700)
    return events


class ProfileTests(unittest.TestCase):
    def test_two_workers_keep_identity_order_and_independent_timings(self):
        raw = encode(fixture())
        report = profile(raw)
        self.assertEqual(report["journal"]["sha256"], hashlib.sha256(raw).hexdigest())
        self.assertEqual(report["worker_policy"]["workers"], 2)
        self.assertEqual(report["subject_population"]["observed_total"], 19)
        self.assertEqual(report["integrity"]["completed_batches"], 3)
        self.assertEqual(report["batch_elapsed_micros"],
                         dict(total=150, median=50, p95=90, max=90, p95_method="nearest_rank"))
        self.assertEqual([row["batch_index"] for row in report["slowest_batches"]], [0, 2, 1])
        self.assertEqual(report["overall_effective_elapsed_micros"], 700)
        self.assertFalse(report["publication_authority"])

    def test_scoped_serial_span_is_labeled_and_findings_do_not_reject_profile(self):
        events = fixture(workers=1, enclosing=False)
        events[1]["findings"] = 27
        report = profile(encode(events))
        self.assertEqual(report["overall_effective_elapsed_source"], "first_batch_begin_to_last_batch_end")
        self.assertEqual(report["overall_effective_elapsed_micros"], 500)
        self.assertEqual(report["batch_findings_observed"], 27)

    def test_missing_end_and_truncated_or_missing_stage_are_rejected(self):
        events = fixture()
        with self.assertRaisesRegex(ValueError, "unmatched batch pairs"):
            profile(encode([event for event in events if not
                            (event["stage"] == BATCH and event["batch_index"] == 2 and event["event"] == "end")]))
        with self.assertRaisesRegex(ValueError, "truncated"):
            profile(encode(events)[:-1])
        with self.assertRaisesRegex(ValueError, "unclosed stage"):
            profile(encode(events[:-1]))

    def test_duplicate_and_mismatched_pairs_are_rejected(self):
        events = fixture()
        events.insert(2, copy.deepcopy(events[1]))
        with self.assertRaisesRegex(ValueError, "duplicate batch begin"):
            profile(encode(events))
        events = fixture()
        events[3]["last_subject"] = str(UUID(int=7))
        with self.assertRaisesRegex(ValueError, "mismatched batch pair"):
            profile(encode(events))

    def test_population_workers_and_order_cannot_drift(self):
        for key, value in [("total_subjects", 20), ("workers", 1)]:
            events = fixture()
            events[2][key] = value
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, "population drift"):
                profile(encode(events))
        events = fixture()
        events[3]["batch_index"], events[4]["batch_index"] = 1, 0
        with self.assertRaisesRegex(ValueError, "ordered bounded-worker"):
            profile(encode(events))

    def test_missing_batch_population_and_overlapping_ranges_are_rejected(self):
        events = [event for event in fixture() if event.get("batch_index") != 2]
        with self.assertRaisesRegex(ValueError, "missing or unexpected batch"):
            profile(encode(events))
        events = fixture()
        for event in events:
            if event.get("batch_index") == 1:
                event["first_subject"] = str(UUID(int=8))
        with self.assertRaisesRegex(ValueError, "ranges overlap"):
            profile(encode(events))

    def test_full_journal_other_stage_pairs_remain_consistent(self):
        events = fixture()
        before = [dict(events[0], stage="graph_restore", run_elapsed_micros=10),
                  dict(events[-1], stage="graph_restore", run_elapsed_micros=20, elapsed_micros=10)]
        after = [dict(events[0], stage="identity_provenance_final_audit", run_elapsed_micros=900),
                 dict(events[-1], stage="identity_provenance_final_audit", run_elapsed_micros=1000, elapsed_micros=100)]
        report = profile(encode(before + events + after))
        self.assertEqual(report["integrity"]["completed_stages"],
                         ["graph_restore", EFFECTIVE, "identity_provenance_final_audit"])
        with self.assertRaisesRegex(ValueError, "unclosed stage"):
            profile(encode(before + events + after[:-1]))

    def test_cli_writes_explicit_failure_and_never_changes_journal(self):
        script = Path(__file__).with_name("profile_systems_effective_audit.py")
        with tempfile.TemporaryDirectory() as directory:
            journal, output = Path(directory) / "events.jsonl", Path(directory) / "profile.json"
            for events, expected in [(fixture(), 0), (fixture()[:-1], 1)]:
                raw = encode(events)
                journal.write_bytes(raw)
                completed = subprocess.run([sys.executable, str(script), "--journal", str(journal),
                                            "--output", str(output)], capture_output=True, text=True, check=False)
                self.assertEqual(completed.returncode, expected, completed.stderr)
                self.assertEqual(journal.read_bytes(), raw)
                report = json.loads(output.read_bytes())
                self.assertEqual(report["journal_consistent"], expected == 0)
                self.assertFalse(report["publication_authority"])
                if expected:
                    self.assertNotIn("batch_elapsed_micros", report)
            completed = subprocess.run([sys.executable, str(script), "--journal", str(journal),
                                        "--output", str(journal)], capture_output=True, text=True, check=False)
            self.assertEqual(completed.returncode, 2)
            self.assertEqual(journal.read_bytes(), raw)


if __name__ == "__main__":
    unittest.main()
