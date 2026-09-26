# Post-proof freshness proposal helper

`freshness_proposal.py` emits `proposal.patch` and `proposal.json` in a new directory. It never updates `standards/sysml-publication-inputs.json`, runs semantic commands, publishes a release, or establishes publication authority. The existing historical proposal helper is unchanged.

The approved scope is fixed by the normalized SHA-256 of `freshness-review-snapshot.json`: exactly 20 replacements and three new production inputs. Every other captured field must match, including authority and the original accepted Cargo.lock pin after the existing language-lock compatibility proof. A later production change requires an independently reviewed snapshot and helper pin; there is no permissive recapture option.

## Proof inputs

Place the downloaded candidate oracle and runtime-asset evidence under a common directory. Create `proof-set.json` there, with this shape. Replace each `REVIEWED_SHA256` only after reviewing the actual file and the successful CI/local run which produced it. Artifact paths are relative to this proof file and cannot escape its directory.

```json
{
  "format": "agentique-alpha-freshness-proof-set/1",
  "reviewer": "Actual reviewer identity",
  "review_note": "Exact source/proof review, applicable semantic unit/parity gate receipts, and originating CI run/job links",
  "oracle": {
    "build": {"path": "candidate-oracle/build.json", "sha256": "REVIEWED_SHA256"},
    "invocation": {"path": "candidate-oracle/oracle/invocation.json", "sha256": "REVIEWED_SHA256"},
    "result": {"path": "candidate-oracle/oracle/observations/result.json", "sha256": "REVIEWED_SHA256"},
    "command_metrics": {"path": "candidate-oracle/oracle/observations/command-metrics.json", "sha256": "REVIEWED_SHA256"},
    "cold_metrics": {"path": "candidate-oracle/oracle/observations/cold-metrics.json", "sha256": "REVIEWED_SHA256"},
    "command_observations": {"path": "candidate-oracle/oracle/observations/command-observations.json", "sha256": "REVIEWED_SHA256"},
    "cold_observations": {"path": "candidate-oracle/oracle/observations/cold-observations.json", "sha256": "REVIEWED_SHA256"}
  },
  "runtime": {
    "transport": {"path": "runtime-evidence/transport.json", "sha256": "REVIEWED_SHA256"},
    "qualification": {"path": "runtime-evidence/lifecycle/qualification.json", "sha256": "REVIEWED_SHA256"}
  }
}
```

The runtime lifecycle directory must also retain the six `verify/install/status` stdout/stderr files. Their hashes are checked against the qualification receipt. The accepted package SHA-256 and bundle identity are fixed in the helper; arbitrary replacement transport cannot qualify. The helper checks ordinary verify/install output shapes, both accepted profile/publication identities, install binding, command exits and output hashes.

The oracle must use `agentique-create-part-performance/4`, with successful build/invocation receipts for the exact executable and harness. Both command and independent cold metrics must record validation and no semantic cache use. The helper compares the complete retained observation maps, required semantic/query families, canonical population, final metrics, all 16 malformed reuse rejections, and the parent result. Success flags or equal counts alone are insufficient.

Both recorded source commits must exist locally. `git diff <proof-commit>` over the full relevant language/kernel/modeling/runtime implementation, Cargo inputs, standards and self-model source must be empty except for the single exact test addition reviewed below; untracked inputs are refused. Thus an old baseline oracle cannot qualify current production code. Unrelated Studio UI, evidence and documentation changes do not invalidate a language proof. Baseline harness overlays are deliberately not accepted as candidate qualification evidence.

### One reviewed test-only source difference

Runtime qualification run `36231482800` and the candidate oracle build use source `a6e1f41d2b549a8be10a79dfa3ec786c090b65b3`. Commit `69287bdb9e0ad838d6355d981bf5b9ce7131edac` subsequently adds four adversarial tests in `crates/kerml-semantics/tests/unit/closed_query_audit.rs`. Its only compiler inclusion is `#[cfg(test)] #[path = "../tests/unit/closed_query_audit.rs"] mod tests;` at the end of the unchanged production `src/closed_query_audit.rs`. These additions are outside the language library used by the oracle integration executable and ordinary runtime installer. No production file or oracle harness is exempted.

The helper permits only that exact file transition, normalized SHA-256 `43864451aa3925cc0a8a888417b81b20dc85839fccad5356aa3fd8181c2e1cfc` → `457b57ba88bf32921006bd61d29d12e78658093bcc6f1f3d88c1269ae6ef2534`. It reads the before bytes from the proof commit, checks the current after bytes, and requires the exact retained local test receipt/output. Any further test edit or another changed test path is rejected. Applied exceptions and both hashes appear in the emitted proof summary.

The reviewed gate is `cargo test --locked --offline -p agq-kerml-semantics --lib closed_query_audit -- --nocapture`, exit 0, eight passed, zero failed/ignored. The test ran against the changed working file before its commit; the receipt honestly retains that dirty-source context. Its pinned receipt is `verification/native-studio-alpha/checks/acceptance-closed-audit-adversarial-tests.json` (normalized SHA-256 `abe245d4f36d405e92f9b9b1ed56113fc405136b9b72d9f53516a9d654b59915`), with output SHA-256 `43502c2357db190a615356e20f0536bfab99e82ecb3184885e0b80d4b682b1d4`. These local tests supplement the exact oracle; they do not replace it.

Receipt hashes provide content binding, not an external signature proving a run occurred. The reviewer must establish the origin of the CI/local evidence and record its run/job references. Applicable semantic unit, archive/checkpoint and strict audit parity gates from the review plan remain required and must be reviewed separately. This helper verifies the oracle/runtime artifacts and source consistency; it is not an authority issuer or a replacement for those other gates.

## Emit and review

After independently reviewing and hashing the proof set:

```powershell
python verification/native-studio-acceptance/freshness_proposal.py --root . --proof-set 'C:/acceptance/proofs/proof-set.json' --proof-set-sha256 '<independently-reviewed-proof-set-sha256>' --output 'verification/generated/native-studio-acceptance/freshness-proposal-01'
if ($LASTEXITCODE -ne 0) { throw 'Proof or reviewed source scope does not match' }
```

The helper rejects missing, stale or altered evidence, duplicate JSON keys, source changes during verification, unexpected inventory changes and existing output directories. It rechecks sources and artifact bytes before emission. It preserves the actual ledger bytes. Its report records both proposed ledger hashes, proof/source identities, exact reviewed changes and the lock-compatibility result.

Root reviews the patch, checks it still applies with `git apply --check`, applies it explicitly, and runs `npm run standards:check` plus the remaining required checks. Do not create a proof set from the synthetic unit-test fixtures. No real proposal was generated when implementing this helper because the required current proof gates were not yet available.

The adversarial unit suite runs without a semantic runtime:

```powershell
python -m unittest discover -s verification/native-studio-acceptance -p test_freshness_proposal.py -v
```

It tests unequal proof payloads with equal counts, missing query families, stale source/executable/runtime bindings, failed or cached cold construction, rejected validation, missing population, runtime output tampering, artifact substitution, duplicate JSON, absent explicit review, source races, retained output refusal and byte-for-byte preservation of the input ledger. All success-path proof objects in this suite are explicitly synthetic bookkeeping fixtures.

Implementation check: the command above exited 0 with 12 tests passed; complete output is retained in `freshness-proposal-tests.txt`. A separate read-only `capture` plus `propose` check against the actual checkout exited 0 and reported 23 reviewed entries and 154 proposed inputs; it did not run proof verification, emit a proposal directory, or write the ledger. No passing runtime/oracle evidence is claimed by either bookkeeping check.
