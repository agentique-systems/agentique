"""Read-only implementation review; all mutation probes use temporary fixtures."""
import hashlib
import importlib.util
import json
import os
import pathlib
import subprocess
import sys
import time


root = pathlib.Path(sys.argv[1]).resolve()
output = pathlib.Path(sys.argv[2]).resolve()
sys.dont_write_bytecode = True
started = time.time()
suite_path = root / "verification/scripts/test_runtime_rematerialization.py"
spec = importlib.util.spec_from_file_location("runtime_review_suite", suite_path)
suite = importlib.util.module_from_spec(spec)
spec.loader.exec_module(suite)
fixture = suite.SystemsTransportPreparation()
fixture.setUp()
original_zip = suite.PREPARE.zipfile.ZipFile


def replace_outer_encoding(path, mode="r", *args, **kwargs):
    if mode == "r":
        # Simulate another process replacing the cache after the first hash.
        # Keep every semantic payload byte identical; alter the outer container.
        mutation_path = pathlib.Path(path.name) if hasattr(path, "read") else path
        with original_zip(mutation_path, "a") as archive:
            archive.comment = b"outer bytes changed between preparation reads"
    return original_zip(path, mode, *args, **kwargs)


try:
    suite.PREPARE.zipfile.ZipFile = replace_outer_encoding
    try:
        receipt, evidence = fixture.prepare()
        actual_cache_sha256 = hashlib.sha256(fixture.cache.read_bytes()).hexdigest()
        probe = {
            "prepare_returned_candidate": True,
            "runtime_accepted": evidence["runtime_accepted"],
            "recorded_cache_sha256": evidence["cache_sha256"],
            "actual_cache_sha256": actual_cache_sha256,
            "outer_hash_evidence_matches_final_file": evidence["cache_sha256"] == actual_cache_sha256,
            "semantic_payloads_changed": False,
        }
    except ValueError as error:
        if "Cache changed" not in str(error):
            raise
        probe = {
            "prepare_returned_candidate": False,
            "runtime_accepted": False,
            "concurrent_mutation_rejected": True,
            "error": str(error),
            "semantic_payloads_changed": False,
        }
finally:
    suite.PREPARE.zipfile.ZipFile = original_zip
    fixture.doCleanups()

command = [sys.executable, str(suite_path)]
result = subprocess.run(command, cwd=root, capture_output=True, text=True,
                        env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"})
paths = [
    "crates/kerml-semantics/src/trusted_publication.rs",
    "crates/kerml-text/src/sysml/publication_restore.rs",
    "crates/sysml-semantics/src/context.rs",
    "tools/runtime-recovery/prepare_systems_transport.py",
    "tools/runtime-recovery/compare_systems_contract.py",
    "docs/runtime-rematerialization.md",
    "standards/sysml-accepted-publication.json",
    "standards/sysml-standard-bindings.json",
]
record = {
    "format": "agq-independent-runtime-transport-review/1",
    "runtime_accepted": False,
    "reviewed_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
    "cwd": str(root),
    "source_sha256": {p: hashlib.sha256((root / p).read_bytes()).hexdigest() for p in paths},
    "command": command,
    "exit_code": result.returncode,
    "output": result.stdout + result.stderr,
    "preparer_outer_evidence_race_probe": probe,
    "elapsed_seconds": round(time.time() - started, 3),
}
with output.open("x", encoding="utf-8", newline="\n") as stream:
    json.dump(record, stream, indent=2)
    stream.write("\n")
print(json.dumps(record, indent=2))
raise SystemExit(result.returncode)
