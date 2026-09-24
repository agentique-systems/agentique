# Publication interpretation-input review

The initial frontend/standards stale-input check failed as expected after adding
three workspace packages and changing authored reconstruction. Its failed command
remains in the command ledger. This review authorizes refreshing only the
non-authoritative `sysml-publication-inputs.json` inventory.

Changed interpretation inputs are `Cargo.lock`,
`crates/kerml-text/src/library/construction.rs`, and
`crates/kerml-text/src/source_inputs.rs`. The lock adds only agq-modeling-view,
agq-modeling-agent and agq-studio; existing external package versions, checksums
and dependency lists are unchanged.

The authored optimization takes an explicit previous strict declared snapshot.
It compares complete desired declarations to that snapshot, submitting only
changed/new/removed records. Full construction and publication pass no such
snapshot: they still unconditionally create records and link occurrences. A
review caught and corrected accidental reuse on this full path. The added
collision regression requires the original ReusedIdentity failure with both
no cache and reuse disabled. Exact incremental/full construction oracles cover
records, slots, occurrences, references, provenance, obligations and retained
old state, including a 100-document fixture. See performance command records.

Publication profiles, query rule sets, descriptors, trusted catalogue, accepted
receipts, standard bindings, library bytes and accepted semantic identities are
unchanged. The additional CompilationWork counters are observations; they are
not included in checkpoints or publication identities. Publication construction
cannot activate the authored reconstruction frontier. New view/agent/host crates
depend inward and do not supply language semantics.

The explicit capture command records reviewed source freshness only. Its format
retains `grants_publication_authority: false`. It neither issues a publication nor
establishes fresh accepted-cache, authored-closure, durable-scale or Studio runtime
acceptance. Those runtime gates remain blocked by missing external cache artifacts.
