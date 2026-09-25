"""Preserve an exact measured executable before a subsequent rebuild replaces it."""
import hashlib
import json
import pathlib
import shutil
import sys

source = pathlib.Path(sys.argv[1]).resolve(strict=True)
destination = pathlib.Path(sys.argv[2]).resolve()
destination.parent.mkdir(parents=True, exist_ok=True)
with source.open("rb") as original, destination.open("xb") as retained:
    shutil.copyfileobj(original, retained)
with source.open("rb") as original, destination.open("rb") as retained:
    before = hashlib.file_digest(original, "sha256").hexdigest()
    after = hashlib.file_digest(retained, "sha256").hexdigest()
assert before == after, "Executable changed during preservation"
print(json.dumps({
    "source": str(source),
    "retained": str(destination),
    "sha256": after,
    "bytes": destination.stat().st_size,
}))
