"""Check declared Cargo workspace dependencies without compiling any crate.

All normal/build/dev, optional and target-specific workspace dependency edges
count. Cargo metadata supplies actual package names, including renamed imports.
This checks architecture outside the language engine, not SysML model semantics.
"""
import argparse
from collections import deque
import json
from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parents[2]
GEN1 = frozenset({
    "agq-model", "agq-syntax", "agq-semantics", "agq-workspace",
    "agq-simulation", "agq-application", "agq-storage", "agq-assistant",
    "agq-server", "agentique",
})
REQUIRED = frozenset({
    "agq-kernel", "agq-kerml", "agq-kerml-semantics", "agq-kerml-syntax",
    "agq-kerml-text", "agq-sysml", "agq-sysml-semantics",
    "agq-standard-libraries",
})


def language(name):
    return any(name == family or name.startswith(family + "-")
               for family in ("agq-kerml", "agq-sysml"))


def protected(name):
    return language(name) or name.startswith("agq-modeling-") or name in {
        "agq-kernel", "agq-standard-libraries", "agq-modeling-workspace",
    }


def audit(metadata):
    """Return deterministic package paths violating the generation boundary.

    The graph is intentionally the union of declared workspace configurations;
    a currently inactive feature or target cannot hide a forbidden dependency.
    External packages are leaves: third-party dependency policy is separate.
    """
    members = set(metadata["workspace_members"])
    packages = {p["name"]: p for p in metadata["packages"] if p["id"] in members}
    missing = sorted(REQUIRED - packages.keys())
    if missing:
        raise ValueError("missing required workspace packages: " + ", ".join(missing))
    edges = {name: sorted({dependency["name"] for dependency in package["dependencies"]})
             for name, package in packages.items()}
    violations = []
    for origin in sorted(name for name in packages if protected(name)):
        pending = deque([(origin,)])
        visited = {origin}
        while pending:
            path = pending.popleft()
            for target in edges.get(path[-1], []):
                if target in visited:
                    continue
                visited.add(target)
                next_path = (*path, target)
                if target in GEN1:
                    violations.append({"rule": "gen2-must-not-depend-on-gen1",
                                       "path": list(next_path)})
                if origin == "agq-kernel" and (language(target) or target in {
                        "agq-standard-libraries", "agq-modeling-workspace"}
                        or target.startswith("agq-modeling-")):
                    violations.append({"rule": "kernel-must-remain-language-agnostic",
                                       "path": list(next_path)})
                if (language(origin) or origin in {"agq-kernel", "agq-standard-libraries", "agq-modeling-workspace"}) and target in {
                        "agq-modeling-repository", "agq-modeling-service", "agq-modeling-api", "agq-modeling-http", "agq-modeling-sqlite", "rusqlite", "axum"}:
                    violations.append({"rule": "language-workspace-must-not-depend-on-platform-adapters",
                                       "path": list(next_path)})
                pending.append(next_path)
    return {
        "format": "agentique-generation-boundary/1",
        "workspace_packages": len(packages),
        "protected_packages": sorted(name for name in packages if protected(name)),
        "checked_dependency_kinds": ["normal", "build", "dev", "optional", "target"],
        "scope": "all declared workspace edges; third-party packages are leaves",
        "violations": violations,
    }


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--manifest-path", type=Path, default=ROOT / "Cargo.toml")
    args = parser.parse_args()
    command = ["cargo", "metadata", "--locked", "--offline", "--no-deps",
               "--format-version", "1", "--manifest-path", str(args.manifest_path)]
    result = subprocess.run(command, capture_output=True, text=True, check=False)
    if result.returncode:
        print(result.stderr, end="")
        return result.returncode
    report = audit(json.loads(result.stdout))
    print(json.dumps(report, indent=2))
    return 1 if report["violations"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
