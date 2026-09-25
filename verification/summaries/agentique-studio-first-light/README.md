# Agentique Studio First Light

**AGENTIQUE STUDIO FIRST LIGHT INCOMPLETE**

Branch: `platform/agentique-studio-first-light`, created from clean fetched main
`8b7f5ef79d51e6a3380f659bd1b32897cb6f6b2b`.

The remaining product barrier is the missing accepted KerML Operational v9 and
SysML Operational v3 cache bytes. No copy was located locally, no runtime release
exists, and the inspected CI archive does not contain them. The
[asset search](asset-search.md) records scope and results. Publications were not
regenerated, accepted receipts were not replaced, and fixtures do not establish
real product acceptance.

## Runtime reality

| Required result | Observed result |
| --- | --- |
| Fresh-checkout runtime bootstrap completed | No: no accepted bundle bytes available |
| KerML installed and authenticated | No |
| SysML installed and authenticated | No |
| Normal launch requires old `verification/generated` state | No |
| Normal launch requires Phase 2 SQLite fixture | No |

`agq-runtime-publications` implements receipt-bound pack, verify, install and
discovery commands. Bundle manifests bind exact transport hashes and lengths to
both existing publication, receipt, binding and cache-format identities.
Authentication uses the existing independent facades. Temporary copies are
promoted only after both succeed. The normal location is
`~/.agentique/publications/<receipt-derived-identity>`; explicit runtime-directory
and bundle overrides support isolated environments. Legacy environment cache
pairs remain an explicit compatibility path.

`agq-studio setup --bundle <path>` installs a local directory or portable package.
Normal Studio launch discovers and reauthenticates installed assets, then opens
`~/.agentique/projects/agentique.sqlite`. Setup has an accessible frontend and
machine-readable phases before the semantic service is available. First-run
seeding has a durable source-intent journal and resumes interrupted baseline or
Agent Fabric commits. Existing authored projects are preserved. Packaged bytes
can be reused offline with identical authentication.

The [distribution procedure](../../../docs/runtime-publication-distribution.md)
contains exact pack, verify and release/upload commands. It explicitly identifies
the unavailable release. Successful packaging, installation promotion and
semantic startup remain unverified until the real cache pair is recovered.

## Real Studio acceptance

Every result below is **No — not exercised with accepted publications in this
milestone**, rather than a claim of semantic rejection:

| Required result | Result |
| --- | --- |
| Real Agentique project seeded / Validated | No / No |
| System World / Graph World / Inspector / Explain | No / No / No / No |
| History / semantic visual diff | No / No |
| Agent semantic view / real candidate edit | No / No |
| Validate and operator commit / restart durability | No / No |
| Real agent view, inspect, explain, diff, propose and denied commit | No |

The non-intercepted browser sequence is prepared separately from transport
fixtures. Missing-runtime UI verification cannot substitute for that sequence.
No real semantic screenshot exists from this run. The checked-in self-model was
not changed under a false validation claim; RuntimePublicationStore,
PublicationBundle and StudioHost architecture additions still need accepted
model validation and visual review.

## Platform and performance

| Gate | Result |
| --- | --- |
| Compact semantic-cache restoration and failure authentication | Fail to establish acceptance: not run |
| Real Gen2 HTTP vertical and stable continuation | Fail to establish acceptance: not run |
| 100-document durable semantic scale | Fail to establish acceptance: not run |

Compact cache v2 was already the normal writer on fetched main; legacy decoding
remains compatibility. The gate now explicitly asserts v2. The
[platform record](platform-gates.md) describes the exact helper that discovers
and verifies the installed runtime before starting any gate. Each gate uses fresh
temporary databases, so none depends on the old Phase 2 SQLite file.

Cold and warm semantic launch, publication restore, durable repository restore,
semantic-cache restore, first real view, read-only visual interaction, candidate
preparation, validation and commit are **unmeasured** with real accepted runtime
inputs. No incremental-versus-full speedup is claimed. Compilation instrumentation
now measures construction, refinement, closure/certification, effective audit and
delta capture outside semantic identities and cache payloads. The retained full
oracle prints both incremental and full timings. Audit reuse was not introduced:
the necessary positive reads, negative/provider searches and potential-writer
footprints are not retained in a reusable audit result.

## Verification

Actual commands, exit codes, tested source identities and output hashes are in
`commands.json` and the workstream command ledgers in this directory. Raw logs
remain under `verification/generated/agentique-studio-first-light`.

| Final regression command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo test --workspace` | Pass: 1,034 passed, 0 failed, 10 ignored; 675.58 seconds including compilation |
| `npm run check` / `npm run build` | Pass / Pass |
| `npm test` | Pass: 20 tests |
| `npm run test:e2e` | Pass: 8 existing browser regressions |
| `npm run test:studio` | Pass: real-backend setup test; real-model sequence explicitly skipped |
| `npm run standards:check` | Pass after reviewed non-authoritative inventory refresh |
| `cargo run --locked --offline -p agq-metamodel-gen -- --check` | Pass |
| Rustdoc for runtime, Studio, language-text and workspace, warnings denied | Pass |
| `npm run format:check` | Pass |
| Generation boundary and boundary regressions | Pass |

The runtime package also passed 11 focused authentication/rejection tests.
The final browser setup observation measured 139.4 ms from starting the
unoptimized host to an observed HTTP response, including readiness polling;
missing-runtime discovery took 5 ms. This was an isolated empty runtime/database
test and is not cold semantic launch. Its setup screenshot shows both
publications as **Not authenticated**, a local bundle path field, installation
and discovery controls, and no semantic graph. The final observation and
screenshots remain in generated evidence. No setup screenshot substitutes for
the requested real semantic screenshot.

Independent runtime review found no concrete receipt-authentication bypass or
installation promotion before authentication. It did find that an authored
Agentique project emptied by its operator could have been mistaken for an
interrupted first seed. That path now requires an empty root revision with no
parent and exactly one revision and branch; a regression covers retained history.
Resuming an existing bootstrap journal still requires its exact acknowledged
parent/source contents. Interrupted real semantic seeding remains untested with
accepted publications.

## Product question

Can a technically curious engineer clone Agentique, install its authenticated
semantic runtime, open the product, and visually understand that Agentique is
modeling Agentique?

**Not yet demonstrated.** The exact next requirement is recovery and distribution
of the already accepted cache pair, followed by the prepared real operator
sequence, semantic screenshot, platform gates and latency measurements. The
installer and first-run surface do not supply those absent bytes.
