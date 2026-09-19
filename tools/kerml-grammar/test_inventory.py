"""Tests of contextual recognition, nullable completion and cyclic grammar."""
import unittest
from inventory import Grammar, GRAMMAR, recognize


class Recognition(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grammar = Grammar(GRAMMAR.read_text())

    def parse(self, words):
        tokens = []
        for i, word in enumerate(words):
            kind = "word" if word.isidentifier() else "symbol"
            if word.startswith("/*"):
                kind = "REGULAR_COMMENT"
            tokens.append([kind, word, i * 100, i * 100 + len(word)])
        return recognize(self.grammar, tokens)

    def test_import_requires_grammar_context_and_visibility(self):
        self.assertIsNotNone(self.parse(["package", "P", "{", "private", "import", "Q", "::", "*", ";", "}"])[0])
        self.assertIsNone(self.parse(["package", "P", "{", "import", "Q", ";", "}"])[0])
        self.assertIsNone(self.parse(["private", "import", ";"])[0])

    def test_doc_text_is_not_keyword_inventory(self):
        root, nodes, *_ = self.parse(["doc", "/* private import Missing::*; */"])
        self.assertIsNotNone(root)
        pending = [root]
        names = set()
        while pending:
            name, _, _, children = nodes[pending.pop()]
            names.add(name)
            pending.extend(children)
        self.assertIn("Documentation", names)
        self.assertNotIn("Import", names)

    def test_nullable_and_indirect_left_recursion(self):
        self.assertIsNotNone(self.parse([])[0])
        self.assertIsNotNone(self.parse(["function", "F", "{", "a", "+", "b", "*", "c", "}"])[0])
        self.assertIsNone(self.parse(["function", "F", "{", "a", "+", "}"])[0])
        self.assertIsNone(self.parse(["package", "P", "{"])[0])


if __name__ == "__main__":
    unittest.main()
