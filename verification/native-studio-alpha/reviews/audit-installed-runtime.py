"""Review actual distribution bytes and retained normal-facade execution evidence.

This does not rerun the facades or treat a manifest as semantic authority.
"""
import datetime
import hashlib
import json
import pathlib
import subprocess
import sys
import time
import zipfile

root, runtime_root, output = map(pathlib.Path, sys.argv[1:4])
started = time.monotonic()
evidence = runtime_root / "verification/native-studio-alpha/runtime"


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def load(path):
    return json.loads(path.read_bytes())


def git(*args):
    return subprocess.check_output(["git", "-C", str(root), *args])


checks = {}


def require(name, condition):
    checks[name] = bool(condition)
    if not condition:
        raise RuntimeError(name)


pin = load(evidence / "current-runtime-installer.json")
binary = pathlib.Path(pin["binary"])
require("same_current_executable", sha(binary) == pin["binary_sha256"] ==
        "0f656a2c026acbe9bf859d9865a62dd9951202e8e3e4b570a7522e87ea553acb")
build = load(root / pin["build_receipt"])
require("build_success_and_retained_log", build["exit_code"] == 0 and
        sha(root / build["output"]) == build["output_sha256"])
require("pin_integrated_before_build", subprocess.run(
    ["git", "-C", str(root), "merge-base", "--is-ancestor",
     pin["integrated_transport_commit"], pin["build_source_commit"]],
    check=False).returncode == 0)

paths = [
    "crates/runtime-publications/src/main.rs",
    "crates/runtime-publications/src/lib.rs",
    "crates/kerml-semantics/src/trusted_publication.rs",
    "crates/kerml-text/src/library/publication_cache.rs",
    "crates/kerml-text/src/sysml/publication_cache.rs",
    "crates/kerml-text/src/sysml/publication_restore.rs",
    "standards/kerml-accepted-publication.json",
    "standards/kerml-standard-bindings.json",
    "standards/sysml-accepted-publication.json",
    "standards/sysml-standard-bindings.json",
    "standards/runtime-transports/sysml-v3-rematerialized-2026-09-25.json",
]
source_hashes = {}
for path in paths:
    actual = (root / path).read_bytes().replace(b"\r\n", b"\n")
    committed = git("show", f"{pin['build_source_commit']}:{path}").replace(b"\r\n", b"\n")
    require(f"build_source_unchanged:{path}", actual == committed)
    source_hashes[path] = hashlib.sha256(actual).hexdigest()
require("exact_reviewed_trust_source", source_hashes[paths[2]] ==
        "f280efebad2e03cc194c88ff64dd0a046d5c5a5d28e5e06003bfb05df423261a")
require("exact_reviewed_finite_pin", source_hashes[paths[-1]] ==
        "12cc2aeff7aa7e4a1f3de16faf13d63aa158dd88248e9b3179ad4629e1ffeda0")

receipts = {}
results = {}
for name in ["pack", "verify", "install"]:
    receipt_path = evidence / f"runtime-{name}.json"
    receipt = load(receipt_path)
    log_path = pathlib.Path(receipt["raw_output"])
    require(f"{name}_success_exact_binary_and_log", receipt["exit_code"] == 0 and
            pathlib.Path(receipt["command"][0]) == binary and
            name in receipt["command"] and sha(log_path) == receipt["output_sha256"])
    text = log_path.read_text(encoding="utf-8")
    phases = text[:text.index("{")].splitlines()
    for phase in ["authenticating_kerml", "authenticating_sysml", "ready"]:
        require(f"{name}_observed_{phase}", json.dumps(phase) in phases)
    results[name] = json.loads(text[text.index("{"):])
    receipts[name] = {"receipt_sha256": sha(receipt_path),
                      "log_sha256": sha(log_path),
                      "exit_code": receipt["exit_code"],
                      "elapsed_seconds": receipt["elapsed_seconds"]}

bundle = runtime_root / "verification/generated/native-studio-alpha/runtime-package/accepted-runtime.agq-runtime"
require("bundle_transport_exact", bundle.stat().st_size == 610190454 and sha(bundle) ==
        "37edf34cc0220ecdded8e0162f3fc1955ee2f3849e6dccf7ba373833ade3b026")
manifest = load(evidence / "runtime-manifest.json")
require("ordinary_commands_same_manifest", manifest == results["pack"] == results["install"]["manifest"])
installed = pathlib.Path(results["install"]["directory"])
require("installed_manifest_exact", load(installed / "manifest.json") == manifest)
assets = {}
with zipfile.ZipFile(bundle) as archive:
    require("exact_bundle_entry_population", sorted(archive.namelist()) ==
            ["kerml.cache", "manifest.json", "systems.cache"])
    require("embedded_manifest_exact", json.loads(archive.read("manifest.json")) == manifest)
    for kind in ["kerml", "systems"]:
        asset = manifest[kind]
        with archive.open(asset["file"]) as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        require(f"{kind}_embedded_and_installed_cache_exact",
                archive.getinfo(asset["file"]).file_size == asset["bytes"] ==
                (installed / asset["file"]).stat().st_size and
                digest == asset["sha256"] == sha(installed / asset["file"]))
        assets[kind] = {"bytes": asset["bytes"], "sha256": digest,
                        "publication": asset["publication"]}
require("reviewed_kerml_cache", assets["kerml"]["sha256"] ==
        "aadf9ff589eea35bcedc5d1e3d0521952058b55f3325614ecb0ab12ae36300c4")
require("reviewed_systems_cache", assets["systems"]["sha256"] ==
        "02ba47ad99ceaddb853b198a556cbe40351a00361f077099a993f3a226bcc323")

before = json.loads(git("show", "1edf050^:standards/sysml-publication-inputs.json"))
after = json.loads(git("show", "1edf050:standards/sysml-publication-inputs.json"))
old_inputs, new_inputs = before.pop("inputs"), after.pop("inputs")
require("freshness_does_not_change_publication_authority", before == after and
        after["grants_publication_authority"] is False)
changes = sorted(key for key in old_inputs.keys() | new_inputs.keys()
                 if old_inputs.get(key) != new_inputs.get(key))
require("freshness_only_exact_reviewed_source_and_transport", changes == sorted([paths[2], paths[-1]]) and
        all(new_inputs[key] == source_hashes[key] for key in changes))
for name in ["transport-freshness-regression", "integrated-standards-transport-fixed"]:
    receipt = load(root / f"verification/native-studio-alpha/checks/{name}.json")
    require(f"{name}_retained_success", receipt["exit_code"] == 0 and
            sha(root / receipt["output"]) == receipt["output_sha256"])

result = {"format": "agq-independent-installed-runtime-review/1",
          "finished_utc": datetime.datetime.now(datetime.UTC).isoformat(),
          "command": sys.argv, "exit_code": 0,
          "elapsed_seconds": round(time.monotonic() - started, 3),
          "audit_script_sha256": sha(pathlib.Path(__file__)),
          "ordinary_facade_authentication_evidence_accepted": True,
          "facades_rerun_by_reviewer": False,
          "native_real_model_acceptance_established": False,
          "checks": checks, "installer": pin,
          "build_source_hashes_lf": source_hashes,
          "command_receipts": receipts,
          "bundle": {"bytes": bundle.stat().st_size, "sha256": sha(bundle),
                     "identity": manifest["identity"]},
          "assets": assets, "installed_directory": str(installed),
          "verify_timings": results["verify"],
          "install_timings": results["install"]["timings"]}
output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8", newline="\n")
print(json.dumps(result, indent=2))
