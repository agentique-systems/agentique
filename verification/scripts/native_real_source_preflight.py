"""Read-only textual preflight for real native acceptance selectors.

This is not parsing, runtime authentication, closure or semantic validation.
It catches drift in the concrete authored inputs before an expensive real run.
"""
import argparse
import hashlib
import json
import pathlib
import re


def preflight(root):
    directory = root / "models/agentique"
    sources = {path.name: path.read_bytes() for path in sorted(directory.glob("*.sysml"))}
    text = {name: content.decode("utf-8") for name, content in sources.items()}
    checks = []

    def check(name, passed, detail):
        checks.append({"name": name, "passed": passed, "detail": detail})

    expected = {
        "Contracts.sysml", "LanguageEngine.sysml", "ModelingPlatform.sysml",
        "ExecutionRuntime.sysml", "Agentique.sysml", "AgentFabric.sysml",
    }
    check("bootstrap source inventory", set(sources) == expected, sorted(sources))
    selectors = {
        "Agentique": "Agentique.sysml",
        "ModelingPlatform": "ModelingPlatform.sysml",
        "ModelRepository": "ModelingPlatform.sysml",
        "ProjectWorkspace": "ModelingPlatform.sysml",
        "AgentRuntime": "AgentFabric.sysml",
    }
    for name, expected_path in selectors.items():
        matches = [path for path, source in text.items()
                   for _ in re.finditer(r"(?m)^\s*part\s+def\s+" + re.escape(name) + r"\b", source)]
        check("unique PartDefinition " + name, matches == [expected_path], matches)

    platform = text.get("ModelingPlatform.sysml", "")
    check("repository inherits interface owner", bool(re.search(
        r"part\s+def\s+ModelRepository\s*:>\s*Repository\s*\{", platform)),
        "ModelRepository specializes Repository")
    check("repository interface declaration", bool(re.search(
        r"part\s+def\s+Repository\s*\{\s*port\s+repositoryRevisions\s*:", platform)),
        "Repository owns port repositoryRevisions")
    check("CreatePartUsage owner has plain body header", bool(re.search(
        r"part\s+def\s+ModelingPlatform\s*\{", platform)),
        "ModelingPlatform is a plain authored PartDefinition")
    check("rename refusal target has plain header and actual textual references",
          bool(re.search(r"part\s+def\s+ProjectWorkspace\s*\{", platform))
          and bool(re.search(r":\s*ProjectWorkspace\s*;", platform)),
          "ProjectWorkspace rename must exercise reference refusal, not complex-header refusal")
    for name in ("alphaStudioObserver", "nativeObserver", "afterRestartObserver", "nativeObserverRenamed"):
        matches = [path for path, source in text.items() if re.search(r"\b" + re.escape(name) + r"\b", source)]
        check("acceptance part name is fresh: " + name, not matches, matches)
    check("authored requirement declaration exists", any(re.search(
        r"(?m)^\s*requirement\s+def\s+[A-Za-z_]", source) for source in text.values()),
        "Requirements World has authored seed declarations")
    return {
        "format": "agentique-real-source-selector-preflight/1",
        "root": str(root),
        "source_preflight_passed": all(item["passed"] for item in checks),
        "runtime_authenticated": False,
        "semantic_validation_performed": False,
        "source_sha256": {name: hashlib.sha256(content).hexdigest() for name, content in sources.items()},
        "checks": checks,
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, type=pathlib.Path)
    args = parser.parse_args()
    report = preflight(args.root.resolve())
    print(json.dumps(report, indent=2))
    raise SystemExit(0 if report["source_preflight_passed"] else 1)
