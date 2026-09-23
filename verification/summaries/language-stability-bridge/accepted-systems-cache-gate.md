# Accepted Systems cache integration gate

`crates/kerml-text/tests/accepted_systems_cache.rs` is an explicit ignored gate.
It requires both original caches and compiled independent Systems receipt
authority before loading any publication. An explicitly requested run fails when
either cache or the receipt is missing; there is no skip or synthetic acceptance.

The gate restores the actual accepted Systems cache, writes a roundtrip cache,
drops the first Systems instance, then restores the roundtrip. It compares
canonical IDs, semantic/publication/context identities, source map, roots, all 69
bindings and their manifest, and the complete closure certificate. KerML is
restored once and shared by Arc; dependency record pointer identity and zero
publication producer counters are asserted. No publication or acquisition API is
called.

Scratch copies of actual cache entries test altered metadata, closure, and graph
bytes, extra and duplicate entries, and truncation. Byte alterations must fail
the compiled receipt digest check before JSON interpretation. Independently
modified binding ID, metaclass and source hash are rejected. Original caches are
read-only. Archive transformations stream data and retain at most one Systems
graph beside the shared KerML instance.

Compile-only verification on the bridge branch (2026-09-23):

| Command | Result | Exit |
| --- | --- | --- |
| `cargo fmt --all -- --check` | clean | 0 |
| `cargo test -p agq-kerml-text --test accepted_systems_cache --no-run` | integration target compiled | 0 |
| `cargo clippy -p agq-kerml-text --test accepted_systems_cache -- -D warnings` | clean | 0 |
| `cargo test -p agq-kerml-text --test accepted_systems_cache -- --list` | one test discovered; none executed | 0 |

Build environment: `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_BUILD_JOBS=2`, isolated
`CARGO_TARGET_DIR=../agentique/target/bridge-self-model`.

**Acceptance pending:** the compiled Systems receipt catalogue is empty. No
actual cache was loaded and no positive or adversarial restoration result is
claimed. After genuine publication acceptance and catalogue activation, run the
gate alone (the cache paths must point to the exact accepted archives):

```powershell
$env:AGENTIQUE_KERML_CACHE = '<exact accepted KerML cache>'
$env:AGENTIQUE_SYSTEMS_CACHE = '<exact accepted Systems cache>'
cargo test -p agq-kerml-text --test accepted_systems_cache accepted_systems_cache_roundtrip_and_tampering -- --ignored --exact --nocapture
```

Do not run beside publication on the 16 GiB development host. One additional
Systems archive of disk space is needed for the scratch roundtrip/tamper file.
