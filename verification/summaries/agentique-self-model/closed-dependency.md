# Producer-closed dependency mounting

`ProducerClosedDependency` authenticates the exact immutable semantic graph,
context, independently expected registry, and all six requirement masks. It
retains shared graph references. Mounting preserves original namespace scopes
and the precise accepted KerML ancestor; an ordinary closed outer graph remains
`LocalProducerClosure`. A digest marker cannot grant publication acceptance.
The SysML consumer independently assembles its full combined registry and
rejects a weaker mounted registry. Accepted role bindings may be reused only
through the authenticated exact dependency graph.

This is consumer infrastructure, not Systems Library acceptance. No publication
receipt was fabricated and no full Systems publication was run here.

Verification on parent `cd15e08` plus this change, low-disk profile
(`CARGO_INCREMENTAL=0`, dev/test debug 0, jobs 2):

| Command | Result |
| --- | --- |
| `cargo test -p agq-kerml-semantics --lib producer_closed_dependency` | exit 0, 4 passed |
| `cargo test -p agq-kerml-semantics --lib producer_closure` | exit 0, 53 passed |
| `cargo test -p agq-sysml-semantics mounted_dependency` | exit 0, 1 passed |
| `cargo clippy -p agq-kerml-semantics -p agq-sysml-semantics -p agq-kerml-text --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |

The private nested-boundary regression supplies the private accepted marker
solely to test provenance separation. Public production callers cannot set it.
The other mount tests obtain closure through the real scheduler or explicitly
exercise rejection of an empty weaker registry.
