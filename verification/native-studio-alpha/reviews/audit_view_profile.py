"""Read a fixed prefix of native stderr once; report actual view timing records only."""
import argparse
import collections
import datetime
import hashlib
import json
import pathlib
import statistics

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("log", type=pathlib.Path)
parser.add_argument("--gallery", type=pathlib.Path)
args = parser.parse_args()
names = collections.defaultdict(set)
gallery_records = []
if args.gallery and args.gallery.is_dir():
    for path in args.gallery.glob("*.json"):
        try:
            shot = json.loads(path.read_bytes())
        except (ValueError, UnicodeDecodeError):
            continue
        binding = shot.get("state", {}).get("binding") or {}
        projection = shot.get("projection", {})
        image = path.with_suffix(".png")
        image_hash = hashlib.sha256(image.read_bytes()).hexdigest() if image.is_file() else None
        gallery_records.append({
            "metadata": str(path.resolve()), "image": str(image.resolve()),
            "image_sha256": image_hash,
            "image_matches_record": image_hash is not None and image_hash == shot.get("image_digest"),
            "fixture": shot.get("fixture"), "semantic_data": shot.get("semantic_data"),
            "binding": binding, "projection_revision": projection.get("revision_id"),
            "projection_nodes": len(projection.get("nodes", [])),
            "projection_edges": len(projection.get("edges", [])),
            "scene_nodes": shot.get("state", {}).get("scene_nodes"),
            "scene_edges": shot.get("state", {}).get("scene_edges"),
            "camera": shot.get("state", {}).get("camera"),
            "scene_build_ms": shot.get("metrics", {}).get("scene_build_ms"),
        })
        if binding.get("revision") != projection.get("revision_id"):
            continue
        for node in projection.get("nodes", []):
            if node.get("revision_id") == projection.get("revision_id"):
                names[(binding.get("project"), node["revision_id"], node["id"])].add(node["name"])
payload = args.log.read_bytes()
# An actively written final line is not a completed observation.
lines = payload.splitlines(keepends=True)
records = []
for line_number, line in enumerate(lines, 1):
    if not line.endswith((b"\n", b"\r")):
        continue
    try:
        record = json.loads(line)
    except (ValueError, UnicodeDecodeError):
        continue
    if not isinstance(record, dict) or record.get("format") != "agentique-modeling-view-profile/1":
        continue
    phases = record["phases"]
    contexts = [
        phase for phase in phases
        if phase["name"] in {
            "connector_kerml_context", "focused_kerml_context",
            "kerml_context", "sysml_profile_context", "sysml_context",
        }
    ]
    context_ms = sum(phase["elapsed_ms"] for phase in contexts)
    focus_names = names.get((record["project"], record["revision"], record["focus"]), set())
    records.append({
        "line": line_number,
        "operation": record["operation"],
        "project": record["project"],
        "revision": record["revision"],
        "focus": record["focus"],
        "focus_names_in_exact_revision_gallery": sorted(focus_names),
        "kind": record["kind"],
        "scope": record["scope"],
        "outcome": record["outcome"],
        "total_ms": record["total_ms"],
        "context_ms": context_ms,
        "context_fraction": context_ms / record["total_ms"] if record["total_ms"] else None,
        "phases": phases,
        "sizes": record["sizes"],
        "view": record["view"],
    })
groups = collections.defaultdict(list)
for record in records:
    groups[(record["revision"], record["operation"], record["kind"])].append(record)
summary = []
for (revision, operation, kind), group in sorted(groups.items()):
    successful = [record for record in group if record["outcome"] == "ok"]
    summary.append({
        "revision": revision, "operation": operation, "kind": kind,
        "observations": len(group), "successful": len(successful),
        "total_ms_min": min((record["total_ms"] for record in successful), default=None),
        "total_ms_median": statistics.median(record["total_ms"] for record in successful) if successful else None,
        "total_ms_max": max((record["total_ms"] for record in successful), default=None),
        "context_ms_median": statistics.median(record["context_ms"] for record in successful) if successful else None,
        "context_fraction_median": statistics.median(record["context_fraction"] for record in successful if record["context_fraction"] is not None) if successful else None,
    })
print(json.dumps({
    "format": "agentique-view-profile-read-audit/1",
    "observed_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "source": str(args.log.resolve()),
    "source_prefix_bytes": len(payload),
    "source_prefix_sha256": hashlib.sha256(payload).hexdigest(),
    "complete_profile_records": len(records),
    "scope": "A single read-only byte-prefix snapshot, possibly while the process runs. Context phases are non-overlapping within each call; inclusive parent phases are not summed. Per-call wall times are not frame/input/GPU latency. No runtime authentication or product acceptance is established by this audit.",
    "summary": summary, "records": records, "gallery": gallery_records,
}, indent=2, ensure_ascii=True))
