# Recovering the accepted runtime

Semantic publication authority, cache encoding and distribution are separate.
The alpha mission explicitly authorizes replaying the already accepted producer
against its exact original inputs. It does not authorize different standards,
profiles, rules, bindings or canonical elements. Normal startup and ordinary CI
never perform this recovery.

The final search on 2026-09-25 found no original runtime in available historical
worktrees, local project archives, Git objects, releases or Actions caches. A fresh
download of Actions run `36150517543` contained 3,046 files and no publication
cache. Exact search commands and outputs are retained in
[`verification/native-studio-alpha/runtime`](../verification/native-studio-alpha/runtime).
This is evidence about available storage, not every external backup.

## Frozen inputs

| Contract | Exact identity |
| --- | --- |
| KerML producer commit | `a1a7b847ef83f8e4c54bea9242cac94a9ce665fe` |
| KerML profile / rule set | `agentique-kerml-1.0-operational/9` / `agq-kerml-query/26` |
| KerML publication digest | `815573353973607bc62a25195ed4461645182027407f8476be521ffc99174f12` |
| Systems producer/finalizer commit | `4ac9b8e58695ad837fed6643b0cedfac05d15635` |
| SysML profile / rule set | `agentique-sysml-2.0-operational/3` / `agq-sysml-query/6` |
| Systems publication digest | `25aeddb099be16462b553d6debcad97f43bf22e89ce8a9bbb53cf72eb7d193fa` |

These commits come from original accepted command records. Systems producer
source is unchanged between original closure commit `257d73cb` and the listed
finalizer commit. Checked-in receipts and bindings remain the authority. Their
complete source, descriptor, profile, registry, closure and binding identities
must match; the table is only a readable locator.

## Reproduction

Use isolated detached worktrees at those commits. Copy the retained
[`kerml_rematerialize.rs`](../tools/runtime-recovery/kerml_rematerialize.rs) wrapper
into the historical KerML checkout's `crates/kerml-text/examples`. Original language
source remains unchanged. From that checkout, build with the pinned toolchain and
locked dependencies:

```powershell
cargo build --release --config profile.release.lto=false --locked --offline -j 2 -p agq-kerml-text --example kerml_rematerialize
```

Prefer the retained provenance-guarded runner from the current checkout:

```powershell
python verification/native-studio-alpha/runtime/rematerialize_kerml.py
```

The mission runner expects the sibling historical checkout named
`agentique-alpha-rematerialize-kerml` and its successful
`historical-kerml-recovery-build.json` command record. It checks the exact Git
commit, an empty tracked diff and equality of the copied wrapper bytes before
invoking the producer. The retained producer record also hashes the wrapper and
executable. Its output directory is
`verification/generated/native-studio-alpha/rematerialized-kerml`.
When adapting this runner to another machine, retain these checks and record fresh
source and binary hashes. The producer commit reported by the wrapper is a fixed
identifier, not independent proof of the executable's provenance. Direct wrapper
invocation therefore requires the same external provenance checks.

Choose fresh output directories. The wrapper has no authority-writing option. It
calls the unchanged full-corpus producer, including every closure, mandatory
reference and capability gate. It requires the existing accepted semantic digest
and full binding-manifest equality before writing a cache candidate. The original
development tool's five scoped preflights and post-publication authored benchmark
are separate from these complete gates and do not need repeating to recover
already accepted bytes. Completion remains a candidate until ordinary restoration
matches existing authority. No failed semantic obligation is waived.

The historical kernel allocates a fresh immutable snapshot revision label. The
original receipt pins that label and full serialized graph separately from
semantic identity. From the current checkout, restore that original label with:

```powershell
python tools/restore-accepted-kerml-transport.py --cache C:/runtime-recovery/kerml/canonical.publication.zip --generated-receipt C:/runtime-recovery/kerml/canonical.publication.receipt.json --output C:/runtime-recovery/kerml/accepted.cache
```

The tool requires the full generated receipt to equal the checked-in receipt
apart from graph transport digest/length and the snapshot label. It changes only
`Snapshot.revision`. Both resulting uncompressed entries must then match the
original accepted payload SHA-256 and length exactly. Changed elements, evidence,
bindings, profiles, sources or semantic digests fail closed. Ordinary language
facade restoration remains mandatory before packaging. The original receipt is
unchanged; the outer ZIP encoding may differ.

Build and run `sysml_systems_publication` from the Systems historical checkout
with `--profile=operational-v3`, the authenticated KerML cache and an explicit
checkpoint directory. Retain the generated journal hash independently. The direct
finalizer accepts that journal through `--finalize-converged` and
`--resume-sha256`, and runs the strict effective audit before emitting a cache.
Compare its receipt, all 69 bindings, graph identity and closure certificate to
the accepted Systems contract. A successful new closure cannot replace equality.

If an entry differs, diagnose semantics versus encoding. The recovery tool never
relaxes authentication. Alternative encoding would require an explicit versioned
transport receipt and independent restoration against unchanged semantic
authority. A semantic mismatch stops runtime acceptance while Studio work continues.

### Versioned Systems transport

The original Systems receipt does not retain its snapshot revision label. If a
fresh historical replay has a different graph digest, the old graph bytes cannot
be reproduced by substituting a known original label as they can for KerML.
The compiled publication catalogue supports a separately versioned transport
receipt anchored to the SHA-256 of the unchanged original semantic receipt.
It requires the identical publication identity, entry names and entry byte
lengths. Only `kernel.jsonl` may have an alternate digest; `facade.json` and
`closure.json` remain pinned to their original exact bytes. The original receipt
and binding manifest remain the objects consumed by all semantic checks.

A transport hash can be registered only after the recorded frozen producer
replay, whole-contract comparison and full binding equality pass. Ordinary facade
restoration must then authenticate, decode and re-encode the actual graph,
recompute its semantic identity, validate original source provenance and bindings,
and restore the original closure certificate against that exact model and
registry. A caller-supplied receipt or hash cannot select new authority.

Without the old Systems graph payload, the precise cause of different serialized
bytes is an inference, not a proved byte comparison. The graph also encodes
identity reservations and reference-contribution transport beyond its canonical
semantic digest. The evidence is the unchanged historical producer and inputs,
independently equal accepted semantics, unchanged facade and certificate bytes,
and the explicitly reviewed new transport pin. Do not report these as original
Systems cache bytes recovered. No alternate transport is active merely because
the mechanism exists.

Prepare the small transport receipt for review with
`tools/runtime-recovery/prepare_systems_transport.py`, supplying the generated
cache, receipt and bindings, a unique transport identifier and a fresh output
directory. It independently hashes actual entries, requires every semantic field
and binding to match, preserves original facade/closure bytes and all entry
lengths, and records the new snapshot revision in `content.json`. It writes only
an unauthenticated review candidate. Keep that content record: a later replay
can restore the known label and demand this registered exact graph digest.
Registration in the compiled catalogue remains an explicit reviewed source
change followed by ordinary language-facade authentication.

The 2026-09-25 replay completed every original Systems capability and effective
audit with zero findings. Its complete non-entry receipt and all 69 bindings
match the original accepted authority. The registered transport is
[`sysml-v3-rematerialized-2026-09-25.json`](../standards/runtime-transports/sysml-v3-rematerialized-2026-09-25.json).
The original facade and closure payloads are byte-exact; its 748,560,767-byte
graph has SHA-256 `ec9d1c78d864d4ad2f671f93371c1988bcb8b952651b82afb9f3624b0756b828`
and snapshot revision `36f83b80-aa6a-4d4b-bd3c-c32117e1104e`.
The new receipt pins only this transport under the unchanged v3 authority.
[`systems-rematerialized-content.json`](../verification/native-studio-alpha/runtime/systems-rematerialized-content.json)
retains actual archive and entry hashes. This preparation record deliberately
does not claim runtime authentication. The subsequent ordinary pack, separate
verify and normal-store install all passed; see the actual
[`runtime-verification.json`](../verification/native-studio-alpha/runtime/runtime-verification.json).
Each command authenticated both current language facades without producer replay.

## Retain the result

After both ordinary facades authenticate, use `agq-publications pack` to create
`accepted-runtime.agq-runtime`, verify it and record its actual outer SHA-256 and
content-addressed manifest. Upload to an immutable draft release, rather than an
expiring verification artifact. The [`runtime-asset` workflow](../.github/workflows/runtime-asset.yml)
downloads an explicit draft asset, checks its independently recorded hash and
authenticates both facades again. It never publishes the draft.
GitHub restricts draft visibility to identities with push access, so the job's
token needs `contents: write` for that read. The token is exposed only to the
download step and checkout does not persist credentials; the workflow contains
no release mutation command. See the [GitHub release API contract](https://docs.github.com/en/rest/releases/releases#list-releases).

See [runtime distribution](runtime-publication-distribution.md) for packaging and
offline installation. Actual authentication results belong to the current
verification record; recovery tooling alone does not establish first light.
