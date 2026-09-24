"""Profile complete observational AuditLog journals; never grant semantic authority.

The current Rust runner uses ordered windows of one or two eight-subject batches.
Full finalization has an enclosing effective-audit stage; scoped audits log only
batches, so their overall timing is explicitly the observed batch event span.
"""
import argparse
import hashlib
import json
from pathlib import Path
from statistics import median
import sys
from uuid import UUID


FORMAT = "agq-systems-finalization-audit/1"
BATCH = "effective_sysml_population_batch"
EFFECTIVE = "effective_sysml_population_audit"
BATCH_SIZE = 8
PROFILE_FORMAT = "agq-systems-effective-audit-profile/1"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def natural(value, label):
    require(type(value) is int and value >= 0, f"invalid {label}")
    return value


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, f"duplicate JSON field: {key}")
        result[key] = value
    return result


def subject(value):
    require(isinstance(value, str), "subject identity must be a UUID string")
    identity = UUID(value)
    require(str(identity) == value, "subject identity must be canonical")
    return identity.int


def profile(raw, journal="<memory>"):
    require(raw and raw.endswith(b"\n"), "empty or truncated journal: missing final newline")
    events = []
    for line_number, line in enumerate(raw.decode("utf-8").splitlines(), 1):
        require(line.strip(), f"empty journal line {line_number}")
        event = json.loads(line, object_pairs_hook=unique_object)
        require(isinstance(event, dict), f"journal line {line_number} is not an object")
        require({"format", "publication_authority", "stage", "event", "elapsed_micros",
                 "run_elapsed_micros", "findings"} <= event.keys(), f"missing event fields at line {line_number}")
        require(event.get("format") == FORMAT and event.get("publication_authority") is False,
                f"invalid observational journal contract at line {line_number}")
        require(isinstance(event.get("stage"), str) and event["stage"]
                and event.get("event") in ("begin", "end"), f"invalid event at line {line_number}")
        natural(event.get("findings"), "findings")
        natural(event.get("run_elapsed_micros"), "run timestamp")
        require(not events or event["run_elapsed_micros"] >= events[-1]["run_elapsed_micros"],
                "run timestamps move backwards")
        events.append(event)

    starts, ends, stages, batch_sequence = {}, {}, {}, []
    active_stage = None
    workers = total = None
    for event in events:
        stage, phase = event["stage"], event["event"]
        if stage != BATCH:
            natural(event.get("elapsed_micros"), "stage elapsed time")
            if phase == "begin":
                require(active_stage is None and stage not in stages, "duplicate or overlapping stage begin")
                require(event["elapsed_micros"] == 0 and event["findings"] == 0,
                        "stage begin contains completion values")
                stages[stage] = [event]
                active_stage = stage
            else:
                require(active_stage == stage, "unmatched or mismatched stage end")
                stages[stage].append(event)
                active_stage = None
            continue
        require(active_stage in (None, EFFECTIVE), "batch outside effective audit stage")
        index = natural(event.get("batch_index"), "batch index")
        size = natural(event.get("subjects"), "batch subject count")
        event_total = natural(event.get("total_subjects"), "total subject count")
        event_workers = natural(event.get("workers"), "workers")
        require(event_workers in (1, 2), "unsupported worker policy")
        require(0 < size <= BATCH_SIZE and event_total >= size, "invalid batch subject population")
        if workers is None:
            workers, total = event_workers, event_total
        require((workers, total) == (event_workers, event_total), "worker or subject population drift")
        first, last = subject(event.get("first_subject")), subject(event.get("last_subject"))
        require((first == last if size == 1 else first < last), "inconsistent batch endpoint range")
        bucket = starts if phase == "begin" else ends
        require(index not in bucket, f"duplicate batch {phase}: {index}")
        if phase == "begin":
            require(event.get("elapsed_micros") is None and event["findings"] == 0,
                    "batch begin contains completion values")
        else:
            natural(event.get("elapsed_micros"), "batch elapsed time")
            require(index in starts, f"unmatched batch end: {index}")
        bucket[index] = event
        batch_sequence.append((phase, index))

    require(active_stage is None, f"unclosed stage: {active_stage}")
    require(starts, "no effective batch observations; population and worker timing are unavailable")
    require(starts.keys() == ends.keys(),
            f"unmatched batch pairs: missing ends {sorted(starts.keys() - ends.keys())}, "
            f"missing begins {sorted(ends.keys() - starts.keys())}")
    count = (total + BATCH_SIZE - 1) // BATCH_SIZE
    require(set(starts) == set(range(count)), "missing or unexpected batch indexes")
    expected_sequence = []
    for window in range(0, count, workers):
        indexes = range(window, min(window + workers, count))
        expected_sequence.extend((phase, index) for phase in ("begin", "end") for index in indexes)
    require(batch_sequence == expected_sequence, "batches violate ordered bounded-worker windows")
    metadata = ("batch_index", "subjects", "total_subjects", "workers", "first_subject", "last_subject")
    rows = []
    for index in range(count):
        begin, end = starts[index], ends[index]
        require(all(begin[key] == end[key] for key in metadata), f"mismatched batch pair: {index}")
        require(begin["subjects"] == min(BATCH_SIZE, total - index * BATCH_SIZE),
                f"inconsistent batch size: {index}")
        require(index == 0 or subject(ends[index - 1]["last_subject"]) < subject(begin["first_subject"]),
                "batch ranges overlap or violate semantic identity order")
        require(end["elapsed_micros"] <= end["run_elapsed_micros"] - begin["run_elapsed_micros"] + 1,
                f"batch duration exceeds its observed event span: {index}")
        rows.append({key: end[key] for key in
                     ("batch_index", "first_subject", "last_subject", "subjects", "elapsed_micros", "findings")})
    first_begin, last_end = starts[0], ends[count - 1]
    span = last_end["run_elapsed_micros"] - first_begin["run_elapsed_micros"]
    elapsed, elapsed_source = span, "first_batch_begin_to_last_batch_end"
    if EFFECTIVE in stages:
        begin, end = stages[EFFECTIVE]
        # Index checks also reject batches before/after a completed stage when
        # timestamps happen to be equal at microsecond resolution.
        require(events.index(begin) < events.index(first_begin)
                and events.index(last_end) < events.index(end), "batches escape effective audit stage")
        elapsed, elapsed_source = end["elapsed_micros"], "recorded_effective_audit_stage"
    else:
        require(not stages, "batch-only scoped journal unexpectedly contains finalization stages")
    durations = sorted(row["elapsed_micros"] for row in rows)
    windows = [rows[index:index + workers] for index in range(0, count, workers)]
    maxima = [max(row["elapsed_micros"] for row in window) for window in windows]
    minima = [min(row["elapsed_micros"] for row in window) for window in windows]
    sum_maxima, sum_minima = sum(maxima), sum(minima)
    # Timers cover each batch's wall duration, not CPU time. A short last window
    # uses only its scheduled batch when computing extrema and imbalance.
    window_timing = {
        "windows": len(windows),
        "two_batch_windows": sum(len(window) == 2 for window in windows),
        "sum_window_max_micros": sum_maxima,
        "sum_window_min_micros": sum_minima,
        "sum_window_imbalance_micros": sum_maxima - sum_minima,
        "imbalance_basis": "max minus min across scheduled batches in each window; singleton windows contribute zero",
        "observed_elapsed_outside_window_maxima_micros": span - sum_maxima,
        "outside_maxima_interpretation": "residual includes journaling, scheduling, merging and imperfect overlap; not isolated overhead",
        "windows_pairing_gt_1s_with_lt_1ms": sum(
            len(window) == 2 and maximum > 1_000_000 and minimum < 1_000
            for window, maximum, minimum in zip(windows, maxima, minima)
        ),
        "timing_occupancy_proxy": {
            "value": sum(durations) / (workers * span) if span else None,
            "formula": "sum_batch_elapsed_micros / (workers * observed_batch_span_micros)",
            "interpretation": "wall-timing capacity proxy; not measured CPU utilization or serial speedup",
        },
    }
    return {
        "format": PROFILE_FORMAT,
        "journal": {"path": str(journal), "sha256": hashlib.sha256(raw).hexdigest(), "bytes": len(raw)},
        "journal_consistent": True,
        "semantic_authority": False, "publication_authority": False,
        "timing_acceptance_threshold": None,
        "integrity": {"completed_batches": count, "missing_batch_indexes": [],
                      "unmatched_batch_begins": [], "unmatched_batch_ends": [],
                      "completed_stages": list(stages)},
        "worker_policy": {"workers": workers, "batch_subject_limit": BATCH_SIZE,
                          "ordered_windows": True, "join_order": "ascending_batch_index"},
        "subject_population": {"declared_total": total, "observed_total": sum(row["subjects"] for row in rows),
                               "first_subject": rows[0]["first_subject"], "last_subject": rows[-1]["last_subject"],
                               "consistent": True,
                               "scope": "logged counts and endpoint ranges; interior subject identities are not logged"},
        "overall_effective_elapsed_micros": elapsed,
        "overall_effective_elapsed_source": elapsed_source,
        "observed_batch_span_micros": span,
        "batch_elapsed_micros": {"total": sum(durations), "median": median(durations),
                                 "p95": durations[(95 * count + 99) // 100 - 1], "max": durations[-1],
                                 "p95_method": "nearest_rank"},
        "bounded_window_timing": window_timing,
        "batch_findings_observed": sum(row["findings"] for row in rows),
        "slowest_batches": sorted(rows, key=lambda row: (-row["elapsed_micros"], row["batch_index"]))[:10],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--journal", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.journal.resolve() == args.output.resolve():
        parser.error("output must not replace the read-only journal")
    raw = args.journal.read_bytes()
    try:
        report = profile(raw, args.journal)
    except (ValueError, KeyError) as error:
        report = {"format": PROFILE_FORMAT, "journal_consistent": False,
                  "journal": {"path": str(args.journal), "sha256": hashlib.sha256(raw).hexdigest(), "bytes": len(raw)},
                  "semantic_authority": False, "publication_authority": False, "error": str(error)}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report))
    return 0 if report["journal_consistent"] else 1


if __name__ == "__main__":
    sys.exit(main())
