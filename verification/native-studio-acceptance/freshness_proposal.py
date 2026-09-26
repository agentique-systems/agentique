"""Emit a reviewed freshness patch only after explicit, hash-pinned proof checks.

Never changes a publication ledger or runs a proof command. Local/CI receipts
are reviewed evidence, not a new source of semantic or publication authority.
"""
import argparse
import copy
import difflib
import hashlib
import json
from pathlib import Path
import re
import subprocess

MANIFEST = "standards/sysml-publication-inputs.json"
REVIEW = Path(__file__).with_name("freshness-review-snapshot.json")
REVIEW_SHA256 = "d1eed39c21c47ed5d2b47c5ca6faa44393329aabbb4094a9a9d640b76b14fe7f"
BUNDLE_SHA256 = "37edf34cc0220ecdded8e0162f3fc1955ee2f3849e6dccf7ba373833ade3b026"
BUNDLE_ID = "633ea89eb39f8a9e301f2bd5199994455cdf28c8d373fdbd69be30a1402ebcf4"
TEST_NAME = "create_part_command_matches_full_self_model_reconstruction"
REVIEWED_TEST_PATH = "crates/kerml-semantics/tests/unit/closed_query_audit.rs"
REVIEWED_TEST_BEFORE = "43864451aa3925cc0a8a888417b81b20dc85839fccad5356aa3fd8181c2e1cfc"
REVIEWED_TEST_AFTER = "457b57ba88bf32921006bd61d29d12e78658093bcc6f1f3d88c1269ae6ef2534"
REVIEWED_TEST_RECEIPT = "verification/native-studio-alpha/checks/acceptance-closed-audit-adversarial-tests.json"
REVIEWED_TEST_RECEIPT_SHA256 = "abe245d4f36d405e92f9b9b1ed56113fc405136b9b72d9f53516a9d654b59915"
REVIEWED_TEST_OUTPUT = "verification/native-studio-alpha/checks/acceptance-closed-audit-adversarial-tests.txt"
REVIEWED_TEST_OUTPUT_SHA256 = "43502c2357db190a615356e20f0536bfab99e82ecb3184885e0b80d4b682b1d4"
SOURCE_PATHS = [
    "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "standards", "models/agentique",
    *["crates/" + name for name in (
        "kernel", "kerml", "kerml-semantics", "kerml-syntax", "kerml-text", "sysml",
        "sysml-semantics", "standard-libraries", "runtime-publications", "modeling-workspace",
        "modeling-repository", "modeling-service", "modeling-agent", "modeling-view")],
    "adapters/modeling-sqlite", "tools/verify-runtime-distribution.py",
]
CAPTURE = r"""
import fs from 'node:fs';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
const root = process.argv[2];
const {capturePublicationInputs} = await import(pathToFileURL(path.join(root, 'tools/sysml-publication-stale.mjs')));
const {verifyLanguageLockCompatibility} = await import(pathToFileURL(path.join(root, 'tools/sysml-lock-compatibility.mjs')));
const expected = JSON.parse(fs.readFileSync(path.join(root, 'standards/sysml-publication-inputs.json')));
const actual = capturePublicationInputs(root);
let lockCompatibility = null;
if (actual.inputs['Cargo.lock'] !== expected.inputs['Cargo.lock']) {
  lockCompatibility = verifyLanguageLockCompatibility(root, expected.inputs['Cargo.lock'],
    ['kernel','kerml','kerml-semantics','kerml-syntax','kerml-text','sysml','sysml-semantics','standard-libraries'].map(n => `agq-${n}`));
  actual.inputs['Cargo.lock'] = expected.inputs['Cargo.lock'];
}
console.log(JSON.stringify({actual, lockCompatibility}));
"""


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def is_hash(value):
    return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f"Duplicate JSON key: {key}")
        result[key] = value
    return result


def decode(data):
    return json.loads(data.decode("utf-8-sig"), object_pairs_hook=unique_object)


def read_review():
    data = REVIEW.read_text(encoding="utf-8").encode()
    require(digest(data) == REVIEW_SHA256, "Reviewed source snapshot changed; independent review required")
    review = decode(data)
    require(len(review["changes"]) == 23, "Unexpected reviewed input population")
    return review


def propose(expected, actual, review):
    """Apply only this frozen reviewed scope, then compare the entire capture."""
    proposed = copy.deepcopy(expected)
    for change in review["changes"]:
        file, old, new = change["file"], change["before"], change["after"]
        require(is_hash(new), f"Invalid reviewed digest: {file}")
        if change["kind"] == "add":
            require(old is None and file not in expected["inputs"], f"Required new input is already present: {file}")
        else:
            require(change["kind"] == "update" and expected["inputs"].get(file) == old,
                    f"Existing input differs from reviewed ledger: {file}")
        require(actual["inputs"].get(file) == new, f"Source differs from reviewed bytes: {file}")
        proposed["inputs"][file] = new
    proposed["inputs"] = dict(sorted(proposed["inputs"].items()))
    require(proposed == actual, "Unreviewed source population, input, receipt, binding or identity change")
    return proposed


class Artifacts:
    def __init__(self, root):
        self.root = root.resolve(strict=True)
        self.files = {}

    def bytes(self, reference):
        require(isinstance(reference, dict) and set(reference) == {"path", "sha256"}, "Expected explicit artifact path and SHA-256")
        require(is_hash(reference["sha256"]), "Invalid artifact SHA-256")
        relative = Path(reference["path"])
        require(not relative.is_absolute(), "Artifact path must be relative to the proof set")
        file = (self.root / relative).resolve(strict=True)
        require(file.is_relative_to(self.root) and file.is_file(), "Artifact escapes the reviewed proof directory")
        data = file.read_bytes()
        require(digest(data) == reference["sha256"], f"Artifact hash mismatch: {relative}")
        self.files[str(file)] = reference["sha256"]
        return data

    def json(self, reference):
        return decode(self.bytes(reference))

    def unchanged(self):
        for file, expected in self.files.items():
            require(digest(Path(file).read_bytes()) == expected, f"Proof changed during verification: {file}")


def reviewed_test_change(root, commit, file):
    """One reviewed cfg(test) addition; no pattern-based test/source exclusion."""
    require(file == REVIEWED_TEST_PATH, f"Unreviewed proof source change: {file}")
    before = subprocess.check_output(["git", "show", commit + ":" + file], cwd=root)
    after = (root / file).read_text(encoding="utf-8").encode()
    require(digest(before) == REVIEWED_TEST_BEFORE and digest(after) == REVIEWED_TEST_AFTER,
            "Test-only exception differs from its exact reviewed before/after bytes")
    receipt_bytes = (root / REVIEWED_TEST_RECEIPT).read_text(encoding="utf-8").encode()
    output = (root / REVIEWED_TEST_OUTPUT).read_text(encoding="utf-8").encode()
    require(digest(receipt_bytes) == REVIEWED_TEST_RECEIPT_SHA256
            and digest(output) == REVIEWED_TEST_OUTPUT_SHA256, "Reviewed test exception evidence changed")
    receipt = decode(receipt_bytes)
    require(zero_exit(receipt.get("exit_code"))
            and receipt.get("command") == ["cargo", "test", "--locked", "--offline", "-p",
                "agq-kerml-semantics", "--lib", "closed_query_audit", "--", "--nocapture"]
            and receipt.get("output") == REVIEWED_TEST_OUTPUT
            and receipt.get("output_sha256") == REVIEWED_TEST_OUTPUT_SHA256
            and b"8 passed; 0 failed; 0 ignored" in output, "Reviewed test exception gate did not pass")
    return {"file": file, "before": REVIEWED_TEST_BEFORE, "after": REVIEWED_TEST_AFTER,
            "review_commit": "69287bdb9e0ad838d6355d981bf5b9ce7131edac",
            "receipt_normalized_sha256": REVIEWED_TEST_RECEIPT_SHA256,
            "output_normalized_sha256": REVIEWED_TEST_OUTPUT_SHA256}


def sources_match(root, commit):
    require(isinstance(commit, str) and re.fullmatch(r"[0-9a-f]{40}", commit), "Proof needs an exact source commit")
    subprocess.run(["git", "cat-file", "-e", commit + "^{commit}"], cwd=root, check=True, capture_output=True)
    changed = subprocess.run(["git", "diff", "--name-only", "-z", commit, "--", *SOURCE_PATHS],
                             cwd=root, check=True, capture_output=True, text=True).stdout
    changes = [file for file in changed.split("\0") if file]
    require(all(file == REVIEWED_TEST_PATH for file in changes),
            f"Proof source differs from current semantic/runtime inputs: {changes}")
    exceptions = [reviewed_test_change(root, commit, file) for file in changes]
    untracked = subprocess.run(["git", "ls-files", "--others", "--exclude-standard", "--", *SOURCE_PATHS],
                               cwd=root, check=True, capture_output=True, text=True).stdout.strip()
    require(not untracked, f"Untracked semantic/runtime inputs cannot use an earlier proof: {untracked}")
    return exceptions


def zero_exit(value):
    return type(value) is int and value == 0


def verify_oracle(artifacts, refs, source_check):
    require(set(refs) == {"build", "invocation", "result", "command_metrics", "cold_metrics",
                          "command_observations", "cold_observations"}, "Incomplete or unsupported oracle proof set")
    values = {name: artifacts.json(ref) for name, ref in refs.items()}
    build, invocation, result = (values[name] for name in ("build", "invocation", "result"))
    require(build.get("format") == "agentique-native-acceptance-build/1" and build.get("component") == "oracle"
            and build.get("outcome") == "passed", "Oracle requires a successful exact executable build")
    test_exceptions = source_check(build["source_commit"]) or []
    require(build.get("commands") and all(zero_exit(item.get("exit_code")) for item in build["commands"])
            and any("build" in item.get("command", []) for item in build["commands"]), "Oracle build commands did not all pass")
    require(invocation.get("source_commit") == build["source_commit"] and invocation.get("outcome") == "passed"
            and zero_exit(invocation.get("exit_code")), "Oracle execution did not pass on the built source")
    binaries = build.get("binaries", [])
    require(len(binaries) == 1 and is_hash(binaries[0].get("sha256"))
            and invocation.get("executable_sha256") == binaries[0]["sha256"], "Oracle executable identity mismatch")
    harness = build.get("source_files", {}).get("crates/modeling-agent/tests/create_part_performance.rs")
    require(is_hash(harness) and invocation.get("oracle_harness_sha256") == harness, "Oracle harness identity mismatch")
    require(invocation.get("runtime_sha256") == BUNDLE_SHA256, "Oracle used a different runtime package")
    command = invocation.get("command", [])
    require(isinstance(command, list) and all(token in command for token in
            [TEST_NAME, "--exact", "--ignored", "--test-threads=1"]), "Oracle test command is missing required execution scope")
    require(result.get("format") == "agentique-create-part-performance/4"
            and result.get("exact_equivalence") is True and result.get("independent_cold_mount_observed") is True,
            "Exact independent cold equivalence did not pass")
    for label, work in [("command_metrics", "command_work"), ("cold_metrics", "full_work")]:
        require(values[label].get("validated") is True and values[label].get(work, {}).get("semantic_cache_used") is False,
                f"{label} did not independently validate without semantic cache reuse")
    require(result.get("validated") is True, "Final result lacks validation")
    require(result.get("owner_and_ancestor_identity_preserved") is True
            and result.get("same_authenticated_mount_observed") is True, "Command identity/mount gate is missing")
    require(values["command_metrics"].get("negative_reuse_cases", 0) >= 16, "Malformed reuse rejection gate is incomplete")
    left, right = values["command_observations"], values["cold_observations"]
    require(isinstance(left, dict) and left == right and left, "Exact oracle observation maps differ or are empty")
    require(all(is_hash(value) for value in left.values()), "Malformed oracle observation digest")
    require(set(["source-checkpoint", "semantic-fingerprint", "semantic-closure", "diagnostics", "strict-audit",
                 "strict-audit-subjects"]).issubset(left), "Required full semantic observations are missing")
    for prefix in ["element/", "occurrence/", "query/", "reference/"]:
        require(any(key.startswith(prefix) for key in left), f"Oracle observation family is absent: {prefix}")
    require(result.get("canonical_elements") == sum(key.startswith("element/") for key in left),
            "Canonical observation population differs from the model")
    require(values["cold_metrics"]["full_work"].get("documents_reparsed") == result.get("authored_documents")
            and result.get("authored_documents", 0) > 0, "Cold oracle did not reparse the complete authored source set")
    require(type(result.get("exact_observation_count")) is int and result["exact_observation_count"] == len(left),
            "Oracle observation count does not match retained maps")
    for key, value in values["command_metrics"].items():
        if key != "validated":
            require(result.get(key) == value, f"Final result differs from command metrics: {key}")
    for key, value in values["cold_metrics"].items():
        require(result.get(key) == value, f"Final result differs from cold metrics: {key}")
    return {"source_commit": build["source_commit"], "executable_sha256": binaries[0]["sha256"],
            "observations": len(left), "reviewed_test_exceptions": test_exceptions}


def verify_runtime(artifacts, refs, review, source_check):
    require(set(refs) == {"transport", "qualification"}, "Incomplete or unsupported runtime proof set")
    transport = artifacts.json(refs["transport"])
    record = artifacts.json(refs["qualification"])
    test_exceptions = source_check(transport["source_commit"]) or []
    require(transport.get("sha256") == BUNDLE_SHA256, "Runtime transport differs from accepted package")
    require(record.get("format") == "agq-runtime-distribution-qualification/1" and record.get("passed") is True,
            "Ordinary runtime verify/install qualification did not pass")
    require(record.get("expected_sha256") == record.get("bundle_sha256") == BUNDLE_SHA256
            and record.get("bundle_identity") == BUNDLE_ID and is_hash(record.get("installer_sha256")),
            "Runtime identity or installer binding mismatch")
    commands = record.get("commands", [])
    require(len(commands) == 3, "Runtime verify/install/status sequence is incomplete")
    directory = Path(refs["qualification"]["path"]).parent
    values = {}
    for operation, command in zip(["verify", "install", "status"], commands):
        require(zero_exit(command.get("exit_code")) and operation in command.get("command", [])
                and command["command"][0] == record.get("installer"), f"Ordinary runtime {operation} did not pass")
        require(command.get("stdout") == f"{operation}.stdout.json" and command.get("stderr") == f"{operation}.stderr.txt",
                "Unexpected runtime output binding")
        values[operation] = artifacts.json({"path": str(directory / command["stdout"]), "sha256": command["stdout_sha256"]})
        artifacts.bytes({"path": str(directory / command["stderr"]), "sha256": command["stderr_sha256"]})
    installed = values["install"]
    manifest = installed["manifest"]
    require(installed.get("directory") == record.get("installed_directory")
            and manifest.get("format") == "agq-accepted-publication-bundle/1"
            and manifest.get("identity") == BUNDLE_ID, "Installed runtime manifest or directory mismatch")
    expected = review["accepted_identity"]
    for family, profile, publication in [
        ("kerml", "agentique-kerml-1.0-operational/9", expected["accepted_kerml_digest"]),
        ("systems", "agentique-sysml-2.0-operational/3", expected["publication_digest"]),
    ]:
        asset = manifest[family]
        require(asset.get("file") == family + ".cache" and asset.get("bytes", 0) > 0 and is_hash(asset.get("sha256"))
                and asset.get("publication", {}).get("profile") == profile
                and asset["publication"].get("publication_digest") == publication, f"Installed {family} authority mismatch")
    for timings in [values["verify"], installed.get("timings", {})]:
        require(all(type(timings.get(field)) in (int, float) and timings[field] >= 0 for field in
                    ["transport_verify_ms", "sources_verify_ms", "kerml_restore_ms", "systems_restore_ms"]),
                "Runtime output is not an ordinary authentication result")
    return {"source_commit": transport["source_commit"], "installer_sha256": record["installer_sha256"],
            "bundle_sha256": BUNDLE_SHA256, "reviewed_test_exceptions": test_exceptions}


def verify_proofs(path, expected_hash, root, review):
    artifacts = Artifacts(path.parent)
    proof = artifacts.json({"path": path.name, "sha256": expected_hash})
    require(set(proof) == {"format", "reviewer", "review_note", "oracle", "runtime"}
            and proof["format"] == "agentique-alpha-freshness-proof-set/1"
            and isinstance(proof["reviewer"], str) and proof["reviewer"].strip()
            and isinstance(proof["review_note"], str) and proof["review_note"].strip(), "Explicit independent proof review is required")
    check = lambda commit: sources_match(root, commit)
    summary = {
        "oracle": verify_oracle(artifacts, proof["oracle"], check),
        "runtime": verify_runtime(artifacts, proof["runtime"], review, check),
        "reviewer": proof["reviewer"], "review_note": proof["review_note"], "proof_set_sha256": expected_hash,
    }
    return artifacts, summary


def capture(root):
    result = subprocess.run(["node", "--input-type=module", "-", str(root)], input=CAPTURE,
                            text=True, encoding="utf-8", capture_output=True, check=True)
    return decode(result.stdout.encode())


def prepare(root, proof_set, proof_hash, output):
    root = root.resolve(strict=True)
    require(not output.exists(), "Refusing to overwrite an existing proposal directory")
    review = read_review()
    original_bytes = (root / MANIFEST).read_bytes()
    original = original_bytes.decode("utf-8").replace("\r\n", "\n")
    require(digest(original.encode()) == review["manifest_normalized_sha256"], "Ledger differs from reviewed original")
    expected = decode(original.encode())
    observed = capture(root)
    proposed = propose(expected, observed["actual"], review)
    artifacts, proof = verify_proofs(proof_set.resolve(strict=True), proof_hash, root, review)
    # The ledger uses this exact stable layout; refuse a broad formatting rewrite.
    require(original == json.dumps(expected, indent=2) + "\n", "Unexpected ledger formatting")
    updated = json.dumps(proposed, indent=2) + "\n"
    require(capture(root) == observed and (root / MANIFEST).read_bytes() == original_bytes,
            "Source or ledger changed during proof review")
    sources_match(root, proof["oracle"]["source_commit"])
    sources_match(root, proof["runtime"]["source_commit"])
    artifacts.unchanged()
    patch = "".join(difflib.unified_diff(original.splitlines(keepends=True), updated.splitlines(keepends=True),
                                       fromfile="a/" + MANIFEST, tofile="b/" + MANIFEST))
    require(patch, "No reviewed changes remain")
    report = {"format": "agentique-alpha-freshness-proposal/1", "status": "review-patch-only",
              "grants_publication_authority": False, "review_snapshot_sha256": REVIEW_SHA256,
              "changes": review["changes"], "proof": proof, "lock_compatibility": observed["lockCompatibility"],
              "before_normalized_sha256": digest(original.encode()), "proposed_normalized_sha256": digest(updated.encode()),
              "publication_identity": expected["publication_identity"], "all_other_fields_unchanged": True}
    output.mkdir(parents=True, exist_ok=False)
    (output / "proposal.patch").write_text(patch, encoding="utf-8", newline="\n")
    (output / "proposal.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(json.dumps({"proposal_directory": str(output), "changed_entries": len(review["changes"])}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--proof-set", type=Path, required=True)
    parser.add_argument("--proof-set-sha256", required=True, help="Independently reviewed proof-set file SHA-256")
    parser.add_argument("--output", type=Path, required=True, help="New proposal directory; never modifies the ledger")
    args = parser.parse_args()
    prepare(args.root, args.proof_set, args.proof_set_sha256, args.output)
