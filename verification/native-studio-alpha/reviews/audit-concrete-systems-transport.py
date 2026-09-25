"""Independent concrete artifact audit; does not register or authenticate a runtime."""
import datetime
import hashlib
import json
import pathlib
import subprocess
import sys
import time
import uuid
import zipfile

root = pathlib.Path(sys.argv[1]).resolve()
destination = pathlib.Path(sys.argv[2]).resolve()
started = time.monotonic()
generated = root / "verification/generated/native-studio-alpha"
finalized = generated / "rematerialized-systems/finalized"
review = generated / "systems-transport-review"
runtime = root / "verification/native-studio-alpha/runtime"


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def load(path):
    return json.loads(path.read_bytes())


def exact(left, right):
    return json.dumps(left, sort_keys=True) == json.dumps(right, sort_keys=True)


authority_path = root / "standards/sysml-accepted-publication.json"
binding_path = root / "standards/sysml-standard-bindings.json"
authority = load(authority_path)
candidate = load(finalized / "accepted-publication.json")
transport = load(review / "transport.json")
content = load(review / "content.json")
report = load(finalized / "report.json")
artifact_gate = load(generated / "systems-independent-artifact-gate.json")
contract_gate = load(generated / "systems-existing-contract-comparison.json")
cache = finalized / "canonical.publication.zip"
checks = {}


def require(name, condition):
    checks[name] = bool(condition)
    if not condition:
        raise RuntimeError(f"Concrete transport review failed: {name}")


require("unchanged_original_semantic_receipt", sha(authority_path) ==
        "becc3cf991e69115dae905957d74292774164e3f5707eecefb06d8eb60e0269a")
require("unchanged_original_bindings", sha(binding_path) ==
        "dbd794bba9ebdfdea5af433945f6bb5d0475f0f05ad1d229cd004f5b57045789")
require("full_non_transport_contract_equal", exact(
    {k: v for k, v in authority.items() if k != "entries"},
    {k: v for k, v in candidate.items() if k != "entries"}))
require("complete_bindings_byte_equal", binding_path.read_bytes() ==
        (finalized / "standard-bindings.json").read_bytes())
require("transport_schema_exact", set(transport) == {
    "format", "transport_id", "publication_catalogue_id", "semantic_authority_sha256",
    "identity", "entries"})
require("transport_authority_exact", transport["format"] == "agq-publication-transport/1"
        and transport["publication_catalogue_id"] == "sysml-systems-operational-v3"
        and bytes(transport["semantic_authority_sha256"]).hex() == sha(authority_path)
        and exact(transport["identity"], authority["identity"])
        and transport["transport_id"] == "sysml-systems-operational-v3-rematerialized-2026-09-25")
require("no_candidate_acceptance_claim", content["runtime_accepted"] is False
        and content["ordinary_facade_authentication_required"] is True
        and content["authority_receipt_changed"] is False)
observed = {}
snapshot_revision = None
with cache.open("rb") as stream:
    outer_hash = hashlib.file_digest(stream, "sha256").hexdigest()
    outer_bytes = stream.tell()
    stream.seek(0)
    with zipfile.ZipFile(stream) as archive:
        require("exact_archive_population", sorted(archive.namelist()) ==
                sorted(authority["entries"]) == sorted(candidate["entries"]) ==
                sorted(transport["entries"]) == ["closure.json", "facade.json", "kernel.jsonl"])
        for name in sorted(authority["entries"]):
            count = 0
            digest = hashlib.sha256()
            with archive.open(name) as entry:
                for line in entry:
                    count += len(line)
                    digest.update(line)
                    if name == "kernel.jsonl" and line.startswith(b'{"Snapshot":'):
                        require("single_snapshot_record", snapshot_revision is None)
                        snapshot_revision = json.loads(line)["Snapshot"]["revision"]
            actual = {"bytes": count, "sha256": list(digest.digest())}
            require(f"{name}_actual_matches_generated_and_pin", exact(actual, candidate["entries"][name])
                    and exact(actual, transport["entries"][name]))
            require(f"{name}_original_length", count == authority["entries"][name]["bytes"]
                    == archive.getinfo(name).file_size)
            original_equal = exact(actual, authority["entries"][name])
            require(f"{name}_allowed_transport_boundary", original_equal or name == "kernel.jsonl")
            observed[name] = {"bytes": count, "sha256": digest.hexdigest(),
                              "original_bytes_equal": original_equal}
    stream.seek(0)
    require("same_handle_stable", hashlib.file_digest(stream, "sha256").hexdigest() == outer_hash)
require("retained_path_stable", cache.stat().st_size == outer_bytes and sha(cache) == outer_hash)
require("outer_container_exact", outer_bytes == 134322830 and outer_hash ==
        "02ba47ad99ceaddb853b198a556cbe40351a00361f077099a993f3a226bcc323")
require("content_record_matches_actual_artifact", content["cache_bytes"] == outer_bytes
        and content["cache_sha256"] == outer_hash and exact(content["entries"], observed)
        and content["snapshot_revision"] == snapshot_revision
        and str(uuid.UUID(snapshot_revision)) == snapshot_revision
        and content["generated_receipt_sha256"] == sha(finalized / "accepted-publication.json"))
require("artifact_gate_matches_retained_bytes", artifact_gate["passed"] is True
        and artifact_gate["grants_publication_authority"] is False
        and artifact_gate["trusted_restore_still_required"] is True
        and artifact_gate["cache_sha256"] == outer_hash
        and artifact_gate["report_sha256"] == sha(finalized / "report.json")
        and artifact_gate["receipt_sha256"] == sha(finalized / "accepted-publication.json")
        and artifact_gate["bindings_sha256"] == sha(binding_path))
require("contract_comparison_retains_original_authority", contract_gate["semantic_contract_equal"] is True
        and contract_gate["accepted_bindings_equal"] is True
        and contract_gate["runtime_accepted"] is False
        and contract_gate["accepted_receipt_changed"] is False)
require("producer_report_identity_equal", report["publication_digest"] == authority["identity"]["publication_digest"]
        and report["semantic_digest"] == authority["identity"]["semantic_digest"]
        and report["producer_closure"]["digest"] == authority["identity"]["producer_closure_digest"]
        and report["producer_closure"]["producer_registry_digest"] == authority["identity"]["producer_registry_digest"]
        and report["producer_closure"]["context_contract_digest"] == authority["identity"]["producer_context_contract_digest"])
require("producer_report_gates", report["accepted_bindings"] == 69
        and report["kernel_obligations"] == 0 and report["final_authority_findings"] == 0
        and report["final_capability_findings"] == 0 and report["publication_gate"]["findings"] == []
        and report["producer_closure"]["fully_closed"] is True
        and report["publication_producers"] == {"completeness": "Complete", "converged": True}
        and report["mandatory_references"]["counts"]["complete"] == 1327
        and report["candidate_cache_restored"] is True)
receipts = {}
for name in ["historical-systems-finalizer", "systems-independent-artifact-gate",
             "systems-existing-contract-comparison", "systems-transport-preparation"]:
    command_record = load(runtime / f"{name}.json")
    require(f"{name}_command_and_log", command_record["exit_code"] == 0
            and sha(pathlib.Path(command_record["raw_output"])) == command_record["output_sha256"])
    receipts[name] = {"receipt_sha256": sha(runtime / f"{name}.json"),
                      "source_commit": command_record["source_commit"],
                      "exit_code": command_record["exit_code"],
                      "output_sha256": command_record["output_sha256"]}
historical = root.parent / "agentique-alpha-rematerialize-systems"
require("historical_producer_source_still_exact", subprocess.check_output(
    ["git", "rev-parse", "HEAD"], cwd=historical, text=True).strip() ==
    "4ac9b8e58695ad837fed6643b0cedfac05d15635"
    and not subprocess.check_output(["git", "diff", "HEAD"], cwd=historical))
producer = load(runtime / "rematerialization-producer.json")
binary = root.parent / "agentique-alpha-rematerialize-kerml/target/release/examples/sysml_systems_publication.exe"
require("retained_producer_binary_exact", sha(binary) == producer["systems_binary_sha256"])
journal = load(runtime / "rematerialized-systems-journal-pin.json")
require("finalizer_journal_exact", sha(pathlib.Path(journal["journal"])) == journal["sha256"] ==
        "c0e02c3e53c40a7a8dee4de4e2dd7b46e3d34367976e12cc07584e8695e092d1")
registered_pin = root / "standards/runtime-transports/sysml-v3-rematerialized-2026-09-25.json"
require("registered_pin_is_exact_reviewed_candidate", registered_pin.read_bytes() ==
        (review / "transport.json").read_bytes())
for retained, actual in {
    "systems-finalized-report.json": finalized / "report.json",
    "systems-generated-bindings.json": finalized / "standard-bindings.json",
    "systems-generated-receipt.json": finalized / "accepted-publication.json",
    "systems-rematerialized-content.json": review / "content.json",
    "systems-existing-contract-result.json": generated / "systems-existing-contract-comparison.json",
    "systems-independent-artifact-result.json": generated / "systems-independent-artifact-gate.json",
}.items():
    require(f"{retained}_retains_exact_evidence_bytes", (runtime / retained).read_bytes() == actual.read_bytes())
record = {
    "format": "agq-independent-concrete-transport-review/1",
    "finished_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "command": [sys.executable, str(pathlib.Path(__file__).resolve()), *sys.argv[1:]],
    "exit_code": 0,
    "elapsed_seconds": round(time.monotonic() - started, 3),
    "reviewed_source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
    "audit_script_sha256": sha(pathlib.Path(__file__)),
    "eligible_for_finite_transport_registration": True,
    "runtime_accepted": False,
    "ordinary_catalogue_facade_authentication_observed": False,
    "checks": checks,
    "cache": {"path": str(cache), "bytes": outer_bytes, "sha256": outer_hash},
    "entries": observed,
    "snapshot_revision": snapshot_revision,
    "transport_sha256": sha(review / "transport.json"),
    "registered_transport_sha256": sha(registered_pin),
    "catalogue_source_sha256": sha(root / "crates/kerml-semantics/src/trusted_publication.rs"),
    "content_record_sha256": sha(review / "content.json"),
    "source_receipt_sha256": sha(authority_path),
    "source_bindings_sha256": sha(binding_path),
    "producer_binary_sha256": sha(binary),
    "command_receipts": receipts,
}
with destination.open("x", encoding="utf-8", newline="\n") as output:
    json.dump(record, output, indent=2)
    output.write("\n")
print(json.dumps(record, indent=2))
