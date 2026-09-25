# Exact Temurin CI version correction

CI [run 36193104237](https://github.com/agentique-systems/agentique/actions/runs/36193104237)
failed in setup-java before Pilot or `npm run verify`: the supplied
`21.0.12.1` is not valid SemVer. Its log is retained in
`checks/ci-fef80ff-failures.txt`. The separate native rustfmt failure is handled
by the integration lead; this patch does not change that code or weaken checks.

Change only the action's version input to **`21.0.12+101.0.LTS`**. This selects the
same intended **Temurin 21.0.12.1+1** release already pinned for Windows Pilot
setup. Distribution, action commit, Java major, Pilot artifacts and verification
steps stay fixed. It is not a floating Java 21 range.

The official [Adoptium release](https://github.com/adoptium/temurin21-binaries/releases/tag/jdk-21.0.12.1%2B1)
exists. A successful HTTP 200 read of the official
[Linux x64 JDK asset metadata](https://api.adoptium.net/v3/assets/feature_releases/21/ga?architecture=x64&image_type=jdk&jvm_impl=hotspot&os=linux&page_size=20&project=jdk&vendor=adoptium)
returned one matching release, with `openjdk_version=21.0.12.1+1-LTS`, patch 1,
build 1, and `semver=21.0.12+101.0.LTS`. Its HotSpot Linux archive is
`OpenJDK21U-jdk_x64_linux_hotspot_21.0.12.1_1.tar.gz`, with vendor checksum
`ce79869e1307ed8ee1e2baa86a412b1eb5b75d10a01006d788a6f968bcfaee94`.
The archive itself was not downloaded or checksum-verified locally. The exact
selected metadata, response digest and retrieval time are retained in
[ci-java-pin-source.json](../checks/ci-java-pin-source.json).

The pinned action's [README](https://github.com/actions/setup-java/blob/cf277c60eb25467037889841efdb72551f06f6c3/README.md#supported-version-syntax)
requires SemVer version syntax. Its
[normalizer](https://github.com/actions/setup-java/blob/cf277c60eb25467037889841efdb72551f06f6c3/src/distributions/base-installer.ts#L130)
checks `semver.validRange` before package lookup. The
[Temurin installer](https://github.com/actions/setup-java/blob/cf277c60eb25467037889841efdb72551f06f6c3/src/distributions/temurin/installer.ts#L31)
matches against Adoptium's `version_data.semver`, and its
[shared comparator](https://github.com/actions/setup-java/blob/cf277c60eb25467037889841efdb72551f06f6c3/src/util.ts#L52)
uses exact `compareBuild` equality when the requested version includes build
metadata. Therefore preserving the full `101.0.LTS` suffix prevents selecting
another build merely sharing the same three-part base version.

The invalid string predates this mission. Exact `git show` inspection found it in
both foundation `ded2a6e7dc616b9305508da2976d65282ebd7b4d` and repository initial
commit `0dd51947fa0e0848028e23ccbbd11feb53bd6312`. `git blame` attributes the input
to the initial commit. Their identities and strings are in the source receipt.

## Actual local checks and limits

HTTP retrieval of the release metadata and five pinned action source files
succeeded. Browsing the API through the web tool failed, and querying its version
route with the raw four-part OpenJDK spelling returned 404; the official feature
release API supplied the successful evidence. No absence claim is based on those
failed routes.

Pure version checks ran with `node -` (exit 0) and npm's existing SemVer 7.6.3,
compatible with the pinned action's `^7.6.0` dependency. They established that
the old input is invalid, the replacement is valid with build metadata, and
exact comparison matches the vendor metadata while rejecting `21.0.12+1.0.LTS`,
`21.0.12+101.0` and `21.0.12+102.0.LTS`. Results are retained in
[ci-java-pin-semver.json](../checks/ci-java-pin-semver.json). The repository does
not itself install SemVer; this check used the existing npm copy under the Node
installation without acquiring a package or changing a lockfile.

`git diff --check` passed. No setup-java action, Java binary, Pilot invocation,
local build, accepted runtime or CI rerun was executed for this patch. The next
CI run must establish actual setup and downstream verification success. The
action's separate v4 deprecation warning is not fixed by changing its version
input; upgrading that pinned action remains a distinct dependency review.

No accepted language input, publication identity, model source, receipt, profile
or verification obligation changes.
