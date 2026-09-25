# Accepted language lock compatibility

`sysml-v3-accepted.Cargo.lock` is the verbatim Git blob at
`9fa5c1357b4faff837fff507173d0f90bb4ead9f:Cargo.lock`, copied without invoking Cargo.
Its SHA-256 is
`f8f56cb388ebfe08af3450edafdbf3021fff9b1e52d0fde4dc5e1a3bc3257583`.
That exact hash already exists in the accepted
`standards/sysml-publication-inputs.json` ledger. The reference is authenticated
against the ledger every time a compatibility proof is required. The receipt,
bindings, input ledger and operational manifests remain byte-identical.

The repository freshness gate retains its original exact comparison when the
workspace lock hash matches. When it differs, the new proof permits changes
outside the complete transitive lock dependency closure of the eight language
roots already covered by the gate: kernel, KerML, KerML semantics, KerML syntax,
KerML text, SysML, SysML semantics and standard libraries. Each reachable package
must retain its exact name, version, source, checksum, and resolved dependency
identities. All target and optional edges present in the lock are included.
Version/source qualification changes in a dependency string are permitted only
when they resolve unambiguously to the same package identity.

The parser supports Cargo's generated version-4 lock subset and fails closed on
unknown fields/tables, duplicate packages or dependencies, absent roots, dangling
references, ambiguous references, absent registry checksums and unsupported
syntax. The reference file cannot be absent or replaced with a newly generated
lock. Every other interpretation input still undergoes the original comparison.
The result is explicitly `accepted-inputs-compatible`, accompanied by both lock
hashes, root identities, package count and closure digest. It does not overwrite
the captured lock hash, issue publication authority, or republish standards.

## Native application isolation

The original attempt to share one lock with the GPU shell did **not** pass this
proof. Native dependencies add `libm` to `num-traits`, `zlib-rs` to `flate2`, and
`futures-macro` to `futures-util`; the lock closure grows from 69 to 72 packages.
Those changed edges are rejected, even though all existing versions/checksums
match. No exception or allowlist covers them.

`crates/studio-native` is consequently a separate Cargo workspace with its own
lock. The reusable `studio-scene` and `studio-platform` crates remain in the
platform workspace. The resulting root lock retains the accepted language
closure exactly (69 packages). The native lock resolves application features
independently and still contains the three additional optional edges; this
compatibility proof does not qualify the native executable's semantic behavior.
Real native self-model/candidate acceptance remains pending availability of the
authenticated accepted runtime bundle. Separate lockfiles do not substitute for
that acceptance test.

Run native checks explicitly, reusing the repository's build output:

```powershell
cargo check --locked --offline --manifest-path crates/studio-native/Cargo.toml --target-dir target
cargo test --locked --offline --manifest-path crates/studio-native/Cargo.toml --target-dir target
cargo clippy --locked --offline --manifest-path crates/studio-native/Cargo.toml --target-dir target --all-targets -- -D warnings
cargo run --locked --offline --manifest-path crates/studio-native/Cargo.toml --target-dir target -- --fixture architecture
```

The root workspace regression excludes the native executable and continues to
cover the shared scene/platform/service crates. Native checks are an additional
required stage. Its default nested `/target` is ignored; `--target-dir target`
avoids unnecessary duplicate graphics builds.
