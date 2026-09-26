"""Authenticate one exact package and exercise installation in a fresh isolated store.

This invokes the ordinary runtime CLI; it cannot issue publication acceptance or
publish a GitHub release. Caches never enter the evidence directory.
"""
import argparse
import datetime
import hashlib
import json
from pathlib import Path
import subprocess
import time
import zipfile


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def run(args):
    root = args.root.resolve()
    bundle = args.bundle.resolve()
    installer = args.publications.resolve()
    store = args.install_dir.resolve()
    evidence = args.evidence_dir.resolve()
    if store.exists():
        raise ValueError("Installation qualification requires a fresh isolated directory")
    evidence.mkdir(parents=True, exist_ok=False)
    record = {
        "format": "agq-runtime-distribution-qualification/1",
        "started_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "checkout": str(root),
        "installer": str(installer),
        "installer_sha256": sha256(installer),
        "bundle": str(bundle),
        "expected_sha256": args.sha256.lower(),
        "bundle_sha256": sha256(bundle),
        "install_dir": str(store),
        "commands": [],
        "passed": False,
    }
    report = evidence / "qualification.json"
    try:
        if record["bundle_sha256"] != record["expected_sha256"]:
            raise ValueError("Package does not match the independently recorded transport identity")
        with zipfile.ZipFile(bundle) as archive:
            if sorted(archive.namelist()) != ["kerml.cache", "manifest.json", "systems.cache"]:
                raise ValueError("Unexpected runtime package entries")
            if not 0 < archive.getinfo("manifest.json").file_size <= 64 * 1024:
                raise ValueError("Runtime manifest exceeds the bounded transport format")
            manifest_bytes = archive.read("manifest.json")
        manifest = json.loads(manifest_bytes)
        (evidence / "manifest.json").write_bytes(manifest_bytes)
        values = {}
        for name, tail in [
            ("verify", ["verify", "--bundle", str(bundle)]),
            ("install", ["--runtime-dir", str(store), "install", "--bundle", str(bundle)]),
            ("status", ["--runtime-dir", str(store), "status"]),
        ]:
            command = [str(installer), "--root", str(root), *tail]
            started = time.monotonic()
            result = subprocess.run(command, cwd=root, capture_output=True, check=False)
            stdout = evidence / f"{name}.stdout.json"
            stderr = evidence / f"{name}.stderr.txt"
            stdout.write_bytes(result.stdout)
            stderr.write_bytes(result.stderr)
            record["commands"].append({
                "command": command,
                "exit_code": result.returncode,
                "elapsed_seconds": time.monotonic() - started,
                "stdout": stdout.name,
                "stdout_sha256": sha256(stdout),
                "stderr": stderr.name,
                "stderr_sha256": sha256(stderr),
            })
            if result.returncode != 0:
                raise RuntimeError(f"Ordinary {name} failed with exit {result.returncode}")
            values[name] = json.loads(result.stdout)
        installed = Path(values["install"]["directory"]).resolve()
        if not installed.is_relative_to(store):
            raise ValueError("Installer returned a directory outside the isolated store")
        if values["install"]["manifest"] != manifest:
            raise ValueError("Installed manifest differs from the verified package")
        for family, filename in [("kerml", "kerml.cache"), ("systems", "systems.cache")]:
            asset = manifest[family]
            path = installed / filename
            if asset["file"] != filename or path.stat().st_size != asset["bytes"] or sha256(path) != asset["sha256"]:
                raise ValueError(f"Installed {family} bytes differ from the authenticated manifest")
        if sha256(bundle) != record["bundle_sha256"] or sha256(installer) != record["installer_sha256"]:
            raise ValueError("Package or installer changed during qualification")
        record["bundle_identity"] = manifest["identity"]
        record["installed_directory"] = str(installed)
        record["passed"] = True
    except (OSError, ValueError, KeyError, RuntimeError, zipfile.BadZipFile) as error:
        record["error"] = f"{type(error).__name__}: {error}"
    finally:
        report.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(record, indent=2))
    return 0 if record["passed"] else 1


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--publications", type=Path, required=True, help="Built agq-publications executable")
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--sha256", required=True, help="Independently recorded package SHA-256")
    parser.add_argument("--install-dir", type=Path, required=True, help="New directory; never the normal operator store")
    parser.add_argument("--evidence-dir", type=Path, required=True, help="New directory for outputs and exit codes")
    raise SystemExit(run(parser.parse_args()))
