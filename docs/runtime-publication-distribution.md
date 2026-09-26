# Distributing the accepted semantic runtime

Publication and distribution are separate lifecycle operations. The checked-in
KerML Operational v9 and SysML Operational v3 receipts are the semantic authority.
The runtime bundle packages existing accepted bytes and authenticates them with
the existing language facades. It does not run producers or regenerate standards.

**Availability:** the 2026-09-25 [exact accepted-cache rematerialization](runtime-rematerialization.md)
succeeded. Both current language facades passed ordinary packaging, separate
bundle verification and normal-store installation. The resulting
`accepted-runtime.agq-runtime` is 610,190,454 bytes with SHA-256
`37edf34cc0220ecdded8e0162f3fc1955ee2f3849e6dccf7ba373833ade3b026`.
The [manifest](../verification/native-studio-alpha/runtime/runtime-manifest.json)
and [actual verification record](../verification/native-studio-alpha/runtime/runtime-verification.json)
retain the accepted identities and command results. The
[runtime draft](https://github.com/agentique-systems/agentique/releases/tag/untagged-05ba0adcc0765264ca72)
now retains all four assets. An independent download matched every local asset
and GitHub-reported digest exactly. Access requires repository push permission;
the draft remains unpublished. See the [download evidence](../verification/native-studio-alpha/runtime/remote-distribution-result.json)
and [handoff](../verification/native-studio-alpha/runtime/distribution-handoff.md).
A fresh ordinary facade run on the downloaded copy also passed in 72.391 seconds,
authenticating both accepted profiles. Its [command and timing evidence](../verification/native-studio-alpha/runtime/remote-runtime-authentication.json)
completes the remote distribution gate without publishing the draft.

## Package and authenticate

Use a checkout of the reviewed release commit, with the pinned library bytes
present. From its root, replace the two input paths with existing accepted caches:

```powershell
cargo run --release --config profile.release.lto=false --locked --offline -p agq-runtime-publications --bin agq-publications -- pack --kerml-cache C:/accepted/kerml/canonical.publication.zip --systems-cache C:/accepted/systems/canonical.publication.zip --output C:/packages/accepted-runtime.agq-runtime
cargo run --release --config profile.release.lto=false --locked --offline -p agq-runtime-publications --bin agq-publications -- verify --bundle C:/packages/accepted-runtime.agq-runtime
Get-FileHash -Algorithm SHA256 -LiteralPath C:/packages/accepted-runtime.agq-runtime
```

The packager refuses an existing output. It emits `manifest.json`, `kerml.cache`
and `systems.cache` in one ZIP container using the `.agq-runtime` extension.
The generated manifest contains the actual asset lengths and SHA-256 digests,
both publication identities, required cache formats, receipt identities and
binding manifest identities. Unknown hashes cannot be filled with placeholders.
Packaging fails unless each cache independently authenticates to its receipt.

Record the source commit, outer package SHA-256, generated manifest, successful
verification output and exit code in the release evidence. The package's outer
hash is useful transport metadata; facade authentication remains mandatory.
Do not put the caches in ordinary Git history or CI verification archives.

## Upload a release asset

After the package has passed verification, a release maintainer can prepare a
draft using the following commands. Use a new immutable tag if this package name
has already been released; never overwrite an existing release asset with
`--clobber`.

```powershell
$runtimeReleaseCommit = git rev-parse HEAD
gh release create runtime-kerml-v9-sysml-v3-bundle1 C:/packages/accepted-runtime.agq-runtime --repo agentique-systems/agentique --target $runtimeReleaseCommit --draft --title "Agentique accepted runtime: KerML v9 / SysML v3" --notes-file C:/packages/runtime-release-notes.md
```

The notes must contain the package SHA-256, bundle identity, source commit and
both accepted publication identities, and link the install instructions. Review
the uploaded draft asset and independently verify its downloaded bytes before
publishing the release. The manual `runtime-asset.yml` workflow performs this
independent download/hash/facade check for an existing draft or published tag and
recorded SHA-256. It then installs into a fresh isolated store and compares both
installed caches to the authenticated manifest. Each actual command, exit code,
stdout/stderr digest and elapsed time is retained by
`tools/verify-runtime-distribution.py`. It never publishes a release. Publishing
is a release operation, never a build step.

For the existing bundle, dispatch qualification on the reviewed application branch:

```powershell
gh workflow run runtime-asset.yml --repo agentique-systems/agentique --ref platform/native-studio-alpha-acceptance -f draft_tag=runtime-kerml-v9-sysml-v3-bundle1 -f transport_sha256=37edf34cc0220ecdded8e0162f3fc1955ee2f3849e6dccf7ba373833ade3b026
```

The existing tag and asset need no recreation. Once the release maintainer has a
successful current facade/install record, publication can make these same bytes
public. Record the workflow run URL and authentication source commit in release
notes; older timing evidence does not qualify a changed application build.

## Install the released bytes

Once that release exists, an engineer can obtain the exact named package:

```powershell
cargo fetch --locked
cargo fetch --locked --manifest-path crates/studio-native/Cargo.toml
gh release download runtime-kerml-v9-sysml-v3-bundle1 --repo agentique-systems/agentique --pattern accepted-runtime.agq-runtime --dir .runtime-download
$runtimePackageSha = (Get-FileHash -Algorithm SHA256 -LiteralPath .runtime-download/accepted-runtime.agq-runtime).Hash.ToLowerInvariant()
if ($runtimePackageSha -ne "37edf34cc0220ecdded8e0162f3fc1955ee2f3849e6dccf7ba373833ade3b026") { throw "Runtime package transport mismatch" }
cargo run --release --config profile.release.lto=false --locked --offline -p agq-runtime-publications --bin agq-publications -- install --bundle .runtime-download/accepted-runtime.agq-runtime
cargo run --locked --offline --manifest-path crates/studio-native/Cargo.toml --target-dir target
```

Use a new download directory. While the release is still a draft, downloading
requires repository push access; ordinary users cannot install from that draft
URL. Release metadata determines whether it has been published. On Linux/macOS,
compare `shasum -a 256` output against the same hash, then run the identical Cargo
installation command. Installation authenticates both facades; a matching outer
hash alone is not an installation pass.

To reproduce the full release qualification locally without touching the normal
operator store, build the CLI and provide unused installation/evidence directories:

```powershell
cargo build --release --config profile.release.lto=false --locked --offline -p agq-runtime-publications --bin agq-publications
python tools/verify-runtime-distribution.py --publications target/release/agq-publications.exe --bundle .runtime-download/accepted-runtime.agq-runtime --sha256 37edf34cc0220ecdded8e0162f3fc1955ee2f3849e6dccf7ba373833ade3b026 --install-dir verification/generated/runtime-qualification-store --evidence-dir verification/generated/runtime-qualification
```

On Linux/macOS omit `.exe`. The helper fails if either directory already exists;
it never overwrites an installed runtime. It leaves the isolated store available
for diagnosis and writes only logs/metadata to evidence, not runtime cache bytes.

For offline installation, copy the same package from removable or local storage
and use the same install command. A bundle directory works as well. Installation
streams to temporary storage, checks transport digests, authenticates KerML and Systems,
and promotes the installed directory only after every check succeeds. Network
location, release name and package labels confer no semantic authority.

`--runtime-dir C:/runtime/isolated` selects an isolated store for installation and launch.
Normal launches discover the installed runtime; they do not download packages,
refer to old `verification/generated` paths or need a retained Phase 2 database.
The first project is seeded from `models/agentique`; existing projects restore
from their durable revisions. See [Native Studio](native-studio.md) and the
[bundle contract](runtime-publications.md).
