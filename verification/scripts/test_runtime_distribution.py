"""Fail-closed command sequencing of the distribution qualification wrapper."""
import argparse
import contextlib
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import zipfile

MODULE = Path(__file__).resolve().parents[2] / "tools/verify-runtime-distribution.py"
SPEC = importlib.util.spec_from_file_location("distribution", MODULE)
distribution = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(distribution)


class DistributionQualification(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        root = Path(self.temporary.name)
        installer = root / "publications"
        installer.write_bytes(b"test executable identity only; commands are mocked")
        bundle = root / "runtime.agq-runtime"
        payload = b"fixture, not accepted semantic content"
        self.manifest = {
            "identity": "test-bundle",
            "kerml": {"file": "kerml.cache", "bytes": len(payload), "sha256": hashlib.sha256(payload).hexdigest()},
            "systems": {"file": "systems.cache", "bytes": len(payload), "sha256": hashlib.sha256(payload).hexdigest()},
        }
        with zipfile.ZipFile(bundle, "w") as archive:
            archive.writestr("manifest.json", json.dumps(self.manifest))
            archive.writestr("kerml.cache", payload)
            archive.writestr("systems.cache", payload)
        self.args = argparse.Namespace(root=root, bundle=bundle, publications=installer,
            install_dir=root / "isolated", evidence_dir=root / "evidence", sha256=distribution.sha256(bundle))

    def invoke(self):
        with contextlib.redirect_stdout(io.StringIO()):
            result = distribution.run(self.args)
        return result, json.loads((self.args.evidence_dir / "qualification.json").read_text())

    def test_wrong_transport_hash_never_runs_the_installer(self):
        self.args.sha256 = "0" * 64
        with patch.object(distribution.subprocess, "run") as command:
            result, report = self.invoke()
        self.assertEqual(result, 1)
        self.assertFalse(report["passed"])
        command.assert_not_called()
        self.assertFalse(self.args.install_dir.exists())

    def test_failed_authentication_records_actual_exit_and_never_attempts_install(self):
        with patch.object(distribution.subprocess, "run", return_value=subprocess.CompletedProcess([], 7, b"{}", b"rejected")) as command:
            result, report = self.invoke()
        self.assertEqual(result, 1)
        self.assertFalse(report["passed"])
        self.assertEqual(command.call_count, 1)
        self.assertEqual(report["commands"][0]["exit_code"], 7)
        self.assertFalse(self.args.install_dir.exists())

    def test_successful_process_exit_does_not_replace_installed_manifest_equivalence(self):
        installed = self.args.install_dir / "test-bundle"
        replies = [
            subprocess.CompletedProcess([], 0, b"{}", b""),
            subprocess.CompletedProcess([], 0, json.dumps({"directory": str(installed), "manifest": {"wrong": True}}).encode(), b""),
            subprocess.CompletedProcess([], 0, b"{}", b""),
        ]
        with patch.object(distribution.subprocess, "run", side_effect=replies):
            result, report = self.invoke()
        self.assertEqual(result, 1)
        self.assertFalse(report["passed"])
        self.assertIn("manifest differs", report["error"])
        self.assertTrue(all(command["exit_code"] == 0 for command in report["commands"]))


if __name__ == "__main__":
    unittest.main()
