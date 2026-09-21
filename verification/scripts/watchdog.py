"""Run one command with process-tree resource limits; raw observations stay ignored.

Example: python verification/scripts/watchdog.py --name publication 
  --wall-seconds 5400 --private-mib 6144 -- target/release/examples/canonical_publication.exe ...
Limits are workflow safeguards, never semantic acceptance criteria.
Create the run's ignored `stop` file to terminate its command tree and retain a summary.
"""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import time

from run import ROOT, source_identity


class WindowsJob:
    """A kill-on-close job accounts for and terminates the entire command tree."""

    def __init__(self):
        from ctypes import wintypes as w

        size = ctypes.c_size_t

        class Basic(ctypes.Structure):
            _fields_ = [("per_process_time", ctypes.c_int64), ("per_job_time", ctypes.c_int64),
                        ("flags", w.DWORD), ("min_ws", size), ("max_ws", size),
                        ("active_limit", w.DWORD), ("affinity", size),
                        ("priority", w.DWORD), ("scheduling", w.DWORD)]

        class Io(ctypes.Structure):
            _fields_ = [(name, ctypes.c_uint64) for name in
                        ("read_ops", "write_ops", "other_ops", "read_bytes", "write_bytes", "other_bytes")]

        class Extended(ctypes.Structure):
            _fields_ = [("basic", Basic), ("io", Io), ("process_limit", size),
                        ("job_limit", size), ("peak_process", size), ("peak_job", size)]

        class Memory(ctypes.Structure):
            _fields_ = [("cb", w.DWORD), ("faults", w.DWORD)] + [
                (name, size) for name in ("peak_ws", "ws", "peak_paged", "paged",
                                         "peak_nonpaged", "nonpaged", "pagefile", "peak_pagefile", "private")]

        class ThreadEntry(ctypes.Structure):
            _fields_ = [("size", w.DWORD), ("usage", w.DWORD), ("thread_id", w.DWORD),
                        ("process_id", w.DWORD), ("base_priority", w.LONG),
                        ("delta_priority", w.LONG), ("flags", w.DWORD)]

        self.extended = Extended
        self.memory = Memory
        self.thread_entry = ThreadEntry
        self.kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        self.psapi = ctypes.WinDLL("psapi", use_last_error=True)
        for name, args, result in [
            ("CreateJobObjectW", [ctypes.c_void_p, w.LPCWSTR], w.HANDLE),
            ("SetInformationJobObject", [w.HANDLE, ctypes.c_int, ctypes.c_void_p, w.DWORD], w.BOOL),
            ("QueryInformationJobObject", [w.HANDLE, ctypes.c_int, ctypes.c_void_p, w.DWORD, ctypes.c_void_p], w.BOOL),
            ("AssignProcessToJobObject", [w.HANDLE, w.HANDLE], w.BOOL),
            ("TerminateJobObject", [w.HANDLE, w.UINT], w.BOOL),
            ("OpenProcess", [w.DWORD, w.BOOL, w.DWORD], w.HANDLE),
            ("CreateToolhelp32Snapshot", [w.DWORD, w.DWORD], w.HANDLE),
            ("Thread32First", [w.HANDLE, ctypes.c_void_p], w.BOOL),
            ("Thread32Next", [w.HANDLE, ctypes.c_void_p], w.BOOL),
            ("OpenThread", [w.DWORD, w.BOOL, w.DWORD], w.HANDLE),
            ("ResumeThread", [w.HANDLE], w.DWORD),
            ("GetExitCodeProcess", [w.HANDLE, ctypes.POINTER(w.DWORD)], w.BOOL),
            ("CloseHandle", [w.HANDLE], w.BOOL),
        ]:
            fn = getattr(self.kernel, name)
            fn.argtypes, fn.restype = args, result
        self.psapi.GetProcessMemoryInfo.argtypes = [w.HANDLE, ctypes.c_void_p, w.DWORD]
        self.psapi.GetProcessMemoryInfo.restype = w.BOOL
        self.handle = self.kernel.CreateJobObjectW(None, None)
        if not self.handle:
            raise ctypes.WinError(ctypes.get_last_error())
        limits = Extended()
        limits.basic.flags = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        if not self.kernel.SetInformationJobObject(self.handle, 9, ctypes.byref(limits), ctypes.sizeof(limits)):
            self.close()
            raise ctypes.WinError(ctypes.get_last_error())

    def attach(self, process):
        if not self.kernel.AssignProcessToJobObject(self.handle, int(process._handle)):
            raise ctypes.WinError(ctypes.get_last_error())

    def resume(self, process):
        """Resume the suspended primary thread only after successful job attachment."""
        snapshot = self.kernel.CreateToolhelp32Snapshot(0x4, 0)  # TH32CS_SNAPTHREAD
        if snapshot == ctypes.c_void_p(-1).value:
            raise ctypes.WinError(ctypes.get_last_error())
        threads = []
        try:
            entry = self.thread_entry()
            entry.size = ctypes.sizeof(entry)
            present = self.kernel.Thread32First(snapshot, ctypes.byref(entry))
            while present:
                if entry.process_id == process.pid:
                    threads.append(entry.thread_id)
                present = self.kernel.Thread32Next(snapshot, ctypes.byref(entry))
            if ctypes.get_last_error() != 18:  # ERROR_NO_MORE_FILES
                raise ctypes.WinError(ctypes.get_last_error())
        finally:
            self.kernel.CloseHandle(snapshot)
        if len(threads) != 1:
            raise RuntimeError(f"suspended process {process.pid} has {len(threads)} threads")
        thread = self.kernel.OpenThread(0x2, False, threads[0])  # THREAD_SUSPEND_RESUME
        if not thread:
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            if self.kernel.ResumeThread(thread) != 1:
                raise RuntimeError("primary thread was not suspended exactly once")
        finally:
            self.kernel.CloseHandle(thread)

    def sample(self):
        # Query JobObjectBasicProcessIdList; grow until every descendant fits.
        capacity = 64
        while True:
            raw = ctypes.create_string_buffer(8 + ctypes.sizeof(ctypes.c_size_t) * capacity)
            if self.kernel.QueryInformationJobObject(self.handle, 3, raw, len(raw), None):
                break
            if ctypes.get_last_error() != 234:  # ERROR_MORE_DATA
                raise ctypes.WinError(ctypes.get_last_error())
            capacity *= 2
        count = ctypes.c_uint32.from_buffer(raw, 4).value
        ids = (ctypes.c_size_t * count).from_buffer(raw, 8)
        rss = private = 0
        for process_id in ids:
            handle = self.kernel.OpenProcess(0x410, False, process_id)
            if not handle:
                if ctypes.get_last_error() == 87:  # Process exited between queries.
                    continue
                raise ctypes.WinError(ctypes.get_last_error())
            try:
                memory = self.memory()
                memory.cb = ctypes.sizeof(memory)
                if self.psapi.GetProcessMemoryInfo(handle, ctypes.byref(memory), memory.cb):
                    rss += memory.ws
                    private += memory.private
                else:
                    raise ctypes.WinError(ctypes.get_last_error())
            finally:
                self.kernel.CloseHandle(handle)
        limits = self.extended()
        if not self.kernel.QueryInformationJobObject(self.handle, 9, ctypes.byref(limits), ctypes.sizeof(limits), None):
            raise ctypes.WinError(ctypes.get_last_error())
        return {"processes": count, "process_ids": list(ids), "rss_bytes": rss, "private_bytes": private,
                "peak_private_bytes": limits.peak_job}

    def terminate(self):
        if not self.kernel.TerminateJobObject(self.handle, 124):
            raise ctypes.WinError(ctypes.get_last_error())

    def close(self):
        if self.handle:
            self.kernel.CloseHandle(self.handle)
            self.handle = None


class PosixGroup:
    def attach(self, process):
        self.pid = process.pid

    def sample(self):
        rss = private = count = 0
        ids = []
        for path in Path("/proc").glob("[0-9]*/stat"):
            try:
                # comm may contain spaces or parentheses; fields after its closing ')' are stable.
                fields = path.read_text().rsplit(")", 1)[1].split()
                if int(fields[2]) != self.pid:  # process group
                    continue
                count += 1
                ids.append(int(path.parent.name))
                rss += int(fields[21]) * os.sysconf("SC_PAGE_SIZE")
                for line in path.with_name("smaps_rollup").read_text().splitlines():
                    if line.startswith(("Private_Clean:", "Private_Dirty:", "Private_Hugetlb:")):
                        private += int(line.split()[1]) * 1024
            except (FileNotFoundError, ProcessLookupError):
                continue
        return {"processes": count, "process_ids": ids, "rss_bytes": rss, "private_bytes": private,
                "peak_private_bytes": private}

    def terminate(self):
        try:
            os.killpg(self.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass

    def close(self):
        pass


def execute(command, output, wall_seconds, private_bytes, interval, environment, progress_pattern=None):
    """Return observations even on failure; failure to monitor terminates the tree."""
    output.mkdir(parents=True, exist_ok=False)
    monitor = WindowsJob() if os.name == "nt" else PosixGroup()
    start = time.monotonic()
    peak_rss = peak_private = samples = 0
    stop = None
    process = None
    monitor_error = None
    command_pid = None
    progress = None
    progress_regex = re.compile(progress_pattern) if progress_pattern else None
    try:
        with (output / "output.log").open("w", encoding="utf-8") as log, \
                (output / "observations.jsonl").open("w", encoding="utf-8") as raw:
            # Attach the actual native command while its primary thread is suspended.
            # Packaged Python launchers can activate a child outside their job;
            # never interpose a Python process between the job and the workload.
            process = subprocess.Popen(
                command, cwd=ROOT, stdin=subprocess.DEVNULL,
                stdout=log, stderr=subprocess.STDOUT, env=environment,
                start_new_session=os.name != "nt",
                creationflags=(subprocess.CREATE_NO_WINDOW | 0x4) if os.name == "nt" else 0,
            )
            command_pid = process.pid
            monitor.attach(process)
            if os.name == "nt":
                monitor.resume(process)
            while True:
                elapsed = time.monotonic() - start
                observation = monitor.sample()
                if progress_regex:
                    # Observe a bounded tail, never copy a growing command log.
                    with (output / "output.log").open("rb") as progress_log:
                        progress_log.seek(max(0, os.fstat(progress_log.fileno()).st_size - 65536))
                        for match in progress_regex.finditer(progress_log.read().decode("utf-8", errors="replace")):
                            progress = match.groupdict() or {"line": match.group(0)}
                    observation["progress"] = progress
                peak_rss = max(peak_rss, observation["rss_bytes"])
                peak_private = max(peak_private, observation["private_bytes"], observation["peak_private_bytes"])
                samples += 1
                raw.write(json.dumps({"elapsed_seconds": round(elapsed, 3), **observation}) + "\n")
                raw.flush()
                if elapsed > wall_seconds:
                    stop = "wall_time"
                elif peak_private > private_bytes:
                    stop = "private_memory"
                elif (output / "stop").exists():
                    stop = "requested"
                if stop:
                    monitor.terminate()
                    break
                if process.poll() is not None and observation["processes"] == 0:
                    break
                time.sleep(interval)
            result = process.wait()
    except BaseException as error:
        stop = "interrupted" if isinstance(error, KeyboardInterrupt) else "monitor_error"
        monitor_error = f"{type(error).__name__}: {error}"
        if process is not None:
            try:
                monitor.terminate()
            except OSError as termination_error:
                # Closing the job below remains a second kill-on-close barrier.
                monitor_error += f"; terminate: {termination_error}"
            finally:
                if process.poll() is None:
                    process.kill()
                process.wait()
        result = process.returncode if process is not None else 125
    finally:
        monitor.close()
    return {
        "exit_code": 125 if stop == "monitor_error" else 124 if stop else result,
        "command_exit_code": result, "command_pid": command_pid,
        "monitor_error": monitor_error,
        "last_progress": progress,
        "safety_stop": stop, "duration_seconds": round(time.monotonic() - start, 3),
        "peak_rss_bytes": peak_rss, "peak_private_bytes": peak_private, "samples": samples,
        "limits": {"wall_seconds": wall_seconds, "private_bytes": private_bytes, "sample_seconds": interval},
        "output_sha256": hashlib.sha256((output / "output.log").read_bytes()).hexdigest(),
    }


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--name", required=True)
    parser.add_argument("--wall-seconds", type=float, default=600)
    parser.add_argument("--private-mib", type=float, default=6144)
    parser.add_argument("--sample-seconds", type=float, default=1)
    parser.add_argument("--progress-pattern", help="Optional regex; named groups become sampled progress counters")
    parser.add_argument("--summary", default="verification/summaries/overnight-convergence/commands.json")
    parser.add_argument("--env", action="append", default=[])
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command or min(args.wall_seconds, args.private_mib, args.sample_seconds) <= 0:
        parser.error("a command and positive limits are required")
    if Path(args.name).name != args.name or args.name in (".", ".."):
        parser.error("name must be a filename component")
    if args.progress_pattern:
        try:
            re.compile(args.progress_pattern)
        except re.error as error:
            parser.error(f"invalid progress regex: {error}")
    summary = ROOT / args.summary
    commit, digest = source_identity(Path(args.summary).as_posix())
    output = ROOT / "verification/generated/overnight-convergence" / args.name
    suffix = 1
    while output.exists():
        suffix += 1
        output = output.with_name(f"{args.name}-{suffix}")
    overrides = dict(item.split("=", 1) for item in args.env)
    executable = shutil.which(command[0]) or command[0]
    result = execute([executable, *command[1:]], output, args.wall_seconds,
                     int(args.private_mib * 1024**2), args.sample_seconds, {**os.environ, **overrides}, args.progress_pattern)
    record = {"name": args.name, "command": command, "source_commit": commit,
              "working_changes_sha256": digest, "environment_overrides": overrides,
              "watchdog_python": sys.version.split()[0], "progress_pattern": args.progress_pattern,
              "output": output.relative_to(ROOT).as_posix(), **result}
    (output / "result.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
    summary.parent.mkdir(parents=True, exist_ok=True)
    lock = summary.with_suffix(".lock")
    while True:
        try:
            descriptor = os.open(lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
            break
        except FileExistsError:
            time.sleep(0.05)
    try:
        data = json.loads(summary.read_text()) if summary.exists() else {"format": "agentique-watchdog-summary/1", "commands": []}
        data["commands"].append(record)
        summary.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    finally:
        os.close(descriptor)
        lock.unlink()
    print(f"{args.name}: exit {result['exit_code']}; {result['duration_seconds']}s; "
          f"peak private {result['peak_private_bytes'] / 1024**2:.1f} MiB; stop={result['safety_stop']}", flush=True)
    raise SystemExit(result["exit_code"])


if __name__ == "__main__":
    main()
