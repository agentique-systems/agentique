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
    return check(*parse_model({"Example.sysml": model}), crates)


class CheckTest(unittest.TestCase):
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
        self.assertEqual(problems(ui=["state", "tokio", "keyring-core"]), [
            "ui (Studio) depends on tokio, which only Providers may use",
            "ui (Studio) depends on keyring-core, which only Providers may use"])

    def test_temporary_library_use_is_allowed(self):
        model = MODEL.replace("part def Studio { part 'ui' : Crate; }",
                              "part def Studio { part 'ui' : Crate; }\n"
                              "    part def Assistant { part 'agq-assistant' : Crate; }")
        self.assertEqual(problems(model, **{"agq-assistant": ["reqwest"]}), [])
        self.assertEqual(problems(model, **{"agq-assistant": ["rig-core"]}), [
            "agq-assistant (Assistant) depends on rig-core, which only Providers may use"])

    def test_language_core_depends_on_nothing(self):
        model = MODEL.rstrip()[:-1] + "    dependency from LanguageCore to History;\n}\n"
        self.assertEqual(problems(model), ["LanguageCore must not depend on History"])

    def test_dev_dependencies_are_ignored(self):
        metadata = {"packages": [{"name": "state", "dependencies": [
            {"name": "core", "kind": None},
            {"name": "gen", "kind": "build"},
            {"name": "ui", "kind": "dev"}]}]}
        self.assertEqual(crates_from_metadata(metadata), {"state": ["core", "gen"]})

    def test_unexpected_syntax_is_rejected(self):
        for text in ("package P { part def A { dependency from A to A; } }",
                     "package P { part def A :> B; }",
                     "package P { part def A { port p : P; } }",
                     "package P { part def A { part 'a' : Crate; }"):
            with self.assertRaises(ModelError, msg=text):
                parse_model({"Bad.sysml": text})


if __name__ == "__main__":
    unittest.main()
