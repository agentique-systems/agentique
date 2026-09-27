# Accepted semantic runtime distribution

Agentique Native Studio loads KerML Operational v9 and SysML Operational v3 as installed
runtime assets. The immutable publications already exist as semantic authority;
installing their bytes is a separate lifecycle from publishing language semantics.
The distribution adapter is `agq-runtime-publications`, outside the kernel and
language crates. It calls their existing receipt-authenticated restore facades.

The accepted bytes are not included in ordinary Git. See the First Light
verification record for whether an actual bundle is available and accepted in
this checkout. A working installer and rejection tests do not establish successful
real-publication installation.

## Operator path

With an accepted bundle supplied as a local directory or `.agq-runtime` file:

```text
cargo run --locked --offline -p agq-runtime-publications --bin agq-publications -- install --bundle /path/to/accepted-runtime.agq-runtime
cargo run --locked --offline --manifest-path crates/studio-native/Cargo.toml --target-dir target
```

`--offline` applies to Cargo after dependencies are cached. It does not grant
semantic authority. The install operation itself performs no network request;
the same authentication applies to bytes downloaded separately and bytes copied
from an offline drive. Normal Studio startup never downloads or republishes.

The standalone lifecycle tool is also available:

```text
cargo run --locked --offline -p agq-runtime-publications --bin agq-publications -- install --bundle /path/to/runtime.agq-runtime
cargo run --locked --offline -p agq-runtime-publications --bin agq-publications -- verify --bundle /path/to/runtime.agq-runtime
cargo run --locked --offline -p agq-runtime-publications --bin agq-publications -- status
cargo run --locked --offline -p agq-runtime-publications --bin agq-publications -- contract
```

`status` reports discovery only; it never reports authentication. `verify` actually
restores both accepted publications. `contract` prints the existing authority
identities without inventing cache transport digests.

## Store and discovery

The normal directory is `~/.agentique` (`%USERPROFILE%\.agentique` on Windows):

```text
.agentique/
  publications/
    <accepted-bundle-identity>/
      manifest.json
      kerml.cache
      systems.cache
```

`accepted-bundle-identity` is SHA-256 over the bundle format and the two compiled
receipt canonical-JSON digests. It is independent of the download URL or checkout.

Discovery order is explicit bundle directory or paired CLI cache paths; installed
store under explicit runtime directory, `AGENTIQUE_RUNTIME_DIR`, or the normal
home directory; then paired `AGENTIQUE_KERML_CACHE` and `AGENTIQUE_SYSTEMS_CACHE`
development overrides. Both cache overrides are required together. An explicit
missing bundle or a damaged installed directory fails closed, rather than silently
selecting a different installation. There are no `verification/generated` defaults.
Archive files are installed explicitly before discovery; ordinary launch does not
extract packages. `--root` identifies the checkout containing pinned library bytes.

## Bundle contract and authentication

A portable `.agq-runtime` file is a ZIP with exactly the same three files as the
directory layout. Cache entries remain unchanged; the outer ZIP stores them without
another compression pass. `manifest.json` uses `agq-accepted-publication-bundle/1`.
For each cache it binds its filename, byte length, SHA-256, operational profile,
publication digest, accepted receipt canonical-JSON identity, binding-manifest
identity and required archive/facade formats. The receipt paths identify the
existing checked-in authorities, not package-supplied replacement receipts.

Canonical JSON identity is SHA-256 of `serde_json::Value` serialized with its
ordered object keys and no insignificant whitespace. This avoids checkout line
ending differences without changing the existing accepted-receipt semantics.

The packager issues a manifest only after both copied cache files independently
pass the normal KerML and Systems restore APIs. A correctly hashed arbitrary file
is still rejected by those APIs. The SysML API also verifies its accepted KerML
dependency. Receipt-bound entry bytes, graph identities, bindings, original library
source bytes and the retained closure certificate remain their existing authority.
No producer closure or standards publication runs.

Installation streams named files into a temporary sibling directory, rejects
length/digest mismatch, authenticates KerML and SysML, syncs file contents, then
atomically renames the entire directory. Archive paths are never extracted;
only the three fixed entry names are read. Interrupted or rejected copies are
never discoverable as installations. Unix additionally syncs directory metadata;
Windows uses file sync and atomic rename. A hard power loss may require reinstall,
but the next load always authenticates the assets again.

An existing installation is independently authenticated and reused. It is never
overwritten by setup. To recover a corrupt installation, stop Studio, move the
reported identity directory aside for diagnosis, then install again. A valid
replacement is still authenticated in staging before it becomes discoverable.

Progress reports only observed phases: locating, copying, authenticating KerML,
authenticating SysML, installing, ready. Loaded facades are retained after setup so
the host can open its repository without immediately restoring them again.
Timings separate transport verification, pinned source verification, KerML restore
and Systems restore; repository/viewport timings belong to Studio.

## Packaging existing accepted bytes

```text
cargo run --locked --offline -p agq-runtime-publications --bin agq-publications -- pack --kerml-cache /path/to/kerml.zip --systems-cache /path/to/systems.zip --output agentique-runtime-kerml-v9-sysml-v3.agq-runtime
```

The output must not exist. An output ending in `.agq-runtime` or `.zip` creates a
portable file; other names create a directory. Packing has no publication API and
cannot fabricate an accepted runtime. The resulting transport manifest and package
can be distributed as release assets; an upload location never changes authority.
Do not substitute a fresh standards publication when the accepted bytes are absent.
