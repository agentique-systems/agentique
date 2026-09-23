"""Guard regressions: equal counts cannot mask changed semantic evidence."""
import copy
import unittest

from medium_frontier_gate import CERTIFICATE_DIGESTS, compare


def fixture():
    certificate = dict(applicable_pairs=14791, closed_pairs=14791, incomplete_pairs=0,
                       closed_requirements=417786, required_requirements=417786,
                       fully_closed=True, revalidation_digest=[7] * 32)
    certificate.update({key: [index] * 32 for index, key in enumerate(CERTIFICATE_DIGESTS)})
    report = dict(scoped_preflight_passed=True, construction_complete=True,
                  systems_documents_parsed=13, systems_documents_constructed=13,
                  systems_documents_byte_exact=13, kernel_obligations=0,
                  kernel_obligation_details=[], authority_conflicts=[],
                  publication_accepted=False, publication_attempted=False,
                  kerml_producers_replayed=False, accepted_kerml_digest=[8] * 32,
                  producer_closure=certificate, construction_reference_semantic_digest=[9] * 32,
                  incremental_certificate_reference_check=True,
                  scope=["synthetic guard fixture"], sysml_profile="fixture",
                  accepted_kerml_profile="fixture", documents=[], authority_targets=[],
                  local_elements=1, reference_audit_scope="construction", closure_explanations=[],
                  construction_producers=dict(converged=True, completeness="Complete",
                                              final_predicates=True, round_limit_reached=False,
                                              diagnostics=[]),
                  mandatory_references=dict(total=695, failures=[], counts=dict(
                      complete=695, unresolved=0, incomplete=0, ambiguous=0, invalid=0,
                      endpoint_mismatch=0)),
                  checkpoint_session=dict(accepted_authority=False, statistics=dict(
                      restored_invocations=0, restored_completed_invocations=0,
                      skipped_rounds=0, committed_checkpoints=2)))
    return report


class GateTests(unittest.TestCase):
    def setUp(self):
        self.baseline = fixture()
        self.uninterrupted = copy.deepcopy(self.baseline)
        self.resumed = copy.deepcopy(self.baseline)
        self.resumed["checkpoint_session"]["statistics"].update(
            restored_invocations=2, restored_completed_invocations=1, skipped_rounds=15)

    def check(self):
        return compare(self.baseline, self.uninterrupted, self.resumed)

    def test_identical_semantics_with_actual_resume_pass(self):
        self.assertTrue(self.check()["passed"])

    def test_changed_graph_proof_certificate_transport_and_queries_fail(self):
        for key in (*CERTIFICATE_DIGESTS, "revalidation_digest"):
            with self.subTest(key=key):
                original = self.resumed["producer_closure"][key]
                self.resumed["producer_closure"][key] = [255] * 32
                with self.assertRaises(ValueError):
                    self.check()
                self.resumed["producer_closure"][key] = original
        self.resumed["construction_reference_semantic_digest"] = [255] * 32
        with self.assertRaisesRegex(ValueError, "query results"):
            self.check()

    def test_completed_only_restore_is_insufficient(self):
        self.resumed["checkpoint_session"]["statistics"]["restored_completed_invocations"] = 2
        with self.assertRaisesRegex(ValueError, "unfinished"):
            self.check()

    def test_missing_oracle_and_incomplete_reference_fail(self):
        self.uninterrupted["incremental_certificate_reference_check"] = False
        with self.assertRaisesRegex(ValueError, "reference comparison"):
            self.check()
        self.uninterrupted["incremental_certificate_reference_check"] = True
        self.resumed["mandatory_references"]["counts"].update(complete=694, incomplete=1)
        with self.assertRaisesRegex(ValueError, "reference acceptance"):
            self.check()


if __name__ == "__main__":
    unittest.main()
