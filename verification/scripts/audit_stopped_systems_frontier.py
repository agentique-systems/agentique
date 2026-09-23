"""Authenticate a stopped full-run frontier without resuming or issuing authority."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import subprocess
import time
import zipfile

from frontier_artifact_evidence import StateReader, content_hash, graph_evidence, pin, require


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def audit(root, journal_path, expected_pin):
    started = time.monotonic()
    journal_raw = journal_path.read_bytes()
    require(sha(journal_raw) == expected_pin, "independent journal pin")
    journal = json.loads(journal_raw)
    require(journal["format"] == "agq-unaccepted-publication-frontier/1", "unaccepted format")
    executable_record = json.loads((root / "verification/summaries/final-language-acceptance/publication-executable-handoff.json").read_bytes())
    run_path = root / "verification/generated/overnight-convergence/foundation-systems-full/result.json"
    run_raw = run_path.read_bytes()
    run = json.loads(run_raw)
    require(run["exit_code"] == 124 and run["safety_stop"] == "wall_time", "stopped full attempt")
    executable = root / run["command"][0]
    with executable.open("rb") as source:
        require(hashlib.file_digest(source, "sha256").hexdigest() == executable_record["sha256"], "original executable pin")
    require(executable.stat().st_size == executable_record["bytes"], "executable size")
    build_commit = executable_record["source_commit"]

    def original(path):
        return subprocess.check_output(["git", "show", f"{build_commit}:{path}"], cwd=root)

    manifest = json.loads(original("standards/normative/sysml-2.0/library-set.json"))
    kerml_raw = original("standards/kerml-accepted-publication.json")
    kerml = json.loads(kerml_raw)
    accepted = kerml["complete_overlay"]["identity"]
    require(kerml["status"] == "accepted" and accepted["operational_profile"] == "agentique-kerml-1.0-operational/9", "accepted KerML pin")
    systems, = [artifact for artifact in manifest["artifacts"] if artifact["specification"] == "SysML"]
    source_digest = hashlib.sha256(b"agq-systems-frontier-source/1\0")
    source_digest.update(bytes(accepted["semantic_digest"]))
    source_digest.update(b"\0")  # Full invocation: paths=None, never scoped audit.

    def input_bytes(raw):
        source_digest.update(len(raw).to_bytes(8, "little"))
        source_digest.update(raw)

    for value in (manifest["id"], "agentique-sysml-2.0-operational/2"):
        input_bytes(value.encode())
    with (root / systems["path"]).open("rb") as source:
        require(hashlib.file_digest(source, "sha256").hexdigest() == systems["sha256"], "original Systems KPAR")
        source.seek(0)
        with zipfile.ZipFile(source) as archive:
            documents = sorted((entry for entry in systems["entries"] if entry["path"].endswith(".sysml")), key=lambda entry: entry["path"])
            require(len(documents) == 21, "full source population")
            for entry in documents:
                raw = archive.read(entry["path"])
                require(len(raw) == entry["bytes"] and sha(raw) == entry["sha256"], "original source bytes")
                raw.decode("utf-8")
                input_bytes(entry["path"].encode())
                input_bytes(raw)
    require(source_digest.hexdigest() == pin(journal["source_identity"]), "independently recomputed full source context")
    entry = journal["entries"][-1]
    archive_path = journal_path.parent / f"{pin(entry['archive_sha256'])}.zip"
    with archive_path.open("rb") as source:
        require(hashlib.file_digest(source, "sha256").hexdigest() == pin(entry["archive_sha256"]), "checkpoint ZIP pin")
        source.seek(0)
        with zipfile.ZipFile(source) as archive:
            require(sorted(archive.namelist()) == ["graph.jsonl", "state.json"], "checkpoint inventory")
            with archive.open("state.json") as data:
                reader = StateReader(data)
                state = reader.fields({"round", "converged", "stratum", "model_digest", "context_contract", "counters", "worklist", "status"}, {"certificate": {"receipt"}})
                reader.whitespace()
                require(not reader.peek() and reader.digest.hexdigest() == pin(entry["state_sha256"]), "decoded state pin")
            with archive.open("graph.jsonl") as data:
                graph = graph_evidence(data)
            require(graph["graph_sha256"] == pin(entry["graph_sha256"]), "decoded graph pin")
    receipt = state["certificate"]["receipt"]
    header = graph["header"]["DependentHeader"]
    require(header["format"] == "agq-kernel-dependent-publication-frontier/1", "strict unaccepted frontier format")
    require(header["dependency"] == kerml["complete_overlay"]["graph_sha256"], "accepted KerML graph dependency pin")
    require(receipt["model_digest"] == state["model_digest"] and receipt["context_contract_digest"] == state["context_contract"], "certificate internal graph/context binding")
    require(state["converged"] is True and state["round"] == 28 and state["worklist"] == [], "final converged frontier")
    output_directory = root / "verification/generated/final-language-acceptance/systems-full"
    absent = {name: not (output_directory / name).exists() for name in
              ("report.json", "canonical.publication.zip", "accepted-publication.json", "standard-bindings.json")}
    require(all(absent.values()), "unexpected publication artifact")
    stages_raw = (output_directory / "report.stages.jsonl").read_bytes()
    stage = json.loads(stages_raw.splitlines()[-1])
    require(stage["phase"] == "publication" and stage["stage"] == 27 and stage["completeness"] == "Complete"
            and stage["diagnostics"] == [], "last strict scheduler observation")
    counters = state["counters"]
    for field, value in (("applicable_subject_family_pairs", 26532), ("closed_producer_pairs", 26532),
                         ("incomplete_producer_pairs", 0), ("closed_producer_effects", 452052)):
        require(counters[field] == value, f"final counter {field}")
    log_raw = (root / run["output"] / "output.log").read_bytes()
    require(sha(log_raw) == run["output_sha256"] and expected_pin.encode() in log_raw, "watchdog log and externally recorded pin")
    statuses = Counter(value[0] for value in state["status"].values())
    require(set(statuses) == {"Complete"}, "scheduler status population")
    graph = {key: value for key, value in graph.items() if key != "header"}
    graph.update(format=header["format"], descriptor_registry_digest=pin(header["registry"]),
                 accepted_dependency_graph_sha256=pin(header["dependency"]))
    return dict(format="agq-unaccepted-converged-systems-frontier-audit/1", artifact_authentication_passed=True,
        accepted_publication=False, grants_publication_authority=False, resumed=False,
        journal=str(journal_path), journal_sha256=expected_pin,
        invocation=len(journal["entries"]) - 1, round=state["round"], stratum=state["stratum"], converged=state["converged"],
        status_population=dict(statuses), pending_worklist=0,
        applicable_pairs=counters["applicable_subject_family_pairs"], closed_pairs=counters["closed_producer_pairs"],
        incomplete_pairs=counters["incomplete_producer_pairs"], closed_requirements=counters["closed_producer_effects"],
        certificate_subjects=len(receipt["subjects"]), producer_families=receipt["families"],
        certificate_receipt_digest=content_hash(receipt),
        model_digest=pin(state["model_digest"]), context_contract_digest=pin(state["context_contract"]),
        producer_registry_digest=pin(receipt["registry_digest"]), certificate_digest=pin(receipt["digest"]),
        source_identity=source_digest.hexdigest(), source_identity_recomputed_from_original_bytes=True,
        systems_kpar_sha256=systems["sha256"], source_content_set=manifest["id"],
        accepted_kerml_digest=pin(accepted["semantic_digest"]), accepted_kerml_receipt_source_sha256=sha(kerml_raw),
        executable_sha256=executable_record["sha256"], executable_build_commit=build_commit,
        source_identity_implementation_sha256=sha(original("crates/kerml-text/src/sysml.rs")),
        publication_implementation_sha256=sha(original("crates/kerml-text/src/sysml/publication.rs")),
        scheduler_implementation_sha256=sha(original("crates/kerml-semantics/src/producer_worklist.rs")),
        archive_sha256=pin(entry["archive_sha256"]), decoded_state_sha256=pin(entry["state_sha256"]), graph_evidence=graph,
        watchdog=dict(exit_code=run["exit_code"], safety_stop=run["safety_stop"], duration_seconds=run["duration_seconds"],
                      result_sha256=sha(run_raw), output_sha256=sha(log_raw)),
        final_scheduler_stage=dict(stage=stage["stage"], phase=stage["phase"], completeness=stage["completeness"],
                                   elapsed_seconds=stage["elapsed_seconds"], stages_sha256=sha(stages_raw)),
        publication_artifacts_absent=absent, audit_elapsed_seconds=round(time.monotonic() - started, 3),
        limitations="Authenticates unaccepted stored scheduler evidence only. This verifier did not run Rust restore or publication audits. Final strict references, capability/provenance audit and accepted bindings/facade/receipt/cache remain unproven; the precise interrupted post-checkpoint audit step is not recorded.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, required=True)
    parser.add_argument("--journal", type=Path, required=True)
    parser.add_argument("--journal-sha256", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = audit(args.repository, args.journal, args.journal_sha256)
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
