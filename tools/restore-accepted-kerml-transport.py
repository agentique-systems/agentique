"""Recover accepted cache transport after exact historical producer replay.

This tool cannot change acceptance authority. It permits one transformation only:
restore the receipt's immutable snapshot revision label. Every resulting payload
byte must then match the original checked-in receipt. A subsequent language-facade
restore is mandatory before packaging/distribution.
"""
import argparse
import hashlib
import json
import os
import pathlib
import re
import zipfile


def digest_hex(value):
    return bytes(value).hex()


def check_receipt(authority, candidate):
    """Require all accepted semantic and capability identities before byte work."""
    comparable = json.loads(json.dumps(candidate))
    for field in ("graph_sha256", "graph_bytes"):
        comparable["complete_overlay"][field] = authority["complete_overlay"][field]
    comparable["complete_overlay"]["identity"]["revision"] = authority["complete_overlay"]["identity"]["revision"]
    if comparable != authority:
        raise ValueError("Rematerialized receipt differs from accepted semantic/capability contract")


def restore_snapshot_label(line, original_revision, accepted_revision):
    """Modify the single explicitly named Snapshot header, preserving all bytes."""
    if not line.startswith(b'{"Snapshot":'):
        return line, False
    pattern = rb'("revision":")' + re.escape(original_revision.encode("ascii")) + rb'(")'
    changed, count = re.subn(pattern, lambda match: match[1] + accepted_revision.encode("ascii") + match[2], line)
    if count != 1:
        raise ValueError("Snapshot does not have the exact regenerated revision label")
    return changed, True


def restore(cache, candidate_receipt, authority, output):
    check_receipt(authority, candidate_receipt)
    if output.exists():
        raise ValueError("Output already exists")
    accepted = authority["complete_overlay"]
    candidate = candidate_receipt["complete_overlay"]
    temporary = output.with_name(output.name + ".partial")
    output.parent.mkdir(parents=True, exist_ok=True)
    measurements = {}
    snapshot_count = 0
    # Exclusive creation means an unrelated retained partial cannot be overwritten.
    with temporary.open("xb") as target:
        try:
            with zipfile.ZipFile(cache) as source, zipfile.ZipFile(target, "w", compression=zipfile.ZIP_DEFLATED,
                                                                  compresslevel=1, allowZip64=True) as archive:
                if sorted(source.namelist()) != ["facade.json", "kernel.jsonl"]:
                    raise ValueError("Unexpected or duplicate cache entries")
                for name in ("facade.json", "kernel.jsonl"):
                    observed = hashlib.sha256()
                    restored = hashlib.sha256()
                    observed_size = restored_size = 0
                    with source.open(name) as stream, archive.open(name, "w", force_zip64=True) as destination:
                        for line in stream:
                            observed.update(line)
                            observed_size += len(line)
                            if name == "kernel.jsonl":
                                line, snapshot = restore_snapshot_label(line, candidate["identity"]["revision"],
                                                                        accepted["identity"]["revision"])
                                snapshot_count += int(snapshot)
                            restored.update(line)
                            restored_size += len(line)
                            destination.write(line)
                    if name == "facade.json":
                        expected_hash = digest_hex(authority["facade_metadata_sha256"])
                        expected_size = authority["facade_metadata_bytes"]
                        candidate_hash = digest_hex(candidate_receipt["facade_metadata_sha256"])
                        candidate_size = candidate_receipt["facade_metadata_bytes"]
                    else:
                        expected_hash = digest_hex(accepted["graph_sha256"])
                        expected_size = accepted["graph_bytes"]
                        candidate_hash = digest_hex(candidate["graph_sha256"])
                        candidate_size = candidate["graph_bytes"]
                    if (observed.hexdigest(), observed_size) != (candidate_hash, candidate_size):
                        raise ValueError(f"{name}: input bytes differ from generated receipt")
                    if (restored.hexdigest(), restored_size) != (expected_hash, expected_size):
                        raise ValueError(f"{name}: exact original accepted payload was not reproduced")
                    measurements[name] = dict(bytes=restored_size, sha256=restored.hexdigest())
                if snapshot_count != 1:
                    raise ValueError("Expected exactly one immutable snapshot label")
            target.flush()
            os.fsync(target.fileno())
        except BaseException:
            # Leave diagnostic bytes explicitly partial; they cannot be installed.
            raise
    # A hard link promotes without ever replacing another process's output.
    os.link(temporary, output)
    temporary.unlink()
    return {"format": "agq-exact-accepted-cache-rematerialization/1",
            "publication_semantic_digest": digest_hex(accepted["identity"]["semantic_digest"]),
            "accepted_revision": accepted["identity"]["revision"],
            "regenerated_revision": candidate["identity"]["revision"],
            "original_payload_bytes_reproduced": True, "entries": measurements,
            "facade_authentication_required": True, "authority_receipt_changed": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cache", type=pathlib.Path, required=True)
    parser.add_argument("--generated-receipt", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    root = pathlib.Path(__file__).resolve().parents[1]
    authority = json.loads((root / "standards/kerml-accepted-publication.json").read_text(encoding="utf-8"))
    candidate = json.loads(args.generated_receipt.read_text(encoding="utf-8"))
    print(json.dumps(restore(args.cache, candidate, authority, args.output), indent=2))


if __name__ == "__main__":
    main()
