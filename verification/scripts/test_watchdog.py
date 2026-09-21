"""Focused process-tree watchdog regressions; requires no third-party packages."""
import json
import ctypes
import os
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch

from watchdog import ROOT, WindowsJob, PosixGroup, execute


class WatchdogTests(unittest.TestCase):
    def setUp(self):
        root = ROOT / "verification/generated/watchdog-tests"
        root.mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(dir=root)
        self.directory = Path(self.temporary.name)
        self.addCleanup(self.temporary.cleanup)
        self.sequence = 0

    def assert_exited(self, process_ids):
        if os.name != "nt":
            return
        from ctypes import wintypes as w
        kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]
        kernel.OpenProcess.restype = w.HANDLE
        kernel.WaitForSingleObject.argtypes = [w.HANDLE, w.DWORD]
        kernel.WaitForSingleObject.restype = w.DWORD
        kernel.CloseHandle.argtypes = [w.HANDLE]
        for process_id in set(process_ids):
            handle = kernel.OpenProcess(0x100000, False, process_id)  # SYNCHRONIZE
            if not handle:
                self.assertEqual(ctypes.get_last_error(), 87)
                continue
            try:
                self.assertEqual(kernel.WaitForSingleObject(handle, 2000), 0,
                                 f"workload process {process_id} survived watchdog completion")
            finally:
                kernel.CloseHandle(handle)

    def run_argv(self, command, wall=10, memory=512 * 1024**2):
        self.sequence += 1
        output = self.directory / f"run-{self.sequence}"
        result = execute(command, output, wall, memory, 0.025, dict(os.environ))
        observations = [json.loads(line) for line in (output / "observations.jsonl").read_text().splitlines()]
        process_ids = [result["command_pid"]] if result["command_pid"] else []
        process_ids.extend(pid for sample in observations for pid in sample["process_ids"])
        self.assert_exited(process_ids)
        return result, observations

    def run_command(self, code, **limits):
        return self.run_argv([sys.executable, "-c", code], **limits)

    def test_success_and_nonzero_exit_are_distinct(self):
        success, _ = self.run_command("print('verified')")
        failure, _ = self.run_command("raise SystemExit(7)")
        self.assertEqual(success["exit_code"], 0)
        self.assertEqual(failure["exit_code"], 7)
        self.assertIsNone(failure["safety_stop"])
        self.assertGreater(success["peak_private_bytes"], 0)

    def test_wall_limit_terminates_descendants(self):
        result, samples = self.run_command(
            "import subprocess,sys,time; subprocess.Popen([sys.executable,'-c','import time; time.sleep(5)']); time.sleep(5)",
            wall=0.8,
        )
        self.assertEqual(result["exit_code"], 124)
        self.assertEqual(result["safety_stop"], "wall_time")
        self.assertLess(result["duration_seconds"], 5)
        self.assertGreaterEqual(max(s["processes"] for s in samples), 2)

    def test_child_memory_is_counted_after_parent_exits(self):
        result, samples = self.run_command(
            "import subprocess,sys; subprocess.Popen([sys.executable,'-c','import time; x=bytearray(96*1024**2); time.sleep(5)'])",
            memory=80 * 1024**2,
        )
        self.assertEqual(result["exit_code"], 124)
        self.assertEqual(result["safety_stop"], "private_memory")
        self.assertGreater(result["peak_private_bytes"], 80 * 1024**2)
        self.assertLess(result["duration_seconds"], 5)

    @unittest.skipUnless(os.name == "nt", "native Windows process regression")
    def test_native_powershell_memory_is_counted(self):
        powershell = shutil.which("powershell.exe")
        self.assertIsNotNone(powershell)
        result, samples = self.run_argv(
            [powershell, "-NoProfile", "-NonInteractive", "-Command",
             "[byte[]]$allocation = New-Object byte[] (200*1024*1024); Start-Sleep -Seconds 5"],
            memory=160 * 1024**2,
        )
        self.assertEqual(result["safety_stop"], "private_memory")
        self.assertEqual(result["exit_code"], 124)
        self.assertGreater(result["peak_private_bytes"], 160 * 1024**2)
        self.assertLess(result["duration_seconds"], 5)

    @unittest.skipUnless(os.name == "nt", "native Windows process regression")
    def test_native_process_tree_stays_in_job_and_leaves_no_survivors(self):
        powershell = shutil.which("powershell.exe")
        script = (
            "$info = New-Object Diagnostics.ProcessStartInfo; "
            f"$info.FileName = '{powershell.replace(chr(39), chr(39)*2)}'; "
            "$info.Arguments = '-NoProfile -NonInteractive -Command Start-Sleep -Seconds 5'; "
            "$info.UseShellExecute = $false; $info.CreateNoWindow = $true; "
            "$child = [Diagnostics.Process]::Start($info); Start-Sleep -Seconds 5"
        )
        result, samples = self.run_argv(
            [powershell, "-NoProfile", "-NonInteractive", "-Command", script], wall=1.5,
        )
        self.assertEqual(result["safety_stop"], "wall_time")
        self.assertEqual(result["exit_code"], 124)
        self.assertGreaterEqual(max(s["processes"] for s in samples), 2)

    def test_stop_marker_terminates_command_and_records_reason(self):
        output = self.directory / "run-1"
        result, _ = self.run_command(
            f"from pathlib import Path; import time; Path({str(output / 'stop')!r}).touch(); time.sleep(5)"
        )
        self.assertEqual(result["safety_stop"], "requested")
        self.assertEqual(result["exit_code"], 124)
        self.assertLess(result["duration_seconds"], 3)

    def test_monitor_error_fails_closed_and_is_reported(self):
        monitor = WindowsJob if os.name == "nt" else PosixGroup
        with patch.object(monitor, "sample", side_effect=OSError("injected accounting failure")):
            result, _ = self.run_command("import time; time.sleep(5)")
        self.assertEqual(result["safety_stop"], "monitor_error")
        self.assertEqual(result["exit_code"], 125)
        self.assertIn("injected accounting failure", result["monitor_error"])


if __name__ == "__main__":
    unittest.main()
