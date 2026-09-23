# Incremental closure certificate maintenance

Base: `7528a97762c7d808fcac8e1e7bd772bcba9638ff`.
Worktree: `agentique-incremental-certificate`, branch
`foundation/incremental-certificate`.

The monolithic worklist uses a scheduler-local `ClosureCertificateBuilder`.
Graph topology rows retain subject, owned-carrier and chain-target footprints;
only rows intersecting the validated frontier delta are reconstructed. Effect
mask rows retain their exact effective evaluation states. Graph-sensitive writer
scopes invalidate conservatively on graph changes. Registry, semantic contract,
provider and binding changes reset incompatible caches. Absent subjects are
removed; evaluation/class/effect changes replace rows instead of appending them.

The global producer dependency least fixed point, source provider masks and final
requirement propagation are recomputed. Their broad conservative proof boundaries
are unchanged. This is delta maintenance of the reusable certificate rows, not a
claim that all causal closure is incremental. Already blocked readers skip repeated
effect classification because no additional path can change their blocked state.
Accepted-dependency checks are evaluated once per subject instead of per family.

`ProducerClosureCertificate::issue` remains the full, uncached rebuild path and
ReferenceFullScan continues to use it. Set `AGQ_CERTIFICATE_VERIFY_FULL_REBUILD=1`
to compare every worklist-issued certificate against a full rebuild. The comparator
checks the exact subject population, producer-pair state bytes, requirement and
authority-source bits, all transport read atoms (including negative/provider/search
evidence), identity bindings, counts and both certificate digests. It does not
compare allocation identity. `AGQ_CERTIFICATE_TRACE=1` emits one bounded row-count
summary per certificate. The full oracle adds deliberate verification overhead.

Five permanent regressions cover unchanged-row reuse, changed evaluation states,
removed subjects and changed class applicability, changed producer effects/scopes,
changed negative/provider reads, chain endpoint retargeting and disappearing source
provider masks. The existing package suite additionally guards scopes, providers,
future writers, chains, expression/result waves and full-scan equivalence.

Historical timing correction: `certificate_build_micros` was accumulated with
`+=`, and each stage was originally emitted before its own certificate build.
The historical medium values 100,935,348, 105,391,928 and 109,544,480 microseconds
therefore represent cumulative time (increments 4.457 and 4.153 seconds), not a
100-second single-frontier build. No full Systems attempt was made by this worker.
Medium and resumed-medium acceptance remain lead integration gates.

Actual focused verification and exit codes are recorded in
`incremental-certificate-commands.json`. Raw final-oracle output is retained under
`verification/generated/final-language-acceptance/certificate/` in the isolated
worktree. This work does not accept Systems or change KerML publication authority.
