"""Scoped schema/authentication regressions; synthetic reports confer no authority."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import systems_semantic_slice_gate as gate

ROOT = Path(__file__).resolve().parents[2]


def witness_fixture(plan, expected):
    literals, definitions, counts = gate.corpus_expectations(plan, expected)
    definition_rows = []
    for document in sorted(set(definitions.values())):
        owners = sorted(subject for subject, path in definitions.items() if path == document)
        names = plan["enumeration_definitions"][Path(document).stem]
        definition_rows.extend(dict(subject=subject, document=document, name=name,
            is_variation=True, passed=True) for subject, name in zip(owners, names))
    return dict(schema="agq.systems-corpus-witnesses/v1", publication_authority=False,
        source_documents=list(expected), expected=counts,
        enumeration_definitions=definition_rows,
        enumeration_literals=[dict(subject=subject, document=row["document"],
            owning_definition=row["owning_definition"], variant_membership=row["variant_membership"],
            feature_typings=[gate.stable_id(["synthetic witness edge", subject])],
            owner_typing_targets=[row["owning_definition"]], typed_enumeration_definitions=[row["owning_definition"]],
            usage_types=[row["owning_definition"]], types_completeness="Complete", names_completeness="Complete",
            names=[row["name"]], data_value_subsetting_retained=True, passed=True)
            for subject, row in literals.items()],
        interface_ends=[dict(subject=subject, expected=list(gate.INTERFACE_ENDS), actual=list(gate.INTERFACE_ENDS),
            completeness="Complete", canonical_owners=[gate.BINARY_INTERFACE] * 2,
            canonical_port_usages=True, passed=True) for subject in gate.INTERFACE_SUBJECTS],
        flows=[dict(subject=subject, expected_parameters=list(gate.FLOW_PARAMETERS), parameters=list(gate.FLOW_PARAMETERS),
            parameters_completeness="Complete", expected_ends=list(ends), ends=list(ends), ends_completeness="Complete",
            canonical_end_owners=[subject] * 2, canonical_parameter_owners=[gate.MESSAGE] * 2, passed=True)
            for subject, ends in gate.FLOW_ENDS.items()],
        connections=[dict(subject=subject, happens_during=gate.HAPPENS_DURING, feature_typings=[typing],
            direct_typing_targets=[gate.HAPPENS_DURING],
            usage_types=[gate.HAPPENS_DURING, gate.stable_id(["synthetic connection definition"])],
            types_completeness="Complete", projections=[dict(operation=operation,
                values=[gate.stable_id(["synthetic connection definition"])], completeness="Complete",
                targets_conform=True, excludes_plain_association=True)
                for operation in ("occurrence", "item", "part", "connection")], passed=True)
            for subject, typing in gate.CONNECTION_TYPINGS.items()],
        unchanged_canonical_element_count=True, findings=[], passed=True)


def fixture(scope):
    plan, expected, _, _ = gate.reviewed_scope(ROOT, scope)
    count, exact_references = gate.SCOPES[scope]
    references = exact_references or 1001  # Synthetic schema evidence only.
    documents = []
    for index, (path, source) in enumerate(expected.items()):
        total = references - count + 1 if index == 0 else 1
        documents.append(dict(path=path, document=source["document"], sha256=source["sha256"],
            profile=gate.PROFILE, parsed=True, byte_exact=True, recovery_count=0,
            production_count=1, construction_gap=None, reference_audit_scope="construction",
            mandatory_references=dict(total=total, counts=dict(complete=total))))
    accepted = json.loads((ROOT / "standards/kerml-accepted-publication.json").read_bytes())
    certificate = dict(fully_closed=True, applicable_pairs=42, closed_pairs=42,
        incomplete_pairs=0, required_requirements=100, closed_requirements=100)
    certificate.update({field: [index] * 32 for index, field in enumerate(gate.CERTIFICATE_DIGESTS)})
    report = dict(format="agq-sysml-systems-publication-audit/1", scope=list(expected),
        sysml_profile=gate.PROFILE, scoped_preflight_passed=True, construction_complete=True,
        publication_attempted=False, publication_accepted=False,
        systems_documents_parsed=count, systems_documents_constructed=count,
        systems_documents_byte_exact=count, kernel_obligations=0, kernel_obligation_details=[],
        authority_conflicts=[], accepted_kerml_profile="agentique-kerml-1.0-operational/9",
        kerml_producers_replayed=False, accepted_kerml_digest=accepted["complete_overlay"]["identity"]["semantic_digest"],
        construction_reference_semantic_digest=[8] * 32, reference_audit_scope="construction",
        mandatory_references=dict(total=references, failures=[],
            counts={state: references if state == "complete" else 0 for state in gate.REFERENCE_STATES}),
        documents=documents, producer_closure=certificate,
        construction_producers=dict(final_predicates=True, converged=True, completeness="Complete",
            diagnostics=[], round_limit_reached=False, rounds=2, round_limit=160),
        effective_sysml_audit=dict(publication_authority=False, findings=[], workers=2,
            elapsed_seconds=1.5, checked={family: 1 for family in gate.AUDIT_FAMILIES}),
        elapsed_seconds=2.5, checkpoint_session=None, corpus_witnesses=witness_fixture(plan, expected))
    return report, plan, expected


def replace(report, path, value):
    target = report
    for field in path[:-1]:
        target = target[field]
    target[path[-1]] = value


class SemanticSliceGateTests(unittest.TestCase):
    def test_both_exact_scopes_with_pinned_sources_and_no_publication_authority(self):
        for scope in gate.SCOPES:
            with self.subTest(scope=scope), tempfile.TemporaryDirectory() as directory:
                report, _, _ = fixture(scope)
                path = Path(directory) / "report.json"
                path.write_bytes(gate.encoded(report))
                result = gate.validate(path, ROOT, scope)
                self.assertTrue(result["passed"])
                self.assertFalse(result["publication_authority"])
                self.assertEqual(result["evidence"][0]["sha256"], gate.sha(path.read_bytes()))
                self.assertEqual(result["effective_findings"], 0)
                self.assertEqual(result["corpus_witness_counts"], report["corpus_witnesses"]["expected"])
                self.assertEqual(result["corpus_witness_digest"], gate.sha(gate.encoded(report["corpus_witnesses"])))
                self.assertEqual(result["prior_classification"]["distinct_diagnostics"], 238)
                if scope == "H1-H3-combined":
                    self.assertEqual(result["prior_subject_coverage"]["local"], 47)
                    self.assertEqual(result["prior_subject_coverage"]["total"], 50)
                else:
                    self.assertEqual(result["mandatory_references_complete"], 695)
                self.assertEqual(result["prior_subject_coverage"]["accepted_dependency_targets"], 3)

    def test_completed_scheduler_cannot_hide_missing_or_failed_effective_audit(self):
        base, plan, expected = fixture("H4-medium")
        for field in ("effective_sysml_audit", "producer_closure", "construction_producers", "scope", "corpus_witnesses"):
            report = copy.deepcopy(base)
            del report[field]
            with self.subTest(missing=field), self.assertRaises((KeyError, ValueError)):
                gate.accept_report(report, "H4-medium", expected, plan)
        for path, value in (
            (("effective_sysml_audit", "findings"), ["KQ_END_CYCLE"]),
            (("effective_sysml_audit", "checked"), {}),
            (("effective_sysml_audit", "checked", "DefinitionUsage"), 0),
            (("effective_sysml_audit", "checked", "DefinitionUsage"), True),
            (("effective_sysml_audit", "publication_authority"), True),
            (("effective_sysml_audit", "workers"), 3),
            (("effective_sysml_audit", "workers"), True),
            (("effective_sysml_audit", "elapsed_seconds"), float("nan")),
            (("effective_sysml_audit", "elapsed_seconds"), -1),
            (("elapsed_seconds",), float("inf")),
        ):
            report = copy.deepcopy(base)
            replace(report, path, value)
            with self.subTest(path=path, value=value), self.assertRaises(ValueError):
                gate.accept_report(report, "H4-medium", expected, plan)

    def test_open_pairs_requirements_and_producer_diagnostics_are_not_success(self):
        base, plan, expected = fixture("H4-medium")
        for path, value in (
            (("producer_closure", "fully_closed"), False),
            (("producer_closure", "closed_pairs"), 41),
            (("producer_closure", "incomplete_pairs"), 1),
            (("producer_closure", "closed_requirements"), 99),
            (("producer_closure", "closed_requirements"), True),
            (("construction_producers", "converged"), False),
            (("construction_producers", "final_predicates"), False),
            (("construction_producers", "completeness"), "Incomplete"),
            (("construction_producers", "diagnostics"), ["producer error"]),
            (("construction_producers", "round_limit_reached"), True),
            (("construction_producers", "rounds"), 161),
        ):
            report = copy.deepcopy(base)
            replace(report, path, value)
            with self.subTest(path=path), self.assertRaises(ValueError):
                gate.accept_report(report, "H4-medium", expected, plan)
        for field in gate.CERTIFICATE_DIGESTS:
            report = copy.deepcopy(base)
            report["producer_closure"][field] = [True] * 32
            with self.subTest(digest=field), self.assertRaisesRegex(ValueError, "digest"):
                gate.accept_report(report, "H4-medium", expected, plan)

    def test_same_document_counts_cannot_substitute_another_scope_or_source(self):
        base, plan, expected = fixture("H4-medium")
        for path, value in (
            (("scope", 0), "Systems Library/Metadata.sysml"),
            (("scope", 0), base["scope"][1]),
            (("documents", 0, "path"), "Systems Library/Metadata.sysml"),
            (("documents", 0, "sha256"), "ff" * 32),
            (("documents", 0, "document"), "00000000-0000-0000-0000-000000000000"),
            (("documents", 0, "profile"), "agentique-sysml-2.0-operational/2"),
            (("documents", 0, "parsed"), False),
            (("documents", 0, "byte_exact"), False),
            (("documents", 0, "construction_gap"), "unsupported syntax"),
            (("documents", 0, "recovery_count"), 1),
            (("systems_documents_parsed",), 12),
            (("kernel_obligations",), 1),
            (("kernel_obligation_details",), ["pending"]),
        ):
            report = copy.deepcopy(base)
            replace(report, path, value)
            with self.subTest(path=path, value=value), self.assertRaises(ValueError):
                gate.accept_report(report, "H4-medium", expected, plan)

    def test_reference_failures_miscount_and_wrong_authority_fail(self):
        base, plan, expected = fixture("H4-medium")
        for path, value in (
            (("mandatory_references", "counts", "incomplete"), 1),
            (("mandatory_references", "counts", "complete"), 694),
            (("mandatory_references", "failures"), ["missing endpoint"]),
            (("mandatory_references", "total"), 694),
            (("documents", 0, "mandatory_references", "total"), 0),
            (("documents", 0, "mandatory_references", "counts", "invalid"), 1),
            (("reference_audit_scope",), "accepted_publication"),
            (("authority_conflicts",), ["unresolved interpretation"]),
            (("sysml_profile",), "agentique-sysml-2.0-operational/2"),
            (("accepted_kerml_profile",), "agentique-kerml-1.0-operational/8"),
            (("kerml_producers_replayed",), True),
            (("publication_attempted",), True),
            (("publication_accepted",), True),
            (("checkpoint_session",), dict(accepted_authority=True)),
        ):
            report = copy.deepcopy(base)
            replace(report, path, value)
            with self.subTest(path=path), self.assertRaises(ValueError):
                gate.accept_report(report, "H4-medium", expected, plan)
        report = copy.deepcopy(base)
        report["mandatory_references"]["total"] = 694
        report["mandatory_references"]["counts"]["complete"] = 694
        report["documents"][0]["mandatory_references"]["total"] -= 1
        report["documents"][0]["mandatory_references"]["counts"]["complete"] -= 1
        with self.assertRaisesRegex(ValueError, "exact reference count"):
            gate.accept_report(report, "H4-medium", expected, plan)
        report = copy.deepcopy(base)
        report["documents"][0]["mandatory_references"]["total"] += 1
        report["documents"][0]["mandatory_references"]["counts"]["complete"] += 1
        with self.assertRaisesRegex(ValueError, "document/global reference population mismatch"):
            gate.accept_report(report, "H4-medium", expected, plan)

    def test_independent_accepted_dependency_identity_cannot_be_relabelled(self):
        report, plan, expected = fixture("H4-medium")
        report["accepted_kerml_digest"] = [9] * 32
        with self.assertRaisesRegex(ValueError, "accepted KerML identity"):
            gate.source_pins(ROOT, report, plan, expected)

    def test_missing_failed_or_self_reduced_witness_populations_fail(self):
        base, plan, expected = fixture("H1-H3-combined")
        for field in ("enumeration_definitions", "enumeration_literals", "interface_ends", "flows", "connections"):
            report = copy.deepcopy(base)
            del report["corpus_witnesses"][field]
            with self.subTest(missing=field), self.assertRaises(KeyError):
                gate.accept_report(report, "H1-H3-combined", expected, plan)
            report = copy.deepcopy(base)
            report["corpus_witnesses"][field].pop()
            with self.subTest(short=field), self.assertRaisesRegex(ValueError, "exact witnessed subjects"):
                gate.accept_report(report, "H1-H3-combined", expected, plan)
        for path, value in (
            (("passed",), False),
            (("publication_authority",), True),
            (("findings",), ["failed canonical result"]),
            (("unchanged_canonical_element_count",), False),
            (("source_documents",), []),
            (("expected", "enumeration_literals"), 20),
            (("expected", "interface_subjects"), True),
            (("flows", 0, "passed"), False),
        ):
            report = copy.deepcopy(base)
            replace(report["corpus_witnesses"], path, value)
            with self.subTest(path=path), self.assertRaises(ValueError):
                gate.accept_report(report, "H1-H3-combined", expected, plan)
        report = copy.deepcopy(base)
        report["corpus_witnesses"]["enumeration_literals"].pop()
        report["corpus_witnesses"]["expected"]["enumeration_literals"] -= 1
        with self.assertRaisesRegex(ValueError, "exact expected corpus witness counts"):
            gate.accept_report(report, "H1-H3-combined", expected, plan)
        medium, medium_plan, medium_expected = fixture("H4-medium")
        medium["corpus_witnesses"]["enumeration_literals"] = base["corpus_witnesses"]["enumeration_literals"]
        with self.assertRaisesRegex(ValueError, "exact witnessed subjects"):
            gate.accept_report(medium, "H4-medium", medium_expected, medium_plan)

    def test_literal_witnesses_require_canonical_owner_typing_and_complete_names(self):
        base, plan, expected = fixture("H1-H3-combined")
        first = base["corpus_witnesses"]["enumeration_literals"][0]
        wrong = "00000000-0000-0000-0000-000000000000"
        for path, value in (
            (("enumeration_definitions", 0, "is_variation"), False),
            (("enumeration_definitions", 0, "name"), "UnknownEnumeration"),
            (("enumeration_literals", 0, "subject"), wrong),
            (("enumeration_literals", 0, "variant_membership"), wrong),
            (("enumeration_literals", 0, "owning_definition"), wrong),
            (("enumeration_literals", 0, "feature_typings"), []),
            (("enumeration_literals", 0, "feature_typings"), [wrong, wrong]),
            (("enumeration_literals", 0, "owner_typing_targets"), [wrong]),
            (("enumeration_literals", 0, "typed_enumeration_definitions"), [wrong]),
            (("enumeration_literals", 0, "usage_types"), []),
            (("enumeration_literals", 0, "types_completeness"), "Incomplete"),
            (("enumeration_literals", 0, "names_completeness"), "Incomplete"),
            (("enumeration_literals", 0, "names"), ["inherited-name"]),
            (("enumeration_literals", 0, "data_value_subsetting_retained"), False),
            (("enumeration_literals", 1, "feature_typings"), first["feature_typings"]),
        ):
            report = copy.deepcopy(base)
            replace(report["corpus_witnesses"], path, value)
            with self.subTest(path=path), self.assertRaises(ValueError):
                gate.accept_report(report, "H1-H3-combined", expected, plan)

    def test_structure_witnesses_reject_copies_wrong_order_and_incomplete_queries(self):
        base, plan, expected = fixture("H4-medium")
        wrong = "00000000-0000-0000-0000-000000000000"
        for path, value in (
            (("interface_ends", 0, "actual"), [wrong, gate.INTERFACE_ENDS[1]]),
            (("interface_ends", 0, "actual"), list(reversed(gate.INTERFACE_ENDS))),
            (("interface_ends", 0, "expected"), [wrong, gate.INTERFACE_ENDS[1]]),
            (("interface_ends", 0, "completeness"), "Incomplete"),
            (("interface_ends", 0, "canonical_port_usages"), False),
            (("interface_ends", 0, "canonical_owners"), [wrong] * 2),
            (("flows", 0, "parameters"), list(reversed(gate.FLOW_PARAMETERS))),
            (("flows", 0, "parameters_completeness"), "Incomplete"),
            (("flows", 0, "canonical_parameter_owners"), [wrong] * 2),
            (("flows", 0, "ends"), []),
            (("flows", 0, "ends_completeness"), "Incomplete"),
            (("flows", 0, "canonical_end_owners"), [gate.MESSAGE] * 2),
        ):
            report = copy.deepcopy(base)
            replace(report["corpus_witnesses"], path, value)
            with self.subTest(path=path), self.assertRaises(ValueError):
                gate.accept_report(report, "H4-medium", expected, plan)
        report = copy.deepcopy(base)
        changed = [wrong, gate.INTERFACE_ENDS[1]]
        report["corpus_witnesses"]["interface_ends"][0].update(expected=changed, actual=changed)
        with self.assertRaisesRegex(ValueError, "original BinaryInterface"):
            gate.accept_report(report, "H4-medium", expected, plan)

    def test_connection_witnesses_retain_original_carriers_and_narrow_typed_subsets(self):
        base, plan, expected = fixture("H4-medium")
        wrong = "00000000-0000-0000-0000-000000000000"
        for path, value in (
            (("feature_typings",), [wrong]),
            (("direct_typing_targets",), [wrong]),
            (("usage_types",), [wrong]),
            (("types_completeness",), "Invalid"),
            (("projections", 0, "completeness"), "Invalid"),
            (("projections", 0, "targets_conform"), False),
            (("projections", 0, "excludes_plain_association"), False),
            (("projections", 0, "values"), [gate.HAPPENS_DURING]),
            (("projections", 0, "values"), []),
            (("projections", 0, "operation"), "item"),
            (("projections", 2, "values"), [wrong]),
        ):
            report = copy.deepcopy(base)
            replace(report["corpus_witnesses"]["connections"][0], path, value)
            with self.subTest(path=path), self.assertRaises(ValueError):
                gate.accept_report(report, "H4-medium", expected, plan)

    def test_reviewed_classification_and_mapping_cannot_be_rewritten(self):
        for name in ("prior-classification.json", "slices.json"):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                target = root / gate.FIXTURES
                target.mkdir(parents=True)
                for fixture_name in ("prior-classification.json", "slices.json"):
                    (target / fixture_name).write_bytes((ROOT / gate.FIXTURES / fixture_name).read_bytes())
                original = json.loads((target / name).read_bytes())
                # Whitespace and checkout line endings do not alter reviewed values.
                (target / name).write_text(json.dumps(original, indent=1), encoding="utf-8")
                gate.reviewed_scope(root, "H1-H3-combined")
                if name == "prior-classification.json":
                    original["summary"]["categories"]["Other"]["distinct_diagnostics"] = 1
                else:
                    original["all_prior_subjects"].pop()
                (target / name).write_bytes(gate.encoded(original))
                with self.assertRaisesRegex(ValueError, "changed reviewed fixture"):
                    gate.reviewed_scope(root, "H1-H3-combined")


if __name__ == "__main__":
    unittest.main()
