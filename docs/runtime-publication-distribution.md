# Distributing the accepted semantic runtime

Publication and distribution are separate lifecycle operations. The checked-in
KerML Operational v9 and SysML Operational v3 receipts are the semantic authority.
The runtime bundle packages existing accepted bytes and authenticates them with
the existing language facades. It does not run producers or regenerate standards.

**Availability:** no runtime Release asset was found during First Light work.
The commands below are the release procedure, not a claim that a downloadable
package exists. A maintainer must recover the two accepted caches from retained
storage before a package can be issued. See the [asset search record](../verification/summaries/agentique-studio-first-light/asset-search.md).

## Package and authenticate

Use a checkout of the reviewed release commit, with the pinned library bytes
present. From its root, replace the two input paths with existing accepted caches:

```powershell
cargo run --release --config profile.release.lto=false --locked --offline -p agq-runtime-publications --bin agq-publications -- pack --kerml-cache C:/accepted/kerml/canonical.publication.zip --systems-cache C:/accepted/systems/canonical.publication.zip --output C:/packages/agentique-runtime-kerml-v9-sysml-v3.agq-runtime
cargo run --release --config profile.release.lto=false --locked --offline -p agq-runtime-publications --bin agq-publications -- verify --bundle C:/packages/agentique-runtime-kerml-v9-sysml-v3.agq-runtime
Get-FileHash -Algorithm SHA256 -LiteralPath C:/packages/agentique-runtime-kerml-v9-sysml-v3.agq-runtime
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
gh release create runtime-kerml-v9-sysml-v3-bundle1 C:/packages/agentique-runtime-kerml-v9-sysml-v3.agq-runtime --repo agentique-systems/agentique --target $runtimeReleaseCommit --draft --title "Agentique accepted runtime: KerML v9 / SysML v3" --notes-file C:/packages/runtime-release-notes.md
```

The notes must contain the package SHA-256, bundle identity, source commit and
both accepted publication identities, and link the install instructions. Review
the uploaded draft asset and independently verify its downloaded bytes before
publishing the release. Publishing is a release operation, never a build step.

## Install the released bytes

Once that release exists, an engineer can obtain the exact named package:

```powershell
cargo fetch --locked
gh release download runtime-kerml-v9-sysml-v3-bundle1 --repo agentique-systems/agentique --pattern agentique-runtime-kerml-v9-sysml-v3.agq-runtime --dir .runtime-download
cargo run --release --config profile.release.lto=false --locked --offline -p agq-studio -- setup --bundle .runtime-download/agentique-runtime-kerml-v9-sysml-v3.agq-runtime
npm ci
npm run build
cargo run --release --config profile.release.lto=false --locked --offline -p agq-studio
```

For offline installation, copy the same package from removable or local storage
and use the same setup command. A bundle directory works as well. Setup streams
to temporary storage, checks transport digests, authenticates KerML and Systems,
and promotes the installed directory only after every check succeeds. Network
location, release name and package labels confer no semantic authority.

`--runtime-dir C:/runtime/isolated` selects an isolated store for setup and launch.
Normal launches discover the installed runtime; they do not download packages,
refer to old `verification/generated` paths or need a retained Phase 2 database.
The first project is seeded from `models/agentique`; existing projects restore
from their durable revisions. See [Studio](agentique-studio.md) and the
[bundle contract](runtime-publications.md).
