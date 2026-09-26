# Held reconstruction qualification sequence

Prepared from `089fde4`, `edb8775` and `0cbc54a`; **commands below have not
been run**. No build, runtime consumer or pin edit was performed while preparing
this sequence. Begin after the real native journey has exited and the lead has
allocated the sole standards-consumer slot. Keep the six-document self-model
unchanged through both measurements; held self-model `95924654` stays separate.

Use the integration worktree and its existing `target/release`. Preserve the
before-sharing executable and evidence already recorded in
`performance/create-part-before-sharing.*`; its retained executable SHA-256 is
`ea007d101d7ed846038415c6ff8f31e41b1c30e8fb20eb2e6eac70dd5e49678d`.

## 1. Fixed inputs and build/run helpers

These exact installed filenames were checked without opening a runtime facade.
The tests themselves authenticate their contents through ordinary facades.

```powershell
Set-Location 'C:\Users\phili\github\agentique-systems\agentique'
$env:CARGO_TARGET_DIR = Join-Path (Get-Location) 'target'
$env:CARGO_BUILD_JOBS = '1'
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
$env:AGENTIQUE_RUNTIME_DIR = 'C:\Users\phili\.agentique'
$qualificationRuntime = Join-Path $env:AGENTIQUE_RUNTIME_DIR 'publications\633ea89eb39f8a9e301f2bd5199994455cdf28c8d373fdbd69be30a1402ebcf4'
$env:AGENTIQUE_KERML_CACHE = Join-Path $qualificationRuntime 'kerml.cache'
$env:AGENTIQUE_SYSTEMS_CACHE = Join-Path $qualificationRuntime 'systems.cache'
Remove-Item Env:AGENTIQUE_STUDIO_BUNDLE -ErrorAction SilentlyContinue
$qualificationEvidence = 'verification/native-studio-alpha/performance'

function Build-QualificationTest {
    param([string]$Stem, [string]$TargetName, [string[]]$TestArguments)
    $buildLines = & cargo test --release --config profile.release.lto=false --locked --offline -j 1 --no-run --message-format=json @TestArguments 2> "$Stem.build.stderr.log"
    $buildExit = $LASTEXITCODE
    $buildLines | Set-Content -Encoding utf8 "$Stem.build.jsonl"
    if ($buildExit -ne 0) { throw "Build failed ($buildExit): $Stem" }
    $artifacts = @($buildLines | ForEach-Object { ConvertFrom-Json $_ } | Where-Object {
        $_.reason -eq 'compiler-artifact' -and $_.target.name -eq $TargetName -and $_.profile.test -and $_.executable
    })
    if ($artifacts.Count -ne 1) { throw "Expected one test executable: $Stem" }
    return $artifacts[0].executable
}

function Run-QualificationTest {
    param([string]$Stem, [string]$Executable, [string]$TestName)
    if ((Test-Path "$Stem.json") -or (Test-Path "$Stem.log")) { throw "Evidence already exists: $Stem" }
    & python verification/scripts/measure-phase2-frontend.py "$Stem.json" $Executable $TestName --exact --ignored --nocapture --test-threads=1
    if ($LASTEXITCODE -ne 0) { throw "Runtime gate failed: $Stem" }
    if (-not (Select-String -LiteralPath "$Stem.log" -SimpleMatch -Quiet -Pattern 'test result: ok. 1 passed; 0 failed; 0 ignored;')) {
        throw "Expected exactly one executed passing test: $Stem"
    }
}
```

The measurement wrapper retains exact executable hash, source commit, command,
exit, complete output, wall time and sampled whole-process memory. Invoking the
exact executable selected from Cargo's output excludes compilation from runtime
measurements and avoids accidentally selecting the preserved old executable.
Run each command serially; stop this qualification stream on any failed gate.

## 2. Mount sharing alone

Integrate the two reviewed mount commits, without updating freshness inputs:

```powershell
git cherry-pick 089fde4 edb8775
if ($LASTEXITCODE -ne 0) { throw 'Resolve mount integration before proceeding' }
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { throw 'Formatting failed' }
$mountStem = "$qualificationEvidence/create-part-shared-mount"
$mountTest = Build-QualificationTest "$mountStem" 'create_part_performance' @('-p', 'agq-modeling-agent', '--features', 'verification', '--test', 'create_part_performance')
Run-QualificationTest "$mountStem" $mountTest 'create_part_command_matches_full_self_model_reconstruction'
```

Require the emitted `agentique-create-part-performance/3` report: exact
equivalence, both validations, observed predecessor mount sharing, independently
different cold mount, and all 16 malformed-request refusals. The unchanged cold
checkpoint restore must reparse every document without a semantic cache. Review
the exact checkpoint, canonical identity/provenance/order, closure, references
and full query-context/evidence assertions; passing work counters alone is not
equivalence. Only fresh nested KerML revision labels are normalized.

## 3. Duplicate-answer removal, then combined oracle

```powershell
git cherry-pick 0cbc54a
if ($LASTEXITCODE -ne 0) { throw 'Resolve audit integration before proceeding' }
$parityStem = "$qualificationEvidence/effective-audit-answer-parity"
$parityTest = Build-QualificationTest "$parityStem" 'agq_kerml_text' @('-p', 'agq-kerml-text', '--lib')
Run-QualificationTest "$parityStem" $parityTest 'sysml::publication::authored_audit_parity_tests::authored_audit_observer_matches_two_pass_queries_and_diagnostics'
$combinedStem = "$qualificationEvidence/create-part-shared-mount-and-audit"
$combinedTest = Build-QualificationTest "$combinedStem" 'create_part_performance' @('-p', 'agq-modeling-agent', '--features', 'verification', '--test', 'create_part_performance')
Run-QualificationTest "$combinedStem" $combinedTest 'create_part_command_matches_full_self_model_reconstruction'
```

Parity uses the real accepted pair and ordinary/variation source fixtures. It
compares exact answers with no normalization, subject order, all audit-family
counts/findings, and stored capability-before-audit diagnostic order. Complete
and Incomplete cases exercise the population flow; actual wrong-kind Invalid
and missing-ID Incomplete answers additionally exercise the private delivery
boundary, without broadening the production guard.

Compare mount-only versus combined command/compile timings to assess the audit
change. Retain hot-command wall time, independent cold-checkpoint wall time,
both complete compile profiles and validation times. Whole-process memory covers
restoration, seeding, both reconstructions and validation, not one edit. Mixed
noncompile residuals are not mount timings. Certificate time is already included
in final closure; cumulative counters are not additional wall time. No local
semantic incrementality or lower peak memory follows from removing one query.

## 4. Durable source-only restoration and normal checks

The existing real platform gate exercises committed Create Part and bounded
Rename, cache-backed restart, source-only restart and subsequent editing. Its
database is temporary; do not substitute the active native journey database.

```powershell
$restartStem = "$qualificationEvidence/shared-reconstruction-durable-restart"
$restartTest = Build-QualificationTest "$restartStem" 'real_model' @('-p', 'agq-studio-platform', '--test', 'real_model')
Run-QualificationTest "$restartStem" $restartTest 'native_in_process_self_model_candidate_commit_and_restore'
cargo clippy --locked --offline -p agq-kerml-text -p agq-modeling-service -p agq-modeling-workspace -p agq-modeling-agent --features agq-modeling-agent/verification --all-targets -j 1 -- -D warnings
cargo doc --locked --offline --no-deps -p agq-kerml-text -p agq-modeling-workspace -j 1
```

Record each check's actual command/output/exit using the integration harness.
Complete the repository's remaining normal workspace gates before acceptance.

## 5. Separate source-freshness review after success

Only these three entries in `standards/sysml-publication-inputs.json` are in
scope. The hashes normalize CRLF to LF, exactly as
`tools/sysml-publication-stale.mjs` does. Recompute and independently review the
integrated sources; a conflict resolution can change the proposed hash.

| Source | Existing normalized SHA-256 | Reviewed proposed normalized SHA-256 |
| --- | --- | --- |
| `crates/kerml-text/src/source_checkpoint.rs` | `eb86d74ead07e68664081b131c3dbd152877fdb12c1374d3ba2175764a57aa0e` | `8465c80bf7707c540399265d0447f5b0d44dba7730ff4ec3818a6f5e62010368` |
| `crates/kerml-text/src/source_inputs.rs` | `b5b9aebdfbb0f5ecafb730fbd68beee8b7200a0eb86d0eebc601b6c3259c1e87` | `f15171c15dd27c2e61c05c3dfffc426b1714b14cc09e7c8716bbb046d1f30128` |
| `crates/kerml-text/src/sysml/publication.rs` | `c3c8c13d7a7a8c0ecbee5042e13b919ab15035b8e32340c0e536d096b481d8ac` | `682f079a4e2c6c412b897d159f93f3b495f7fd8d69a675aec11f8ae6df9fbef7` |

`publication_audit_tests.rs` is excluded by the existing test-file inventory;
workspace/service/agent source files are outside that interpretation inventory.
The freshness gate is expected to fail while these held source edits are
unreviewed. Do not use blanket `--capture`, amend accepted receipts, regenerate
profiles/rules, or update transport/binding/normative input pins. A later,
separately reviewed three-entry replacement must prove every other inventory
entry and all publication identity/receipt/binding fields unchanged. Then run
`npm run standards:check` and retain its actual result.
