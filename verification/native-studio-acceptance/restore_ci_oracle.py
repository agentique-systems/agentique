"""Restore retained oracle evidence exactly, without executing it or replacing files."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import subprocess


def require(value, message):
    if not value:
        raise ValueError(message)


def unique(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f"Duplicate JSON key: {key}")
        result[key] = value
    return result


def digest(file):
    with file.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def relative(name):
    require(isinstance(name, str) and name and "\\" not in name and ":" not in name,
            "Expected a canonical relative POSIX artifact path")
    path = PurePosixPath(name)
    require(not path.is_absolute() and str(path) == name and ".." not in path.parts,
            f"Unsafe artifact path: {name}")
    for part in path.parts:
        require(part.rstrip(" .") == part and not re.search(r'[<>"|?*\x00-\x1f]', part),
                f"Unsafe path component: {part}")
        require(not re.fullmatch(r"(?i)(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\..*)?", part),
                f"Reserved device path: {part}")
    require(path.suffix.lower() not in {".exe", ".dll", ".com", ".scr"}
            and "bin" not in [part.lower() for part in path.parts], "Executables are not retained evidence")
    return Path(*path.parts)


def bounded(root, path):
    candidate = root / path
    require(candidate.resolve().is_relative_to(root), f"Path escapes evidence root: {path}")
    for item in [candidate, *candidate.parents]:
        if item == root:
            break
        require(not item.is_symlink() and not item.is_junction(), f"Linked artifact path: {path}")
    return candidate


def stream(source, record, destination=None):
    require(digest(source) == record["stored_sha256"], f"Stored hash mismatch: {source}")
    opener = gzip.open if record["compression"] == "gzip" else open
    result = hashlib.sha256()
    size = 0
    with opener(source, "rb") as decoded:
        while chunk := decoded.read(1024 * 1024):
            size += len(chunk)
            require(size <= record["original_bytes"], f"Expanded size exceeds manifest: {source}")
            result.update(chunk)
            if destination is not None:
                destination.write(chunk)
    require(size == record["original_bytes"] and result.hexdigest() == record["original_sha256"],
            f"Original size/hash mismatch: {source}")


def restore(root, retained, output, manifest_sha256, verify_only=False):
    root = root.resolve(strict=True)
    retained = retained.resolve(strict=True)
    require(retained.is_relative_to(root / "verification"), "Retained evidence must be under verification")
    output = output.absolute()
    generated = root / "verification" / "generated"
    require(output.is_relative_to(generated) and output != generated, "Output must be below verification/generated")
    output = bounded(generated, output.relative_to(generated))
    require(not output.exists(), "Refusing existing output directory, including an empty directory")
    ignored = subprocess.run(["git", "check-ignore", "-q", "--", str(output.relative_to(root) / "evidence.json")],
                             cwd=root, capture_output=True)
    require(ignored.returncode == 0, "Output must be Git-ignored")
    manifest = retained / "artifact-manifest.json"
    require(re.fullmatch(r"[0-9a-f]{64}", manifest_sha256) is not None and digest(manifest) == manifest_sha256,
            "Retained manifest hash differs from explicit reviewed SHA-256")
    data = json.loads(manifest.read_text(encoding="utf-8-sig"), object_pairs_hook=unique)
    require(data.get("format") == "agentique-retained-ci-oracle/1", "Unsupported retained artifact format")
    entries = data.get("files")
    require(isinstance(entries, dict) and entries, "Manifest has no evidence files")
    names = set()
    sources = set()
    checked = []
    for name, record in entries.items():
        target = relative(name)
        require(name.lower() != "restore-receipt.json" and name.lower() not in names, "Duplicate/reserved output path")
        names.add(name.lower())
        require(set(record) == {"path", "original_bytes", "original_sha256", "stored_sha256", "compression"},
                "Unsupported manifest record")
        require(record["compression"] in (None, "gzip"), "Unsupported compression")
        require(record["path"] == name + (".gz" if record["compression"] else ""), "Stored/original path mismatch")
        stored = relative(record["path"])
        require(record["path"].lower() not in sources, "Aliased source path")
        sources.add(record["path"].lower())
        require(type(record["original_bytes"]) is int and record["original_bytes"] >= 0, "Invalid expanded size")
        require(all(isinstance(record[key], str) and re.fullmatch(r"[0-9a-f]{64}", record[key])
                    for key in ("original_sha256", "stored_sha256")), "Invalid evidence hash")
        source = bounded(retained, stored)
        require(source.is_file(), f"Missing retained artifact: {stored}")
        checked.append((source, target, record))
    for name in names:
        require(not any(str(parent) in names for parent in PurePosixPath(name).parents if str(parent) != "."),
                "Artifact file/directory collision")
    # Verify before creating output. Verification streams large maps and never
    # executes scripts/source evidence. Writing repeats all size/hash checks.
    for source, _, record in checked:
        stream(source, record)
    require(digest(manifest) == manifest_sha256, "Manifest changed during verification")
    result = {"format": "agentique-oracle-evidence-restore/1", "manifest_sha256": manifest_sha256,
              "files": len(checked), "original_bytes": sum(row[2]["original_bytes"] for row in checked),
              "executables_restored": False, "verify_only": verify_only,
              "qualification": "Evidence bytes only; no new semantic, runtime, performance or Alpha qualification"}
    if not verify_only:
        output.mkdir(parents=True, exist_ok=False)
        for source, target, record in checked:
            destination = bounded(output, target)
            destination.parent.mkdir(parents=True, exist_ok=True)
            with destination.open("xb") as sink:
                stream(source, record, sink)
        require(digest(manifest) == manifest_sha256, "Manifest changed during restoration")
        with (output / "restore-receipt.json").open("x", encoding="utf-8", newline="\n") as sink:
            json.dump(result, sink, indent=2)
            sink.write("\n")
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--retained", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest-sha256", required=True)
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args()
    print(json.dumps(restore(args.root, args.retained, args.output, args.manifest_sha256, args.verify_only)))
