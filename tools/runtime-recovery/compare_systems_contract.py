"""Compare a rebuilt Systems candidate to existing authority; never accept it."""
import argparse
import json
import pathlib


ROOT = pathlib.Path(__file__).resolve().parents[2]


def compare_contract(accepted, generated, original_bindings, new_bindings):
    """Only archive transport entry bytes may differ from the accepted contract."""
    expected = {key: value for key, value in accepted.items() if key != "entries"}
    actual = {key: value for key, value in generated.items() if key != "entries"}
    fields = {key: actual.get(key) == expected.get(key)
              and (key in actual) == (key in expected)
              for key in sorted(expected.keys() | actual.keys())}
    original_entries = accepted.get("entries", {})
    new_entries = generated.get("entries", {})
    entries_valid = (isinstance(original_entries, dict) and isinstance(new_entries, dict)
                     and set(original_entries) == set(new_entries)
                     == {"closure.json", "facade.json", "kernel.jsonl"})
    for entry in new_entries.values() if isinstance(new_entries, dict) else ():
        entries_valid = entries_valid and (
            isinstance(entry, dict) and set(entry) == {"bytes", "sha256"}
            and type(entry["bytes"]) is int and entry["bytes"] > 0
            and isinstance(entry["sha256"], list) and len(entry["sha256"]) == 32
            and all(type(value) is int and 0 <= value <= 255 for value in entry["sha256"]))
    return {
        "semantic_contract_equal": expected == actual,
        "semantic_fields": fields,
        "accepted_bindings_equal": original_bindings == new_bindings,
        "transport_entry_schema_equal": entries_valid,
        "original_transport_entries_equal": original_entries == new_entries,
        "ordinary_facade_authentication_required": True,
        "accepted_receipt_changed": False,
        "runtime_accepted": False,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--generated-receipt", required=True, type=pathlib.Path)
    parser.add_argument("--generated-bindings", required=True, type=pathlib.Path)
    parser.add_argument("--output", required=True, type=pathlib.Path)
    args = parser.parse_args()
    result = compare_contract(
        json.loads((ROOT / "standards/sysml-accepted-publication.json").read_text()),
        json.loads(args.generated_receipt.read_text()),
        json.loads((ROOT / "standards/sysml-standard-bindings.json").read_text()),
        json.loads(args.generated_bindings.read_text()))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("x", encoding="utf-8") as output:
        json.dump(result, output, indent=2)
    print(json.dumps(result), flush=True)
    if not all(result[key] for key in (
            "semantic_contract_equal", "accepted_bindings_equal", "transport_entry_schema_equal")):
        raise SystemExit("Existing Systems contract mismatch; runtime acceptance paused")


if __name__ == "__main__":
    main()
