"""Run real native restart/soak with process-tree memory observations.

Requires the successful first-process real journey, its exact isolated database,
and psutil. It never edits semantic state or acceptance reports.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import subprocess
import time

import psutil


def assess_journey(path, seconds, memory_samples):
    """Require completed native evidence independently of the process exit code."""
    result = {"passed": False, "errors": [], "report_sha256": None}
    errors = result["errors"]
    if memory_samples <= 0:
        errors.append("No process memory samples were observed")
    try:
        payload = path.read_bytes()
        result["report_sha256"] = hashlib.sha256(payload).hexdigest()
        report = json.loads(payload)
    except (OSError, ValueError) as error:
        errors.append(f"Cannot read completed native report: {error}")
        return result
    if not isinstance(report, dict):
        errors.append("Native report is not an object")
        return result
    for field, expected in (("format", "agentique-native-real-acceptance/1"),
                            ("scenario", "real-restart"), ("outcome", "passed"),
                            ("passed", True), ("restart_verified", True), ("failure", None)):
        result[field if field != "passed" else "native_report_passed"] = report.get(field)
        if field not in report or type(report[field]) is not type(expected) or report[field] != expected:
            errors.append(f"Native report {field} must equal {expected!r}")
    assertions = report.get("assertions")
    if not isinstance(assertions, list) or not assertions or any(
            not isinstance(item, dict) or item.get("passed") is not True for item in assertions):
        errors.append("Native report must contain only successful completed assertions")
    soak = report.get("soak")
    result["soak"] = soak
    if not isinstance(soak, dict):
        errors.append("Native report has no soak evidence")
    else:
        elapsed = soak.get("elapsed_ms")
        if type(elapsed) is not int or elapsed < seconds * 1000:
            errors.append(f"Native soak did not complete at least {seconds} seconds")
        cycles = soak.get("completed_cycles")
        if type(cycles) is not int or cycles < 1:
            errors.append("Native soak completed no interaction cycle")
            cycles = 1
        for field in ("navigation_restorations", "cancelled_preparations", "validated_renames",
                      "graph_reads_completed_during_preparation"):
            count = soak.get(field)
            if type(count) is not int or count < cycles:
                errors.append(f"Native soak lacks {field} for every completed cycle")
        observations = soak.get("binding_observations")
        if type(observations) is not int or observations < 1:
            errors.append("Native soak has no revision-binding observations")
        anchor_error = soak.get("maximum_zoom_anchor_error_world")
        if type(anchor_error) not in (int, float) or not math.isfinite(anchor_error) or not 0 <= anchor_error <= 0.025:
            errors.append("Native soak has no passing zoom-anchor observation")
    result["passed"] = not errors
    return result


def acceptance_exit_code(exit_code, timed_out, assessment):
    return exit_code if exit_code != 0 else (0 if not timed_out and assessment["passed"] else 2)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--native", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--runtime-dir", type=Path, required=True)
    parser.add_argument("--database", type=Path, required=True)
    parser.add_argument("--restart-report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--seconds", type=int, default=600)
    parser.add_argument("--timeout", type=int, default=3600)
    args = parser.parse_args()
    if args.seconds < 600 or args.timeout <= args.seconds:
        parser.error("Use at least 600 soak seconds and a larger total timeout")
    paths = {key: value.resolve() for key, value in vars(args).items() if isinstance(value, Path)}
    if paths["output"].exists():
        parser.error("Output already exists; preserve prior evidence")
    for name in ("native", "database", "restart_report"):
        if not paths[name].is_file():
            parser.error(f"Missing {name}: {paths[name]}")
    output = paths["output"]
    output.mkdir(parents=True)
    journey = output / "journey.json"
    command = [str(paths["native"]), "--root", str(paths["root"]), "--runtime-dir", str(paths["runtime_dir"]),
               "--database", str(paths["database"]), "--no-restore", "--scenario", "real-restart",
               "--restart-report", str(paths["restart_report"]), "--scenario-report", str(journey),
               "--gallery", str(output / "gallery"), "--soak-seconds", str(args.seconds),
               "--scenario-timeout-seconds", str(args.timeout)]
    with paths["native"].open("rb") as binary:
        executable_digest = hashlib.file_digest(binary, "sha256").hexdigest()
    source = subprocess.run(["git", "rev-parse", "HEAD"], cwd=paths["root"], capture_output=True, text=True)
    started = time.monotonic()
    initial = final = soak_initial = None
    peak_rss = peak_private = peak_process = 0
    memory_samples = 0
    timed_out = False
    last_report_check = 0
    with (output / "stdout.log").open("w", encoding="utf-8") as stdout, (output / "stderr.log").open("w", encoding="utf-8") as stderr, (output / "memory.jsonl").open("w", encoding="utf-8") as samples:
        process = subprocess.Popen(command, cwd=paths["root"], stdout=stdout, stderr=stderr)
        observed = psutil.Process(process.pid)
        while process.poll() is None:
            elapsed = time.monotonic() - started
            if elapsed > args.timeout + 30:
                process.terminate()
                timed_out = True
                break
            try:
                memory = observed.memory_info()
                children = observed.children(recursive=True)
                resident = memory.rss
                private = getattr(memory, "private", 0)
                for child in children:
                    try:
                        child_memory = child.memory_info()
                        resident += child_memory.rss
                        private += getattr(child_memory, "private", 0)
                    except psutil.Error:
                        pass
                sample = {"elapsed_seconds": elapsed, "process_rss_bytes": memory.rss,
                          "tree_rss_bytes": resident, "tree_private_bytes": private,
                          "process_peak_working_set_bytes": getattr(memory, "peak_wset", None)}
                samples.write(json.dumps(sample) + "\n")
                samples.flush()
                memory_samples += 1
                initial = initial or sample
                final = sample
                peak_rss = max(peak_rss, resident)
                peak_private = max(peak_private, private)
                peak_process = max(peak_process, getattr(memory, "peak_wset", memory.rss))
                if soak_initial is None and elapsed - last_report_check >= 2:
                    last_report_check = elapsed
                    try:
                        report = json.loads(journey.read_text(encoding="utf-8"))
                        if report.get("soak", {}).get("elapsed_ms", 0) > 0:
                            soak_initial = sample
                    except (OSError, ValueError):
                        pass  # A partial atomicity-free diagnostic write is retried.
            except psutil.Error:
                pass
            time.sleep(0.25)
        exit_code = process.wait(timeout=30)
    assessment = assess_journey(journey, args.seconds, memory_samples)
    result_code = acceptance_exit_code(exit_code, timed_out, assessment)
    receipt = {"command": command, "cwd": str(paths["root"]), "source_commit": source.stdout.strip(),
               "executable_sha256": executable_digest, "exit_code": exit_code, "timed_out": timed_out,
               "acceptance_exit_code": result_code, "passed": result_code == 0,
               "journey_assessment": assessment, "memory_sample_count": memory_samples,
               "wall_seconds": time.monotonic() - started, "sample_interval_seconds": 0.25,
               "memory_start": initial, "memory_soak_start": soak_initial, "memory_end": final,
               "sampled_tree_peak_rss_bytes": peak_rss, "sampled_tree_peak_private_bytes": peak_private,
               "process_peak_working_set_bytes": peak_process,
               "scope": "RSS observations include native runtime/renderer/semantic memory; first sample includes process startup. Soak start is the first report-observed interaction-cycle sample. Samples cannot alone establish allocation leaks."}
    (output / "process.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(receipt, indent=2))
    return result_code


if __name__ == "__main__":
    raise SystemExit(main())
