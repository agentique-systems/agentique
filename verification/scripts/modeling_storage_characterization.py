"""Read-only, bounded-memory compression measurements of an exact Gen2 cache blob.

Run from any directory; the database must already exist. The selected SHA-256
must belong to a semantic-cache blob. No source graph or large JSON array is
decoded, and compressed output is counted without writing an artifact.
"""

import argparse
import ctypes
import hashlib
import json
import platform
import re
import sqlite3
import sys
import time
import zlib
from ctypes import wintypes
from pathlib import Path


CHUNK_BYTES = 1_048_576


def process_memory():
    """Windows process counters; other platforms report unavailable, not zero."""
    if sys.platform != "win32":
        return None

    class Counters(ctypes.Structure):
        _fields_ = [("cb", wintypes.DWORD), ("PageFaultCount", wintypes.DWORD)] + [
            (name, ctypes.c_size_t)
            for name in (
                "PeakWorkingSetSize", "WorkingSetSize", "QuotaPeakPagedPoolUsage",
                "QuotaPagedPoolUsage", "QuotaPeakNonPagedPoolUsage",
                "QuotaNonPagedPoolUsage", "PagefileUsage", "PeakPagefileUsage",
            )
        ]

    current = ctypes.windll.kernel32.GetCurrentProcess
    current.restype = wintypes.HANDLE
    read = ctypes.windll.psapi.GetProcessMemoryInfo
    read.argtypes = [wintypes.HANDLE, ctypes.POINTER(Counters), wintypes.DWORD]
    read.restype = wintypes.BOOL
    counters = Counters()
    counters.cb = ctypes.sizeof(counters)
    if not read(current(), ctypes.byref(counters), counters.cb):
        raise ctypes.WinError()
    return {
        "working_set_bytes": counters.WorkingSetSize,
        "peak_working_set_bytes": counters.PeakWorkingSetSize,
        "peak_pagefile_bytes": counters.PeakPagefileUsage,
    }


def measure(connection, rowid, digest, raw_bytes, level):
    compressor = zlib.compressobj(level)
    read_seconds = compression_seconds = 0.0
    compressed_bytes = source_bytes = 0
    checksum = hashlib.sha256()
    started = time.perf_counter()
    with connection.blobopen("blobs", "bytes", rowid, readonly=True) as blob:
        while True:
            stamp = time.perf_counter()
            chunk = blob.read(CHUNK_BYTES)
            read_seconds += time.perf_counter() - stamp
            if not chunk:
                break
            source_bytes += len(chunk)
            checksum.update(chunk)
            stamp = time.perf_counter()
            compressed_bytes += len(compressor.compress(chunk))
            compression_seconds += time.perf_counter() - stamp
        stamp = time.perf_counter()
        compressed_bytes += len(compressor.flush())
        compression_seconds += time.perf_counter() - stamp
    if source_bytes != raw_bytes or checksum.hexdigest() != digest:
        raise ValueError("cache blob size or SHA-256 does not match its identity")
    return {
        "codec": "zlib-deflate", "level": level, "raw_bytes": source_bytes,
        "compressed_bytes": compressed_bytes,
        "raw_to_compressed_ratio": round(source_bytes / compressed_bytes, 2),
        "size_reduction_percent": round(100 * (1 - compressed_bytes / source_bytes), 2),
        "compression_seconds": round(compression_seconds, 3),
        "sqlite_read_seconds": round(read_seconds, 3),
        "wall_seconds": round(time.perf_counter() - started, 3),
        "source_sha256": digest, "memory": process_memory(),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--database", required=True, type=Path)
    parser.add_argument("--blob-digest", required=True)
    parser.add_argument("--levels", nargs="+", type=int, choices=range(1, 10), default=[1, 6])
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{64}", args.blob_digest):
        parser.error("--blob-digest must be a canonical lowercase SHA-256 digest")
    path = args.database.resolve(strict=True)
    connection = sqlite3.connect(path.as_uri() + "?mode=ro", uri=True)
    try:
        connection.execute("PRAGMA query_only=ON")
        connection.execute("BEGIN")
        selected = connection.execute(
            "SELECT b.rowid,length(b.bytes) FROM blobs b WHERE b.digest=? "
            "AND EXISTS(SELECT 1 FROM semantic_caches c WHERE c.blob_digest=b.digest)",
            (args.blob_digest,),
        ).fetchone()
        if selected is None or selected[1] == 0:
            raise ValueError("the requested nonempty semantic-cache blob is absent")
        wal = Path(str(path) + "-wal")
        report = {
            "format": "agentique-modeling-storage-characterization/1",
            "database": str(path), "selected_cache_sha256": args.blob_digest,
            "tools": {"python": platform.python_version(), "sqlite": sqlite3.sqlite_version,
                      "zlib_compile": zlib.ZLIB_VERSION, "zlib_runtime": zlib.ZLIB_RUNTIME_VERSION},
            "read_only": True, "chunk_bytes": CHUNK_BYTES,
            "inventory": {
                "database_file_bytes": path.stat().st_size,
                "wal_file_bytes": wal.stat().st_size if wal.exists() else 0,
                "page_size_bytes": connection.execute("PRAGMA page_size").fetchone()[0],
                "page_count": connection.execute("PRAGMA page_count").fetchone()[0],
                "freelist_pages": connection.execute("PRAGMA freelist_count").fetchone()[0],
                "total_blob_payload_bytes": connection.execute("SELECT SUM(length(bytes)) FROM blobs").fetchone()[0],
                "tables": {
                    table: connection.execute("SELECT COUNT(*) FROM " + table).fetchone()[0]
                    for table in ("projects", "branches", "revisions", "revision_documents",
                                  "source_revisions", "blobs", "semantic_caches")
                },
            },
            "baseline_memory": process_memory(),
            "results": [],
            "limitations": [
                "Repository totals reflect this read snapshot; selected cache identity is fixed.",
                "Reads can be warm; this does not measure cold semantic restoration.",
                "Record concurrent workloads separately. Peak memory covers this Python process.",
                "Compression uses zlib framing, not a service codec; no decompression is measured.",
            ],
        }
        for level in args.levels:
            report["results"].append(measure(connection, *selected[:1], args.blob_digest, selected[1], level))
        print(json.dumps(report, indent=2))
    finally:
        connection.close()


if __name__ == "__main__":
    main()
