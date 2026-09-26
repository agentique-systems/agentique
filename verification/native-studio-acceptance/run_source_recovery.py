"""Create, commit, discard caches and independently restore an isolated project.

Only the database created by this invocation can be modified. Authoritative
tables and every mandatory blob are byte-compared across cache removal.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import time

import psutil


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--runtime-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    executable = args.executable.resolve(strict=True)
    root = args.root.resolve(strict=True)
    database = output / "recovery.sqlite"
    expectation = output / "expected.json"
    record = {
        "source_commit": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
        "executable_sha256": digest(executable),
        "commands": [], "outcome": "incomplete",
    }

    def save():
        (output / "recovery.json").write_text(
            json.dumps(record, indent=2) + "\n", encoding="utf-8")

    def run(mode):
        command = list(map(str, [executable, mode, root,
            args.runtime_dir.resolve(strict=True), database, expectation]))
        log_path = output / f"{mode}.log"
        started = time.monotonic()
        peak = 0
        with log_path.open("w", encoding="utf-8") as log:
            process = subprocess.Popen(command, cwd=root, stdout=log,
                stderr=subprocess.STDOUT,
                env={**os.environ, "AGENTIQUE_STARTUP_PROFILE": "1"})
            observed = psutil.Process(process.pid)
            while process.poll() is None:
                try:
                    memory = observed.memory_info()
                    peak = max(peak, getattr(memory, "peak_wset", memory.rss))
                except psutil.Error:
                    pass
                time.sleep(0.25)
        record["commands"].append({"command": command,
            "elapsed_seconds": time.monotonic() - started,
            "exit_code": process.returncode, "peak_working_set_bytes": peak,
            "log": log_path.name, "log_sha256": digest(log_path)})
        save()
        if process.returncode:
            raise RuntimeError(f"{mode} process failed: {process.returncode}")

    try:
        run("create")
        with sqlite3.connect(database) as connection:
            connection.execute("PRAGMA foreign_keys=ON")
            tables = ("repository_metadata", "projects", "revisions",
                      "revision_documents", "source_revisions", "branches",
                      "operation_receipts")

            def authoritative_rows():
                return {table: connection.execute(
                    f"SELECT * FROM {table} ORDER BY rowid").fetchall()
                    for table in tables}

            before = authoritative_rows()
            manifests = [json.loads(row[0]) for row in connection.execute(
                "SELECT manifest FROM revisions")]
            mandatory = set()
            for manifest in manifests:
                mandatory.add(manifest["checkpoint_digest"])
                mandatory.update(doc["content_digest"]
                                 for doc in manifest["documents"])
            saved_blobs = {key: connection.execute(
                "SELECT bytes FROM blobs WHERE digest=?", (key,)).fetchone()[0]
                for key in mandatory}
            caches = {row[0] for row in connection.execute(
                "SELECT blob_digest FROM semantic_caches")}
            if len(caches) < 2 or caches & mandatory:
                raise RuntimeError("Expected two discardable caches, disjoint from authority")
            for key in sorted(caches):
                connection.execute("DELETE FROM blobs WHERE digest=?", (key,))
            connection.execute("DELETE FROM semantic_caches")
            if authoritative_rows() != before or any(connection.execute(
                    "SELECT bytes FROM blobs WHERE digest=?", (key,)).fetchone()[0] != value
                    for key, value in saved_blobs.items()):
                raise RuntimeError("Cache removal changed authoritative inputs")
            if connection.execute("PRAGMA foreign_key_check").fetchall():
                raise RuntimeError("Repository foreign key check failed")
            record["cache_removal"] = {"database": str(database),
                "removed_blob_digests": sorted(caches),
                "mandatory_blobs_retained": len(mandatory),
                "authoritative_tables_and_blobs_unchanged": True}
        save()
        run("restore")
        if digest(executable) != record["executable_sha256"]:
            raise RuntimeError("Executable changed during acceptance run")
        record["outcome"] = "passed"
    except Exception as error:
        record["outcome"] = "failed"
        record["error"] = str(error)
        raise
    finally:
        record["executable_unchanged"] = digest(executable) == record["executable_sha256"]
        save()
        print(json.dumps(record))


if __name__ == "__main__":
    main()
