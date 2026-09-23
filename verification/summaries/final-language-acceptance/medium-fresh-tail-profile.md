# Completed fresh medium scheduler tail

Observational profile of completed frontiers; this document does not establish publication acceptance.
The zero-based stage threshold is 15. Counter resets identify separate invocations; JSONL row ordinals are never treated as rounds.

| Invocation | Phase | Completed stages | Tail frontiers |
| --- | --- | --- | ---: |
| 0 | construction | 0..12 | 0 |
| 1 | construction | 0..11 | 0 |
| 2 | construction | 0..11 | 0 |
| 3 | construction | 0..32 | 18 |

This snapshot ends at invocation 3, construction stage 32 (ContextualBindings, Complete). The independently recorded scoped audit completed; no later frontiers belong to this run. The earlier partial profile remains unchanged in medium-tail-profile.md/JSON.

The recorded command exited 0 after 2389.688s: 695/695 references Complete, 0 kernel obligations, 14791/14791 producer pairs closed, 417786/417786 requirements closed. The scoped construction scheduler converged Complete and its certificate is fully closed. Publication attempted/accepted are both false; this profile makes no full Systems acceptance or uninterrupted/resumed equivalence claim.

Run source commit `7223938437dc8ab7c684961bfb76eaba6c6c92f0`, working patch SHA-256 `594015caa86e3eab460d8476b6d010edc27d934e5fc3bec703dac6fd5f0317ad`. Incremental/full certificate comparison was enabled. Resource figures come from the completed watchdog result; its last observation can precede process completion.

## Timing by invocation and stratum

All durations are seconds. Certificate timing includes incremental issuance, the full reference rebuild, and exact comparison in this verification run. It is not the production-only incremental cost.

| Invocation / stratum | Stages | Total | Planning incl. queries/cache | Certificate | Materialization | Read indexing | Certificate revalidation | Other |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 3 / ContextualBindings | 23..32 | 931.487 | 772.044 | 63.773 | 7.604 | 22.857 | 0.705 | 64.504 |
| 3 / StableProperties | 15..22 | 237.785 | 167.397 | 43.548 | 2.050 | 4.420 | 0.021 | 20.349 |

Other time includes uninstrumented round work such as context construction and checkpoint capture. No distinct query/cache timer exists; queries and cache work remain inside planning.

## Measured producer costs

Exclusive family blocks can be ranked against planning. Shared values are inclusive callback attribution: they overlap across participating families and must not be summed.

Across the observed tail, planning occupies 80.3% of measured round time and certificate construction including its full oracle occupies 9.2%. VariableFeaturing dominates StableProperties; VariableFeaturing, FeatureReferenceExpression and PositionalRedefinition dominate ContextualBindings. These are measured profile priorities, not evidence that any existing negative/provider read can be removed.

### Invocation 3, ContextualBindings

| Family | Attempts | Exclusive seconds | Share of planning | Overlapping shared seconds |
| --- | ---: | ---: | ---: | ---: |
| KerML.VariableFeaturing | 13715 | 259.573 | 33.6% | 0.000 |
| KerML.FeatureReferenceExpression | 585 | 218.255 | 28.3% | 0.000 |
| KerML.PositionalRedefinition | 13715 | 78.959 | 10.2% | 0.000 |
| KerML.CrossDomain | 13715 | 34.216 | 4.4% | 0.000 |
| KerML.OwnedCrossing | 13715 | 26.612 | 3.4% | 0.000 |
| KerML.ExpressionResult | 1072 | 2.121 | 0.3% | 0.000 |
| deriveUsageMayTimeVary | 2755 | 0.000 | 0.0% | 73.003 |
| checkOccurrenceUsageSpecialization | 1194 | 0.000 | 0.0% | 40.042 |
| checkOccurrenceUsageSuboccurrenceSpecialization | 1194 | 0.000 | 0.0% | 40.042 |

The combined FeatureValue/FeatureValuation block costs 37.186s, counted once. Neither family's share is independently measured.

Subjects evaluated/skipped/reopened: 13775/1687/13115. Planned element outputs: 18561; accepted elements/occurrences: 2347/0.

| Next-frontier reason | Subject enqueues | Share of next-frontier enqueues |
| --- | ---: | ---: |
| ProviderSearchChanged | 12749 | 83.4% |
| GraphFactChanged | 7294 | 47.7% |
| NewRelationshipEndpoint | 2887 | 18.9% |
| NewHelperCreated | 2347 | 15.4% |
| NewProducerOpportunity | 2347 | 15.4% |
| CertificateTransportChanged | 4 | 0.0% |

Denominator: 15281 summed next-frontier subject enqueues. Categories overlap and percentages must not be added.

| Dominant exclusive family | Attempt carrying a reason | Count / attempts |
| --- | --- | ---: |
| KerML.VariableFeaturing | ProviderSearchChanged | 12741/13715 (92.9%) |
| KerML.VariableFeaturing | GraphFactChanged | 5551/13715 (40.5%) |
| KerML.VariableFeaturing | NewRelationshipEndpoint | 2884/13715 (21.0%) |
| KerML.FeatureReferenceExpression | ProviderSearchChanged | 520/585 (88.9%) |
| KerML.FeatureReferenceExpression | GraphFactChanged | 272/585 (46.5%) |
| KerML.FeatureReferenceExpression | NewRelationshipEndpoint | 215/585 (36.8%) |
| KerML.PositionalRedefinition | ProviderSearchChanged | 12741/13715 (92.9%) |
| KerML.PositionalRedefinition | GraphFactChanged | 5551/13715 (40.5%) |
| KerML.PositionalRedefinition | NewRelationshipEndpoint | 2884/13715 (21.0%) |

Certificate row work across these frontiers: topology rebuilt 3255, retained 692241; producer scope rebuilt 11929, retained 683567. Retained counts are per-frontier reuse events, not distinct rows.

Observed row reuse: topology 99.53%, scope 98.28%. Largest completed certificate build in this stratum's tail: 8.838s at stage 30; this includes the full oracle. Summed certificate time is cumulative, not a single frontier's cost.

### Invocation 3, StableProperties

| Family | Attempts | Exclusive seconds | Share of planning | Overlapping shared seconds |
| --- | ---: | ---: | ---: | ---: |
| KerML.VariableFeaturing | 2342 | 112.111 | 67.0% | 0.000 |
| KerML.PositionalRedefinition | 2342 | 10.500 | 6.3% | 0.000 |
| KerML.CrossDomain | 2342 | 2.401 | 1.4% | 0.000 |
| KerML.ExpressionResult | 141 | 1.103 | 0.7% | 0.000 |
| KerML.OwnedCrossing | 2342 | 0.941 | 0.6% | 0.000 |
| KerML.FeatureChainExpression | 32 | 0.083 | 0.0% | 0.000 |
| deriveUsageMayTimeVary | 1075 | 0.000 | 0.0% | 30.237 |
| checkOccurrenceUsageSpecialization | 506 | 0.000 | 0.0% | 18.632 |
| checkOccurrenceUsageSuboccurrenceSpecialization | 506 | 0.000 | 0.0% | 18.632 |

The combined FeatureValue/FeatureValuation block costs 2.810s, counted once. Neither family's share is independently measured.

Subjects evaluated/skipped/reopened: 2344/14/2340. Planned element outputs: 428; accepted elements/occurrences: 18/0.

| Next-frontier reason | Subject enqueues | Share of next-frontier enqueues |
| --- | ---: | ---: |
| ProviderSearchChanged | 1912 | 76.1% |
| GraphFactChanged | 1179 | 46.9% |
| NewRelationshipEndpoint | 826 | 32.9% |
| CertificateTransportChanged | 333 | 13.3% |
| ContextualBindingDependency | 181 | 7.2% |
| NewHelperCreated | 18 | 0.7% |
| NewProducerOpportunity | 18 | 0.7% |

Denominator: 2513 summed next-frontier subject enqueues. Categories overlap and percentages must not be added.

| Dominant exclusive family | Attempt carrying a reason | Count / attempts |
| --- | --- | ---: |
| KerML.VariableFeaturing | ProviderSearchChanged | 1910/2342 (81.6%) |
| KerML.VariableFeaturing | GraphFactChanged | 1163/2342 (49.7%) |
| KerML.VariableFeaturing | NewRelationshipEndpoint | 824/2342 (35.2%) |
| KerML.PositionalRedefinition | ProviderSearchChanged | 1910/2342 (81.6%) |
| KerML.PositionalRedefinition | GraphFactChanged | 1163/2342 (49.7%) |
| KerML.PositionalRedefinition | NewRelationshipEndpoint | 824/2342 (35.2%) |
| KerML.CrossDomain | ProviderSearchChanged | 1910/2342 (81.6%) |
| KerML.CrossDomain | GraphFactChanged | 1163/2342 (49.7%) |
| KerML.CrossDomain | NewRelationshipEndpoint | 824/2342 (35.2%) |

Certificate row work across these frontiers: topology rebuilt 34, retained 538222; producer scope rebuilt 2323, retained 535933. Retained counts are per-frontier reuse events, not distinct rows.

Observed row reuse: topology 99.99%, scope 99.57%. Largest completed certificate build in this stratum's tail: 6.060s at stage 22; this includes the full oracle. Summed certificate time is cumulative, not a single frontier's cost.

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

Source prefix: 70 completed rows, 1253854 bytes, SHA-256 `703775644d8a1eb724d2784d6bbbd537a609b5b19f26fb219d6b6f5f7201bf33`. This is the completed fresh run's entire stage file.

`medium-tail-profile.py` reads only stage JSONL and watchdog logs. It validates per-round sums against each invocation's cumulative evaluated/skipped/reopened/certificate counters, rejects overlapping timing categories, checks paired shared timers, and matches certificate trace records to completed phase/stage records. The tracked JSON preserves per-frontier measurements and per-family reopen aggregates.

Completed-run mode additionally reads the scoped audit and watchdog result, authenticates the output-log hash, requires a zero command/monitor exit and no safety stop, and checks the final stage's closure counts against the scoped report. It does not rerun the publication or the exact archive comparator.

Watchdog completion: 2389.688s, peak private memory 4.997 GiB, minimum disk reserve 2.620 GiB. These are resource observations, not acceptance evidence.
