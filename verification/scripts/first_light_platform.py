"""Run retained Phase 2 gates only after authenticating an installed runtime.

No publication acquisition or rebuild is performed. Installed paths come from
the runtime CLI, not a second implementation of the receipt or bundle contract.
Each gate starts with fresh temporary repository state and runs serially.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys

from run import ROOT


SUMMARY = "verification/summaries/agentique-studio-first-light/platform-runtime.json"
CARGO = ["cargo", "--quiet"]
PROFILE = ["--release", "--config", "profile.release.lto=false", "--locked", "--offline"]
LOW_ARTIFACT = {
    "CARGO_PROFILE_RELEASE_DEBUG": "0",
    "CARGO_INCREMENTAL": "0",
    "CARGO_BUILD_JOBS": "2",
}
GATES = {
    "compact": ("agq-modeling-service", "durable_platform", "durable_cache_and_failure_authentication"),
    "lifecycle": ("agq-modeling-service", "durable_platform", "durable_restore_branch_binding_diff_and_cas"),
    "http": ("agq-modeling-http", "vertical", "durable_project_revision_http_vertical_and_stable_continuation"),
    "scale": ("agq-modeling-service", "durable_platform", "durable_scale_100_mixed_documents_10_revisions_four_readers"),
    "edit-oracle": ("agq-modeling-workspace", "phase2_checkpoint", "local_edit_classes_match_full_semantic_oracle"),
}


def recorded(name, command, environment, overrides):
    runner = [sys.executable, str(ROOT / "verification/scripts/run.py"),
              "--name", name, "--summary", SUMMARY]
    for key, value in overrides.items():
        runner += ["--env", f"{key}={value}"]
    result = subprocess.run([*runner, "--", *command], cwd=ROOT, env=environment, check=False)
    if result.returncode:
        raise SystemExit(result.returncode)
    records = json.loads((ROOT / SUMMARY).read_text(encoding="utf-8"))["commands"]
    return next(record for record in reversed(records) if record["name"] == name)


def output_object(record):
    text = (ROOT / record["output"]).read_text(encoding="utf-8")
    # Cargo diagnostics precede stdout; the CLI's final JSON object is the
    # machine-readable value. Do not infer paths from human progress messages.
    for match in reversed(list(re.finditer(r"^\{", text, flags=re.MULTILINE))):
        try:
            return json.loads(text[match.start():])
        except json.JSONDecodeError:
            continue
    raise RuntimeError("runtime status did not return a JSON location")


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--runtime-dir", type=Path, help="Installed Agentique runtime store; normal per-user default when omitted")
    parser.add_argument("--gate", choices=["all", *GATES], default="all")
    args = parser.parse_args()
    environment = os.environ.copy()
    # Discovery must use the installed bundle even in a developer shell that
    # still has old verification-cache compatibility overrides.
    for name in ["AGENTIQUE_KERML_CACHE", "AGENTIQUE_SYSTEMS_CACHE"]:
        environment.pop(name, None)
    runtime = [*CARGO, "run", *PROFILE, "-p", "agq-runtime-publications",
               "--bin", "agq-publications", "--", "--root", str(ROOT)]
    if args.runtime_dir:
        runtime += ["--runtime-dir", str(args.runtime_dir.resolve())]
    location = output_object(recorded("platform-installed-discovery", [*runtime, "status"], environment, LOW_ARTIFACT))
    if location.get("origin") != "installed_bundle" or not location.get("bundle_dir"):
        raise RuntimeError("platform gates require an installed bundle; run Studio setup first")
    recorded("platform-installed-authentication",
             [*runtime, "verify", "--bundle", location["bundle_dir"]], environment, LOW_ARTIFACT)
    overrides = {
        **LOW_ARTIFACT,
        "AGENTIQUE_KERML_CACHE": str(Path(location["kerml_cache"]).resolve()),
        "AGENTIQUE_SYSTEMS_CACHE": str(Path(location["systems_cache"]).resolve()),
    }
    selected = ["compact", "lifecycle", "http", "scale"] if args.gate == "all" else [args.gate]
    if args.gate == "scale":
        selected.insert(0, "lifecycle")
    for name in selected:
        package, target, test = GATES[name]
        command = [*CARGO, "test", *PROFILE, "-p", package]
        if package != "agq-modeling-http":
            command += ["--features", "verification"]
        command += ["--test", target, test, "--", "--exact", "--ignored", "--nocapture", "--test-threads=1"]
        recorded(f"first-light-{name}", command, environment, overrides)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
