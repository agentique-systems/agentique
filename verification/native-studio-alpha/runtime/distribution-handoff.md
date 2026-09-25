# Accepted runtime draft handoff

This handoff preserves the rematerialized runtime independently of local
worktrees. It is a draft runtime asset, not a Studio release. The repository is
public, but draft assets require a maintainer with push access. Read-only checks
on 2026-09-25 found no existing releases and confirmed that the current GitHub
identity has push, maintain and admin access. See `draft-release-permissions.json`
and `draft-release-existing.json` with their retained command outputs.

Proposed tag: `runtime-kerml-v9-sysml-v3-bundle1`.

Proposed title: `Agentique accepted runtime: KerML v9 / SysML v3`.

Assets prepared after successful ordinary facade gates:

- `accepted-runtime.agq-runtime`: the actual authenticated package.
- `runtime-manifest.json`: the package's exact content-addressed manifest.
- `accepted-runtime.agq-runtime.sha256`: the actual outer package hash.
- `runtime-verification.json`: completed pack, verify and install evidence with
  source/binary identities. This file is evidence, not semantic authority.

The release body is `runtime-release-notes.md`, generated from that manifest and
the actual command records. Its source commit must be reachable on GitHub and
must contain the reviewed transport registration. Original accepted semantic
receipts and bindings remain unchanged.

The integration lead creates the draft only after ordinary `pack`, separate
`verify`, normal-store `install` and independent boundary review succeed. Never
overwrite assets with `--clobber`. Do not publish this draft during alpha work.

After uploading, download the exact named package into a fresh directory and
compare it with the retained SHA-256 before running normal facade verification:

```powershell
gh release download runtime-kerml-v9-sysml-v3-bundle1 --repo agentique-systems/agentique --pattern accepted-runtime.agq-runtime --dir verification/generated/native-studio-alpha/runtime-redownload
Get-FileHash -Algorithm SHA256 verification/generated/native-studio-alpha/runtime-redownload/accepted-runtime.agq-runtime
target/release/agq-publications.exe --root . verify --bundle verification/generated/native-studio-alpha/runtime-redownload/accepted-runtime.agq-runtime
```

The `runtime-asset.yml` workflow provides the same independent download and
facade gate once that workflow and registration exist on the selected remote
source revision. Dispatch it with the exact draft tag and the independently
recorded package hash. It never publishes a release. A local package or an
undispatched workflow alone does not establish remote distribution.

Offline installation uses the identical asset:

```powershell
cargo run --release --config profile.release.lto=false --locked --offline -p agq-runtime-publications --bin agq-publications -- install --bundle C:/packages/accepted-runtime.agq-runtime
cargo run --locked --offline --manifest-path crates/studio-native/Cargo.toml --target-dir target
```

Pinned Rust dependencies must already be available for offline compilation.
Installation itself does not download sources or regenerate standards. The
default store is `~/.agentique/publications/<bundle identity>`; project databases
remain separate from the content-addressed runtime.
