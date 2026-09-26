# Alpha acceptance code freshness: review and conditional update plan

**Do not apply a freshness update yet.** This review establishes the required inventory and update procedure. It does not establish semantic equivalence, runtime authentication, or Alpha acceptance. No pins, receipts, manifests, standard-library bytes or runtime assets were modified by this review.

The read-only snapshot in `freshness-review-snapshot.json` observed source at `5e0dfb0734ad3cb0a53c8c0239125c084b8a0a03` against fetched main `98c881f0bbac2c55e2777664633a50cb67ab7bf0`. HEAD was unchanged during capture. The ledger's normalized SHA-256 was `3476f5bf570fcfd7ba88c973b4a6750ba05e11b1300014bc05f2af84de1e1041` and remained unchanged during the probe. Later source edits require a new snapshot and review.

## What the existing workflow does

`tools/sysml-publication-stale.mjs` separates code freshness from accepted publication authority. `capturePublicationInputs` validates the accepted receipts, bindings, semantic manifest, source-content-set and KerML dependency identity, then recursively inventories production Rust sources under eight language/kernel roots. It includes their Cargo manifests, grammar/operational/errata manifests and runtime transport JSON population. Source hashes normalize CRLF to LF.

`verifySystemsPublicationFreshness` compares the complete captured object with `standards/sysml-publication-inputs.json`, so additions and removals fail as well as changed hashes. An independently authenticated language-lock closure proof can retain the accepted Cargo.lock pin when unrelated workspace dependencies change. It does not recapture that pin.

The earlier `verification/scripts/native_alpha_freshness_plan.py` is intentionally narrow: three exact old/new hashes for the previous mount/audit optimization, only `mount` and `mount-and-audit` phases, no inventory additions. Its full-object comparison rejects anything else. Those reviewed hashes no longer describe this implementation. Do not run its old patch, turn its assertions into warnings, or use the stale-checker's blanket `--capture` command to accept current code.

## Exact inventory changes currently required

The existing inventory contains 151 entries; the current capture contains 154. There are **20 changed existing entries, three additions and no removals**. Every exact old/new hash is retained in the snapshot's `changes` array. The complete publication identity, receipt/binding hashes and all other non-input fields compare equal.

The three additional production entries are required:

| New module | Why it belongs in freshness |
| --- | --- |
| `crates/kerml-semantics/src/closed_query_audit.rs` | Closed-context audit read/signature transport and invalidation evidence. |
| `crates/kerml-text/src/source_effective_audit.rs` | Authored strict effective audit orchestration and successful-outcome reuse. It is loaded through a `#[path]` module. |
| `crates/kerml-text/src/runtime_restore_trace.rs` | Runtime restore tracing in the language facade. It is production code even though its purpose is measurement. |

All three are already found by the recursive checker. **No source-root or filename-filter relaxation is needed.** Omitting them from the updated ledger would and should remain stale. These modules must not be renamed to test-style filenames to evade capture.

The changed existing production entries are:

| Area | Files below that crate's `src/` |
| --- | --- |
| `kerml-semantics` | `context.rs`, `lib.rs`, `producer_closure.rs`, `producer_closure_incremental.rs`, `producer_closure_rebind.rs`, `publication_restore.rs`, `queries.rs`, `read_dependencies.rs` |
| `kerml-text` | `lib.rs`, `library/publication_cache.rs`, `source_checkpoint.rs`, `source_inputs.rs`, `sysml.rs`, `sysml/publication.rs`, `sysml/publication_restore.rs`, `sysml/source.rs` |
| `kernel` | `archive.rs`, `derived.rs`, `derived/archive_restore.rs` |
| `sysml-semantics` | `queries.rs` |

All six changed language/kernel files excluded by this inventory are actual test modules or files under `tests/`; their names and capture status are also retained in the snapshot. The new closed-audit test is excluded because it is under `tests/unit`, while its production implementation is captured. Tests still belong in exact executable/build receipts and must run; exclusion from publication-input hashing is not exemption from qualification.

Modeling workspace/service/view, runtime-distribution tooling, Studio platform/scene/native sources and their new modules are outside the existing language interpretation inventory. This review does not propose adding the entire application to accepted language freshness. They require their own exact build receipts and durability/native gates. In particular, the workspace checkpoint caller and service restoration path cannot claim qualification merely because the language-input ledger becomes current.

## Authority that must remain identical

The snapshot confirms the following six protected documents are unchanged from fetched main: both accepted publication receipts, both binding manifests, `standards/runtime-transports/sysml-v3-rematerialized-2026-09-25.json`, and the normative SysML library-set manifest. All other captured manifests are unchanged as well.

Preserve every field of the accepted publication identity, including semantic/publication digests, closure/registry/context/dependency contracts, descriptors, profile/rule set, grammar/correction manifests, KerML dependency, KPAR and library content set. Preserve the compiled authority catalogue, normative artifact bytes, transport digests and accepted runtime archive. No new semantic profile, republishing, cache-package rematerialization or changed validation receipt is implied by a code-freshness edit.

Current observed Systems identity:

* Publication: `25aeddb099be16462b553d6debcad97f43bf22e89ce8a9bbb53cf72eb7d193fa`
* Semantic: `71b55e0f529e58a8dc1c69c95c9372a00777aa43918cd67c19de8040dfc177ec`
* Accepted KerML semantic: `815573353973607bc62a25195ed4461645182027407f8476be521ffc99174f12`
* Producer closure: `39b22b5ea1b536369255d73ca80a26633a0ebc97fab30c14d2a9bc91e9657b11`
* Systems KPAR: `df7d8b2c6e08232ca7ce123a63148949c383fcbeaeba8d89c27ceece43793a1f`
* Systems source-content-set: `6cceb50286d6edd411f327201b6016b5651744d4a30175c78e019810d482e928`

The accepted Cargo.lock pin remains `f8f56cb388ebfe08af3450edafdbf3021fff9b1e52d0fde4dc5e1a3bc3257583`. The current lock differs, but the existing checker proves all 69 reachable accepted language packages and their exact edges/checksums unchanged; closure digest `5cd40b34553cfc18534928eaad2465888325770e7755d4d85b1ef0550b48b340`. Do not replace the accepted lock pin with the current workspace lock hash.

## Concrete procedure after proof passes

1. Freeze the exact production source tree being qualified. Retain the release executable hashes, source hashes/commit and commands for the successful separate-process command/cold oracle. Require both reconstructions to validate and all exact observations to compare equal, including full effective-query evidence/completeness and strict-audit outcomes. A preparation timing, completed process, skipped test, failed baseline or equal aggregate counts is insufficient.
2. Retain current-code ordinary runtime authentication results for the existing accepted KerML v9 and Systems v3 asset. The immutable dependency/closure optimizations must pass the adversarial producer/closed-audit tests, archive roundtrip/checkpoint restoration tests, and strict audit parity gates. A freshness snapshot is not a substitute for those proofs. If runtime authentication or exact equivalence fails, keep the ledger stale and continue diagnosis.
3. Re-run the read-only probe against that exact source tree and compare its entire difference set with the reviewed snapshot. Resolve any new source hash or population change through source review and the applicable proof again. Formatting changes also change the input hash. Record exact proof artifact hashes alongside the new observation.
4. Extend the narrow proposal helper with a separate `alpha-acceptance` review scope; retain the historical phases unchanged. Its frozen reviewed entries must include the exact 20 `(old, new)` pairs and three `(absent, new)` additions. A newly added entry must be absent in the prior ledger and equal its independently reviewed normalized hash. Reject all unexpected additions/removals, different old pins, source drift, authority changes and incompatible language locks. Do not derive its permitted hash list blindly from whatever the workspace currently contains.
5. The helper should produce a proposal only, in a fresh evidence directory. Preserve every ledger field outside the reviewed `inputs` entries, including the old Cargo.lock pin. Require the proposed parsed JSON to equal the complete freshly captured object after only the existing lock-compatibility substitution. Retain normalized before/proposed ledger hashes and the required proof receipt identities. Existing proposal directories must remain non-overwritable.
6. Review the resulting patch as exactly 20 replacement values plus three added entries, sorted consistently. Before application, repeat the exact source/hash comparison and `git apply --check`. Root applies the reviewed patch; no script should silently rewrite the manifest as a side effect of checking it.
7. Run `npm run standards:check` and retain command/output/exit. Re-run the stale-checker and proposal-tool regression tests; the new proposal scope needs cases for a missing required new module, an unexpected module, a changed reviewed hash, removal, identity/receipt/binding drift, lock incompatibility and non-overwrite of proposal evidence. Complete the remaining required repository checks against the final tree. A green standards check demonstrates reviewed freshness/identity consistency, not Alpha acceptance.

The only intended standards mutation is the reviewed `inputs` change in `standards/sysml-publication-inputs.json`. If the proof reveals an accepted semantic identity change, this plan no longer applies; investigate the optimization instead of changing accepted identity pins to make it pass.

## Commands performed for this review

* `node verification/native-studio-acceptance/freshness_review_probe.mjs` — exit 0; complete observation retained in `freshness-review-snapshot.json`. The probe calls the read-only capture export; it does not invoke the `--capture` writer.
* `python -m unittest discover -s verification/scripts -p test_native_alpha_freshness_plan.py` — exit 0; five bookkeeping tests passed; output retained in `freshness-plan-legacy-tests.txt`.
* `node --test tools/sysml-publication-stale.test.mjs` — exit 0; 15 tests passed, including new source-population and transport-population rejection; output retained in `freshness-checker-tests.txt`.

No Cargo build, semantic runtime execution, pin edit, release action or Git commit was performed by this review. Root alone stages and commits these new evidence files.
