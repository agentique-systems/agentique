# Systems bindings and trusted restoration

Status at `c38f373`: **implementation and executable acceptance harness prepared;
actual Systems acceptance and restoration remain gated**. The compiled catalogue
is empty; `checked_in("sysml-systems-operational-v2")` fails. No accepted Systems
facade, binding manifest or receipt has been fabricated. The original KerML
Operational v9 `/26` publication and the current default SysML profile are unchanged.

## Contract and implemented checks

An independent C1 inventory found 69 implemented algorithmic roles: 66 direct
producer targets plus three Operational v2 correction targets. The manifest binds
canonical ID, library, qualified path, actual/expected metaclass, public visibility,
source document/revision/node/range/path/hash, Operational v2 and rule set, Systems
KPAR/source content, both publication digests, combined descriptors, producer
registry and closure identity. This source audit found no schema gap; it is not
accepted-binding evidence.

`CanonicalSysmlSystemsLibrary::restore_cache(reader, sources, accepted_kerml)`
selects independent compiled authority, checks exact source and interpretation
identities, and requires precisely `facade.json`, `closure.json`, `kernel.jsonl`.
Entry sizes and hashes bound decoding. Metadata and closure are parsed from the
same authenticated bytes; the graph is decoded under the independent combined
registry, then re-encoded and hashed to reject changed-reader substitution.
Canonical provenance, roots, ordered source maps, all 21 documents, 1,327 reference
assertions and checked-family populations are revalidated. Public standard bindings
are reconstructed and their complete regenerated manifest must match authority.

`TrustedPublicationReceipt` has no caller-data constructor, deserializer, mutable
documents or implementable authority trait. Its catalogue selects a format label
and opaque binding payload as data. KerML owns envelope, byte and exact producer
context/certificate authentication; the SysML facade owns Systems schema and
cross-field interpretation. Generic authority tests use a non-SysML format and
opaque bindings; facade tests reject changed or mutually omitted required fields.

Producer-aware binding identity includes original declared slots. The live facade,
restoration and mounted authored dependencies use `for_producer_overlay`, which
independently assembles the full registry before validating/attaching bindings.
A raw aggregate-graph binding cannot silently upgrade to producer-aware identity;
wrong graph, raw-bound upgrade and weaker registry certificates are rejected.
The exact supplied KerML Arc and record identities are shared. Restoration rebuilds
historical audit observations separately from zero local producer counters and
never replays producers.

| Preparation / concrete finding | Integrated correction and observed verification |
| --- | --- |
| Exact restore scaffold | `c28fd16` based on authenticated mount `de501b5`; initial focused authority/archive tests plus five existing KerML restoration guards passed. Catalogue stays empty |
| SysML schema interpretation leaked into the KerML authority layer | `2a070b4`; opaque generic authentication and facade-specific field checks; four generic authority and four SysML restoration tests passed |
| Aggregate binding digest differed from producer graph digest containing original slots | Constructor correction `a82a03e` and restore follow-up `b8fbd44`; combined review `3f2728d` on `5e57467`; matching contexts attach, raw/changed/weaker inputs reject |
| Combined prepared path | Four trusted-authority tests, four text restoration tests, five SysML overlay tests, all-target three-package Clippy, strict Rustdoc and formatting pass. These checks supersede the historical compilation deferral during Actions |
| Actual accepted-cache gate | `88ffe4a`: ignored integration target compiles, focused Clippy/fmt pass, one test discovered with none executed. No cache loaded |

The integrated preparation supersedes earlier held-branch cherry-pick instructions.
Exact commands/source/patch/output hashes remain in
[commands.json](commands.json). Its `source_ledgers` ranges preserve provenance
from `systems-restore-preparation.json`, `systems-receipt-layering.json`,
`held-restore-integration.json` and the original ordinary ledger. Initial
compile/lint corrections are preserved without collapsing repeated commands.
The mistaken `context_overlay` filter selected zero tests; the corrected
`overlay_tests` command passed five. Zero selected tests are not gate evidence.

## Actual accepted-cache gate, still unexecuted

`crates/kerml-text/tests/accepted_systems_cache.rs` requires both readable original
cache paths and a compiled independently accepted Systems receipt **before** loading
KerML. A requested run fails if either input or authority is missing. It restores
KerML once, shares that Arc, restores Systems, writes a roundtrip, drops the first
Systems instance, then restores the roundtrip. It checks canonical IDs, context /
semantic / publication identities, roots/source map, all 69 bindings and manifest,
complete closure certificate, shared dependency record pointers and zero counters.

Scratch copies test changed metadata, closure and graph bytes (required to fail
receipt digest authentication before JSON interpretation), extra/duplicate entries,
truncation and changed binding ID/metaclass/source hash. Originals are read-only;
archive transforms stream data, holding at most one Systems graph beside KerML.
Local graph reconstruction must retain its conservative proof fallback wherever
optional contribution metadata is unavailable; this preparation claims no actual
accepted-cache result.

Only after a genuine full publication, check in the generated receipt and binding
manifest and activate the finite catalogue entry (`agq-sysml-accepted-publication/1`).
The successful example writes `canonical.publication.zip`,
`accepted-publication.json` and `standard-bindings.json` beside its report.
Pin the latter two under `standards/` and add the single
`sysml-systems-operational-v2` catalogue entry. `SysmlBaselineProfile` currently
has no `OPERATIONAL` alias; add it as `OPERATIONAL_V2` only after acceptance.
The read-only activation review confirms that the historical KerML `/26`
receipt has no combined producer-registry or closure-certificate fields: new
Systems descriptor digests are independently bound by the Systems receipt and
do not change that historical restoration path. No cache was loaded by this review.
At that point the unit test rejecting the currently unavailable Systems identifier
must become a positive checked-in-authority assertion plus rejection of an unknown
identifier; retain its caller-supplied JSON rejection. Do not leave an assertion
that the newly accepted identifier is unavailable. Then run this gate alone,
followed by the authored acceptance harness:

```powershell
$env:AGENTIQUE_KERML_CACHE = '<exact accepted KerML cache>'
$env:AGENTIQUE_SYSTEMS_CACHE = '<exact accepted Systems cache>'
cargo test -p agq-kerml-text --test accepted_systems_cache accepted_systems_cache_roundtrip_and_tampering -- --ignored --exact --nocapture
```

The 16 GiB development host must not run this beside publication. One extra
Systems archive of scratch disk is required. Actual positive roundtrip, adversarial
cache results and accepted authored integration remain prerequisites for C2; private
fixture checks or completed commands cannot substitute for them.

## Full-run memory review

At `6350177`, cache writing releases the already-serialized facade and closure
JSON before writing the kernel graph. Independent review confirms unchanged ZIP
entry order, serialization, digest and receipt paths. This removes an allocation
overlap; its corpus memory saving is unmeasured. The cache test already drops each
restored Systems graph before loading the next and streams archive mutations.
Run its exact ignored gate alone after receipt activation.

At `4a8c9d1`, the self-model gate completes independent Case/programmatic fixtures
before retaining its authored edit history, and drops unused empty projects
before parallel reads. All prior assertions remain, including simultaneous
retention of r1/r2/r3. Independent review approved these lifetime changes.
Formatting passes at `4f5a052` (`bridge-acceptance-review-format` in
`commands.json`); compilation and actual accepted-cache execution remain pending.

<details>
<summary>Retained command observations from the consolidated Markdown</summary>

The JSON ledgers retain exact source/patch identities and output hashes where recorded.
These observations preserve the additional original arguments and exits. For an
observation lacking a corresponding JSON record, **output hash is unavailable**;
its precise tested source/patch and tool version are unrecorded unless stated.
An implementation commit in the matrix is not a substitute for a tested-tree identity.
Repeated commands at different stages are distinct observations, not new acceptance.

| ID | Exact observed command |
| --- | --- |
| C1 | `cargo test --locked --offline -p agq-kerml-semantics --lib trusted_publication -- --nocapture` |
| C2 | `cargo test --locked --offline -p agq-kerml-text --lib restoration -- --nocapture` |
| C3 | `cargo test --locked --offline -p agq-sysml-semantics --lib overlay_tests -- --nocapture` |
| C4 | `cargo clippy --locked --offline -p agq-kerml-semantics -p agq-sysml-semantics -p agq-kerml-text --all-targets -- -D warnings` |
| C5 | `RUSTDOCFLAGS=-D warnings cargo doc --locked --offline -p agq-kerml-semantics -p agq-sysml-semantics -p agq-kerml-text --no-deps` |
| C6 | `cargo fmt --all -- --check` |
| C7 | `cargo test -p agq-sysml-semantics producer_overlay_bindings_attach_only_to_the_exact_producer_graph -- --nocapture` |
| C8 | `cargo test -p agq-sysml-semantics context::overlay_tests -- --nocapture` |
| C9 | `cargo check -p agq-kerml-text` |
| C10 | `cargo clippy -p agq-sysml-semantics -p agq-kerml-text --all-targets -- -D warnings` |
| C11 | `git diff --check` |
| C12 | `cargo test -p agq-kerml-text --test accepted_systems_cache --no-run` |
| C13 | `cargo clippy -p agq-kerml-text --test accepted_systems_cache -- -D warnings` |
| C14 | `cargo test -p agq-kerml-text --test accepted_systems_cache -- --list` |

| Command | Original record / tested source where stated | Observed result | Exit |
| --- | --- | --- | --- |
| C1 | systems-restore-scaffold | 4 passed; exact authority, unavailable catalogue, altered bytes/context/bounds, weaker registry and recomputed certificate rejection | 0 |
| C2 | systems-restore-scaffold | 4 passed; receipt schema, archive bounds/population, stale sources and incomplete facade populations | 0 |
| C3 | systems-restore-scaffold | 5 passed; exact producer-bound roles, wrong graph, weaker registry, forged dependency and ordinary current-graph distinction | 0 |
| C4 | systems-restore-scaffold | Finished with no warnings | 0 |
| C5 | systems-restore-scaffold | Three public APIs documented without warnings | 0 |
| C6 | systems-restore-scaffold | No output | 0 |
| C7 | producer-overlay-bindings | exit 0; 1 passed | included in result |
| C8 | producer-overlay-bindings | exit 0; 5 passed | included in result |
| C9 | producer-overlay-bindings | exit 0 | included in result |
| C10 | producer-overlay-bindings | exit 0 | included in result |
| C6 | producer-overlay-bindings | exit 0 | included in result |
| C11 | producer-overlay-bindings | exit 0 | included in result |
| C6 | accepted-systems-cache-gate | clean | 0 |
| C12 | accepted-systems-cache-gate | integration target compiled | 0 |
| C13 | accepted-systems-cache-gate | clean | 0 |
| C14 | accepted-systems-cache-gate | one test discovered; none executed | 0 |

</details>
