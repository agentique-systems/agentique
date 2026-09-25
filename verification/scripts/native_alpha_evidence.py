"""Check retained process-output hashes; optionally restore proven line endings."""
import argparse
import hashlib
import json
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[2]
DIRECTORY = ROOT / "verification/native-studio-alpha"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repair-line-endings", action="store_true")
    parser.add_argument("--git-index", action="store_true")
    args = parser.parse_args()
    if args.git_index and args.repair_line_endings:
        parser.error("Index verification never rewrites evidence")
    checked, repaired, failures = 0, [], []
    def read_index(path):
        result = subprocess.run(["git", "show", f":{path}"], cwd=ROOT,
                                capture_output=True, check=False)
        if result.returncode:
            raise ValueError(f"Missing staged blob: {path}")
        return result.stdout

    if args.git_index:
        receipts = [ROOT / path for path in subprocess.check_output([
            "git", "ls-files", "--", "verification/native-studio-alpha/checks/*.json"
        ], cwd=ROOT, text=True).splitlines()]
    else:
        receipts = sorted((DIRECTORY / "checks").glob("*.json"))
    for receipt in receipts:
        try:
            raw = (read_index(receipt.relative_to(ROOT).as_posix())
                   if args.git_index else receipt.read_bytes())
            record = json.loads(raw.decode("utf-8-sig"))
        except (OSError, ValueError) as error:
            failures.append({"receipt": str(receipt), "error": str(error)})
            continue
        name, expected = record.get("output"), record.get("output_sha256")
        if not name or not expected:
            continue
        output = (ROOT / name).resolve()
        if not output.is_relative_to(DIRECTORY):
            failures.append({"receipt": str(receipt), "error": "missing or external output"})
            continue
        checked += 1
        try:
            original = (read_index(output.relative_to(ROOT).as_posix())
                        if args.git_index else output.read_bytes())
        except (OSError, ValueError) as error:
            failures.append({"receipt": str(receipt), "error": str(error)})
            continue
        if digest(original) == expected:
            continue
        unix = original.replace(b"\r\n", b"\n")
        variants = (unix, unix.replace(b"\n", b"\r\n"))
        proven = next((data for data in variants if digest(data) == expected), None)
        if args.repair_line_endings and proven is not None:
            output.write_bytes(proven)
            repaired.append({"output": name, "before_sha256": digest(original),
                             "restored_sha256": expected})
        else:
            failures.append({"output": name, "expected_sha256": expected,
                             "actual_sha256": digest(original),
                             "line_endings_only": proven is not None})
    print(json.dumps({"checked": checked, "repaired": repaired, "failures": failures}, indent=2))
    return bool(failures)


if __name__ == "__main__":
    raise SystemExit(main())
