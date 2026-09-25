# Accepted runtime draft handoff

The [runtime draft](https://github.com/agentique-systems/agentique/releases/tag/untagged-05ba0adcc0765264ca72)
now preserves the rematerialized bytes independently of local worktrees. It is
unpublished and requires repository push access. The integration lead pushed
the reviewed source and created this draft; the runtime stream only inspected
and downloaded it.

Tag: `runtime-kerml-v9-sysml-v3-bundle1`.

Title: `Agentique accepted runtime: KerML v9 / SysML v3`.

Actual release target: `c00de91f348bc59934c721fd75b17cbd09217163`.
The installer build source remains separately recorded as
`23b7910dc79faac0d53a9d0113954a0c0fa6ada9`.

Package SHA-256:
`37edf34cc0220ecdded8e0162f3fc1955ee2f3849e6dccf7ba373833ade3b026`.
Package size: 610,190,454 bytes. Bundle identity:
`633ea89eb39f8a9e301f2bd5199994455cdf28c8d373fdbd69be30a1402ebcf4`.

The four uploaded assets are:

- `accepted-runtime.agq-runtime`: the authenticated package.
- `runtime-manifest.json`: its exact content-addressed manifest.
- `accepted-runtime.agq-runtime.sha256`: the actual outer package hash.
- `runtime-verification.json`: completed local pack, verify and install evidence,
  including source and executable identities.

A fresh download of every asset completed in 54.125 seconds. Independent
streaming checks took 2.125 seconds and matched all local asset bytes and
GitHub-reported hashes/lengths. The release body matches the reviewed notes,
its published date remains null, and the embedded manifest matches the separate
asset. `remote-distribution-result.json` retains this evidence alongside the
exact metadata, download and comparison command outputs.

All three local ordinary facade gates passed and independent boundary review
accepted those results. A fresh facade run on the downloaded copy remains
queued until the real native journey releases the runtime. The comparison
helper never invokes a standards consumer. The uploaded verification asset
accurately records local authentication at upload time; subsequent remote
checks are retained in Git separately.

A maintainer with an authenticated `gh` session can download from a checkout
containing the reviewed transport registration:

```powershell
gh release download runtime-kerml-v9-sysml-v3-bundle1 --repo agentique-systems/agentique --pattern accepted-runtime.agq-runtime --dir .runtime-download
$runtimePackageSha = (Get-FileHash -Algorithm SHA256 .runtime-download/accepted-runtime.agq-runtime).Hash.ToLowerInvariant()
if ($runtimePackageSha -ne "37edf34cc0220ecdded8e0162f3fc1955ee2f3849e6dccf7ba373833ade3b026") { throw "Runtime package transport mismatch" }
cargo run --release --config profile.release.lto=false --locked --offline -p agq-runtime-publications --bin agq-publications -- verify --bundle .runtime-download/accepted-runtime.agq-runtime
cargo run --release --config profile.release.lto=false --locked --offline -p agq-runtime-publications --bin agq-publications -- install --bundle .runtime-download/accepted-runtime.agq-runtime
cargo run --locked --offline --manifest-path crates/studio-native/Cargo.toml --target-dir target
```

Use a fresh download directory. Never overwrite existing release assets with
`--clobber`; this draft must remain unpublished during alpha work. The
`runtime-asset.yml` workflow can independently download and authenticate the
same draft using its exact tag and package hash. It has not been dispatched.

The retained downloaded copy is in
`verification/generated/native-studio-alpha/runtime-remote-download-2026-09-25/`
of the runtime worktree. Once the integration lead releases the memory gate,
its ordinary verification command from the integration checkout is:

```powershell
target/native-alpha/release/agq-publications.exe --root . verify --bundle C:/Users/phili/github/agentique-systems/agentique-alpha-runtime/verification/generated/native-studio-alpha/runtime-remote-download-2026-09-25/accepted-runtime.agq-runtime
```

For offline installation, transfer the same package locally and use the install
command with that path. Pinned Rust dependencies must already be cached for
offline compilation. Installation never downloads sources or regenerates
standards. The default store is `~/.agentique/publications/<bundle identity>`;
project databases remain separate.
