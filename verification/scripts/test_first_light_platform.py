"""Gate orchestration tests; these do not establish semantic acceptance."""
import unittest
from unittest.mock import patch

import first_light_platform as gate


class InstalledRuntimeGateTests(unittest.TestCase):
    def location(self, origin="installed_bundle"):
        return {
            "origin": origin,
            "bundle_dir": str(gate.ROOT / "runtime-test/publications/accepted"),
            "kerml_cache": str(gate.ROOT / "runtime-test/publications/accepted/kerml.cache"),
            "systems_cache": str(gate.ROOT / "runtime-test/publications/accepted/systems.cache"),
        }

    def test_development_locations_never_start_authentication_or_gates(self):
        with patch.object(gate.sys, "argv", ["gate"]), \
                patch.object(gate, "recorded", return_value={}) as recorded, \
                patch.object(gate, "output_object", return_value=self.location("development_environment")):
            with self.assertRaisesRegex(RuntimeError, "installed bundle"):
                gate.main()
        self.assertEqual(recorded.call_count, 1)

    def test_rejected_installation_never_starts_a_semantic_gate(self):
        with patch.object(gate.sys, "argv", ["gate"]), \
                patch.object(gate, "recorded", side_effect=[{}, SystemExit(1)]) as recorded, \
                patch.object(gate, "output_object", return_value=self.location()):
            with self.assertRaises(SystemExit):
                gate.main()
        self.assertEqual([call.args[0] for call in recorded.call_args_list],
                         ["platform-installed-discovery", "platform-installed-authentication"])

    def test_scale_authenticates_then_runs_lifecycle_with_installed_paths(self):
        stale = {"AGENTIQUE_KERML_CACHE": "obsolete-kerml", "AGENTIQUE_SYSTEMS_CACHE": "obsolete-systems"}
        with patch.object(gate.sys, "argv", ["gate", "--gate", "scale"]), \
                patch.dict(gate.os.environ, stale), \
                patch.object(gate, "recorded", return_value={}) as recorded, \
                patch.object(gate, "output_object", return_value=self.location()):
            self.assertEqual(gate.main(), 0)
        self.assertEqual([call.args[0] for call in recorded.call_args_list],
                         ["platform-installed-discovery", "platform-installed-authentication",
                          "first-light-lifecycle", "first-light-scale"])
        for call in recorded.call_args_list:
            self.assertNotIn("AGENTIQUE_KERML_CACHE", call.args[2])
            self.assertNotIn("AGENTIQUE_SYSTEMS_CACHE", call.args[2])
        for call in recorded.call_args_list[2:]:
            self.assertEqual(call.args[3]["AGENTIQUE_KERML_CACHE"], self.location()["kerml_cache"])
            self.assertEqual(call.args[3]["AGENTIQUE_SYSTEMS_CACHE"], self.location()["systems_cache"])


if __name__ == "__main__":
    unittest.main()
