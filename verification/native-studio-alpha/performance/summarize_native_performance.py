"""Summarize completed native evidence without starting Studio or a runtime consumer."""

import argparse
import collections
import datetime
import hashlib
import json
import math
from pathlib import Path
import statistics


PROFILE = "agentique-modeling-view-profile/1"
CACHES = {"agentique-studio-inspector-cache/1", "agentique-studio-projection-cache/1"}
JOURNEYS = {"agentique-native-real-acceptance/1": 1, "agentique-native-real-acceptance/2": 2}
STAGES = {
    "prepare": "prepare real source-backed candidate while current remains responsive",
    "validate": "validate the retained semantic candidate",
    "commit": "commit only the validated candidate",
}
METRICS = (
    "adapter", "fixture", "frame_count", "frame_intervals_ms",
    "warmup_frame_intervals_discarded", "scene_build_ms",
    "scene_layout_routing_diff_ms", "scene_index_lookup_outliner_ms",
    "scene_background_pending", "layout_included_in_scene_build", "hit_test_us",
    "input_pipeline", "handled_input_to_next_ui_update_ms", "gpu_upload_cpu_ms",
    "gpu_uploaded_bytes", "gpu_upload_count", "gpu_instances", "scene_draw_calls",
    "visible_nodes", "total_nodes", "total_edges", "gpu_timestamp_ms",
    "gpu_timestamp_scope", "gpu_timestamp_errors", "physical_input_to_photon_ms", "note",
)
STATE = (
    "binding", "scene_revision", "selection_revision", "world", "comparison",
    "focus", "candidate_revision", "candidate_phase", "producer_completeness",
    "projection_nodes", "projection_edges", "scene_nodes", "scene_edges",
    "mutation_pending", "pending_requests", "camera",
)


def select(value, names):
    return {key: value.get(key) for key in names}


class Inputs:
    def __init__(self):
        self.artifacts = []

    def read(self, path):
        path = Path(path)
        before = path.stat()
        payload = path.read_bytes()
        after = path.stat()
        if (before.st_size, before.st_mtime_ns) != (after.st_size, after.st_mtime_ns):
            raise ValueError(f"Input changed during read: {path}")
        record = {"path": str(path.resolve()), "bytes": len(payload),
                  "sha256": hashlib.sha256(payload).hexdigest()}
        self.artifacts.append(record)
        return payload, record

    def json(self, path):
        payload, record = self.read(path)
        return json.loads(payload), record


def finite(value):
    return type(value) in (int, float) and math.isfinite(value) and value >= 0


def distribution(values):
    values = list(values)
    if not all(finite(value) for value in values):
        raise ValueError("Non-finite or negative elapsed time")
    return {"observations": len(values), "min_ms": min(values, default=None),
            "median_ms": statistics.median(values) if values else None,
            "max_ms": max(values, default=None)}


def sampled(value):
    """Keep measured windows; suppress low-count p95 rather than extrapolating."""
    if isinstance(value, list):
        return [sampled(item) for item in value]
    if not isinstance(value, dict):
        return value
    result = {key: sampled(item) for key, item in value.items()}
    if {"window_samples", "p95"} <= value.keys() and value["window_samples"] < 20:
        result["p95"] = None
        result["p95_suppressed_below_20_observations"] = True
    return result


def metrics(value):
    return sampled(select(value, METRICS)) if isinstance(value, dict) else None


def completed_process(value):
    if type(value.get("exit_code")) is not int:
        raise ValueError("A completed process receipt with integer exit_code is required")
    return value  # Preserve recorded command, identity, clock and memory boundaries verbatim.


def logs(inputs, paths):
    records, ignored, partial = [], collections.Counter(), 0
    for path in paths:
        payload, artifact = inputs.read(path)
        for number, line in enumerate(payload.splitlines(keepends=True), 1):
            if not line.endswith((b"\r", b"\n")):
                partial += 1
                continue
            try:
                record = json.loads(line)
            except (ValueError, UnicodeDecodeError):
                continue
            if not isinstance(record, dict):
                continue
            kind = record.get("format", "unidentified-json")
            if kind not in CACHES | {PROFILE}:
                ignored[kind] += 1
                continue
            elapsed = record.get("total_ms" if kind == PROFILE else "elapsed_ms")
            if not finite(elapsed):
                raise ValueError(f"Invalid completed profile duration at {path}:{number}")
            records.append({"source_sha256": artifact["sha256"], "line": number,
                            "record": record})
    groups = collections.defaultdict(list)
    for entry in records:
        record = entry["record"]
        view = record.get("view") or {}
        grouping = {**record, "kind": record.get("kind", view.get("kind")),
                    "scope": record.get("scope", view.get("graph_scope"))}
        key = tuple(grouping.get(field) for field in (
            "format", "project", "revision", "operation", "kind", "scope", "cache", "outcome"))
        groups[key].append(record)
    summaries = []
    fields = ("format", "project", "revision", "operation", "kind", "scope", "cache", "outcome")
    for key, group in sorted(groups.items(), key=lambda pair: tuple(str(v) for v in pair[0])):
        field = "total_ms" if key[0] == PROFILE else "elapsed_ms"
        summaries.append({
            **dict(zip(fields, key)), **distribution(record[field] for record in group),
            "distinct_elements_or_focuses": len({record.get("element", record.get("focus")) for record in group}),
            "distinct_full_views": len({json.dumps(record.get("view"), sort_keys=True) for record in group}),
        })
    return {
        "scope": "Independent call samples grouped by exact binding/operation/outcome. "
                 "No p95 from these limited calls. Cache misses include query work; do not sum "
                 "cache and query times. Groups may include different named objects/full views; "
                 "their distinct counts are explicit. No queue latency is inferred. Full phase records are "
                 "retained; use audit_view_profile.py for phase-specific analysis.",
        "summaries": summaries, "records": records,
        "ignored_json_formats": dict(ignored), "ignored_unterminated_lines": partial,
    }


def journey(value):
    input_format = value.get("format")
    if input_format not in JOURNEYS:
        raise ValueError("Expected known real native acceptance format /1 or /2")
    if value.get("outcome") not in {"failed", "passed", "journey_passed_restart_pending"}:
        raise ValueError("The native journey is still running or its outcome is unknown")
    assertions = value.get("assertions", [])
    steps = [{**select(step, ("name", "passed", "elapsed_ms", "since_start_ms")),
              "before": select(step.get("before", {}), STATE),
              "after": select(step.get("after", {}), STATE)} for step in assertions]
    stages = {}
    for stage, name in STAGES.items():
        matches = [step for step in steps if step["name"] == name]
        if len(matches) > 1:
            raise ValueError(f"Ambiguous repeated stage: {name}")
        stages[stage] = matches[0] if matches else None
    first = next((step for step in steps if step["passed"] and step["name"] ==
                  "open authenticated Agentique with Validated semantic closure"), None)
    background = value.get("background_current_inspection")
    if background:
        background = {key: item for key, item in background.items() if key != "inspector"}
    return {
        "input_format": input_format,
        "input_version": JOURNEYS[input_format],
        **select(value, ("scenario", "outcome", "passed", "failure", "restart_verified",
                         "project", "branch", "elapsed_ms", "previous_report_digest",
                         "resumed_baseline", "background_frames", "background_pan_observed")),
        "accepted_publications": (value.get("baseline") or {}).get("accepted_publications"),
        "engineering_evidence": value.get("engineering_evidence"),
        "version_scope": "This extraction summarizes one input journey only. Composition and "
                         "other engineering evidence are retained exactly as reported, without "
                         "filling absent legacy fields or treating /1 as composed-Studio /2 "
                         "acceptance. Supporting evidence does not upgrade its outcome or version.",
        "first_asserted_validated_view_since_runner_start_ms": first["since_start_ms"] if first else None,
        "steps": steps, "semantic_stage_ui_steps": stages,
        "stage_scope": "elapsed_ms starts when the native step is initialized before input "
                       "injection and ends at its state assertion. It includes input sequencing, "
                       "worker queue/work, UI polling/settling and follow-up projections/reads. "
                       "It is not an isolated prepare/validation/commit engine timer. Missing "
                       "stages are null; failed stages are not successful timing results.",
        "preparation_observation": value.get("preparation_responsiveness"),
        "preparation_scope": "Clock begins immediately before injected Prepare-button release; "
                             "ends at the input hook first observing a candidate and no pending "
                             "mutation. Includes queue, semantic work and native response handling; "
                             "not a compiler-only timer. Hook gaps are not frame/presentation latency.",
        "background_current_inspection": background,
        "background_read_scope": "Native selection/request observation to accepted Inspector reply; "
                                 "includes queue and polling. A cache call log excludes that queue.",
        "final_metrics": metrics(value.get("metrics")),
        "last_state": select(value.get("last_state", {}), STATE),
    }


def gallery(inputs, directory):
    output = []
    if directory is None:
        return output
    for path in sorted(Path(directory).glob("*.json")):
        shot, artifact = inputs.json(path)
        image = path.with_suffix(".png")
        image_record = inputs.read(image)[1] if image.is_file() else None
        projection, state = shot.get("projection", {}), shot.get("state", {})
        output.append({
            "sidecar": artifact, "image": image_record,
            "image_matches_sidecar": bool(image_record and image_record["sha256"] == shot.get("image_digest")),
            "fixture": shot.get("fixture"), "semantic_data": shot.get("semantic_data"),
            "state": select(state, STATE), "view": projection.get("view"),
            "projection_revision": projection.get("revision_id"),
            "scene_matches_projection_revision": bool(projection.get("revision_id") and
                state.get("scene_revision") == projection["revision_id"]),
            "projection_nodes": len(projection.get("nodes", [])),
            "projection_edges": len(projection.get("edges", [])),
            "metrics": metrics(shot.get("metrics")),
        })
    return output


def build(args):
    inputs = Inputs()
    report, _ = inputs.json(args.journey)
    process, _ = inputs.json(args.process)
    result = {
        "format": "agentique-native-performance-extraction/1",
        "observed_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "scope": "Offline extraction of completed native evidence. No standards restoration, "
                 "new native run, semantic authentication or product acceptance occurs here.",
        "recorded_launch": completed_process(process),
        "launch_identity_scope": "Source commit and executable SHA are copied from the launch "
                                 "receipt. This does not prove that a later executable/current "
                                 "checkout produced the run, or that the launch checkout was clean.",
        "journey": journey(report),
        "call_profiles": logs(inputs, args.log),
        "gallery": gallery(inputs, args.gallery),
        "gallery_metrics_scope": "Each capture retains its own latest rolling sample window, "
                                 "which may include preceding views. Windows overlap: do not pool, "
                                 "sum, or label them dedicated steady-state scene benchmarks. Scene "
                                 "build/layout values describe the most recent recorded build.",
        "stress": [], "additional_evidence": [],
        "measurement_limits": [
            "Application frame intervals are UI-update intervals, not measured presentation.",
            "GPU timestamps bracket only the custom scene pass, excluding text/chrome/upload/present.",
            "Raw/handled input metrics exclude physical device and OS-delivery latency.",
            "No submission, presentation, or photon latency is inferred from CPU intervals.",
            "Native p95 is retained only for windows with at least 20 samples; counts remain explicit.",
            "No causal speedup or quiet-machine status is inferred from different runs or scene sizes.",
        ],
    }
    for report_path, process_path in args.stress:
        stress, _ = inputs.json(report_path)
        receipt, _ = inputs.json(process_path)
        if stress.get("format") != "agentique-native-stress-v1":
            raise ValueError(f"Unknown stress report: {report_path}")
        result["stress"].append({
            "report": str(Path(report_path).resolve()), "process": completed_process(receipt),
            **select(stress, ("format", "passed", "failure", "warmup_frames", "scope", "camera_checks")),
            "phase_frame_intervals_ms": sampled(stress.get("phase_frame_intervals_ms")),
            "metrics": metrics(stress.get("native_metrics")),
        })
    for path in args.evidence:
        value, artifact = inputs.json(path)
        result["additional_evidence"].append({"artifact": artifact, "record": value})
    inputs.read(Path(__file__))
    result["artifacts"] = inputs.artifacts
    return result


def markdown(result):
    def number(value):
        return f"{value:.3f}" if finite(value) else "unmeasured"

    run = result["journey"]
    lines = ["# Native performance extraction", "", f"Input report: `{run['input_format']}` (journey version {run['input_version']}).", "",
             f"Journey outcome: **{run['outcome']}**.", "",
             "This report preserves failed/missing stages. It does not establish product acceptance.", "",
             f"Recorded source commit: `{result['recorded_launch'].get('source_commit_at_start', 'unrecorded')}`.",
             f"Recorded executable SHA256: `{result['recorded_launch'].get('executable_sha256', 'unrecorded')}`.", "",
             "## Observed semantic workflow", "", "| Native stage | Asserted | Whole UI step, ms |",
             "|---|---|---:|"]
    for name, stage in run["semantic_stage_ui_steps"].items():
        lines.append(f"| {name} | {stage['passed'] if stage else 'not observed'} | {stage['elapsed_ms'] if stage else '—'} |")
    lines.extend(["", run["stage_scope"], "", run["preparation_scope"], "",
                  f"Prepare-release to candidate observation: `{(run['preparation_observation'] or {}).get('elapsed_ms')}` ms.",
                  "", "## Call samples", "", "| Operation / cache / result | Kind / scope | Revision | n | Min ms | Median ms | Max ms |",
                  "|---|---|---|---:|---:|---:|---:|"])
    for group in result["call_profiles"]["summaries"]:
        lines.append(f"| {group['operation']} / {group['cache'] or 'query'} / {group['outcome']} | "
                     f"{group['kind'] or '—'} / {group['scope'] or '—'} | `{group['revision']}` | "
                     f"{group['observations']} | {group['min_ms']:.3f} | {group['median_ms']:.3f} | {group['max_ms']:.3f} |")
    lines.extend(["", result["call_profiles"]["scope"], "", "## Capture windows", "",
                  result["gallery_metrics_scope"], "",
                  "| Capture | Projected N/E | Scene N/E | Build ms | Layout/routing/diff ms | Frame p95 ms | Hit p95 us | Scene GPU median ms |",
                  "|---|---:|---:|---:|---:|---:|---:|---:|"])
    for capture in result["gallery"]:
        measured = capture["metrics"] or {}
        state = capture["state"]
        lines.append(f"| {Path(capture['sidecar']['path']).stem} | {capture['projection_nodes']}/{capture['projection_edges']} | "
                     f"{state['scene_nodes']}/{state['scene_edges']} | {number(measured.get('scene_build_ms'))} | "
                     f"{number(measured.get('scene_layout_routing_diff_ms'))} | "
                     f"{number((measured.get('frame_intervals_ms') or {}).get('p95'))} | "
                     f"{number((measured.get('hit_test_us') or {}).get('p95'))} | "
                     f"{number((measured.get('gpu_timestamp_ms') or {}).get('median'))} |")
    lines.extend(["", "The JSON retains exact view/revision, source sample counts, all input boundaries, GPU availability/errors, and image verification. No window is pooled with another."])
    if result["stress"]:
        lines.extend(["", "## Synthetic camera runs", "",
                      "| Fixture | Passed | Steady p95 ms | Pan p95 ms | Zoom p95 ms |",
                      "|---|---|---:|---:|---:|"])
        for stress in result["stress"]:
            phases = stress["phase_frame_intervals_ms"] or {}
            values = " | ".join(number((phases.get(phase) or {}).get("p95")) for phase in ("steady", "pan", "zoom"))
            lines.append(f"| {(stress['metrics'] or {}).get('fixture')} | {stress['passed']} | {values} |")
        lines.extend(["", "Environment and launch receipts are retained in JSON; inclusion does not assert a quiet machine or a causal comparison."])
    lines.extend(["", "## Limits", ""])
    lines.extend(f"- {limit}" for limit in result["measurement_limits"])
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--journey", required=True, type=Path)
    parser.add_argument("--process", required=True, type=Path)
    parser.add_argument("--log", action="append", default=[], type=Path)
    parser.add_argument("--gallery", type=Path)
    parser.add_argument("--stress", nargs=2, action="append", default=[], metavar=("REPORT", "PROCESS"))
    parser.add_argument("--evidence", action="append", default=[], type=Path,
                        help="Retain an existing phase audit, source/build receipt or environment record verbatim")
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--markdown", type=Path)
    args = parser.parse_args()
    for path in (args.out, args.markdown):
        if path and path.exists():
            parser.error(f"Refusing to overwrite evidence: {path}")
    try:
        result = build(args)
        with args.out.open("x", encoding="utf-8", newline="\n") as output:
            json.dump(result, output, indent=2, ensure_ascii=True)
            output.write("\n")
        if args.markdown:
            with args.markdown.open("x", encoding="utf-8", newline="\n") as output:
                output.write(markdown(result))
    except (OSError, ValueError, KeyError, TypeError) as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
