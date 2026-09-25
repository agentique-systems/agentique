"""Compare downloaded draft assets with independently authenticated local bytes."""
from pathlib import Path
import datetime
import hashlib
import json
import zipfile

evidence = Path(__file__).resolve().parent
generated = evidence.parents[2] / "verification/generated/native-studio-alpha"
local = generated / "runtime-package"
remote = generated / "runtime-remote-download-2026-09-25"
expected_names = {
    "accepted-runtime.agq-runtime", "runtime-manifest.json",
    "accepted-runtime.agq-runtime.sha256", "runtime-verification.json",
}
package_sha = "37edf34cc0220ecdded8e0162f3fc1955ee2f3849e6dccf7ba373833ade3b026"


def identity(path):
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
        size = stream.seek(0, 2)
    return {"bytes": size, "sha256": digest}


for name in ("remote-draft-metadata", "remote-draft-download"):
    record = json.loads((evidence / f"{name}.json").read_text(encoding="utf-8"))
    assert record["exit_code"] == 0, name
    assert identity(evidence / f"{name}.log")["sha256"] == record["output_sha256"]
metadata = json.loads((evidence / "remote-draft-metadata.log").read_text(encoding="utf-8"))
assert metadata["isDraft"] is True and metadata["publishedAt"] is None
assert metadata["tagName"] == "runtime-kerml-v9-sysml-v3-bundle1"
assert metadata["targetCommitish"] == "c00de91f348bc59934c721fd75b17cbd09217163"
assert metadata["body"] == (local / "runtime-release-notes.md").read_text(encoding="utf-8")
assets = {asset["name"]: asset for asset in metadata["assets"]}
assert len(assets) == len(metadata["assets"]) and set(assets) == expected_names
assert {path.name for path in remote.iterdir()} == expected_names
results = {}
for name in sorted(expected_names):
    actual = identity(remote / name)
    original = identity(local / name)
    assert actual == original, name
    assert actual["bytes"] == assets[name]["size"], name
    assert "sha256:" + actual["sha256"] == assets[name]["digest"], name
    assert assets[name]["state"] == "uploaded"
    results[name] = {**actual, "matches_authenticated_local_bytes": True,
                     "matches_github_digest_and_size": True,
                     "asset_api_url": assets[name]["apiUrl"]}
assert results["accepted-runtime.agq-runtime"]["sha256"] == package_sha
manifest_bytes = (remote / "runtime-manifest.json").read_bytes()
manifest = json.loads(manifest_bytes)
with zipfile.ZipFile(remote / "accepted-runtime.agq-runtime") as archive:
    assert sorted(archive.namelist()) == ["kerml.cache", "manifest.json", "systems.cache"]
    assert 0 < archive.getinfo("manifest.json").file_size <= 64 * 1024
    assert archive.read("manifest.json") == manifest_bytes
    for key in ("kerml", "systems"):
        assert archive.getinfo(manifest[key]["file"]).file_size == manifest[key]["bytes"]
assert (remote / "accepted-runtime.agq-runtime.sha256").read_text().strip() == (
    package_sha + "  accepted-runtime.agq-runtime")
verification = json.loads((remote / "runtime-verification.json").read_bytes())
assert verification["ordinary_facades_authenticated"] is True
assert verification["sha256"] == package_sha
assert verification["bundle_identity"] == manifest["identity"]
for name, checked in results.items():
    assert identity(remote / name) == {"bytes": checked["bytes"], "sha256": checked["sha256"]}, name
result = {
    "format": "agq-runtime-remote-distribution-check/1",
    "checked_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "release_url": metadata["url"], "requested_tag": metadata["tagName"],
    "target_commit": metadata["targetCommitish"], "is_draft": True,
    "published": False, "download_directory": str(remote),
    "assets": results, "release_body_matches_reviewed_notes": True,
    "embedded_manifest_matches_separate_asset": True,
    "bundle_identity": manifest["identity"],
    "downloaded_bytes_equal_locally_authenticated_bundle": True,
    "facades_rerun_on_downloaded_copy": False,
    "deferred_gate": "Run ordinary facade verify serially after native real journey releases runtime; no standards consumer invoked by this check.",
}
(evidence / "remote-distribution-result.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
print(json.dumps(result, indent=2))
