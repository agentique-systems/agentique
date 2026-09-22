# Initial partial certificate feasibility

Review and fixture only; no initial-issuance production API was added.

The existing private issuer treats absent evaluation rows as Pending/Inapplicable.
With the complete built-in registry, an empty table and an explicit all-Pending
table produce identical certificate digests. Zero evaluated pairs can still prove
EffectiveOwnership when no registered existing/future producer can change it;
EffectiveTyping remains open. Direct pending Ownership and an inapplicable
Model-scoped Ownership family activated on a future generated subject both keep
the negative ownership predicate Incomplete.

Actual focused command in the documented low-artifact environment:
`cargo test -p agq-kerml-semantics --lib initial_pending_table -- --nocapture`.
Exit 0; 1 passed. No corpus run or additional production verification was performed
for this review.

A future scheduler-owned linking hook would need to construct the entire
KerML/extension registry, reject unregistered extension applicability, bind the
exact graph/context/registry, and derive protected dependency identities from the
kernel frontier rather than a caller-supplied assertion. It must never reuse an
earlier frontier's witness.

Producer closure does not establish source-linking completeness. An unresolved
ownership carrier may not yet identify its future subject, so ordinary current
navigation alone cannot close authored ownership absence. The conservative initial
bootstrap design would expose these early ownership proofs only for protected
accepted dependency subjects while source ownership remains pending. A general
authored-source hook needs explicit source-obligation coverage and tests before
implementation. Existing source construction/pending diagnostics must remain
Incomplete; this review does not claim that broader boundary is implemented.
