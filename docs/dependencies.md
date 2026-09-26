# Dependencies and reproducibility

Cargo.lock pins the exact Rust dependency graph and registry checksums;
package-lock.json pins the npm graph, registry URLs and integrity hashes. Both
belong in version control. Use `--locked` and `npm ci`. Rust is pinned to 1.92.0
in rust-toolchain.toml; CI uses Node 22.11.0. No AI key is required for the build
or deterministic acceptance suite. SQLite is bundled through rusqlite.

Official standards are a separate baseline: standards/lock.json records publication
identifiers, source URLs, exact hashes and selected archive-entry metadata. The
original downloads are separately locked in standards/baseline-lock.json.
The selected libraries are the official corrective 2026-04 publication, pinned
to commit 9baca5908ca28b53da085de69336fde48420ea8f. Run
`npm run standards:check` to verify every artifact, all extracted files and eight
declared library dependencies. The semantic crate's build script independently
refuses changed or extra library source files. `node tools/baseline.mjs` implements
initial acquisition and refuses to replace a selected corrective release.
`node tools/library-release.mjs` reproduces the selected lock without modifying
original bytes. The normal build uses vendored bytes and needs no OMG download.

The npm dependency audit on 2026-09-16 found a moderate malformed-ZIP64 issue in
fflate 0.8.2 (GHSA-px8p-9vwx-vf98). The pinned dependency was upgraded to 0.8.3;
the subsequent npm install audit reported zero vulnerabilities. fflate is used
only in engineering acquisition/check tools; runtime project import uses Rust zip.

GitHub Actions dependencies were resolved with `git ls-remote REPOSITORY refs/tags/v4`
on 2026-09-16 and pinned to their observed commits:

| Official repository | v4 commit |
|---|---|
| https://github.com/actions/checkout | 11d5960a326750d5838078e36cf38b85af677262 |
| https://github.com/actions/setup-node | 49933ea5288caeca8642d1e84afbd3f7d6820020 |
| https://github.com/actions/setup-java | cf277c60eb25467037889841efdb72551f06f6c3 |
| https://github.com/actions/upload-artifact | ea165f8d65b6e75b540449e92b4886f43607fa02 |

The independent `sysml-validate` package is pinned at 0.43.1. It is an external,
Freeware-licensed development tool, not part of the Rust Engine and not represented
as an OMG conformance authority. Its npm package and dependencies are recorded in
package-lock.json. Original OMG copyright and licence notices remain in the PDFs
and library sources. No standard definitions were copied into hand-written
replacement libraries. The generated build embeds the selected official sources.

The stronger independent check uses published SysML pilot 0.59.0, distributed
through the conda-forge package recommended by the official 2026-04 installer.
`node tools/pilot-setup.mjs` acquires its SHA256-pinned archive and, on Windows,
a pinned portable Temurin 21.0.12.1+1 JDK into `.cache`. On other hosts use Java 21
or set `AGENTIQUE_JAVA` to its executable. `fzstd` 0.1.1 decodes the official
package's Zstandard payload during setup; it is an engineering dependency only.
Linux CI selects the same Temurin release with Adoptium's exact SemVer
`21.0.12+101.0.LTS`, which the pinned setup-java action accepts.
`node tools/pilot-validate.mjs` loads byte-identical copies of the active libraries,
checks all starter/fixture files with CheckMode.ALL and records source/JAR hashes.

The Engine build identifier hashes Rust sources and manifests, Cargo.lock, toolchain
configuration, target, build profile and compiler version. Verification also records
a broader source-file hash manifest. Debug and release build identities are distinct.
An experiment can be inspected after a software update, but reset/resume requires
compatible pinned dependencies; it does not silently substitute a new Engine build.

The metamodel importer adds `roxmltree` 0.21.1, a maintained MIT/Apache-2.0 XML parser
with namespace resolution and source positions. Its `memchr` dependency was already
in Cargo.lock. It runs only in `tools/metamodel-gen`; serde, serde_json, sha2, uuid
and test-only tempfile reuse workspace dependencies. The tool depends on agq-kernel
for typed ID construction; the kernel dependency graph and source are unchanged.
No database, async, web, simulation or language parsing dependency is added.

KerML MOF XMI, its JSON serialization schema and referenced UML primitive types are
separately locked in `standards/normative/kerml-1.0/lock.json`. The standard integrity
command verifies all three. `npm run metamodel:check` verifies exact regenerated IR.
See ADR 0002 for the reproducible acquisition command and JSON cross-check scope.
Normal dependency provisioning (`cargo fetch --locked`, `npm ci`, local browser
installation) precedes offline builds/tests; it does not retrieve standards.
