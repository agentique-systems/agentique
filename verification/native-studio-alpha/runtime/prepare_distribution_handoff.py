"""Retain actual bundle metadata after all three ordinary runtime gates pass."""
import hashlib
import json
from pathlib import Path
import shutil
import zipfile

evidence = Path(__file__).resolve().parent
root = evidence.parents[2]
package = root / "verification/generated/native-studio-alpha/runtime-package/accepted-runtime.agq-runtime"
output = package.parent


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def gate(name):
    record = json.loads((evidence / f"{name}.json").read_text(encoding="utf-8"))
    assert record["exit_code"] == 0, name
    log = evidence / f"{name}.log"
    assert sha(log) == record["output_sha256"], name
    raw = log.read_text(encoding="utf-8")
    assert '"authenticating_kerml"' in raw and '"authenticating_sysml"' in raw, name
    assert '"ready"' in raw, name
    result = json.loads(raw[raw.index("{"):])
    return record, result


installer = json.loads((evidence / "current-runtime-installer.json").read_text(encoding="utf-8"))
assert sha(Path(installer["binary"])) == installer["binary_sha256"]
pack, packed_manifest = gate("runtime-pack")
verify, verify_timings = gate("runtime-verify")
install, installed = gate("runtime-install")
for record in (pack, verify, install):
    assert Path(record["command"][0]) == Path(installer["binary"])

with package.open("rb") as stream:
    digest = hashlib.file_digest(stream, "sha256").hexdigest()
    length = stream.seek(0, 2)
    stream.seek(0)
    with zipfile.ZipFile(stream) as archive:
        assert sorted(archive.namelist()) == ["kerml.cache", "manifest.json", "systems.cache"]
        assert 0 < archive.getinfo("manifest.json").file_size <= 64 * 1024
        manifest_bytes = archive.read("manifest.json")
        manifest = json.loads(manifest_bytes)
        assert manifest == packed_manifest == installed["manifest"]
        for key in ("kerml", "systems"):
            asset = manifest[key]
            assert archive.getinfo(asset["file"]).file_size == asset["bytes"]
            with archive.open(asset["file"]) as entry:
                assert hashlib.file_digest(entry, "sha256").hexdigest() == asset["sha256"]
    stream.seek(0)
    assert hashlib.file_digest(stream, "sha256").hexdigest() == digest

store = Path(installed["directory"])
assert json.loads((store / "manifest.json").read_bytes()) == manifest
for key in ("kerml", "systems"):
    asset = manifest[key]
    assert sha(store / asset["file"]) == asset["sha256"]

summary = {
    "format": "agq-runtime-distribution-verification/1",
    "package": str(package), "bytes": length, "sha256": digest,
    "bundle_identity": manifest["identity"], "installer": installer,
    "ordinary_facades_authenticated": True, "installed_directory": str(store),
    "pack": pack, "verify": verify, "install": install,
    "verify_timings": verify_timings, "install_timings": installed["timings"],
    "remote_distribution_verified": False,
}
(output / "runtime-manifest.json").write_bytes(manifest_bytes)
(output / "accepted-runtime.agq-runtime.sha256").write_text(
    f"{digest}  accepted-runtime.agq-runtime\n", encoding="utf-8", newline="\n")
(output / "runtime-verification.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")

notes = f"""# Agentique accepted runtime: KerML v9 / SysML v3

Draft runtime distribution for maintainers. This is not a Studio release or an
overall Native Studio alpha acceptance claim.

The package restores the existing accepted KerML Operational v9 and SysML
Operational v3 identities. Frozen producer replay and independent comparison
preserved all accepted semantics and bindings. KerML uncompressed payloads are
original-byte exact. Systems retains original facade and closure bytes under a
reviewed, finite versioned graph transport receipt. Original semantic receipts
remain unchanged.

- Bundle identity: `{manifest['identity']}`
- Package SHA-256: `{digest}`
- Package bytes: `{length}`
- Installer build source: `{installer['build_source_commit']}`
- Installer binary SHA-256: `{installer['binary_sha256']}`
- KerML publication: `{manifest['kerml']['publication']['publication_digest']}`
- Systems publication: `{manifest['systems']['publication']['publication_digest']}`

Normal pack, separate verify and normal-store install each authenticated both
language facades, with zero producer replay. Attached manifest and verification
records retain actual asset hashes, accepted receipt identities and command
results. Download and authenticate the uploaded bytes independently before
trusting a remote copy. No release publication is authorized by these records.

From a checkout containing the reviewed transport registration:

```powershell
gh release download runtime-kerml-v9-sysml-v3-bundle1 --repo agentique-systems/agentique --pattern accepted-runtime.agq-runtime --dir .runtime-download
cargo run --release --config profile.release.lto=false --locked --offline -p agq-runtime-publications --bin agq-publications -- verify --bundle .runtime-download/accepted-runtime.agq-runtime
cargo run --release --config profile.release.lto=false --locked --offline -p agq-runtime-publications --bin agq-publications -- install --bundle .runtime-download/accepted-runtime.agq-runtime
cargo run --locked --offline --manifest-path crates/studio-native/Cargo.toml --target-dir target
```

Draft access requires repository push access. For offline installation, transfer
the same package locally and use the install command with its local path. Pinned
dependencies must already be cached for offline compilation.

[Runtime installation and distribution](https://github.com/agentique-systems/agentique/blob/platform/native-studio-alpha/docs/runtime-publication-distribution.md)
"""
(output / "runtime-release-notes.md").write_text(notes, encoding="utf-8", newline="\n")
for name in ("runtime-manifest.json", "runtime-verification.json", "accepted-runtime.agq-runtime.sha256", "runtime-release-notes.md"):
    shutil.copyfile(output / name, evidence / name)
print(json.dumps(summary, indent=2))
