"""Tests for check_architecture.py.

Run from the repository root:
    python3 -m unittest discover -s tools -p test_check_architecture.py
"""

import unittest

from check_architecture import ModelError, check, parse_model

MODEL = """
package Example {
    doc /* A small architecture with a transitive dependency. */
    part def Crate;
    part def LanguageCore { part 'core' : Crate; }
    part def SystemState { part 'state' : Crate; }
    part def Studio {
        part surface { part 'ui' : Crate; }
    }
    dependency from Studio to SystemState;
    dependency from SystemState to LanguageCore { doc /* Allowed. */ }
}
"""


def problems(**changes):
    crates = {"core": [], "state": ["core", "serde"], "ui": ["core", "eframe", "state"]}
    crates.update(changes)
    return check(*parse_model({"Example.sysml": MODEL}), crates)


class CheckTest(unittest.TestCase):
    def test_allowed_graph_passes(self):
        # ui reaches core only through SystemState; eframe is fine outside the core parts.
        self.assertEqual(problems(), [])

    def test_forbidden_dependency_fails(self):
        self.assertEqual(problems(core=["state"]), [
            "core (LanguageCore) depends on state (SystemState), "
            "but the model has no dependency path from LanguageCore to SystemState"])

    def test_unmapped_crate_fails(self):
        self.assertEqual(problems(extra=[]), ["extra is not mapped to any part"])

    def test_missing_crate_fails(self):
        crates = {"core": [], "state": []}
        self.assertEqual(check(*parse_model({"Example.sysml": MODEL}), crates),
                         ["ui is mapped to Studio but is not a workspace crate"])

    def test_denied_library_in_language_core_fails(self):
        self.assertEqual(problems(core=["tokio"]),
                         ["core (LanguageCore) depends on tokio, a UI, network or AI library"])

    def test_unexpected_syntax_is_rejected(self):
        for text in ("package P { part def A { dependency from A to A; } }",
                     "package P { part def A :> B; }",
                     "package P { part def A { part 'a' : Crate; }"):
            with self.assertRaises(ModelError, msg=text):
                parse_model({"Bad.sysml": text})


if __name__ == "__main__":
    unittest.main()
