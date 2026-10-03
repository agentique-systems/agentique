"""Tests for check_architecture.py.

Run from the repository root:
    python3 -m unittest discover -s tools -p test_check_architecture.py
"""

import unittest

from check_architecture import ModelError, check, crates_from_metadata, parse_model

MODEL = """
package Example {
    doc /* A small architecture. */
    part def Crate;
    part def LanguageCore { part 'core' : Crate; }
    part def SystemState { part 'state' : Crate; }
    part def History { part 'store' : Crate { doc /* A crate with a note. */ } }
    part def Studio { part 'ui' : Crate; }
    part def Providers { part 'net' : Crate; }
    part def Product { part studio : Studio; }
    dependency from Studio to SystemState;
    dependency from Studio to Providers;
    dependency from SystemState to LanguageCore { doc /* Allowed. */ }
    dependency from SystemState to History;
}
"""


def problems(model=MODEL, **changes):
    crates = {"core": [], "state": ["core", "serde"], "store": [], "ui": ["eframe", "state", "net"],
              "net": ["rig-core", "tokio", "reqwest", "keyring"]}
    crates.update(changes)
    found = check(*parse_model({"Example.sysml": model}), crates)
    # The real temporary exception is not part of these examples.
    return [problem for problem in found if "TEMPORARY_LIBRARY_USES" not in problem]


class CheckTest(unittest.TestCase):
    def test_the_library_uses_no_ui_or_network_library(self):
        model = MODEL.rstrip()[:-1] + (
            "    part def Library { part 'blocks' : Crate; }\n"
            "    dependency from Library to SystemState;\n}\n")
        self.assertEqual(problems(model, blocks=["state"]), [])
        self.assertEqual(problems(model, blocks=["state", "gpui"]),
                         ["blocks (Library) depends on gpui, a UI or network library"])

    def test_allowed_graph_passes(self):
        # eframe is fine outside the core parts.
        self.assertEqual(problems(), [])

    def test_forbidden_dependency_fails(self):
        self.assertEqual(problems(core=["state"]), [
            "core (LanguageCore) depends on state (SystemState), "
            "but the model has no dependency from LanguageCore to SystemState"])

    def test_dependencies_are_not_transitive(self):
        self.assertEqual(problems(ui=["state", "core"]), [
            "ui (Studio) depends on core (LanguageCore), "
            "but the model has no dependency from Studio to LanguageCore"])

    def test_unmapped_crate_fails(self):
        self.assertEqual(problems(extra=[]), ["extra is not mapped to any part"])

    def test_missing_crate_fails(self):
        crates = {"core": [], "state": [], "store": [], "net": []}
        self.assertEqual(check(*parse_model({"Example.sysml": MODEL}), crates),
                         ["ui is mapped to Studio but is not a workspace crate"])

    def test_denied_library_in_a_core_part_fails(self):
        self.assertEqual(problems(core=["tokio"], store=["reqwest"]), [
            "core (LanguageCore) depends on tokio, a UI or network library",
            "store (History) depends on reqwest, a UI or network library"])

    def test_provider_libraries_outside_providers_fail(self):
        # rig, tokio, reqwest and the credential store belong to Providers (R-41).
        self.assertEqual(problems(ui=["state", "tokio", "keyring-core", "rig-typesafeai"]), [
            "ui (Studio) depends on tokio, which only Providers may use",
            "ui (Studio) depends on keyring-core, which only Providers may use",
            "ui (Studio) depends on rig-typesafeai, which only Providers may use"])

    def test_temporary_library_use_is_allowed(self):
        model = MODEL.replace("part def Studio { part 'ui' : Crate; }",
                              "part def Studio { part 'ui' : Crate; }\n"
                              "    part def Assistant { part 'agq-assistant' : Crate; }")
        self.assertEqual(problems(model, **{"agq-assistant": ["reqwest"]}), [])
        self.assertEqual(problems(model, **{"agq-assistant": ["rig-core"]}), [
            "agq-assistant (Assistant) depends on rig-core, which only Providers may use"])

    def test_an_unused_temporary_use_is_reported(self):
        # Once the Assistant no longer uses reqwest, the exception must go too.
        model = MODEL.replace("part def Studio { part 'ui' : Crate; }",
                              "part def Studio { part 'ui' : Crate; }\n"
                              "    part def Assistant { part 'agq-assistant' : Crate; }")
        crates = {"core": [], "state": ["core"], "store": [], "ui": ["state"], "net": [],
                  "agq-assistant": []}
        self.assertEqual(check(*parse_model({"Example.sysml": model}), crates), [
            "the temporary use of reqwest by agq-assistant (until W5.7) is gone: "
            "remove it from TEMPORARY_LIBRARY_USES"])

    def test_language_core_depends_on_nothing(self):
        model = MODEL.rstrip()[:-1] + "    dependency from LanguageCore to History;\n}\n"
        self.assertEqual(problems(model), ["LanguageCore must not depend on History"])

    def test_dev_dependencies_are_ignored(self):
        metadata = {"packages": [{"name": "state", "dependencies": [
            {"name": "core", "kind": None},
            {"name": "gen", "kind": "build"},
            {"name": "ui", "kind": "dev"}]}]}
        self.assertEqual(crates_from_metadata(metadata), {"state": ["core", "gen"]})

    def test_a_malformed_structure_is_rejected(self):
        for text in ("package P { dependency from A; }",
                     "package P { dependency A to B; }",
                     "package P { part def A { part 'a' : Crate; }"):
            with self.assertRaises(ModelError, msg=text):
                parse_model({"Bad.sysml": text})

    def test_constructs_the_check_does_not_need_are_read_past(self):
        # R-41: ports, items, behaviour, requirements and scenarios allow nothing.
        model = MODEL.rstrip()[:-1] + """
    item def Change { attribute madeBy : Actor; }
    port def ChangePort { in item change : Change; }
    part def Assistant {
        part 'assistant' : Crate;
        port turn : ~ChangePort;
        attribute revision : Natural = 0 { doc /* A counter. */ }
        exhibit state s { entry; then ready; state ready;
            transition t first ready accept c : Change via turn if c.madeBy == Actor::operator do send new Change(madeBy = c.madeBy) via turn then ready; }
    }
    requirement def R { subject s : Studio; }
    verification def V { subject s : Studio; send new Change() via s.p; then assert constraint c { 1 == 1 } }
}
"""
        parts, dependencies = parse_model({"Example.sysml": model})
        self.assertEqual(parts["Assistant"], ["assistant"])
        self.assertEqual(len(dependencies), 4)


if __name__ == "__main__":
    unittest.main()
