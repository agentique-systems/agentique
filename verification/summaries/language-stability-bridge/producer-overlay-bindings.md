# Producer-aware Systems binding attachment

`StandardSysmlBindings` retains the exact validated semantic graph identity.
Producer-aware overlays also authenticate original declared populations; their
digest must be established before binding attachment. `for_producer_overlay`
now independently assembles the full KerML/SysML registry before attachment.
Ordinary current-graph constructors retain their existing meaning.

Live Systems acceptance, the accepted publication query facade, and its mounted
dependency witness use that constructor. Final binding validation independently
assembles the registry instead of taking its identity from the certificate.
`with_producer_closure` rechecks bindings after opting in, rejecting a silent
upgrade from bindings validated against the narrower current-graph identity.
The separately held trusted-restoration implementation consumes the same API;
it is not part of this change or an acceptance claim.

Small canonical regression: nonempty Part-role bindings attach to the matching
producer graph; a raw graph, changed graph, raw-bound upgrade, and weaker registry
certificate are rejected. Existing unaccepted-dependency, descriptor, current-
graph phase, and independently scheduled weaker-registry tests still pass.

Verification used one build job, no incremental compilation, dev/test debug=0,
and `target/bridge-closure`. No corpus or accepted-cache load was run.

| Command | Result |
| --- | --- |
| `cargo test -p agq-sysml-semantics producer_overlay_bindings_attach_only_to_the_exact_producer_graph -- --nocapture` | exit 0; 1 passed |
| `cargo test -p agq-sysml-semantics context::overlay_tests -- --nocapture` | exit 0; 5 passed |
| `cargo check -p agq-kerml-text` | exit 0 |
| `cargo clippy -p agq-sysml-semantics -p agq-kerml-text --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `git diff --check` | exit 0 |
