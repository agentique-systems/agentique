"""Prepare a review candidate; never register transport or accept a runtime."""
import argparse
import hashlib
import json
import pathlib
import uuid
import zipfile

from compare_systems_contract import compare_contract, exact_json


ROOT = pathlib.Path(__file__).resolve().parents[2]


def prepare(cache, authority_raw, generated_raw, original_bindings, new_bindings,
            transport_id):
    authority = json.loads(authority_raw)
    generated = json.loads(generated_raw)
    contract = compare_contract(authority, generated, original_bindings, new_bindings)
    if not all(contract[field] for field in (
            "semantic_contract_equal", "accepted_bindings_equal", "transport_entry_schema_equal")):
        raise ValueError("Existing accepted semantic contract or bindings differ")
    if not transport_id.strip():
        raise ValueError("Transport identifier must not be empty")
    for name, entry in generated["entries"].items():
        original = authority["entries"][name]
        if entry["bytes"] != original["bytes"] or (
                name != "kernel.jsonl" and not exact_json(entry, original)):
            raise ValueError(f"{name}: transport exceeds the reviewed entry boundary")
    snapshot_revision = None
    observed_entries = {}
    # One handle binds outer transport and decoded entries to the same file even
    # if another process replaces its path. A second hash rejects mutation while
    # it is being inspected; later authentication still checks the actual input.
    with cache.open("rb") as stream:
        archive_digest = hashlib.sha256()
        archive_bytes = 0
        while chunk := stream.read(1024 * 1024):
            archive_digest.update(chunk)
            archive_bytes += len(chunk)
        stream.seek(0)
        with zipfile.ZipFile(stream) as archive:
            if sorted(archive.namelist()) != sorted(generated["entries"]):
                raise ValueError("Unexpected or duplicate cache entries")
            for name, expected in sorted(generated["entries"].items()):
                if archive.getinfo(name).file_size != expected["bytes"]:
                    raise ValueError(f"{name}: entry byte count differs")
                digest = hashlib.sha256()
                count = 0
                with archive.open(name) as source:
                    for line in source:
                        count += len(line)
                        if count > expected["bytes"]:
                            raise ValueError(f"{name}: decoded entry exceeds bound")
                        digest.update(line)
                        if name == "kernel.jsonl" and line.startswith(b'{"Snapshot":'):
                            if snapshot_revision is not None:
                                raise ValueError("Multiple snapshot labels")
                            snapshot_revision = json.loads(line)["Snapshot"]["revision"]
                            if str(uuid.UUID(snapshot_revision)) != snapshot_revision:
                                raise ValueError("Invalid snapshot label")
                observed = {"bytes": count, "sha256": list(digest.digest())}
                if not exact_json(observed, expected):
                    raise ValueError(f"{name}: actual bytes differ from generated receipt")
                observed_entries[name] = observed
        stream.seek(0)
        if hashlib.file_digest(stream, "sha256").digest() != archive_digest.digest():
            raise ValueError("Cache changed while transport evidence was prepared")
    if snapshot_revision is None:
        raise ValueError("Missing snapshot label")
    authority_digest = hashlib.sha256(authority_raw).digest()
    receipt = {
        "format": "agq-publication-transport/1",
        "transport_id": transport_id,
        "publication_catalogue_id": "sysml-systems-operational-v3",
        "semantic_authority_sha256": list(authority_digest),
        "identity": authority["identity"],
        "entries": observed_entries,
    }
    evidence = {
        "format": "agq-rematerialized-transport-candidate/1",
        "runtime_accepted": False,
        "ordinary_facade_authentication_required": True,
        "authority_receipt_changed": False,
        "transport_id": transport_id,
        "original_semantic_receipt_sha256": authority_digest.hex(),
        "generated_receipt_sha256": hashlib.sha256(generated_raw).hexdigest(),
        "cache_bytes": archive_bytes,
        "cache_sha256": archive_digest.hexdigest(),
        "snapshot_revision": snapshot_revision,
        "contract": contract,
        "entries": {name: {"bytes": entry["bytes"],
                           "sha256": bytes(entry["sha256"]).hex(),
                           "original_bytes_equal": exact_json(entry, authority["entries"][name])}
                    for name, entry in observed_entries.items()},
    }
    return receipt, evidence


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cache", required=True, type=pathlib.Path)
    parser.add_argument("--generated-receipt", required=True, type=pathlib.Path)
    parser.add_argument("--generated-bindings", required=True, type=pathlib.Path)
    parser.add_argument("--transport-id", required=True)
    parser.add_argument("--output", required=True, type=pathlib.Path)
    args = parser.parse_args()
    receipt, evidence = prepare(
        args.cache, (ROOT / "standards/sysml-accepted-publication.json").read_bytes(),
        args.generated_receipt.read_bytes(),
        json.loads((ROOT / "standards/sysml-standard-bindings.json").read_bytes()),
        json.loads(args.generated_bindings.read_bytes()), args.transport_id)
    # A fresh directory distinguishes this unauthenticated review candidate from
    # retained results. No code writes into standards or changes the catalogue.
    args.output.mkdir(parents=True, exist_ok=False)
    for name, content in (("transport.json", receipt), ("content.json", evidence)):
        with (args.output / name).open("x", encoding="utf-8", newline="\n") as stream:
            json.dump(content, stream, indent=2)
            stream.write("\n")
    print(json.dumps(evidence), flush=True)


if __name__ == "__main__":
    main()
