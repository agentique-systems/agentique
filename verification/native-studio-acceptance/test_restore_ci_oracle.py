import gzip
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("restore_ci_oracle", Path(__file__).with_name("restore_ci_oracle.py"))
restore = importlib.util.module_from_spec(spec)
spec.loader.exec_module(restore)


class RestoreEvidence(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name).resolve()
        subprocess.run(["git", "init", "--quiet", str(self.root)], check=True, capture_output=True)
        (self.root / ".gitignore").write_text("/verification/generated/\n")
        self.retained = self.root / "verification" / "retained"
        self.retained.mkdir(parents=True)
        self.output = self.root / "verification" / "generated" / "fresh"
        self.files = {}
        self.add("current/build.json", b'{"outcome":"passed"}\r\n', False)
        self.add("current/oracle/command-observations.json", b'{"proof":"exact"}\n' * 100, True)

    def add(self, name, data, compressed):
        stored = name + (".gz" if compressed else "")
        file = self.retained / stored
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_bytes(gzip.compress(data, mtime=0) if compressed else data)
        self.files[name] = {"path": stored, "compression": "gzip" if compressed else None,
                            "original_bytes": len(data), "original_sha256": hashlib.sha256(data).hexdigest(),
                            "stored_sha256": restore.digest(file)}

    def run_restore(self, verify_only=False):
        manifest = self.retained / "artifact-manifest.json"
        manifest.write_text(json.dumps({"format": "agentique-retained-ci-oracle/1", "files": self.files}))
        return restore.restore(self.root, self.retained, self.output, restore.digest(manifest), verify_only)

    def test_exact_bytes_and_fresh_only(self):
        result = self.run_restore()
        self.assertEqual(result["files"], 2)
        for name, record in self.files.items():
            self.assertEqual(restore.digest(self.output / name), record["original_sha256"])
        with self.assertRaisesRegex(ValueError, "existing output"):
            self.run_restore()

    def test_verify_only_creates_nothing(self):
        self.assertTrue(self.run_restore(True)["verify_only"])
        self.assertFalse(self.output.exists())

    def test_stored_tampering_and_expansion_mismatch_fail_before_write(self):
        name = "current/oracle/command-observations.json"
        file = self.retained / self.files[name]["path"]
        original = file.read_bytes()
        file.write_bytes(original + b"tamper")
        with self.assertRaisesRegex(ValueError, "Stored hash"):
            self.run_restore()
        self.assertFalse(self.output.exists())
        file.write_bytes(original)
        self.files[name]["original_bytes"] -= 1
        with self.assertRaisesRegex(ValueError, "Expanded size"):
            self.run_restore()
        self.assertFalse(self.output.exists())
        self.files[name]["original_bytes"] += 1
        self.files[name]["original_sha256"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "Original size/hash"):
            self.run_restore()

    def test_paths_executables_and_windows_aliases_rejected(self):
        for name in ["../escape", "/absolute", "C:/drive", "a\\b", "a/./b", "a/../b", "bin/tool", "a.exe", "a.EXE", "a/CON.json", "a/name."]:
            with self.subTest(name=name), self.assertRaises(ValueError):
                restore.relative(name)
        self.files["CURRENT/BUILD.JSON"] = self.files["current/build.json"].copy()
        with self.assertRaises(ValueError):
            self.run_restore()
        self.assertFalse(self.output.exists())

    def test_output_must_be_ignored_generated_and_absent(self):
        self.output = self.root / "outside"
        with self.assertRaisesRegex(ValueError, "below verification/generated"):
            self.run_restore()
        self.output = self.root / "verification" / "generated" / "fresh"
        (self.root / ".gitignore").write_text("")
        with self.assertRaisesRegex(ValueError, "Git-ignored"):
            self.run_restore()

    def test_manifest_hash_duplicate_keys_and_compression_contract(self):
        manifest = self.retained / "artifact-manifest.json"
        manifest.write_text("{}")
        with self.assertRaisesRegex(ValueError, "manifest hash"):
            restore.restore(self.root, self.retained, self.output, "0" * 64)
        manifest.write_text('{"format":"one","format":"two"}')
        with self.assertRaisesRegex(ValueError, "Duplicate JSON"):
            restore.restore(self.root, self.retained, self.output, restore.digest(manifest))
        self.files["current/build.json"]["compression"] = "zip"
        with self.assertRaisesRegex(ValueError, "Unsupported compression"):
            self.run_restore()


if __name__ == "__main__":
    unittest.main()
