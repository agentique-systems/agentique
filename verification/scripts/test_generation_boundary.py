"""Adversarial generation-boundary checks; no Rust build or network needed."""
import unittest

from generation_boundary import REQUIRED, audit


def metadata(extra=None):
    packages = {name: {"id": name, "name": name, "dependencies": []}
                for name in REQUIRED}
    for name, dependencies in (extra or {}).items():
        packages[name] = {"id": name, "name": name, "dependencies": dependencies}
    return {"workspace_members": sorted(packages), "packages": list(packages.values())}


def dependency(name, **options):
    return {"name": name, "kind": None, "optional": False, "target": None, **options}


class GenerationBoundaryTests(unittest.TestCase):
    def test_repository_cannot_leak_into_language_or_workspace(self):
        for origin in ("agq-kernel", "agq-kerml-text", "agq-modeling-workspace"):
            report = audit(metadata({origin: [dependency("agq-modeling-repository")]}))
            self.assertTrue(any(v["rule"] == "language-workspace-must-not-depend-on-platform-adapters"
                                for v in report["violations"]))

    def test_platform_generation_remains_separate_from_gen1(self):
        report = audit(metadata({"agq-modeling-service": [dependency("agq-application")]}))
        self.assertIn({"rule": "gen2-must-not-depend-on-gen1",
                       "path": ["agq-modeling-service", "agq-application"]}, report["violations"])

    def test_inward_language_and_platform_dependencies_are_permitted(self):
        report = audit(metadata({
            "agq-kerml": [dependency("agq-kernel")],
            "agq-sysml": [dependency("agq-kerml")],
            "agq-modeling-workspace": [dependency("agq-sysml"), dependency("serde")],
            "agq-application": [dependency("agq-modeling-workspace")],
        }))
        self.assertEqual(report["violations"], [])

    def test_renamed_target_optional_dependency_cannot_hide_gen1(self):
        report = audit(metadata({"agq-sysml-semantics": [dependency(
            "agq-model", rename="compat", optional=True, target='cfg(windows)')]}))
        self.assertIn({"rule": "gen2-must-not-depend-on-gen1",
                       "path": ["agq-sysml-semantics", "agq-model"]},
                      report["violations"])

    def test_build_and_development_paths_are_checked(self):
        for kind in ("build", "dev"):
            with self.subTest(kind=kind):
                report = audit(metadata({"agq-kerml-text": [dependency(
                    "agq-workspace", kind=kind)]}))
                self.assertEqual(report["violations"][0]["path"],
                                 ["agq-kerml-text", "agq-workspace"])

    def test_transitive_workspace_bridge_is_reported(self):
        report = audit(metadata({
            "agq-kerml-text": [dependency("innocent-helper")],
            "innocent-helper": [dependency("agq-application")],
        }))
        self.assertIn({"rule": "gen2-must-not-depend-on-gen1",
                       "path": ["agq-kerml-text", "innocent-helper", "agq-application"]},
                      report["violations"])

    def test_kernel_cannot_acquire_transitive_language_dependency(self):
        report = audit(metadata({
            "agq-kernel": [dependency("shared-helper")],
            "shared-helper": [dependency("agq-kerml")],
        }))
        self.assertIn({"rule": "kernel-must-remain-language-agnostic",
                       "path": ["agq-kernel", "shared-helper", "agq-kerml"]},
                      report["violations"])

    def test_development_cycles_terminate_and_do_not_hide_violation(self):
        report = audit(metadata({
            "agq-kerml-syntax": [dependency("agq-standard-libraries", kind="dev")],
            "agq-standard-libraries": [dependency("agq-kerml-syntax"),
                                       dependency("agq-storage")],
        }))
        self.assertIn({"rule": "gen2-must-not-depend-on-gen1",
                       "path": ["agq-kerml-syntax", "agq-standard-libraries", "agq-storage"]},
                      report["violations"])

    def test_future_language_crates_are_automatically_protected(self):
        report = audit(metadata({"agq-sysml-new": [dependency("agentique")]}))
        self.assertIn({"rule": "gen2-must-not-depend-on-gen1",
                       "path": ["agq-sysml-new", "agentique"]},
                      report["violations"])

    def test_missing_core_packages_cannot_pass_an_empty_audit(self):
        with self.assertRaisesRegex(ValueError, "missing required workspace packages"):
            audit({"workspace_members": [], "packages": []})


if __name__ == "__main__":
    unittest.main()
