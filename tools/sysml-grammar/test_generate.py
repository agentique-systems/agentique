"""Focused contracts for strict extension generation; no source acquisition."""
from pathlib import Path
import importlib.util
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('sysml_generator', Path(__file__).with_name('generate.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class StrictGrammar(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grammar, cls.kinds = module.grammar_source()

    def test_undefined_published_nonterminals_cannot_match(self):
        for name in ['CalculationUsageDeclaration', 'DefinitionExtensionKeyWord', 'FilterPackageImport', 'SendReceiverPart']:
            self.assertEqual(self.grammar.by_name[name], [], name)
        self.assertFalse(any('__UNRESOLVED_' in s for _, rhs in self.grammar.rules for s in rhs))

    def test_corpus_compatibility_alternatives_remain_absent(self):
        alternatives = {name: [self.grammar.rules[i][1] for i in ids] for name, ids in self.grammar.by_name.items()}
        self.assertNotIn(('AllocationDefinition',), alternatives['DefinitionElement'])
        self.assertNotIn(('ReturnParameterMember',), alternatives['CaseBodyItem'])
        self.assertEqual(alternatives['DefaultReferenceUsage'], [('RefPrefix', 'Usage')])
        self.assertEqual(alternatives['OccurrenceUsagePrefix'][0][0], 'BasicUsagePrefix')
        self.assertEqual(alternatives['SatisfyRequirementUsage'][0][1], "'assert'")

    def test_empty_port_actions_and_shared_production_kinds_survive(self):
        self.assertEqual([self.grammar.rules[i][1] for i in self.grammar.by_name['PortConjugation']], [()])
        for name in ['OwnedFeatureTyping', 'OwnedSubclassification', 'PartDefinition', 'DefinitionMember', 'OccurrenceUsageMember', 'ConjugatedPortDefinitionMember']:
            self.assertEqual(self.kinds[name], name)
        self.assertEqual(len(set(self.kinds.values())), 535)


if __name__ == '__main__':
    unittest.main()
