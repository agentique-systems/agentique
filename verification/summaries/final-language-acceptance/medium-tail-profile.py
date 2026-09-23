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


def document(path):
    raw = path.read_bytes()
    return json.loads(raw), {"path": str(path), "bytes": len(raw),
                             "sha256": hashlib.sha256(raw).hexdigest()}


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
    completed = profile.get("completed_run")
    stopped = profile.get("stopped_run")
    finished_measurement = completed or stopped
    title = ("# Full Systems completed-frontier profile: wall-time stop" if stopped
             else "# Completed fresh medium scheduler tail" if completed
             else "# Measured medium scheduler tail")
    lines = [title, "",
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
        suffix = ("The watchdog stopped the sole full attempt before a final publication report or accepted cache/receipt was produced. These are scheduler observations, not accepted publication authority."
                  if stopped else "The independently recorded scoped audit completed; no later frontiers belong to this run. The earlier partial profile remains unchanged in medium-tail-profile.md/JSON."
                  if completed else "It includes only completed measurements available at that point; later frontiers and the independent acceptance report are outside this snapshot.")
        lines += ["", f"This snapshot ends at invocation {latest['ordinal']}, {latest['phase']} stage {last['stage']} ({last['stratum']}, {last['completeness']}). {suffix}"]
    if completed:
        lines += ["", f"The recorded command exited {completed['command_exit_code']} after {completed['duration_seconds']:.3f}s: {completed['references_complete']}/{completed['references_total']} references Complete, {completed['kernel_obligations']} kernel obligations, {completed['closed_pairs']}/{completed['applicable_pairs']} producer pairs closed, {completed['closed_requirements']}/{completed['required_requirements']} requirements closed. The scoped construction scheduler converged Complete and its certificate is fully closed. Publication attempted/accepted are both false; this profile makes no full Systems acceptance or uninterrupted/resumed equivalence claim.",
                  "", f"Run source commit `{completed['source_commit']}`, working patch SHA-256 `{completed['working_changes_sha256']}`. Incremental/full certificate comparison was enabled. Resource figures come from the completed watchdog result; its last observation can precede process completion."]
    if stopped:
        final = stopped['last_completed_frontier']
        lines += ["", f"The watchdog returned {stopped['command_exit_code']} ({stopped['safety_stop']}) at {stopped['duration_seconds']:.3f}s. The final strict frontier closed {final['closed_pairs']}/{final['applicable_pairs']} producer pairs, with {final['incomplete_pairs']} incomplete pairs and {final['closed_requirements']} closed requirements. No final 1,327-reference acceptance audit, publication-capability result, accepted bindings, trusted cache or receipt is established by this profile.",
                  "", f"Runtime binary source: `{stopped['executable']['source_commit']}`, SHA-256 `{stopped['executable']['sha256']}`. The watchdog's checkout pin `{stopped['source_commit']}` is a later evidence/docs revision; it does not identify a different executable build.",
                  "", "## Post-convergence finalization gap", "",
                  f"The producer timer recorded the last Complete frontier at {final['elapsed_seconds']:.6f}s. On the watchdog's own clock it was first observed at {stopped['first_final_frontier_observed_seconds']:.3f}s, followed by {stopped['post_frontier_observed_gap_seconds']:.3f}s until recorded stop completion. The final live-process sample's progress age was {stopped['last_progress_age_seconds']:.3f}s; stop completion also includes monitoring/termination overhead. These same-clock observations avoid subtracting clocks with different launch origins.",
                  "", f"The stdout-pinned terminal checkpoint journal `{stopped['checkpoint_journal']['sha256']}` was committed for invocation {stopped['checkpoint_invocation']}, next round {stopped['checkpoint_next_round']}. This analysis verifies that small journal's bytes against the emitted pin; it does not restore or recompute the archive. Checkpoint authority remains unaccepted.",
                  "", "The interval includes terminal checkpoint capture and subsequent finalization; those operations have no separate timing/progress markers here. Source order after scheduler return performs context/certificate checks, KerML capability and authority checks, mandatory-reference audit, binding validation and SysML capability queries before returning the accepted facade and exporting the cache. The logs do not identify which uninstrumented operation was active at termination or whether it would pass. A stall was not established: the recorded stop is wall time, and the last progress age is below the 600-second stall budget.",
                  "", "The authorized attempt budget is exhausted. No retry or resume was performed for this profile. Earlier medium profiles remain unchanged. Full publication remains incomplete; no acceptance criterion was narrowed."]
    certificate_description = (
        "Certificate timing is incremental issuance without the full rebuild oracle. The launch owner explicitly verified that AGQ_CERTIFICATE_VERIFY_FULL_REBUILD was absent; the recorded full command enables only AGQ_CERTIFICATE_TRACE."
        if stopped else "Certificate timing includes incremental issuance, the full reference rebuild, and exact comparison in this verification run. It is not the production-only incremental cost.")
    lines += ["", "## Timing by invocation and stratum", "",
              "All durations are seconds. " + certificate_description, "",
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
    if finished_measurement:
        tail_time = sum(inv['tail']['timings'].get('elapsed_micros', 0) for inv in profile['invocations'])
        planning = sum(inv['tail']['timings'].get('planning_including_query_cache_micros', 0) for inv in profile['invocations'])
        certificate = sum(inv['tail']['timings'].get('certificate_build_micros', 0) for inv in profile['invocations'])
        oracle_label = "" if stopped else " including its full oracle"
        tail_interpretation = ("The measured family rankings below are profile priorities, not evidence that any existing negative/provider read can be removed. Time after the final completed frontier is outside these round totals."
                               if stopped else "VariableFeaturing dominates StableProperties; VariableFeaturing, FeatureReferenceExpression and PositionalRedefinition dominate ContextualBindings. These are measured profile priorities, not evidence that any existing negative/provider read can be removed.")
        lines += ["", f"Across the observed tail, planning occupies {planning/tail_time*100:.1f}% of measured round time and certificate construction{oracle_label} occupies {certificate/tail_time*100:.1f}%. {tail_interpretation}"]
    for inv in profile["invocations"]:
        for stratum, data in inv["tail_by_stratum"].items():
            lines += ["", f"### Invocation {inv['ordinal']}, {stratum}", "",
                      "| Family | Attempts | Exclusive seconds | Share of planning | Overlapping shared seconds |",
                      "| --- | ---: | ---: | ---: | ---: |"]
            ranked = sorted(data["families"].items(), key=lambda item: item[1]["planning_micros"], reverse=True)
            selected = [(n,m) for n,m in ranked[:6] if m["planning_micros"]]
            shared = sorted(data["families"].items(), key=lambda item: item[1]["shared_planning_micros"], reverse=True)
            selected += [(n,m) for n,m in shared[:3] if m["shared_planning_micros"] and n not in {k for k,_ in selected}]
            for name, family in selected:
                share = family['planning_micros'] / data['timings']['planning_including_query_cache_micros'] * 100
                lines.append(f"| {name} | {family['attempts']} | {family['planning_micros']/1e6:.3f} | {share:.1f}% | {family['shared_planning_micros']/1e6:.3f} |")
            lines += ["", f"The combined FeatureValue/FeatureValuation block costs {data['feature_value_valuation_shared_micros_counted_once']/1e6:.3f}s, counted once. Neither family's share is independently measured."]
            c=data["counts"]; d=data["certificate_delta"]
            lines += ["", f"Subjects evaluated/skipped/reopened: {c['subjects_evaluated']}/{c['subjects_skipped']}/{c['subjects_reopened']}. Planned element outputs: {c['planned_elements']}; accepted elements/occurrences: {c['accepted_elements']}/{c['accepted_occurrences']}."]
            lines += ["", "| Next-frontier reason | Subject enqueues | Share of next-frontier enqueues |", "| --- | ---: | ---: |"]
            for reason,count in sorted(data["next_dirty_by_reason"].items(),key=lambda item:item[1],reverse=True):
                share = count / c['next_dirty_subjects'] * 100 if c['next_dirty_subjects'] else 0
                lines.append(f"| {reason} | {count} | {share:.1f}% |")
            lines += ["", f"Denominator: {c['next_dirty_subjects']} summed next-frontier subject enqueues. Categories overlap and percentages must not be added."]
            if finished_measurement:
                lines += ["", "| Dominant exclusive family | Attempt carrying a reason | Count / attempts |", "| --- | --- | ---: |"]
                for name, family in ranked[:3]:
                    for reason, count in sorted(family['reopened_by_reason'].items(), key=lambda item:item[1], reverse=True)[:3]:
                        lines.append(f"| {name} | {reason} | {count}/{family['attempts']} ({count/family['attempts']*100:.1f}%) |")
            if d:
                lines += ["", f"Certificate row work across these frontiers: topology rebuilt {d['topology_rebuilt']}, retained {d['topology_retained']}; producer scope rebuilt {d['scope_rebuilt']}, retained {d['scope_retained']}. Retained counts are per-frontier reuse events, not distinct rows."]
                if finished_measurement:
                    topology = d['topology_retained'] / (d['topology_retained'] + d['topology_rebuilt']) * 100
                    scope = d['scope_retained'] / (d['scope_retained'] + d['scope_rebuilt']) * 100
                    peak = max(data['frontiers'], key=lambda row:row['certificate_build_micros'])
                    oracle_label = "the full oracle is disabled" if stopped else "this includes the full oracle"
                    lines += ["", f"Observed row reuse: topology {topology:.2f}%, scope {scope:.2f}%. Largest completed certificate build in this stratum's tail: {peak['certificate_build_micros']/1e6:.3f}s at stage {peak['stage']}; {oracle_label}. Summed certificate time is cumulative, not a single frontier's cost."]
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
              f"Source prefix: {profile['source']['completed_lines']} completed rows, {profile['source']['completed_bytes']} bytes, SHA-256 `{profile['source']['sha256']}`. " + ("This is the stopped full run's entire stage file." if stopped else "This is the completed fresh run's entire stage file." if completed else "The source may continue growing; the byte prefix makes this snapshot independently reproducible."), "",
              "`medium-tail-profile.py` reads only stage JSONL and watchdog logs. It validates per-round sums against each invocation's cumulative evaluated/skipped/reopened/certificate counters, rejects overlapping timing categories, checks paired shared timers, and matches certificate trace records to completed phase/stage records. The tracked JSON preserves per-frontier measurements and per-family reopen aggregates."]
    if completed:
        lines += ["", "Completed-run mode additionally reads the scoped audit and watchdog result, authenticates the output-log hash, requires a zero command/monitor exit and no safety stop, and checks the final stage's closure counts against the scoped report. It does not rerun the publication or the exact archive comparator.",
                  "", f"Watchdog completion: {completed['duration_seconds']:.3f}s, peak private memory {completed['peak_private_bytes']/2**30:.3f} GiB, minimum disk reserve {completed['minimum_free_disk_bytes']/2**30:.3f} GiB. These are resource observations, not acceptance evidence."]
    elif stopped:
        lines += ["", f"Watchdog stop: {stopped['duration_seconds']:.3f}s, peak private memory {stopped['peak_private_bytes']/2**20:.3f} MiB, minimum disk reserve {stopped['minimum_free_disk_bytes']/2**30:.3f} GiB. Limits were 3600s, 6656 MiB private memory, 1024 MiB free disk, and 600s without meaningful progress. No memory, disk or stall stop was recorded.",
                  "", "This offline command authenticates its stage/log/observation/result/journal/executable-record inputs and records the absence of final export files. It does not establish final query acceptance, producer replay, archive restoration or a new publication attempt."]
    elif profile.get("watchdog",{}).get("last"):
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
    parser.add_argument("--run-result", type=Path, help="completed watchdog result; requires --audit-report")
    parser.add_argument("--audit-report", type=Path, help="completed scoped audit; requires --run-result")
    parser.add_argument("--stopped-run-result", type=Path, help="watchdog result for the stopped full attempt")
    parser.add_argument("--checkpoint-journal", type=Path, help="terminal journal emitted by the stopped full attempt")
    parser.add_argument("--executable-pin", type=Path, help="independent executable build evidence")
    args = parser.parse_args()
    if bool(args.run_result) != bool(args.audit_report):
        parser.error("--run-result and --audit-report must be supplied together")
    if args.stopped_run_result and (args.run_result or not args.checkpoint_journal or not args.executable_pin or not args.watchdog):
        parser.error("stopped-run mode requires journal/executable pin/watchdog and excludes completed-run mode")
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
    if args.run_result:
        run, run_source = document(args.run_result)
        audit, audit_source = document(args.audit_report)
        assert run['exit_code'] == run['command_exit_code'] == 0
        assert run['safety_stop'] is None and run['monitor_error'] is None
        assert source['completed_bytes'] == args.stages.stat().st_size
        assert trace_source and trace_source['matched_frontiers'] == len(rows)
        assert trace_source['sha256'] == run['output_sha256']
        assert audit['scoped_preflight_passed'] and audit['construction_complete']
        assert audit['incremental_certificate_reference_check']
        assert audit['construction_producers']['converged']
        assert audit['construction_producers']['completeness'] == 'Complete'
        assert audit['producer_closure']['fully_closed']
        assert not audit['publication_attempted'] and not audit['publication_accepted']
        closure = audit['producer_closure']
        final = rows[-1]['closure_counters']
        for key in ('closed_pairs', 'closed_requirements', 'incomplete_pairs'):
            assert final[key] == closure[key]
        refs = audit['mandatory_references']
        assert refs['counts']['complete'] == refs['total'] and not refs['failures']
        assert all(value == 0 for key, value in refs['counts'].items() if key != 'complete')
        result['completed_run'] = {
            'watchdog_result_source': run_source, 'audit_report_source': audit_source,
            **{key:run[key] for key in ('source_commit', 'working_changes_sha256', 'command_exit_code',
                                      'duration_seconds', 'peak_private_bytes', 'minimum_free_disk_bytes')},
            'scope': audit['scope'], 'publication_accepted': False, 'publication_attempted': False,
            'references_total': refs['total'], 'references_complete': refs['counts']['complete'],
            'kernel_obligations': audit['kernel_obligations'],
            **{key:closure[key] for key in ('closed_pairs', 'applicable_pairs', 'closed_requirements', 'required_requirements')},
            'construction_reference_semantic_digest': bytes(audit['construction_reference_semantic_digest']).hex(),
            'certificate_semantic_digest': bytes(closure['semantic_closure_digest']).hex(),
        }
    if args.stopped_run_result:
        run, run_source = document(args.stopped_run_result)
        journal, journal_source = document(args.checkpoint_journal)
        executable, executable_source = document(args.executable_pin)
        assert run['exit_code'] == run['command_exit_code'] == 124 and run['safety_stop'] == 'wall_time'
        assert run['monitor_error'] is None
        assert source['completed_bytes'] == args.stages.stat().st_size
        assert trace_source and trace_source['matched_frontiers'] == len(rows)
        assert trace_source['sha256'] == run['output_sha256']
        assert run['environment_overrides'].get('AGQ_CERTIFICATE_VERIFY_FULL_REBUILD') is None
        assert run['command'][0] == executable['executable']
        emitted = re.findall(r"Publication checkpoint committed: journal=.+ sha256=([a-f0-9]{64}) invocation=(\d+) round=(\d+) stratum=(\w+)", args.output_log.read_text())
        pin, invocation, next_round, stratum = emitted[-1]
        assert journal_source['sha256'] == pin
        assert journal['format'] == 'agq-unaccepted-publication-frontier/1'
        assert int(invocation) == len(journal['entries']) - 1
        final = result['invocations'][-1]['tail']['frontiers'][-1]
        assert rows[-1]['phase'] == 'publication' and final['completeness'] == 'Complete'
        assert final['stage'] + 1 == int(next_round) and final['stratum'] == stratum
        final_observations = [row for row in observations
                              if (row.get('progress') or {}).get('frontier') == str(final['stage'])
                              and (row.get('progress') or {}).get('closed_pairs') == str(final['closed_pairs'])]
        first_observed = final_observations[0]['elapsed_seconds']
        missing = [args.stages.parent / name for name in
                   ('report.json', 'canonical.publication.zip', 'accepted-publication.json', 'standard-bindings.json')]
        assert all(not path.exists() for path in missing)
        result['stopped_run'] = {
            'watchdog_result_source': run_source, 'checkpoint_journal': journal_source,
            'executable_record_source': executable_source, 'executable': executable,
            **{key:run[key] for key in ('source_commit', 'working_changes_sha256', 'command_exit_code',
                                      'safety_stop', 'duration_seconds', 'peak_private_bytes', 'minimum_free_disk_bytes')},
            'checkpoint_invocation': int(invocation), 'checkpoint_next_round': int(next_round),
            'checkpoint_latest_entry': journal['entries'][-1],
            'last_completed_frontier': final,
            'first_final_frontier_observed_seconds': first_observed,
            'post_frontier_observed_gap_seconds': round(run['duration_seconds'] - first_observed, 6),
            'last_progress_age_seconds': observations[-1]['progress_age_seconds'],
            'missing_final_outputs': [str(path) for path in missing],
            'full_reference_rebuild_enabled': False,
            'full_reference_rebuild_evidence': 'launch owner confirmed inherited environment absent; override includes only TRACE=1',
            'publication_accepted': False,
        }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if args.markdown:
        args.markdown.write_text(render(result), encoding="utf-8")
    print(json.dumps({"completed_rows": len(rows), "invocations": [
        {"ordinal": r["ordinal"], "phase": r["phase"], "stages": [r["stage_start"], r["stage_end"]],
         "tail_frontiers": r["tail"]["frontier_count"]} for r in result["invocations"]]}, indent=2))


if __name__ == "__main__":
    main()
