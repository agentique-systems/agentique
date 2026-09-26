"""Diagnostic redraw heartbeat for one explicitly identified native test process.

This sends no semantic command, click, key, or model data. It compensates for an
acceptance-runner scheduling defect; runs using it must retain that limitation.
"""
import argparse
import ctypes
import hashlib
import json
import pathlib
import time
from ctypes import wintypes

import psutil

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--pid", required=True, type=int)
parser.add_argument("--executable", required=True, type=pathlib.Path)
parser.add_argument("--seconds", required=True, type=int)
args = parser.parse_args()
assert 0 < args.seconds <= 600
process = psutil.Process(args.pid)
executable = args.executable.resolve(strict=True)
assert pathlib.Path(process.exe()).resolve() == executable
with executable.open("rb") as stream:
    identity = hashlib.file_digest(stream, "sha256").hexdigest()
user = ctypes.WinDLL("user32", use_last_error=True)
user.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
user.RedrawWindow.argtypes = [wintypes.HWND, ctypes.c_void_p, wintypes.HANDLE, wintypes.UINT]
user.IsWindowVisible.argtypes = [wintypes.HWND]
callback_type = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
windows = []


def each_window(hwnd, _):
    pid = wintypes.DWORD()
    user.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
    if pid.value == args.pid and user.IsWindowVisible(hwnd):
        title = ctypes.create_unicode_buffer(512)
        user.GetWindowTextW(hwnd, title, len(title))
        if title.value.startswith("Agentique"):
            windows.append(hwnd)
    return True


user.EnumWindows(callback_type(each_window), 0)
assert len(windows) == 1, windows
hwnd = windows[0]
started = time.monotonic()
redraws = 0
while time.monotonic() - started < args.seconds and process.is_running():
    owner = wintypes.DWORD()
    user.GetWindowThreadProcessId(hwnd, ctypes.byref(owner))
    if owner.value != args.pid:
        break
    # RDW_INVALIDATE: request ordinary window painting without synchronous waits.
    if not user.RedrawWindow(hwnd, None, None, 0x1):
        break
    redraws += 1
    time.sleep(0.016)
print(json.dumps({
    "scope": "Diagnostic window redraw requests only; externally assisted runner cadence",
    "pid": args.pid, "window": hwnd, "executable": str(executable),
    "executable_sha256": identity, "elapsed_seconds": time.monotonic() - started,
    "redraw_requests": redraws,
}, indent=2))
