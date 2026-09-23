# Full Systems completed-frontier profile: wall-time stop

Observational profile of completed frontiers; this document does not establish publication acceptance.
The zero-based stage threshold is 15. Counter resets identify separate invocations; JSONL row ordinals are never treated as rounds.

| Invocation | Phase | Completed stages | Tail frontiers |
| --- | --- | --- | ---: |
| 0 | construction | 0..12 | 0 |
| 1 | construction | 0..11 | 0 |
| 2 | construction | 0..11 | 0 |
| 3 | publication | 0..27 | 13 |

This snapshot ends at invocation 3, publication stage 27 (ContextualBindings, Complete). The watchdog stopped the sole full attempt before a final publication report or accepted cache/receipt was produced. These are scheduler observations, not accepted publication authority.

The watchdog returned 124 (wall_time) at 3601.109s. The final strict frontier closed 26532/26532 producer pairs, with 0 incomplete pairs and 452052 closed requirements. No final 1,327-reference acceptance audit, publication-capability result, accepted bindings, trusted cache or receipt is established by this profile.

Runtime binary source: `7223938437dc8ab7c684961bfb76eaba6c6c92f0`, SHA-256 `237d2508dfa46f82390d317827deb4be22775e07d7bfab8e49f4a641d0b7d6c6`. The watchdog's checkout pin `ff2990a0c8be71e5fa00c41371382adb8a571d68` is a later evidence/docs revision; it does not identify a different executable build.

## Post-convergence finalization gap

The producer timer recorded the last Complete frontier at 3449.323985s. On the watchdog's own clock it was first observed at 3450.516s, followed by 150.593s until recorded stop completion. The final live-process sample's progress age was 150.172s; stop completion also includes monitoring/termination overhead. These same-clock observations avoid subtracting clocks with different launch origins.

The stdout-pinned terminal checkpoint journal `735481e45453e104dd2bfe5d1e35bdb4ca89974d71b78dce5c51d267adfb6df5` was committed for invocation 3, next round 28. This analysis verifies that small journal's bytes against the emitted pin; it does not restore or recompute the archive. Checkpoint authority remains unaccepted.

The interval includes terminal checkpoint capture and subsequent finalization; those operations have no separate timing/progress markers here. Source order after scheduler return performs context/certificate checks, KerML capability and authority checks, mandatory-reference audit, binding validation and SysML capability queries before returning the accepted facade and exporting the cache. The logs do not identify which uninstrumented operation was active at termination or whether it would pass. A stall was not established: the recorded stop is wall time, and the last progress age is below the 600-second stall budget.

The authorized attempt budget is exhausted. No retry or resume was performed for this profile. Earlier medium profiles remain unchanged. Full publication remains incomplete; no acceptance criterion was narrowed.

## Timing by invocation and stratum

All durations are seconds. Certificate timing is incremental issuance without the full rebuild oracle. The launch owner explicitly verified that AGQ_CERTIFICATE_VERIFY_FULL_REBUILD was absent; the recorded full command enables only AGQ_CERTIFICATE_TRACE.

| Invocation / stratum | Stages | Total | Planning incl. queries/cache | Certificate | Materialization | Read indexing | Certificate revalidation | Other |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 3 / ContextualBindings | 18..27 | 1447.576 | 1252.238 | 47.222 | 7.541 | 39.714 | 1.029 | 99.832 |
| 3 / StableProperties | 15..17 | 37.791 | 7.381 | 21.223 | 0.002 | 0.188 | 0.000 | 8.996 |

Other time includes uninstrumented round work such as context construction and checkpoint capture. No distinct query/cache timer exists; queries and cache work remain inside planning.

## Measured producer costs

Exclusive family blocks can be ranked against planning. Shared values are inclusive callback attribution: they overlap across participating families and must not be summed.

Across the observed tail, planning occupies 84.8% of measured round time and certificate construction occupies 4.6%. The measured family rankings below are profile priorities, not evidence that any existing negative/provider read can be removed. Time after the final completed frontier is outside these round totals.

### Invocation 3, ContextualBindings

| Family | Attempts | Exclusive seconds | Share of planning | Overlapping shared seconds |
| --- | ---: | ---: | ---: | ---: |
| KerML.VariableFeaturing | 20956 | 624.789 | 49.9% | 0.000 |
| KerML.FeatureReferenceExpression | 657 | 216.548 | 17.3% | 0.000 |
| KerML.PositionalRedefinition | 20956 | 94.813 | 7.6% | 0.000 |
| KerML.CrossDomain | 20956 | 46.593 | 3.7% | 0.000 |
| KerML.OwnedCrossing | 20956 | 33.767 | 2.7% | 0.000 |
| KerML.ExpressionResult | 1263 | 2.077 | 0.2% | 0.000 |
| deriveUsageMayTimeVary | 4994 | 0.000 | 0.0% | 119.034 |
| checkOccurrenceUsageSpecialization | 2895 | 0.000 | 0.0% | 84.338 |
| checkOccurrenceUsageSuboccurrenceSpecialization | 2895 | 0.000 | 0.0% | 84.338 |

The combined FeatureValue/FeatureValuation block costs 51.348s, counted once. Neither family's share is independently measured.

Subjects evaluated/skipped/reopened: 21026/792/20726. Planned element outputs: 19693; accepted elements/occurrences: 1092/0.

| Next-frontier reason | Subject enqueues | Share of next-frontier enqueues |
| --- | ---: | ---: |
| ProviderSearchChanged | 20319 | 94.0% |
| GraphFactChanged | 7935 | 36.7% |
| NewRelationshipEndpoint | 3919 | 18.1% |
| NewHelperCreated | 1092 | 5.1% |
| NewProducerOpportunity | 1092 | 5.1% |
| CertificateTransportChanged | 4 | 0.0% |

Denominator: 21622 summed next-frontier subject enqueues. Categories overlap and percentages must not be added.

| Dominant exclusive family | Attempt carrying a reason | Count / attempts |
| --- | --- | ---: |
| KerML.VariableFeaturing | ProviderSearchChanged | 20311/20956 (96.9%) |
| KerML.VariableFeaturing | GraphFactChanged | 7077/20956 (33.8%) |
| KerML.VariableFeaturing | NewRelationshipEndpoint | 3916/20956 (18.7%) |
| KerML.FeatureReferenceExpression | ProviderSearchChanged | 584/657 (88.9%) |
| KerML.FeatureReferenceExpression | GraphFactChanged | 306/657 (46.6%) |
| KerML.FeatureReferenceExpression | NewRelationshipEndpoint | 243/657 (37.0%) |
| KerML.PositionalRedefinition | ProviderSearchChanged | 20311/20956 (96.9%) |
| KerML.PositionalRedefinition | GraphFactChanged | 7077/20956 (33.8%) |
| KerML.PositionalRedefinition | NewRelationshipEndpoint | 3916/20956 (18.7%) |

Certificate row work across these frontiers: topology rebuilt 1682, retained 750997; producer scope rebuilt 13835, retained 738844. Retained counts are per-frontier reuse events, not distinct rows.

Observed row reuse: topology 99.78%, scope 98.16%. Largest completed certificate build in this stratum's tail: 7.112s at stage 19; the full oracle is disabled. Summed certificate time is cumulative, not a single frontier's cost.

### Invocation 3, StableProperties

| Family | Attempts | Exclusive seconds | Share of planning | Overlapping shared seconds |
| --- | ---: | ---: | ---: | ---: |
| KerML.VariableFeaturing | 45 | 4.907 | 66.5% | 0.000 |
| KerML.PositionalRedefinition | 45 | 0.675 | 9.1% | 0.000 |
| KerML.CrossDomain | 45 | 0.003 | 0.0% | 0.000 |
| KerML.OwnedCrossing | 45 | 0.002 | 0.0% | 0.000 |
| KerML.ExpressionResult | 3 | 0.000 | 0.0% | 0.000 |
| deriveUsageMayTimeVary | 45 | 0.000 | 0.0% | 1.407 |
| checkOccurrenceUsageSpecialization | 11 | 0.000 | 0.0% | 0.426 |
| checkOccurrenceUsageSuboccurrenceSpecialization | 11 | 0.000 | 0.0% | 0.426 |

The combined FeatureValue/FeatureValuation block costs 0.142s, counted once. Neither family's share is independently measured.

Subjects evaluated/skipped/reopened: 45/0/45. Planned element outputs: 0; accepted elements/occurrences: 0/0.

| Next-frontier reason | Subject enqueues | Share of next-frontier enqueues |
| --- | ---: | ---: |
| ContextualBindingDependency | 196 | 92.5% |
| CertificateTransportChanged | 16 | 7.5% |

Denominator: 212 summed next-frontier subject enqueues. Categories overlap and percentages must not be added.

| Dominant exclusive family | Attempt carrying a reason | Count / attempts |
| --- | --- | ---: |
| KerML.VariableFeaturing | CertificateTransportChanged | 45/45 (100.0%) |
| KerML.PositionalRedefinition | CertificateTransportChanged | 45/45 (100.0%) |
| KerML.CrossDomain | CertificateTransportChanged | 45/45 (100.0%) |

Certificate row work across these frontiers: topology rebuilt 0, retained 222750; producer scope rebuilt 27, retained 222723. Retained counts are per-frontier reuse events, not distinct rows.

Observed row reuse: topology 100.00%, scope 99.99%. Largest completed certificate build in this stratum's tail: 7.783s at stage 16; the full oracle is disabled. Summed certificate time is cumulative, not a single frontier's cost.

Reason categories overlap. Graph/provider labels describe conservative invalidation keys, not independently proved changed answers. Family-specific reason counts are in the JSON artifact; a subject reopening evaluates its applicable family blocks together.

## Retained writer and read guards

No scope was narrowed and no running executable was changed for this analysis.

| Family | Existing scope / correctness boundary retained |
| --- | --- |
| VariableFeaturing | Subject plus owning Type; Featuring is subject-only. Snapshot selection reads effective owner features, redefinitions, featuring and the negative candidate population; creating a snapshot requires complete evidence. |
| PositionalRedefinition | Subject-only Redefinition. Result/parameter/end role, canonical ownership, inherited ordered populations and explicit invocation redefinitions remain prerequisites. |
| FeatureValue / FeatureValuation | Separate binding and valuation contracts share one implementation timer. Binding is contextual and subject-scoped; complete featuring domains and provider reads remain required. |
| FeatureReference | Contextual subject binding with actual referent/raw-result/domain evidence; declared referent identity is preserved. |
| Expression / Function result | Subject scope, explicit relationship classes and fresh end population; no FeatureValue carrier or retyping of the subject. |
| Feature-chain result | Subject plus selected owned Features, with direct result/source-target selection and existing first-input membership support. |
| Index / Select result | Subject plus owned results for operational profiles; exact ownership, Array guard and result evidence remain required. |
| deriveUsageMayTimeVary | StableProperties; only the subject's mayTimeVary/isVariable scalars. Positive selected canonical paths and complete negative ownership/typing/exclusion evidence are retained. |

Contracts are defined in [producer_worklist.rs](../../../crates/kerml-semantics/src/producer_worklist.rs), [result_structure.rs](../../../crates/kerml-semantics/src/result_structure.rs), [implicit.rs](../../../crates/kerml-semantics/src/implicit.rs), and [SysML producers](../../../crates/sysml-semantics/src/producers.rs). Existing scope counterexamples remain in the producer closure/worklist suites; this observational run adds no semantic claim beyond those guards.

## Reproduction and limits

Source prefix: 65 completed rows, 1420483 bytes, SHA-256 `17e7c5c0274357415cde3f0cc11aae178e97aac2af6cf41d7a8d0619175c3116`. This is the stopped full run's entire stage file.

`medium-tail-profile.py` reads only stage JSONL and watchdog logs. It validates per-round sums against each invocation's cumulative evaluated/skipped/reopened/certificate counters, rejects overlapping timing categories, checks paired shared timers, and matches certificate trace records to completed phase/stage records. The tracked JSON preserves per-frontier measurements and per-family reopen aggregates.

Watchdog stop: 3601.109s, peak private memory 5777.352 MiB, minimum disk reserve 2.729 GiB. Limits were 3600s, 6656 MiB private memory, 1024 MiB free disk, and 600s without meaningful progress. No memory, disk or stall stop was recorded.

This offline command authenticates its stage/log/observation/result/journal/executable-record inputs and records the absence of final export files. It does not establish final query acceptance, producer replay, archive restoration or a new publication attempt.
