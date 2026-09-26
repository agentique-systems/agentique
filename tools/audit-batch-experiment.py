"""Run exact cold-oracle batch comparisons sequentially; never alter a repository."""
import argparse
import datetime
import gc
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

import psutil


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def save(path, value):
    Path(path).write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def compare_observations(baseline, candidate):
    required = {"source-checkpoint", "strict-audit", "semantic-closure"}
    if not required.issubset(baseline) or not any(k.startswith("query/") for k in baseline):
        raise ValueError("Baseline lacks the required semantic and full query observations")
    missing = sorted(set(baseline) - set(candidate))
    extra = sorted(set(candidate) - set(baseline))
    different = sorted(k for k in baseline.keys() & candidate.keys()
                       if baseline[k] != candidate[k])
    return {"exact_equivalence": not (missing or extra or different),
            "observation_count": len(baseline),
            "query_observation_count": sum(k.startswith("query/") for k in baseline),
            "missing": missing, "extra": extra, "different": different}


def larger_batch_is_safe(previous, total_bytes, available_bytes, reserve_bytes):
    if previous.get("exit_code") != 0 or previous.get("terminated_reason"):
        return False, "128-subject child did not complete successfully"
    peak = previous["peak_rss_bytes"]
    # A deliberately conservative allowance, not a proof of future memory use.
    # Every child also has a live low-memory guard; failed runs stay evidence.
    allowed = min(total_bytes * 0.65, available_bytes - 2 * reserve_bytes)
    if peak > allowed or previous["minimum_host_available_bytes"] < 2 * reserve_bytes:
        return False, "128-subject peak or host headroom does not permit the 256-subject trial"
    return True, "128-subject child passed the conservative memory-headroom gate"


def terminate_tree(process):
    try:
        children = psutil.Process(process.pid).children(recursive=True)
    except psutil.Error:
        children = []
    for child in children:
        try:
            child.terminate()
        except psutil.Error:
            pass
    if process.poll() is not None:
        return
    try:
        process.terminate()
    except OSError:
        if process.poll() is None:
            raise
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait()


def run_child(command, cwd, environment, destination, timeout_seconds,
              reserve_bytes, interval=0.25):
    destination = Path(destination)
    started = time.monotonic()
    record = {"command": command, "cwd": str(cwd),
              "started_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
              "sample_interval_seconds": interval, "terminated_reason": None,
              "peak_rss_bytes": 0, "peak_private_bytes": 0,
              "minimum_host_available_bytes": psutil.virtual_memory().available,
              "memory_scope": "single cold-oracle process; no simultaneous semantic child"}
    low_samples = 0
    with (destination / "process.log").open("wb") as output, \
            (destination / "memory.jsonl").open("w", encoding="utf-8") as memory:
        process = subprocess.Popen(command, cwd=cwd, env=environment,
                                   stdout=output, stderr=subprocess.STDOUT)
        observed = psutil.Process(process.pid)
        while process.poll() is None:
            elapsed = time.monotonic() - started
            host = psutil.virtual_memory()
            record["minimum_host_available_bytes"] = min(
                record["minimum_host_available_bytes"], host.available)
            try:
                current = observed.memory_info()
                record["peak_rss_bytes"] = max(record["peak_rss_bytes"], current.rss,
                                               getattr(current, "peak_wset", 0))
                record["peak_private_bytes"] = max(record["peak_private_bytes"],
                                                   getattr(current, "private", 0))
                memory.write(json.dumps({"elapsed_seconds": elapsed,
                                         "rss_bytes": current.rss,
                                         "private_bytes": getattr(current, "private", None),
                                         "host_available_bytes": host.available}) + "\n")
            except psutil.Error:
                pass
            low_samples = low_samples + 1 if host.available < reserve_bytes else 0
            if elapsed > timeout_seconds:
                record["terminated_reason"] = "per-child timeout"
            elif low_samples >= 3:
                record["terminated_reason"] = "host available memory below recorded reserve"
            if record["terminated_reason"]:
                terminate_tree(process)
                break
            time.sleep(interval)
        record["exit_code"] = process.wait()
    record["elapsed_seconds"] = time.monotonic() - started
    record["output_sha256"] = digest(destination / "process.log")
    save(destination / "process.json", record)
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", required=True, type=Path)
    parser.add_argument("--build-directory", required=True, type=Path)
    parser.add_argument("--inputs-directory", required=True, type=Path)
    parser.add_argument("--expected-observations", required=True, type=Path)
    parser.add_argument("--input-receipt", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--kerml-cache", required=True, type=Path)
    parser.add_argument("--systems-cache", required=True, type=Path)
    parser.add_argument("--timeout-seconds", type=int, default=1200)
    parser.add_argument("--reserve-mib", type=int, default=512)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("output must be a new directory; prior evidence is never overwritten")
    if args.timeout_seconds < 1 or args.reserve_mib < 128:
        parser.error("positive timeout and at least 128 MiB memory reserve required")
    args.output.mkdir(parents=True)
    summary = {"format": "agentique-audit-batch-experiment/1", "outcome": "incomplete",
               "batches": {}, "same_runner": True, "same_executable": True,
               "reserve_bytes": args.reserve_mib * 1024 ** 2}
    try:
        source = args.source_root.resolve(strict=True)
        build_dir = args.build_directory.resolve(strict=True)
        build = load(build_dir / "build.json")
        commit = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"],
                                         text=True).strip()
        if build["outcome"] != "passed" or build["component"] != "oracle" \
                or build["source_commit"] != commit:
            raise ValueError("Successful exact-source oracle build receipt required")
        if subprocess.check_output(["git", "-C", str(source), "diff", "HEAD", "--name-only"],
                                   text=True).strip():
            raise ValueError("Experiment requires a clean tracked source checkout")
        for path, expected in build["source_files"].items():
            if digest(source / path) != expected:
                raise ValueError(f"Build source input changed: {path}")
        binary = build_dir / "bin/create_part_performance.exe"
        binary_hash = digest(binary)
        if len(build["binaries"]) != 1 or build["binaries"][0]["sha256"] != binary_hash:
            raise ValueError("Executable differs from recorded oracle build")
        inputs = args.inputs_directory.resolve(strict=True)
        input_hashes = {name: digest(inputs / name)
                        for name in ("checkpoint.json", "sources.json")}
        input_receipt = load(args.input_receipt)
        if input_receipt["outcome"] != "authenticated":
            raise ValueError("Authenticated input receipt required")
        for name, expected in input_hashes.items():
            if input_receipt["files"][name]["sha256"] != expected:
                raise ValueError(f"Authoritative input differs from authenticated receipt: {name}")
        prior_hash = digest(args.expected_observations)
        if input_receipt["files"]["cold-observations.json"]["sha256"] != prior_hash:
            raise ValueError("Prior observation map differs from authenticated receipt")
        runtime = {"kerml": digest(args.kerml_cache), "systems": digest(args.systems_cache)}
        summary.update(source_commit=commit, executable_sha256=binary_hash,
                       build_receipt_sha256=digest(build_dir / "build.json"),
                       input_sha256=input_hashes, runtime_cache_sha256=runtime,
                       input_receipt_sha256=digest(args.input_receipt),
                       prior_observations_sha256=prior_hash,
                       physical_memory_bytes=psutil.virtual_memory().total,
                       observer_sha256=digest(__file__))
        save(args.output / "result.json", summary)
        baseline_path = None
        for batch_size in (32, 128, 256):
            if batch_size == 256:
                host = psutil.virtual_memory()
                previous = summary["batches"].get("128", {}).get("process", {})
                safe, reason = larger_batch_is_safe(previous, host.total, host.available,
                                                    summary["reserve_bytes"])
                if not safe:
                    summary["batches"]["256"] = {"outcome": "skipped", "reason": reason}
                    save(args.output / "result.json", summary)
                    continue
            destination = args.output / f"batch-{batch_size}"
            destination.mkdir()
            for name, expected in input_hashes.items():
                shutil.copyfile(inputs / name, destination / name)
                if digest(destination / name) != expected:
                    raise ValueError(f"Copied authoritative input changed: {name}")
            environment = os.environ.copy()
            exact_environment = {
                "AGENTIQUE_SOURCE_ROOT": str(source),
                "AGENTIQUE_KERML_CACHE": str(args.kerml_cache.resolve()),
                "AGENTIQUE_SYSTEMS_CACHE": str(args.systems_cache.resolve()),
                "AGENTIQUE_CREATE_PART_ORACLE_STAGE": "cold",
                "AGENTIQUE_CREATE_PART_ORACLE_DIR": str(destination.resolve()),
                "AGENTIQUE_AUDIT_BATCH_SIZE": str(batch_size),
            }
            environment.update(exact_environment)
            command = [str(binary), "create_part_command_matches_full_self_model_reconstruction",
                       "--exact", "--ignored", "--nocapture", "--test-threads=1"]
            trial = {"environment": exact_environment, "outcome": "incomplete"}
            summary["batches"][str(batch_size)] = trial
            save(args.output / "result.json", summary)
            process = run_child(command, source, environment, destination,
                                args.timeout_seconds, summary["reserve_bytes"])
            trial["process"] = process
            try:
                if process["exit_code"] != 0 or process["terminated_reason"]:
                    raise ValueError("Cold child failed; original process evidence retained")
                metrics = load(destination / "cold-metrics.json")
                if metrics["validated"] is not True:
                    raise ValueError("Cold child did not independently validate")
                observations_path = destination / "cold-observations.json"
                observations = load(observations_path)
                # Load maps only after the semantic child exits, then release them
                # before the next child; observer memory must not crowd the runtime.
                baseline = load(baseline_path) if baseline_path else observations
                comparison = compare_observations(baseline, observations)
                if batch_size == 32:
                    trial["prior_oracle_comparison"] = compare_observations(
                        load(args.expected_observations), observations)
                    if not trial["prior_oracle_comparison"]["exact_equivalence"]:
                        raise ValueError("Batch32 differs from the pinned prior passing cold oracle")
                    baseline_path = observations_path
                elif baseline_path is None:
                    raise ValueError("No successful 32-subject baseline to compare")
                trial.update(metrics=metrics, comparison=comparison,
                             observations_sha256=digest(destination / "cold-observations.json"))
                if not comparison["exact_equivalence"]:
                    raise ValueError("Complete semantic observation map differs from batch 32")
                trial["outcome"] = "passed"
            except (OSError, KeyError, ValueError) as error:
                trial.update(outcome="failed", error=str(error))
            finally:
                # These locals may be absent after a failed child. Rebinding frees
                # previous large maps without retaining any semantic authority.
                baseline = observations = None
                gc.collect()
            save(args.output / "result.json", summary)
        passed = all(summary["batches"].get(str(size), {}).get("outcome") == "passed"
                     for size in (32, 128))
        passed = passed and summary["batches"]["256"]["outcome"] in ("passed", "skipped")
        summary["outcome"] = "passed" if passed else "failed"
        summary["contract"] = (
            "Same authoritative checkpoint/source bytes and executable; exact whole observation "
            "maps and independent validation, with timings and whole-process memory including "
            "the subsequent full-proof export. A passing experiment is not Alpha acceptance.")
    except Exception as error:
        summary.update(outcome="failed", error=str(error))
        raise
    finally:
        save(args.output / "result.json", summary)
    print(json.dumps(summary), flush=True)
    return 0 if summary["outcome"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
