# Agentique accepted runtime: KerML v9 / SysML v3

Draft runtime distribution for maintainers. This is not a Studio release or an
overall Native Studio alpha acceptance claim.

The package restores the existing accepted KerML Operational v9 and SysML
Operational v3 identities. Frozen producer replay and independent comparison
preserved all accepted semantics and bindings. KerML uncompressed payloads are
original-byte exact. Systems retains original facade and closure bytes under a
reviewed, finite versioned graph transport receipt. Original semantic receipts
remain unchanged.

- Bundle identity: `633ea89eb39f8a9e301f2bd5199994455cdf28c8d373fdbd69be30a1402ebcf4`
- Package SHA-256: `37edf34cc0220ecdded8e0162f3fc1955ee2f3849e6dccf7ba373833ade3b026`
- Package bytes: `610190454`
- Installer build source: `23b7910dc79faac0d53a9d0113954a0c0fa6ada9`
- Installer binary SHA-256: `0f656a2c026acbe9bf859d9865a62dd9951202e8e3e4b570a7522e87ea553acb`
- KerML publication: `815573353973607bc62a25195ed4461645182027407f8476be521ffc99174f12`
- Systems publication: `25aeddb099be16462b553d6debcad97f43bf22e89ce8a9bbb53cf72eb7d193fa`

Normal pack, separate verify and normal-store install each authenticated both
language facades, with zero producer replay. Attached manifest and verification
records retain actual asset hashes, accepted receipt identities and command
results. Download and authenticate the uploaded bytes independently before
trusting a remote copy. No release publication is authorized by these records.

From a checkout containing the reviewed transport registration:

```powershell
gh release download runtime-kerml-v9-sysml-v3-bundle1 --repo agentique-systems/agentique --pattern accepted-runtime.agq-runtime --dir .runtime-download
cargo run --release --config profile.release.lto=false --locked --offline -p agq-runtime-publications --bin agq-publications -- verify --bundle .runtime-download/accepted-runtime.agq-runtime
cargo run --release --config profile.release.lto=false --locked --offline -p agq-runtime-publications --bin agq-publications -- install --bundle .runtime-download/accepted-runtime.agq-runtime
cargo run --locked --offline --manifest-path crates/studio-native/Cargo.toml --target-dir target
```

Draft access requires repository push access. For offline installation, transfer
the same package locally and use the install command with its local path. Pinned
dependencies must already be cached for offline compilation.

[Runtime installation and distribution](https://github.com/agentique-systems/agentique/blob/platform/native-studio-alpha/docs/runtime-publication-distribution.md)
