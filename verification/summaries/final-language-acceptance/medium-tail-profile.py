"""Offline aggregation of completed publication frontiers; no publication authority."""
import argparse
import collections
import hashlib
import json
import re
from pathlib import Path

TIMINGS = (
    "planning_including_query_cache_micros", "dependency_index_micros",
    "model_materialization_micros", "certificate_revalidation_micros",
    "certificate_build_micros", "elapsed_micros",
)
COUNTS = (
    "subjects_evaluated", "subjects_skipped", "subjects_reopened",
    "planned_elements", "accepted_elements", "accepted_occurrences", "next_dirty_subjects",
)


def snapshot(path):
    raw = path.read_bytes()
    lines = raw.splitlines(keepends=True)
    if lines and not lines[-1].endswith(b"\n"):
        lines.pop()  # A concurrently written final row is not a completed record.
    retained = b"".join(lines)
    rows = [json.loads(line) for line in lines if line.strip()]
    return rows, {"path": str(path), "completed_bytes": len(retained),
                  "completed_lines": len(rows), "sha256": hashlib.sha256(retained).hexdigest()}


def aggregate(rows):
    counts = collections.Counter()
    timings = collections.Counter()
    reasons = collections.Counter()
    families = {}
    certificate_delta = collections.Counter()
    valuation_shared = 0
    frontiers = []
    for row in rows:
        cumulative = row["closure_counters"]
        metrics = cumulative["round"]
        counts.update({key: metrics.get(key, 0) for key in COUNTS})
        certificate_delta.update(row.get("certificate_delta", {}))
        timings.update({key: metrics.get(key, 0) for key in TIMINGS})
        reasons.update(metrics["next_dirty_by_reason"])
        for name, data in metrics["families"].items():
            family = families.setdefault(name, {"attempts": 0, "planning_micros": 0,
                                                "shared_planning_micros": 0,
                                                "reopened_by_reason": collections.Counter()})
            for key in ("attempts", "planning_micros", "shared_planning_micros"):
                family[key] += data[key]
            family["reopened_by_reason"].update(data["reopened_by_reason"])
        value = metrics["families"].get("KerML.FeatureValue", {})
        valuation = metrics["families"].get("KerML.FeatureValuation", {})
        assert value.get("shared_planning_micros", 0) == valuation.get("shared_planning_micros", 0)
        valuation_shared += value.get("shared_planning_micros", 0)
        measured = sum(metrics.get(key, 0) for key in TIMINGS if key != "elapsed_micros")
        assert metrics["elapsed_micros"] >= measured, "reported timing categories overlap"
        frontiers.append({"stage": row["stage"], "stratum": row["stratum"],
                          "completeness": row["completeness"], "elapsed_seconds": row["elapsed_seconds"],
                          "closed_pairs": cumulative["closed_pairs"],
                          "applicable_pairs": cumulative["applicable_subject_family_pairs"],
                          "incomplete_pairs": cumulative["incomplete_pairs"],
                          "closed_requirements": cumulative["closed_requirements"],
                          **{key: metrics.get(key, 0) for key in COUNTS},
                          **{key: metrics.get(key, 0) for key in TIMINGS},
                          "next_dirty_by_reason": metrics["next_dirty_by_reason"],
                          "unattributed_micros": metrics["elapsed_micros"] - measured})
    return {"frontier_count": len(rows), "counts": dict(counts), "timings": dict(timings),
            "unattributed_micros": timings["elapsed_micros"] - sum(timings[k] for k in TIMINGS if k != "elapsed_micros"),
            "next_dirty_by_reason": dict(reasons), "families": families,
            "certificate_delta": dict(certificate_delta),
            "feature_value_valuation_shared_micros_counted_once": valuation_shared,
            "frontiers": frontiers}



def render(profile):
    lines = ["# Measured medium scheduler tail", "",
             "Observational profile of completed frontiers; this document does not establish publication acceptance.",
             "The zero-based stage threshold is 15. Counter resets identify separate invocations; JSONL row ordinals are never treated as rounds.", "",
             "| Invocation | Phase | Completed stages | Tail frontiers |",
             "| --- | --- | --- | ---: |"]
    for inv in profile["invocations"]:
        lines.append(f"| {inv['ordinal']} | {inv['phase']} | {inv['stage_start']}..{inv['stage_end']} | {inv['tail']['frontier_count']} |")
    latest = profile["invocations"][-1]
    latest_tail = latest["tail"]["frontiers"]
    if latest_tail:
        last = latest_tail[-1]
        lines += ["", f"This snapshot ends at invocation {latest['ordinal']}, {latest['phase']} stage {last['stage']} ({last['stratum']}, {last['completeness']}). It includes only completed measurements available at that point; later frontiers and the independent acceptance report are outside this snapshot."]
    lines += ["", "## Timing by invocation and stratum", "",
              "All durations are seconds. Certificate timing includes incremental issuance, the full reference rebuild, and exact comparison in this verification run. It is not the production-only incremental cost.", "",
              "| Invocation / stratum | Stages | Total | Planning incl. queries/cache | Certificate | Materialization | Read indexing | Certificate revalidation | Other |",
              "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"]
    for inv in profile["invocations"]:
        for stratum, data in inv["tail_by_stratum"].items():
            t = data["timings"]; f = data["frontiers"]
            values = [t[k]/1e6 for k in ("elapsed_micros", "planning_including_query_cache_micros", "certificate_build_micros", "model_materialization_micros", "dependency_index_micros", "certificate_revalidation_micros")]
            values.append(data["unattributed_micros"]/1e6)
            lines.append(f"| {inv['ordinal']} / {stratum} | {f[0]['stage']}..{f[-1]['stage']} | " + " | ".join(f"{v:.3f}" for v in values) + " |")
    lines += ["", "Other time includes uninstrumented round work such as context construction and checkpoint capture. No distinct query/cache timer exists; queries and cache work remain inside planning.", "",
              "## Measured producer costs", "",
              "Exclusive family blocks can be ranked against planning. Shared values are inclusive callback attribution: they overlap across participating families and must not be summed."]
    for inv in profile["invocations"]:
        for stratum, data in inv["tail_by_stratum"].items():
            lines += ["", f"### Invocation {inv['ordinal']}, {stratum}", "",
                      "| Family | Attempts | Exclusive seconds | Overlapping shared seconds |",
                      "| --- | ---: | ---: | ---: |"]
            ranked = sorted(data["families"].items(), key=lambda item: item[1]["planning_micros"], reverse=True)
            selected = [(n,m) for n,m in ranked[:6] if m["planning_micros"]]
            shared = sorted(data["families"].items(), key=lambda item: item[1]["shared_planning_micros"], reverse=True)
            selected += [(n,m) for n,m in shared[:3] if m["shared_planning_micros"] and n not in {k for k,_ in selected}]
            for name, family in selected:
                lines.append(f"| {name} | {family['attempts']} | {family['planning_micros']/1e6:.3f} | {family['shared_planning_micros']/1e6:.3f} |")
            lines += ["", f"The combined FeatureValue/FeatureValuation block costs {data['feature_value_valuation_shared_micros_counted_once']/1e6:.3f}s, counted once. Neither family's share is independently measured."]
            c=data["counts"]; d=data["certificate_delta"]
            lines += ["", f"Subjects evaluated/skipped/reopened: {c['subjects_evaluated']}/{c['subjects_skipped']}/{c['subjects_reopened']}. Planned element outputs: {c['planned_elements']}; accepted elements/occurrences: {c['accepted_elements']}/{c['accepted_occurrences']}."]
            lines += ["", "| Next-frontier reason | Subject enqueues |", "| --- | ---: |"]
            for reason,count in sorted(data["next_dirty_by_reason"].items(),key=lambda item:item[1],reverse=True):
                lines.append(f"| {reason} | {count} |")
            if d:
                lines += ["", f"Certificate row work across these frontiers: topology rebuilt {d['topology_rebuilt']}, retained {d['topology_retained']}; producer scope rebuilt {d['scope_rebuilt']}, retained {d['scope_retained']}. Retained counts are per-frontier reuse events, not distinct rows."]
    lines += ["", "Reason categories overlap. Graph/provider labels describe conservative invalidation keys, not independently proved changed answers. Family-specific reason counts are in the JSON artifact; a subject reopening evaluates its applicable family blocks together.", "",
              "## Retained writer and read guards", "",
              "No scope was narrowed and no running executable was changed for this analysis.", "",
              "| Family | Existing scope / correctness boundary retained |",
              "| --- | --- |",
              "| VariableFeaturing | Subject plus owning Type; Featuring is subject-only. Snapshot selection reads effective owner features, redefinitions, featuring and the negative candidate population; creating a snapshot requires complete evidence. |",
              "| PositionalRedefinition | Subject-only Redefinition. Result/parameter/end role, canonical ownership, inherited ordered populations and explicit invocation redefinitions remain prerequisites. |",
              "| FeatureValue / FeatureValuation | Separate binding and valuation contracts share one implementation timer. Binding is contextual and subject-scoped; complete featuring domains and provider reads remain required. |",
              "| FeatureReference | Contextual subject binding with actual referent/raw-result/domain evidence; declared referent identity is preserved. |",
              "| Expression / Function result | Subject scope, explicit relationship classes and fresh end population; no FeatureValue carrier or retyping of the subject. |",
              "| Feature-chain result | Subject plus selected owned Features, with direct result/source-target selection and existing first-input membership support. |",
              "| Index / Select result | Subject plus owned results for operational profiles; exact ownership, Array guard and result evidence remain required. |",
              "| deriveUsageMayTimeVary | StableProperties; only the subject's mayTimeVary/isVariable scalars. Positive selected canonical paths and complete negative ownership/typing/exclusion evidence are retained. |", "",
              "Contracts are defined in [producer_worklist.rs](../../../crates/kerml-semantics/src/producer_worklist.rs), [result_structure.rs](../../../crates/kerml-semantics/src/result_structure.rs), [implicit.rs](../../../crates/kerml-semantics/src/implicit.rs), and [SysML producers](../../../crates/sysml-semantics/src/producers.rs). Existing scope counterexamples remain in the producer closure/worklist suites; this observational run adds no semantic claim beyond those guards.", "",
              "## Reproduction and limits", "",
              f"Source prefix: {profile['source']['completed_lines']} completed rows, {profile['source']['completed_bytes']} bytes, SHA-256 `{profile['source']['sha256']}`. The source may continue growing; the byte prefix makes this snapshot independently reproducible.", "",
              "`medium-tail-profile.py` reads only stage JSONL and watchdog logs. It validates per-round sums against each invocation's cumulative evaluated/skipped/reopened/certificate counters, rejects overlapping timing categories, checks paired shared timers, and matches certificate trace records to completed phase/stage records. The tracked JSON preserves per-frontier measurements and per-family reopen aggregates."]
    if profile.get("watchdog",{}).get("last"):
        w=profile["watchdog"]
        lines += ["", f"Watchdog at snapshot: elapsed {w['last']['elapsed_seconds']:.3f}s, peak private memory {w['peak_private_bytes']/2**30:.3f} GiB, minimum disk reserve {w['minimum_free_disk_bytes']/2**30:.3f} GiB. These are resource observations, not acceptance evidence."]
    return "\n".join(lines)+"\n"

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("stages", type=Path)
    parser.add_argument("--watchdog", type=Path)
    parser.add_argument("--output-log", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--markdown", type=Path)
    args = parser.parse_args()
    rows, source = snapshot(args.stages)
    trace_source = None
    if args.output_log:
        raw = args.output_log.read_bytes()
        trace_source = {"path": str(args.output_log), "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}
        pending = collections.Counter()
        matched = 0
        for line in raw.decode("utf-8", errors="replace").splitlines():
            delta = re.search(r"certificate delta: topology rows rebuilt=(\d+) retained=(\d+), scope rows rebuilt=(\d+) retained=(\d+)", line)
            if delta:
                pending.update(dict(zip(("topology_rebuilt", "topology_retained", "scope_rebuilt", "scope_retained"), map(int, delta.groups()))))
            frontier = re.search(r"Systems(?P<publication> publication)?: (?:producer )?frontier=(\d+) ", line)
            if frontier and matched < len(rows):
                row = rows[matched]
                assert row["stage"] == int(frontier.group(2))
                assert row["phase"] == ("publication" if frontier.group("publication") else "construction")
                row["certificate_delta"] = dict(pending)
                pending.clear()
                matched += 1
        trace_source["matched_frontiers"] = matched
    invocations = []
    previous = None
    for row in rows:
        # A JSONL row ordinal is not a scheduler round. Each outer preparation
        # pass restarts stage 0 and cumulative counters; publication is distinct.
        if previous is None or row["phase"] != previous["phase"] or row["stage"] <= previous["stage"]:
            invocations.append([])
        invocations[-1].append(row)
        previous = row
    result = {"source": source, "stage_threshold": 15,
              "interpretation": {"stage": "zero-based scheduler frontier, resets per invocation",
                                 "shared_family_time": "inclusive; never sum across families",
                                 "reopen_reasons": "overlapping invalidation-key categories; not proved changed semantic answers",
                                 "query_cache_time": "included in planning; no distinct exclusive timer",
                                 "certificate_time": "includes full oracle plus exact comparison when AGQ_CERTIFICATE_VERIFY_FULL_REBUILD is set"},
              "invocations": []}
    for number, invocation in enumerate(invocations):
        if invocation[0]["stage"] == 0:
            for per_round, cumulative in (("subjects_evaluated", "subjects_evaluated"),
                                          ("subjects_skipped", "subjects_skipped_by_applicability"),
                                          ("subjects_reopened", "dirty_reevaluations"),
                                          ("certificate_build_micros", "certificate_build_micros")):
                assert sum(row["closure_counters"]["round"][per_round] for row in invocation) == invocation[-1]["closure_counters"][cumulative]
        tail = [row for row in invocation if row["stage"] >= 15]
        all_metrics = aggregate(invocation)
        # Keep early passes compact; their individual frames remain in the
        # hashed source prefix and are not part of the requested tail ranking.
        all_metrics.pop("frontiers")
        all_metrics.pop("families")
        result["invocations"].append({"ordinal": number, "phase": invocation[0]["phase"],
                                      "stage_start": invocation[0]["stage"], "stage_end": invocation[-1]["stage"],
                                      "all": all_metrics,
                                      "tail": aggregate(tail),
                                      "tail_by_stratum": {stratum: aggregate([row for row in tail if row["stratum"] == stratum]) for stratum in sorted({row["stratum"] for row in tail})}})
    if trace_source:
        result["certificate_trace_source"] = trace_source
    if args.watchdog:
        observations, watchdog_source = snapshot(args.watchdog)
        result["watchdog"] = {"source": watchdog_source,
                               "last": observations[-1] if observations else None,
                               "peak_private_bytes": max((r.get("peak_private_bytes", 0) for r in observations), default=0),
                               "minimum_free_disk_bytes": min((r["free_disk_bytes"] for r in observations), default=0)}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if args.markdown:
        args.markdown.write_text(render(result), encoding="utf-8")
    print(json.dumps({"completed_rows": len(rows), "invocations": [
        {"ordinal": r["ordinal"], "phase": r["phase"], "stages": [r["stage_start"], r["stage_end"]],
         "tail_frontiers": r["tail"]["frontier_count"]} for r in result["invocations"]]}, indent=2))


if __name__ == "__main__":
    main()
