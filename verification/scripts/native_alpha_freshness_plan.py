"""Prepare a narrowly reviewed freshness diff; never modify publication inputs.

This is source bookkeeping, not semantic acceptance or a publication operation.
Apply its proposed patch only after independent source review and runtime proof.
"""
import argparse
import copy
import difflib
import hashlib
import json
from pathlib import Path
import subprocess

MANIFEST = "standards/sysml-publication-inputs.json"
REVIEWED = {
    "crates/kerml-text/src/source_checkpoint.rs": (
        "eb86d74ead07e68664081b131c3dbd152877fdb12c1374d3ba2175764a57aa0e",
        "8465c80bf7707c540399265d0447f5b0d44dba7730ff4ec3818a6f5e62010368",
    ),
    "crates/kerml-text/src/source_inputs.rs": (
        "b5b9aebdfbb0f5ecafb730fbd68beee8b7200a0eb86d0eebc601b6c3259c1e87",
        "f15171c15dd27c2e61c05c3dfffc426b1714b14cc09e7c8716bbb046d1f30128",
    ),
    "crates/kerml-text/src/sysml/publication.rs": (
        "c3c8c13d7a7a8c0ecbee5042e13b919ab15035b8e32340c0e536d096b481d8ac",
        "682f079a4e2c6c412b897d159f93f3b495f7fd8d69a675aec11f8ae6df9fbef7",
    ),
}
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


def propose(expected, comparable_actual, phase):
    """Require exact reviewed hashes and equality of every other ledger field."""
    if phase not in ("mount", "mount-and-audit"):
        raise ValueError("unknown qualification phase")
    selected = list(REVIEWED)[:1] if phase == "mount" else list(REVIEWED)
    proposed = copy.deepcopy(expected)
    changes = []
    for source in selected:
        old, new = REVIEWED[source]
        previous = expected["inputs"].get(source)
        if previous not in (old, new):
            raise ValueError(f"unreviewed existing fingerprint: {source}")
        if comparable_actual["inputs"].get(source) != new:
            raise ValueError(f"source differs from reviewed patch: {source}")
        proposed["inputs"][source] = new
        if previous != new:
            changes.append({"source": source, "before": previous, "after": new})
    if proposed != comparable_actual:
        raise ValueError("unreviewed input population, source, receipt, binding or identity change")
    return proposed, changes


def prepare(root, phase, output):
    root = root.resolve(strict=True)
    original = (root / MANIFEST).read_text(encoding="utf-8")
    expected = json.loads(original)
    capture = subprocess.run(
        ["node", "--input-type=module", "-", str(root)], input=CAPTURE,
        text=True, encoding="utf-8", capture_output=True, check=True,
    )
    observed = json.loads(capture.stdout)
    proposed, changes = propose(expected, observed["actual"], phase)
    updated = original
    for item in changes:
        before = json.dumps(item["source"]) + ": " + json.dumps(item["before"])
        after = json.dumps(item["source"]) + ": " + json.dumps(item["after"])
        if updated.count(before) != 1:
            raise ValueError("unexpected ledger formatting or duplicate source key")
        updated = updated.replace(before, after, 1)
    if json.loads(updated) != proposed:
        raise ValueError("text proposal differs from independently checked ledger")
    patch = "".join(difflib.unified_diff(
        original.splitlines(keepends=True), updated.splitlines(keepends=True),
        fromfile="a/" + MANIFEST, tofile="b/" + MANIFEST,
    ))
    report = {
        "format": "agentique-native-alpha-freshness-plan/1",
        "status": "proposal-only-runtime-proof-and-independent-review-required",
        "grants_publication_authority": False,
        "root": str(root), "phase": phase, "changes": changes,
        "manifest": MANIFEST,
        "before_normalized_sha256": hashlib.sha256(original.encode()).hexdigest(),
        "proposed_normalized_sha256": hashlib.sha256(updated.encode()).hexdigest(),
        "lock_compatibility": observed["lockCompatibility"],
        "all_other_inventory_and_authority_fields_unchanged": True,
        "publication_identity": expected["publication_identity"],
    }
    # Exclusive creation prevents silently replacing retained review evidence.
    # The only writes are new proposal artifacts in the supplied directory.
    output.mkdir(parents=True, exist_ok=False)
    (output / "proposal.patch").write_text(patch, encoding="utf-8", newline="\n")
    (output / "proposal.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(json.dumps({"proposal_directory": str(output), "changed_entries": len(changes)}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--phase", required=True, choices=("mount", "mount-and-audit"))
    parser.add_argument("--output", required=True, type=Path)
    arguments = parser.parse_args()
    prepare(arguments.root, arguments.phase, arguments.output)
