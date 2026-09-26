"""Copy a committed SQLite snapshot for an oracle, including retained WAL data.

The source is opened read-only. This checks storage and report identity only;
the consuming oracle must authenticate the runtime and restore semantics normally.
"""
import argparse
import hashlib
import json
import pathlib
import sqlite3
import time


def identity(path):
    if not path.exists():
        return None
    with path.open("rb") as stream:
        return {"bytes": path.stat().st_size,
                "sha256": hashlib.file_digest(stream, "sha256").hexdigest()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=pathlib.Path, required=True)
    parser.add_argument("--snapshot", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    started = time.perf_counter()
    report_bytes = args.report.read_bytes()
    report = json.loads(report_bytes)
    baseline = report["baseline"]
    assert report["format"] == "agentique-native-real-acceptance/1"
    assert "Validated" in baseline["validation"]
    source = pathlib.Path(report["database"]).resolve(strict=True)
    snapshot = args.snapshot.resolve()
    assert source != snapshot and not args.output.exists()
    for suffix in ("", "-wal", "-shm", "-journal"):
        assert not pathlib.Path(str(snapshot) + suffix).exists(), "Fresh snapshot path required"
    source_files = [source, pathlib.Path(str(source) + "-wal")]
    before = {str(path): identity(path) for path in source_files}
    snapshot.parent.mkdir(parents=True, exist_ok=True)
    # Reserve this path exclusively before SQLite opens it. Failed evidence is
    # retained for diagnosis; this helper never deletes or replaces a database.
    with snapshot.open("xb"):
        pass
    with sqlite3.connect(source.as_uri() + "?mode=ro", uri=True) as original:
        original.execute("PRAGMA query_only=ON")
        with sqlite3.connect(snapshot) as copied:
            original.backup(copied)
            assert copied.execute("PRAGMA integrity_check").fetchall() == [("ok",)]
            assert copied.execute("PRAGMA foreign_key_check").fetchall() == []
            raw, expected = copied.execute(
                "SELECT manifest,digest FROM revisions WHERE id=? AND project_id=?",
                (json.dumps(baseline["revision_id"]), json.dumps(baseline["project_id"])),
            ).fetchone()
            raw = bytes(raw)
            assert hashlib.sha256(raw).hexdigest() == expected
            assert json.loads(raw) == baseline, "Snapshot differs from retained baseline manifest"
            branch = copied.execute(
                "SELECT head_id FROM branches WHERE id=? AND project_id=?",
                (json.dumps(report["branch"]), json.dumps(baseline["project_id"])),
            ).fetchone()
            assert branch and json.loads(branch[0]) == baseline["revision_id"]
            assert copied.execute("PRAGMA journal_mode=DELETE").fetchone()[0] == "delete"
        copied.close()
    original.close()
    after = {str(path): identity(path) for path in source_files}
    assert before == after, "Source database or WAL changed during snapshot"
    for suffix in ("-wal", "-journal"):
        sidecar = pathlib.Path(str(snapshot) + suffix)
        assert not sidecar.exists() or sidecar.stat().st_size == 0
    result = {
        "format": "agentique-oracle-sqlite-snapshot/1",
        "scope": "Read-only source, SQLite backup includes committed WAL. Storage identity only; semantic authentication remains mandatory in the oracle.",
        "source_report": str(args.report),
        "source_report_sha256": hashlib.sha256(report_bytes).hexdigest(),
        "source_files_before": before, "source_files_after": after,
        "snapshot": str(snapshot), "snapshot_identity": identity(snapshot),
        "project": baseline["project_id"], "revision": baseline["revision_id"],
        "branch": report["branch"], "baseline_manifest_sha256": expected,
        "integrity_check": "ok", "foreign_key_violations": 0,
        "elapsed_seconds": time.perf_counter() - started,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("x", encoding="utf-8") as output:
        json.dump(result, output, indent=2)
        output.write("\n")
    print(json.dumps(result))


if __name__ == "__main__":
    main()
