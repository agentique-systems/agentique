# Initial certificate topology reuse draft

`initial-certificate-cache.patch` is unapplied and independent of the source
context patch. Its source/check identities are in
`initial-certificate-cache-patch.json`.

The scheduler currently issues its initial pending certificate without the
private `ClosureCertificateBuilder`, discards the derived topology rows, then
builds those rows again for its first subsequent certificate. The draft uses the
same builder for initial and later worklist issuance. Its new private entry point
still verifies the expected registry, starts with an empty evaluation table and
computes all pending/provider/writer masks. No old completed evaluation is imported.
Subsequent validated changes use the existing conservative topology footprint
invalidation. A changed graph with no supplied delta clears the cache, as before.

The `ReferenceFullScan` scheduler retains the original uncached initial-certificate
path. The existing full-certificate comparison checks every subject/state/mask,
transport read, count and digest; it now also covers builder-based initialization
when `AGQ_CERTIFICATE_VERIFY_FULL_REBUILD` is enabled.

Two new regressions compare initial and subsequent results with the independent
uncached oracle, require actual topology allocation reuse on an unchanged graph,
ensure reinitialization does not retain completed proofs, reject a foreign
registry without mutating the cache, and invalidate topology under changed
provider scope or graph content.

Expected savings are limited to duplicate initial/first-issue graph topology and
unchanged scope-row construction. Global blocker propagation, complete
certificate hashing and scheduler context authentication still run. The disjoint
closure profiling patch must quantify this work before any latency claim.

After coordinated application, run:

```text
cargo test --locked --offline -p agq-kerml-semantics --lib initial_certificate_ -- --nocapture
```

Then run existing certificate-incrementality/full-rebuild tests and the real
command/cold oracle. Isolated formatting and `git apply --check` passed. The
regressions have not been compiled or executed while the local soak is active.
