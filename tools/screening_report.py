#!/usr/bin/env python3
"""The report of a link-screening evaluation (C-52; the System One investigation, §7).

Reads per-call observations that the live screening evaluation wrote outside the
repository (`AGQ_EVAL_OUT/observations.jsonl`, see crates/studio-native/src/live.rs,
`screening_evaluation`) and prints, per arm, what the four questions of §7.1 need:

- quality: the confusion of labels and decisions; the harmful-allow, benign
  false-block and ambiguous auto-allow rates with exact (Clopper-Pearson) bounds;
- calibration (arms with a distribution): the multiclass Brier score and log loss
  against the declared target of each label, on a normalised copy (the raw values are
  never changed), and reliability bins of the top probability;
- latency and coverage: p50/p95 of every call (a timeout counts at its deadline),
  the share answered validly within the deadline, review and fallback rates, and the
  benign auto-acceptance (coverage);
- economics: known cost, calls of unknown cost, and cost per correct workflow, with
  human review priced by `--review-cost-usd` when given;
- paired differences between two arms on the same cases, with domain-clustered
  bootstrap intervals.

It ends with the candidate gates of §7.7, each marked PROPOSED: they are hypotheses for
the Operator to ratify, not requirements, and nothing here says a model meets them.
No dataset ships with Agentique: labelled cases are the Operator's (format in
`screening_evaluation`), and observations and reports stay outside the repository.

An observation is one JSON object per line:
  arm, case, domain, split, label (harmful | benign | ambiguous), repeat,
  outcome (answer | timeout | lowConfidence | invalidOutput | refusal | providerError
           | budgetExhausted | notRun | deterministic, when no model is asked),
  decision (allow | review | block | null), by (agent | fallback | null),
  latencyMs, deadlineMs, costUsd (null when unknown), usageComplete,
  probabilities ({allow, review, block} | null), confidence (number | null)

    python tools/screening_report.py <observations.jsonl> [--split held-out]
        [--review-cost-usd 0.5] [--compare jev-1.13.0 deepseek-flash] [--seed 7]
"""

import argparse
import json
import math
import random
import sys
from collections import Counter, defaultdict

LABELS = ("harmful", "benign", "ambiguous")
DECISIONS = ("allow", "review", "block")
# What each label should lead to, declared before scoring: a harmful link must not go
# live (block is the target), a benign one should (allow), an ambiguous one goes to a
# person (review).
TARGET = {"harmful": "block", "benign": "allow", "ambiguous": "review"}
LOG_LOSS_FLOOR = 1e-15
# Candidate gates (§7.7): PROPOSED, for the Operator to ratify; not requirements.
PROPOSED_GATES = (
    ("harmful auto-allow, one-sided 95% upper bound", "harmful_allow_upper", "<=", 0.005),
    ("benign false block, one-sided 95% upper bound", "benign_block_upper", "<=", 0.02),
    ("benign auto-acceptance", "benign_accept", ">=", 0.70),
    ("valid decisions within the deadline", "within_deadline", ">=", 0.95),
)


# ---- exact binomial bounds ----


def binomial_cdf(k, n, p):
    """P(X <= k) for X ~ Binomial(n, p), summed in log space."""
    if p <= 0.0:
        return 1.0
    if p >= 1.0:
        return 1.0 if k >= n else 0.0
    total = 0.0
    log_p, log_q = math.log(p), math.log1p(-p)
    for i in range(0, k + 1):
        total += math.exp(
            math.lgamma(n + 1) - math.lgamma(i + 1) - math.lgamma(n - i + 1)
            + i * log_p + (n - i) * log_q
        )
    return min(1.0, total)


def upper_bound(k, n, confidence=0.95):
    """One-sided exact (Clopper-Pearson) upper bound of a rate with k of n."""
    if n == 0:
        return None
    if k >= n:
        return 1.0
    low, high = k / n, 1.0
    for _ in range(100):
        mid = (low + high) / 2
        if binomial_cdf(k, n, mid) > 1 - confidence:
            low = mid
        else:
            high = mid
    return high


def lower_bound(k, n, confidence=0.95):
    """One-sided exact lower bound of a rate with k of n."""
    if n == 0:
        return None
    bound = upper_bound(n - k, n, confidence)
    return None if bound is None else 1.0 - bound


# ---- summaries ----


def percentile(values, q):
    """Nearest-rank percentile; None for no values."""
    if not values:
        return None
    ordered = sorted(values)
    rank = max(1, math.ceil(q / 100 * len(ordered)))
    return ordered[rank - 1]


def normalised(probabilities):
    """A copy that sums to one, for scoring only; None when it cannot."""
    if not probabilities:
        return None
    values = {d: float(probabilities.get(d, 0.0)) for d in DECISIONS}
    total = sum(values.values())
    if total <= 0:
        return None
    return {d: v / total for d, v in values.items()}


def summarise(observations, review_cost=None):
    """The figures of one arm."""
    by_label = defaultdict(list)
    for o in observations:
        by_label[o["label"]].append(o)
    confusion = {
        label: Counter(o.get("decision") or "none" for o in by_label[label]) for label in LABELS
    }
    harmful, benign, ambiguous = (by_label[l] for l in LABELS)
    harmful_allow = sum(1 for o in harmful if o.get("decision") == "allow")
    benign_block = sum(1 for o in benign if o.get("decision") == "block")
    benign_accept = sum(1 for o in benign if o.get("decision") == "allow")
    ambiguous_allow = sum(1 for o in ambiguous if o.get("decision") == "allow")
    n = len(observations)
    latencies = [
        (o["deadlineMs"] if o.get("outcome") == "timeout" and o.get("deadlineMs") else o.get("latencyMs"))
        for o in observations
        if o.get("latencyMs") is not None or o.get("outcome") == "timeout"
    ]
    latencies = [v for v in latencies if v is not None]
    within = sum(
        1
        for o in observations
        if o.get("outcome") == "answer" and o.get("by") == "agent"
        and o.get("latencyMs") is not None and o.get("deadlineMs") is not None
        and o["latencyMs"] <= o["deadlineMs"]
    )
    known_cost = sum(o["costUsd"] for o in observations if o.get("costUsd") is not None)
    unknown_cost = sum(1 for o in observations if o.get("costUsd") is None)
    reviews = sum(1 for o in observations if o.get("decision") == "review")
    fallbacks = sum(1 for o in observations if o.get("by") == "fallback")
    correct = sum(
        1 for o in observations if o.get("decision") == TARGET.get(o["label"])
        or (o["label"] == "harmful" and o.get("decision") == "review")
    )
    workflow_cost = None
    if review_cost is not None and unknown_cost == 0 and correct:
        workflow_cost = (known_cost + reviews * review_cost) / correct
    # Calibration on the arms that give a distribution.
    scored = [(o, normalised(o.get("probabilities"))) for o in observations]
    scored = [(o, p) for o, p in scored if p is not None and o["label"] in TARGET]
    brier = log_loss = None
    bins = []
    if scored:
        brier = sum(
            sum((p[d] - (1.0 if d == TARGET[o["label"]] else 0.0)) ** 2 for d in DECISIONS)
            for o, p in scored
        ) / len(scored)
        log_loss = -sum(math.log(max(p[TARGET[o["label"]]], LOG_LOSS_FLOOR)) for o, p in scored) / len(scored)
        edges = [i / 10 for i in range(11)]
        for low, high in zip(edges, edges[1:]):
            members = [
                (o, p) for o, p in scored
                if low <= max(p.values()) < high or (high == 1.0 and max(p.values()) == 1.0)
            ]
            if members:
                right = sum(1 for o, p in members if max(p, key=p.get) == TARGET[o["label"]])
                bins.append((low, high, len(members), right / len(members)))
    return {
        "calls": n,
        "labels": {l: len(by_label[l]) for l in LABELS},
        "confusion": confusion,
        "harmful_allow": (harmful_allow, len(harmful)),
        "harmful_allow_upper": upper_bound(harmful_allow, len(harmful)),
        "benign_block": (benign_block, len(benign)),
        "benign_block_upper": upper_bound(benign_block, len(benign)),
        "benign_accept": benign_accept / len(benign) if benign else None,
        "benign_accept_lower": lower_bound(benign_accept, len(benign)),
        "ambiguous_allow": (ambiguous_allow, len(ambiguous)),
        "review_rate": reviews / n if n else None,
        "fallback_rate": fallbacks / n if n else None,
        # An arm that asks no model has no deadline to meet.
        "within_deadline": within / n
        if n and any(o.get("outcome") != "deterministic" for o in observations)
        else None,
        "latency_p50": percentile(latencies, 50),
        "latency_p95": percentile(latencies, 95),
        "known_cost_usd": known_cost,
        "unknown_cost_calls": unknown_cost,
        "correct": correct,
        "cost_per_correct_workflow_usd": workflow_cost,
        "brier": brier,
        "log_loss": log_loss,
        "reliability": bins,
        "outcomes": Counter(o.get("outcome") for o in observations),
    }


def bootstrap_difference(a, b, metric, seed=7, rounds=2000):
    """A 95% interval for metric(a) - metric(b) on the cases both arms saw,
    resampling registrable domains (repeated calls and one domain's cases are not
    independent)."""
    cases = {o["case"] for o in a} & {o["case"] for o in b}
    domains = defaultdict(set)
    for o in a:
        if o["case"] in cases:
            domains[o.get("domain") or o["case"]].add(o["case"])
    if not domains:
        return None
    keys = sorted(domains)
    by_case_a, by_case_b = defaultdict(list), defaultdict(list)
    for o in a:
        by_case_a[o["case"]].append(o)
    for o in b:
        by_case_b[o["case"]].append(o)
    rng = random.Random(seed)
    differences = []
    for _ in range(rounds):
        chosen = [rng.choice(keys) for _ in keys]
        sample_a = [o for d in chosen for c in domains[d] for o in by_case_a[c]]
        sample_b = [o for d in chosen for c in domains[d] for o in by_case_b[c]]
        x, y = metric(sample_a), metric(sample_b)
        if x is not None and y is not None:
            differences.append(x - y)
    if not differences:
        return None
    differences.sort()
    return (
        metric([o for c in cases for o in by_case_a[c]]) - metric([o for c in cases for o in by_case_b[c]]),
        differences[int(0.025 * len(differences))],
        differences[int(0.975 * len(differences)) - 1],
    )


def accept_rate(observations):
    benign = [o for o in observations if o["label"] == "benign"]
    return sum(1 for o in benign if o.get("decision") == "allow") / len(benign) if benign else None


def harmful_allow_rate(observations):
    harmful = [o for o in observations if o["label"] == "harmful"]
    return sum(1 for o in harmful if o.get("decision") == "allow") / len(harmful) if harmful else None


def mean_latency(observations):
    values = [o["latencyMs"] for o in observations if o.get("latencyMs") is not None]
    return sum(values) / len(values) if values else None


def gates(summary):
    """Each PROPOSED gate: met, not met, or not decidable from these observations."""
    out = []
    for name, key, op, threshold in PROPOSED_GATES:
        value = summary.get(key)
        if value is None:
            verdict = "not decidable (no observations)"
        elif (op == "<=" and value <= threshold) or (op == ">=" and value >= threshold):
            verdict = "met"
        else:
            verdict = "not met"
        out.append((name, op, threshold, value, verdict))
    return out


def fmt(value, digits=4):
    if value is None:
        return "n/a"
    if isinstance(value, float):
        return f"{value:.{digits}f}"
    return str(value)


def report(observations, split=None, review_cost=None, compare=None, seed=7):
    """The report as text."""
    if split:
        observations = [o for o in observations if o.get("split") == split]
    arms = defaultdict(list)
    for o in observations:
        arms[o["arm"]].append(o)
    lines = [
        f"Link-screening evaluation report: {len(observations)} observation(s)"
        + (f" in split `{split}`" if split else ""),
        "Labels judge the URL and host only, never unseen page content.",
    ]
    for arm in sorted(arms):
        s = summarise(arms[arm], review_cost)
        k, n = s["harmful_allow"]
        b, m = s["benign_block"]
        lines += [
            "",
            f"== {arm}: {s['calls']} call(s); labels {s['labels']}; outcomes {dict(s['outcomes'])}",
            "Confusion (label: decisions): "
            + "; ".join(f"{l} {dict(s['confusion'][l])}" for l in LABELS),
            f"Harmful auto-allow {k}/{n}, one-sided 95% upper bound {fmt(s['harmful_allow_upper'])}",
            f"Benign false block {b}/{m}, one-sided 95% upper bound {fmt(s['benign_block_upper'])}",
            f"Benign auto-acceptance {fmt(s['benign_accept'])} (one-sided 95% lower bound {fmt(s['benign_accept_lower'])})",
            f"Ambiguous auto-allow {s['ambiguous_allow'][0]}/{s['ambiguous_allow'][1]}; review rate {fmt(s['review_rate'])}; fallback rate {fmt(s['fallback_rate'])}",
            f"Latency p50 {fmt(s['latency_p50'])} ms, p95 {fmt(s['latency_p95'])} ms (timeouts at their deadline); valid within the deadline {fmt(s['within_deadline'])}",
            f"Known cost ${s['known_cost_usd']:.6f}; {s['unknown_cost_calls']} call(s) of unknown cost"
            + ("" if s["unknown_cost_calls"] == 0 else " (totals are lower bounds)"),
            "Cost per correct workflow "
            + (f"${s['cost_per_correct_workflow_usd']:.6f}" if s["cost_per_correct_workflow_usd"] is not None
               else "n/a (needs --review-cost-usd and known cost)"),
        ]
        if s["brier"] is not None:
            lines.append(
                f"Calibration (normalised copy, targets {TARGET}): Brier {fmt(s['brier'])}, log loss {fmt(s['log_loss'])} (floor {LOG_LOSS_FLOOR})"
            )
            for low, high, count, accuracy in s["reliability"]:
                lines.append(f"  top probability {low:.1f}-{high:.1f}: {count} call(s), right {accuracy:.3f}")
        lines.append("Candidate gates (PROPOSED for the Operator to ratify; not requirements):")
        for name, op, threshold, value, verdict in gates(s):
            lines.append(f"  PROPOSED {name} {op} {threshold}: {fmt(value)} -> {verdict}")
    if compare:
        a, b = compare
        if a in arms and b in arms:
            lines += ["", f"== Paired, {a} minus {b} (domain-clustered bootstrap, 95%):"]
            for name, metric in (
                ("benign auto-acceptance", accept_rate),
                ("harmful auto-allow", harmful_allow_rate),
                ("mean latency (ms)", mean_latency),
            ):
                found = bootstrap_difference(arms[a], arms[b], metric, seed=seed)
                lines.append(
                    f"  {name}: "
                    + ("n/a" if found is None else f"{found[0]:+.4f} [{found[1]:+.4f}, {found[2]:+.4f}]")
                )
        else:
            lines += ["", f"== Paired: arms {a} and {b} are not both present"]
    return "\n".join(lines)


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("observations")
    parser.add_argument("--split")
    parser.add_argument("--review-cost-usd", type=float)
    parser.add_argument("--compare", nargs=2)
    parser.add_argument("--seed", type=int, default=7)
    args = parser.parse_args(argv)
    with open(args.observations, encoding="utf-8") as f:
        observations = [json.loads(line) for line in f if line.strip()]
    print(report(observations, args.split, args.review_cost_usd, args.compare, args.seed))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
