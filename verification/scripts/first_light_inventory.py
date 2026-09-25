"""Inventory existing First Light assets without creating publication authority."""
import argparse
import json
import os
from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--ci-directory", type=Path)
    args = parser.parse_args()
    checks = []
    for command in [
        ["git", "rev-parse", "HEAD", "origin/main"],
        ["gh", "api", "repos/agentique-systems/agentique/releases",
         "--jq", "[.[] | {tag_name, assets: [.assets[] | {name,size,browser_download_url}]}]"],
        ["gh", "api", "repos/agentique-systems/agentique/actions/artifacts?per_page=100",
         "--jq", "{total_count, artifacts: [.artifacts[] | {id,name,size_in_bytes,expired}]}"],
    ]:
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
        checks.append({"command": command, "exit_code": result.returncode,
                       "stdout": result.stdout.strip(), "stderr": result.stderr.strip()})
    runtime_root = Path(os.environ.get("AGENTIQUE_RUNTIME_DIR", Path.home() / ".agentique"))
    candidates = []
    for variable, fallback in [
        ("AGENTIQUE_KERML_CACHE", ROOT / "verification/generated/kerml-v9-publication/canonical.publication.zip"),
        ("AGENTIQUE_SYSTEMS_CACHE", ROOT / "verification/generated/final-audit-semantic-closure/accepted/canonical.publication.zip"),
    ]:
        path = Path(os.environ.get(variable, fallback))
        candidates.append({"path": str(path), "override": variable in os.environ,
                           "exists": path.is_file(),
                           "bytes": path.stat().st_size if path.is_file() else None})
    installed = sorted(str(p) for p in runtime_root.glob("publications/*/manifest.json"))
    ci = None
    if args.ci_directory:
        directory = args.ci_directory.resolve()
        files = [p for p in directory.rglob("*") if p.is_file()]
        ci = {"path": str(directory), "files": len(files),
              "publication_archives": [str(p.relative_to(directory)) for p in files
                                       if p.name == "canonical.publication.zip"],
              "zip_files": [str(p.relative_to(directory)) for p in files if p.suffix == ".zip"]}
    print(json.dumps({"format": "agentique-first-light-asset-inventory/1",
                      "grants_publication_authority": False,
                      "checks": checks, "runtime_root": str(runtime_root),
                      "installed_manifests": installed, "legacy_candidates": candidates,
                      "ci_archive_inspection": ci}, indent=2))
    return 0 if all(check["exit_code"] == 0 for check in checks) else 1


if __name__ == "__main__":
    raise SystemExit(main())
