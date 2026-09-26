"""Read-only evidence audit; never authenticates semantics or authorizes recovery."""
import argparse
import hashlib
import json
import pathlib
import sqlite3
import time


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=pathlib.Path, required=True)
    parser.add_argument("--report", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise ValueError("Refusing to replace audit evidence")
    started = time.perf_counter()
    raw_report = args.report.read_bytes()
    report = json.loads(raw_report)
    baseline = report["baseline"]
    assert report["format"] == "agentique-native-real-acceptance/1"
    assert report["outcome"] == "failed" and not report["passed"]
    assert report["committed"] is None and not report["restart_verified"]
    assert "Validated" in baseline["validation"]
    db = pathlib.Path(report["database"])
    assert db.is_absolute() and db.is_file()
    connection = sqlite3.connect(db.as_uri() + "?mode=ro", uri=True)
    connection.execute("PRAGMA query_only=ON")
    connection.execute("BEGIN")
    assert connection.execute("PRAGMA integrity_check").fetchall() == [("ok",)]
    assert connection.execute("PRAGMA foreign_key_check").fetchall() == []
    metadata = connection.execute(
        "SELECT repository_id,format FROM repository_metadata"
    ).fetchall()
    assert len(metadata) == 1 and metadata[0][1] == "agentique-modeling-sqlite/1"
    inventory = {}
    parsed = {}
    for table, payload, fields in (
        ("projects", "data", "id"),
        ("branches", "data", "id,project_id,name,head_id"),
        ("revisions", "manifest", "id,project_id,parent_id"),
        ("operation_receipts", "data", "operation_id,request_digest,revision_id"),
    ):
        rows = connection.execute(
            f"SELECT {fields},{payload},digest FROM {table} ORDER BY 1"
        ).fetchall()
        inventory[table] = []
        parsed[table] = {}
        for row in rows:
            keys, raw, expected = row[:-2], bytes(row[-2]), row[-1]
            assert digest(raw) == expected
            parsed[table][json.loads(keys[0])] = json.loads(raw)
            item = parsed[table][json.loads(keys[0])]
            if table == "projects":
                assert item["id"] == json.loads(keys[0])
            elif table == "branches":
                assert (item["id"], item["project_id"], item["name"], item["head"]) == (
                    json.loads(keys[0]), json.loads(keys[1]), keys[2], json.loads(keys[3])
                )
            elif table == "revisions":
                assert (item["revision_id"], item["project_id"], item["parent_revision_id"]) == (
                    json.loads(keys[0]), json.loads(keys[1]),
                    json.loads(keys[2]) if keys[2] is not None else None
                )
            else:
                assert item["operation_id"] == json.loads(keys[0])
                assert item["revision_id"] == json.loads(keys[2])
            inventory[table].append({"keys": keys, "sha256": expected, "bytes": len(raw)})
    actual = parsed["revisions"][baseline["revision_id"]]
    assert actual == baseline
    project = parsed["projects"][report["project"]]
    branch = parsed["branches"][report["branch"]]
    assert project["default_branch"] == report["branch"]
    assert branch["project_id"] == report["project"]
    assert branch["head"] == baseline["revision_id"]
    chain = []
    cursor = baseline["revision_id"]
    while cursor is not None:
        assert cursor not in chain
        revision = parsed["revisions"][cursor]
        assert revision["project_id"] == report["project"]
        chain.append(cursor)
        cursor = revision["parent_revision_id"]
    assert set(chain) == set(parsed["revisions"])
    assert all(b["head"] in chain for b in parsed["branches"].values())
    receipts = list(parsed["operation_receipts"].values())
    assert len(receipts) == len(chain)
    assert {r["revision_id"] for r in receipts} == set(chain)
    blobs = {}
    for expected, size in connection.execute("SELECT digest,length(bytes) FROM blobs ORDER BY digest"):
        raw = bytes(connection.execute("SELECT bytes FROM blobs WHERE digest=?", (expected,)).fetchone()[0])
        assert len(raw) == size and digest(raw) == expected
        blobs[expected] = {"bytes": size, "sha256_verified": True}
    for revision in parsed["revisions"].values():
        assert revision["checkpoint_digest"] in blobs
        assert all(d["content_digest"] in blobs for d in revision["documents"])
    for document in baseline["documents"]:
        raw = (args.root / "models/agentique" / document["path"]).read_bytes()
        assert digest(raw) == document["content_digest"]
        assert b"alphaStudioObserver" not in raw
    connection.rollback()
    connection.close()
    assert args.report.read_bytes() == raw_report
    result = {
        "format": "agentique-interrupted-proposal-storage-review/1",
        "scope": "Read-only consistent SQLite snapshot. Storage identity only; not semantic authentication, rollback, or a resumed-journey result.",
        "database": str(db),
        "failed_report": str(args.report),
        "failed_report_sha256": digest(raw_report),
        "audit_script_sha256": digest(pathlib.Path(__file__).read_bytes()),
        "repository_metadata": metadata,
        "inventory": inventory,
        "head_manifest_equals_failed_report_baseline": True,
        "head": baseline["revision_id"],
        "complete_revision_population_equals_baseline_ancestry": chain,
        "all_operation_receipts_bound_only_to_baseline_ancestry": True,
        "all_blob_content_addresses_verified": blobs,
        "baseline_source_population_matches_current_model_files": True,
        "candidate_name_absent_from_baseline_source_bytes": True,
        "sqlite_integrity_check": "ok",
        "foreign_key_violations": [],
        "elapsed_seconds": time.perf_counter() - started,
        "exit_code": 0,
    }
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(f"PASS: {len(chain)} ancestor revisions, {len(receipts)} receipts, {len(blobs)} authenticated blob addresses; unchanged baseline {baseline['revision_id']}")


if __name__ == "__main__":
    main()
