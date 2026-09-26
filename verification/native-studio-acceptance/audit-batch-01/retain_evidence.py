"""Stream-verify the downloaded batch artifact and retain its exact evidence."""

from __future__ import annotations

import gzip
import hashlib
import json
from pathlib import Path
import shutil
import sys
from datetime import datetime, timezone


def digest(path: Path, compressed: bool = False) -> str:
    result = hashlib.sha256()
    opener = gzip.open if compressed else open
    with opener(path, "rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            result.update(chunk)
    return result.hexdigest()


def main() -> None:
    repository = Path(__file__).resolve().parents[3]
    source = repository / "verification/generated/native-studio-acceptance/audit-batch-01"
    target = Path(__file__).resolve().parent
    metadata = json.loads((source / "artifact-metadata.json").read_bytes())
    result = json.loads((source / "measurements/result.json").read_bytes())
    build = json.loads((source / "build/build.json").read_bytes())
    expected = "0beea518acd35a012856c938f0e34ee90ba07b098b0e274d4d6d4d725e6f508a"
    assert metadata["workflow_run"]["id"] == 36235278112
    assert result["source_commit"] == "167cbeca9f4c6e276c68459e51fb16b62e3b383c"
    assert build["source_commit"] == result["source_commit"]
    assert result["outcome"] == "passed"
    assert result["same_runner"] and result["same_executable"]
    assert digest(source / "artifact.zip") == metadata["digest"].removeprefix("sha256:")
    assert digest(source / "build/build.json") == result["build_receipt_sha256"]
    assert digest(source / "build/bin/create_part_performance.exe") == result["executable_sha256"]
    assert digest(source / "provenance/inputs.json") == result["input_receipt_sha256"]
    for command in build["commands"]:
        assert command["exit_code"] == 0
        assert digest(source / "build" / command["output"]) == command["output_sha256"]
    trials = {}
    for batch in ("32", "128", "256"):
        trial = result["batches"][batch]
        directory = source / "measurements" / ("batch-" + batch)
        process = json.loads((directory / "process.json").read_bytes())
        metrics = json.loads((directory / "cold-metrics.json").read_bytes())
        comparison = trial["comparison"]
        assert process == trial["process"]
        assert metrics == trial["metrics"]
        assert process["exit_code"] == 0 and process["terminated_reason"] is None
        assert metrics["validated"] and comparison["exact_equivalence"]
        assert comparison["observation_count"] == 697419
        assert comparison["query_observation_count"] == 1633
        assert comparison["missing"] == comparison["extra"] == comparison["different"] == []
        assert digest(directory / "process.log") == process["output_sha256"]
        for name, expected_input in result["input_sha256"].items():
            assert digest(directory / name) == expected_input
        actual = digest(directory / "cold-observations.json")
        assert actual == trial["observations_sha256"] == expected
        trials[batch] = {"observations_sha256": actual, "process_exit_code": 0}
    prior = repository / "verification/native-studio-acceptance/ci-oracle-01/current/oracle/observations/cold-observations.json.gz"
    assert digest(prior, compressed=True) == expected
    observations = target / "observations/cold-observations.json.gz"
    observations.parent.mkdir(parents=True, exist_ok=True)
    with (source / "measurements/batch-32/cold-observations.json").open("rb") as incoming:
        with observations.open("wb") as raw:
            with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as output:
                shutil.copyfileobj(incoming, output, length=1024 * 1024)
    assert digest(observations, compressed=True) == expected
    files = {}
    for path in sorted(source.rglob("*")):
        if not path.is_file():
            continue
        relative = path.relative_to(source)
        entry = {"bytes": path.stat().st_size, "sha256": digest(path)}
        if path.name == "cold-observations.json":
            entry["retained_as"] = observations.relative_to(target).as_posix()
            entry["encoding"] = "gzip; one byte-identical map shared by all three trials"
        elif path.suffix == ".exe" or path.name == "artifact.zip":
            entry["retained_as"] = None
            entry["reason"] = "Generated local copy and authenticated remote artifact; binary not committed"
        else:
            destination = target / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, destination)
            assert digest(destination) == entry["sha256"]
            entry["retained_as"] = relative.as_posix()
            entry["encoding"] = "original bytes"
        files[relative.as_posix()] = entry
    receipt = {
        "format": "agentique-audit-batch-independent-retention/1",
        "outcome": "passed",
        "command": [sys.executable, str(Path(__file__).resolve())],
        "exit_code": 0,
        "verified_utc": datetime.now(timezone.utc).isoformat(),
        "source_commit": result["source_commit"],
        "run": 36235278112,
        "artifact_id": metadata["id"],
        "trials": trials,
        "prior_retained_observations_sha256": expected,
        "retained_gzip_sha256": digest(observations),
        "retention_script_sha256": digest(Path(__file__)),
        "files": files,
        "contract": "Streaming SHA-256 compares all original observation bytes, including the previously retained cold oracle. CI performed full parsed-map comparison. No local semantic process was run. Evidence was copied byte-for-byte; the one shared gzip map decompresses to all three original trial maps.",
    }
    (target / "independent-verification.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"outcome": "passed", "source_files_verified": len(files), "all_observations_byte_equal": True, "observations_sha256": expected}))


if __name__ == "__main__":
    main()
