"""Compare artifacts byte-exactly and ordinary Git text with EOL normalization."""
from pathlib import Path
import hashlib
import json
import subprocess

ROOT = Path(__file__).resolve().parents[2]
BASE = "190588dd1ed84cd49a437aec83a6d933ff9e968a"
PREFIXES = (
    "standards/normative/", "standards/generated/", "standards/generated-src/",
    "crates/kernel/", "crates/kerml/", "crates/sysml/", "tools/metamodel-gen/",
    "crates/model/", "crates/syntax/", "crates/semantics/", "crates/workspace/",
    "crates/simulation/", "crates/application/", "crates/server/", "crates/cli/",
    "adapters/", "apps/", "src/", "console/", "tests/", "verification/",
)
EXACT = {
    "AGENTS.md", "standards/coverage.json", "verification/traceability.json",
    "standards/baseline-anomalies.json", "standards/lock.json",
    "standards/baseline-lock.json", "docs/language-core-foundation-review.md",
    "docs/language-core-runtime-contract.md", "docs/architecture.md",
    "docs/semantic-kernel.md",
    "package.json", "package-lock.json", "playwright.config.ts",
}


def main():
    entries = subprocess.check_output(
        ["git", "ls-tree", "-rz", BASE], cwd=ROOT
    ).split(b"\0")
    expected = []
    for entry in entries:
        if not entry:
            continue
        metadata, raw_path = entry.split(b"\t", 1)
        path = raw_path.decode("utf-8")
        if (path.startswith(PREFIXES) or path in EXACT
                or path.startswith("docs/adr/")
                or path.lower().endswith((".pdf", ".html", ".kpar"))):
            expected.append((path, metadata.split()[2]))
    # Batch reads use Git's raw blobs; no shell transforms or line-ending repair.
    blobs = subprocess.check_output(
        ["git", "cat-file", "--batch"],
        input=b"\n".join(blob for _, blob in expected) + b"\n", cwd=ROOT,
    )
    offset = 0
    hashes = {}
    byte_exact = 0
    for path, blob in expected:
        newline = blobs.index(b"\n", offset)
        header = blobs[offset:newline].split()
        assert header[0] == blob and header[1] == b"blob"
        size = int(header[2])
        original = blobs[newline + 1:newline + 1 + size]
        offset = newline + 1 + size + 1
        current = (ROOT / path).read_bytes()
        exact = (path.startswith(("standards/normative/", "standards/libraries/",
                                  "standards/libraries-2026-04/", "standards/artifacts/",
                                  "verification/language-core-completion-v3/"))
                 or path.lower().endswith((".pdf", ".html", ".kpar", ".png")))
        if exact:
            assert original == current, f"Protected artifact bytes changed: {path}"
            byte_exact += 1
        else:
            assert original.replace(b"\r\n", b"\n") == current.replace(b"\r\n", b"\n"), \
                f"Protected Git text changed: {path}"
        hashes[path] = {"sha256": hashlib.sha256(current).hexdigest(),
                        "comparison": "byte-exact" if exact else "git-text-eol-normalized"}
    print(json.dumps({"base": BASE, "protected_files": len(hashes),
                      "byte_exact_files": byte_exact, "files": hashes}, indent=2))


if __name__ == "__main__":
    main()
