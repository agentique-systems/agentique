# Measured medium scheduler tail

Observational profile of completed frontiers; this document does not establish publication acceptance.
The zero-based stage threshold is 15. Counter resets identify separate invocations; JSONL row ordinals are never treated as rounds.

| Invocation | Phase | Completed stages | Tail frontiers |
| --- | --- | --- | ---: |
| 0 | construction | 0..12 | 0 |
| 1 | construction | 0..11 | 0 |
| 2 | construction | 0..11 | 0 |
| 3 | construction | 0..26 | 12 |

This snapshot ends at invocation 3, construction stage 26 (ContextualBindings, Incomplete). It includes only completed measurements available at that point; later frontiers and the independent acceptance report are outside this snapshot.

## Timing by invocation and stratum

All durations are seconds. Certificate timing includes incremental issuance, the full reference rebuild, and exact comparison in this verification run. It is not the production-only incremental cost.

| Invocation / stratum | Stages | Total | Planning incl. queries/cache | Certificate | Materialization | Read indexing | Certificate revalidation | Other |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 3 / ContextualBindings | 23..26 | 344.599 | 267.076 | 23.207 | 3.469 | 7.168 | 0.271 | 43.408 |
| 3 / StableProperties | 15..22 | 190.567 | 131.032 | 36.930 | 1.461 | 3.277 | 0.017 | 17.851 |

Other time includes uninstrumented round work such as context construction and checkpoint capture. No distinct query/cache timer exists; queries and cache work remain inside planning.

## Measured producer costs

Exclusive family blocks can be ranked against planning. Shared values are inclusive callback attribution: they overlap across participating families and must not be summed.

### Invocation 3, ContextualBindings

| Family | Attempts | Exclusive seconds | Overlapping shared seconds |
| --- | ---: | ---: | ---: |
| KerML.VariableFeaturing | 5256 | 86.320 | 0.000 |
| KerML.FeatureReferenceExpression | 260 | 85.191 | 0.000 |
| KerML.PositionalRedefinition | 5256 | 26.566 | 0.000 |
| KerML.CrossDomain | 5256 | 10.787 | 0.000 |
| KerML.OwnedCrossing | 5256 | 8.305 | 0.000 |
| KerML.ExpressionResult | 482 | 0.705 | 0.000 |
| deriveUsageMayTimeVary | 1038 | 0.000 | 24.208 |
| checkOccurrenceUsageSpecialization | 446 | 0.000 | 13.311 |
| checkOccurrenceUsageSuboccurrenceSpecialization | 446 | 0.000 | 13.311 |

The combined FeatureValue/FeatureValuation block costs 11.863s, counted once. Neither family's share is independently measured.

Subjects evaluated/skipped/reopened: 5260/1606/4630. Planned element outputs: 8137; accepted elements/occurrences: 2250/0.

| Next-frontier reason | Subject enqueues |
| --- | ---: |
| ProviderSearchChanged | 6048 |
| GraphFactChanged | 4580 |
| NewHelperCreated | 2250 |
| NewProducerOpportunity | 2250 |
| NewRelationshipEndpoint | 1974 |

Certificate row work across these frontiers: topology rebuilt 3099, retained 274649; producer scope rebuilt 6696, retained 271052. Retained counts are per-frontier reuse events, not distinct rows.

### Invocation 3, StableProperties

| Family | Attempts | Exclusive seconds | Overlapping shared seconds |
| --- | ---: | ---: | ---: |
| KerML.VariableFeaturing | 2342 | 87.710 | 0.000 |
| KerML.PositionalRedefinition | 2342 | 8.307 | 0.000 |
| KerML.CrossDomain | 2342 | 1.889 | 0.000 |
| KerML.ExpressionResult | 141 | 0.854 | 0.000 |
| KerML.OwnedCrossing | 2342 | 0.733 | 0.000 |
| KerML.FeatureChainExpression | 32 | 0.069 | 0.000 |
| deriveUsageMayTimeVary | 1075 | 0.000 | 23.902 |
| checkOccurrenceUsageSpecialization | 506 | 0.000 | 14.748 |
| checkOccurrenceUsageSuboccurrenceSpecialization | 506 | 0.000 | 14.748 |

The combined FeatureValue/FeatureValuation block costs 2.237s, counted once. Neither family's share is independently measured.

Subjects evaluated/skipped/reopened: 2344/14/2340. Planned element outputs: 428; accepted elements/occurrences: 18/0.

| Next-frontier reason | Subject enqueues |
| --- | ---: |
| ProviderSearchChanged | 1912 |
| GraphFactChanged | 1179 |
| NewRelationshipEndpoint | 826 |
| CertificateTransportChanged | 333 |
| ContextualBindingDependency | 181 |
| NewHelperCreated | 18 |
| NewProducerOpportunity | 18 |

Certificate row work across these frontiers: topology rebuilt 34, retained 538222; producer scope rebuilt 2323, retained 535933. Retained counts are per-frontier reuse events, not distinct rows.

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

Source prefix: 64 completed rows, 1131376 bytes, SHA-256 `947e96ea4472d4a6c0684463ec7408015b29e4fb38676e9e9478fb46bf53d7a7`. The source may continue growing; the byte prefix makes this snapshot independently reproducible.

`medium-tail-profile.py` reads only stage JSONL and watchdog logs. It validates per-round sums against each invocation's cumulative evaluated/skipped/reopened/certificate counters, rejects overlapping timing categories, checks paired shared timers, and matches certificate trace records to completed phase/stage records. The tracked JSON preserves per-frontier measurements and per-family reopen aggregates.

Watchdog at snapshot: elapsed 1569.015s, peak private memory 4.876 GiB, minimum disk reserve 6.165 GiB. These are resource observations, not acceptance evidence.
