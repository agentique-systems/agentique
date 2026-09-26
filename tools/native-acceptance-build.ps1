[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateSet('native', 'helpers', 'oracle')]
    [string] $Component,
    [Parameter(Mandatory)]
    [string] $ArtifactDirectory,
    [string] $CheckoutDirectory,
    [string] $ExpectedSourceCommit,
    [string] $ExpectedOracleHarnessSha256,
    [ValidateSet('Disabled', 'Enabled')]
    [string] $OracleControl = 'Disabled',
    [ValidateSet('build', 'oracle')]
    [string] $Phase = 'build'
)

# Separate Windows CI capacity for ordinary release builds and the optional,
# explicitly selected semantic oracle. Native interaction remains a separate gate.
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
Set-StrictMode -Version Latest
$driverRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$checkout = if ($CheckoutDirectory) { (Resolve-Path -LiteralPath $CheckoutDirectory).Path } else { $driverRoot }
$artifact = [IO.Path]::GetFullPath($ArtifactDirectory)
$oracleTest = if ($OracleControl -eq 'Enabled') { 'create_part_controlled' } else { 'create_part_performance' }
$oracleEntry = if ($OracleControl -eq 'Enabled') {
    'enabled_control_create_part_matches_full_self_model_reconstruction'
} else {
    'create_part_command_matches_full_self_model_reconstruction'
}
if ($OracleControl -eq 'Enabled' -and ($Component -ne 'oracle' -or $ExpectedOracleHarnessSha256)) {
    throw 'Enabled controls require the current-only oracle; baseline overlays use the portable ordinary API'
}
if ($Phase -eq 'oracle') {
    if ($Component -ne 'oracle') { throw 'Only the oracle component has an execution phase' }
    $build = Get-Content -LiteralPath (Join-Path $artifact 'build.json') -Raw | ConvertFrom-Json
    if ($build.outcome -ne 'passed' -or $build.component -ne 'oracle') { throw 'A successful oracle build receipt is required' }
    if ($build.oracle_control -ne $OracleControl -or $build.oracle_test -ne $oracleTest -or $build.oracle_entry -ne $oracleEntry) {
        throw 'Requested oracle control mode differs from its build receipt'
    }
    $currentCommit = & git -C $checkout rev-parse HEAD
    if ($LASTEXITCODE -ne 0 -or $currentCommit -ne $build.source_commit) { throw 'Oracle source commit differs from its build' }
    foreach ($sourceFile in $build.source_files.PSObject.Properties) {
        $path = Join-Path $checkout $sourceFile.Name
        if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $sourceFile.Value) {
            throw "Oracle build input changed: $($sourceFile.Name)"
        }
    }
    $changedPaths = @(& git -C $checkout diff HEAD --name-only)
    if ($LASTEXITCODE -ne 0 -or ($changedPaths | Where-Object { $_ -ne 'crates/modeling-agent/tests/create_part_performance.rs' })) {
        throw 'Oracle source has unrelated modifications after building'
    }
    $binary = Join-Path $artifact "bin/$oracleTest.exe"
    $digest = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($build.binaries.Count -ne 1 -or $build.binaries[0].sha256 -ne $digest) { throw 'Oracle executable changed after its build' }
    $evidence = Join-Path $artifact 'oracle'
    if (Test-Path -LiteralPath $evidence) { throw 'Refusing to overwrite prior oracle evidence' }
    [IO.Directory]::CreateDirectory($evidence) | Out-Null
    $observer = Join-Path $evidence 'run_record.py'
    Copy-Item -LiteralPath (Join-Path $driverRoot 'verification/native-studio-acceptance/semantics/run_record.py') -Destination $observer
    $env:AGENTIQUE_SOURCE_ROOT = $checkout
    $env:AGENTIQUE_CREATE_PART_ORACLE_OUTPUT = Join-Path $evidence 'observations'
    $arguments = @($observer, '--cwd', $checkout, '--name', 'windows-create-part-oracle', '--', $binary,
        $oracleEntry, '--exact', '--ignored', '--nocapture', '--test-threads=1')
    $invocation = [ordered]@{
        command = @('python') + $arguments
        source_commit = $build.source_commit
        executable_sha256 = $digest
        oracle_control = $OracleControl
        oracle_test = $oracleTest
        oracle_entry = $oracleEntry
        observer_sha256 = (Get-FileHash -LiteralPath $observer -Algorithm SHA256).Hash.ToLowerInvariant()
        runtime_sha256 = $env:RUNTIME_SHA256
        source_root = $checkout
        oracle_harness_sha256 = $build.source_files.'crates/modeling-agent/tests/create_part_performance.rs'
        scope = 'Repository self-model only; no operator database uploaded'
        outcome = 'pending'
    }
    $invocationPath = Join-Path $evidence 'invocation.json'
    $invocation | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $invocationPath -Encoding utf8
    try {
        python @arguments 2>&1 | Tee-Object -FilePath (Join-Path $artifact 'logs/oracle-observer.log')
        $invocation.exit_code = $LASTEXITCODE
        if ($LASTEXITCODE -ne 0) { throw "Semantic oracle failed with exit $LASTEXITCODE" }
        $metrics = Get-Content -LiteralPath (Join-Path $env:AGENTIQUE_CREATE_PART_ORACLE_OUTPUT 'command-metrics.json') -Raw | ConvertFrom-Json
        $expectedMode = if ($OracleControl -eq 'Enabled') { 'enabled-not-requested' } else { 'default-disabled' }
        if ($metrics.command_control_mode -ne $expectedMode) { throw 'Oracle executed a different compilation control mode' }
        $invocation.outcome = 'passed'
    }
    catch {
        $invocation.outcome = 'failed'
        $invocation.error = $_.Exception.Message
        throw
    }
    finally {
        $invocation | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $invocationPath -Encoding utf8
    }
    return
}
if (Test-Path -LiteralPath $artifact) {
    throw "Refusing to overwrite prior build evidence: $artifact"
}
[IO.Directory]::CreateDirectory($artifact) | Out-Null
foreach ($directory in @('logs', 'source', 'bin')) {
    [IO.Directory]::CreateDirectory((Join-Path $artifact $directory)) | Out-Null
}

$toolchain = '1.92.0'
$targetDirectory = Join-Path $checkout "target/native-acceptance-$Component"
$watch = [Diagnostics.Stopwatch]::StartNew()
$script:receipt = [ordered]@{
    format = 'agentique-native-acceptance-build/1'
    component = $Component
    oracle_control = $OracleControl
    oracle_test = $oracleTest
    oracle_entry = $oracleEntry
    outcome = 'incomplete'
    started_utc = [DateTime]::UtcNow.ToString('o')
    source_commit = $null
    source_checkout = $checkout
    build_driver_sha256 = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()
    toolchain = $toolchain
    target = 'x86_64-pc-windows-msvc'
    release_profile = 'checked-in default release profile; thin LTO; no release overrides'
    environment = [ordered]@{}
    source_files = [ordered]@{}
    commands = [Collections.Generic.List[object]]::new()
    binaries = [Collections.Generic.List[object]]::new()
    ci = [ordered]@{
        repository = $env:GITHUB_REPOSITORY
        workflow_commit = $env:GITHUB_SHA
        run_id = $env:GITHUB_RUN_ID
        run_attempt = $env:GITHUB_RUN_ATTEMPT
        runner_os = $env:RUNNER_OS
        image_os = $env:ImageOS
        image_version = $env:ImageVersion
    }
    contract = 'Build artifacts only. Run against the exact source commit and separately authenticated runtime; no semantic or interactive qualification is implied.'
}

function Write-Receipt {
    $script:receipt.elapsed_seconds = $watch.Elapsed.TotalSeconds
    $script:receipt | ConvertTo-Json -Depth 12 |
        Set-Content -LiteralPath (Join-Path $artifact 'build.json') -Encoding utf8
}

function Invoke-RecordedCommand {
    param([string] $Executable, [string[]] $Arguments, [string] $Label)
    $logName = '{0:D2}-{1}.log' -f ($script:receipt.commands.Count + 1), $Label
    $log = Join-Path $artifact "logs/$logName"
    $started = [DateTime]::UtcNow
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $exitCode = 1
    try {
        & $Executable @Arguments 2>&1 | ForEach-Object {
            $line = $_.ToString()
            Write-Host $line
            $line
        } | Out-File -LiteralPath $log -Encoding utf8
        $exitCode = $LASTEXITCODE
    }
    finally {
        $entry = [ordered]@{
            command = @($Executable) + $Arguments
            cwd = $checkout
            started_utc = $started.ToString('o')
            elapsed_seconds = $timer.Elapsed.TotalSeconds
            exit_code = $exitCode
            output = "logs/$logName"
            output_sha256 = if (Test-Path -LiteralPath $log) {
                (Get-FileHash -LiteralPath $log -Algorithm SHA256).Hash.ToLowerInvariant()
            } else { $null }
        }
        $script:receipt.commands.Add($entry)
        Write-Receipt
    }
    if ($exitCode -ne 0) {
        throw "Command '$Label' failed with exit $exitCode; see $log"
    }
    return $log
}

function Save-Binary {
    param([string] $Source, [string] $Name)
    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) {
        throw "Expected build output is absent: $Source"
    }
    $destination = Join-Path $artifact "bin/$Name"
    Copy-Item -LiteralPath $Source -Destination $destination
    $script:receipt.binaries.Add([ordered]@{
        path = "bin/$Name"
        build_output = $Source
        bytes = (Get-Item -LiteralPath $destination).Length
        sha256 = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant()
    })
    Write-Receipt
}

Push-Location -LiteralPath $checkout
try {
    if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
        throw 'This build qualification targets Windows x86_64 MSVC only'
    }
    # Refuse hidden release overrides: the downloaded binaries must use the same
    # checked-in release settings as the local acceptance measurements.
    $overrides = Get-ChildItem Env: | Where-Object {
        $_.Name -like 'CARGO_PROFILE_RELEASE_*' -or
        $_.Name -like 'CARGO_TARGET_*_RUSTFLAGS' -or $_.Name -in @(
            'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'CARGO_BUILD_RUSTFLAGS',
            'CARGO_BUILD_TARGET', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER'
        )
    }
    if ($overrides) {
        throw ('Unexpected compiler override: ' + (($overrides | ForEach-Object Name) -join ', '))
    }
    $env:CARGO_BUILD_JOBS = '2'
    $env:CARGO_INCREMENTAL = '0'
    $env:CARGO_PROFILE_DEV_DEBUG = '0'
    $env:CARGO_PROFILE_TEST_DEBUG = '0'
    foreach ($name in @('CARGO_BUILD_JOBS', 'CARGO_INCREMENTAL', 'CARGO_PROFILE_DEV_DEBUG', 'CARGO_PROFILE_TEST_DEBUG')) {
        $script:receipt.environment[$name] = [Environment]::GetEnvironmentVariable($name)
    }
    $commitLog = Invoke-RecordedCommand 'git' @('rev-parse', 'HEAD') 'source-commit'
    $script:receipt.source_commit = (Get-Content -LiteralPath $commitLog -Raw).Trim()
    $expectedCommit = if ($ExpectedSourceCommit) { $ExpectedSourceCommit } else { $env:GITHUB_SHA }
    if ($expectedCommit -and $expectedCommit -ne $script:receipt.source_commit) {
        throw 'Checked-out commit differs from the dispatched workflow commit'
    }
    $statusLog = Invoke-RecordedCommand 'git' @('status', '--porcelain', '--untracked-files=no') 'tracked-status'
    $initialStatus = [string](Get-Content -LiteralPath $statusLog -Raw)
    $harness = 'crates/modeling-agent/tests/create_part_performance.rs'
    if ($ExpectedOracleHarnessSha256) {
        if ($Component -ne 'oracle' -or $ExpectedOracleHarnessSha256 -notmatch '^[0-9a-f]{64}$' -or
            (Get-FileHash -LiteralPath $harness -Algorithm SHA256).Hash.ToLowerInvariant() -ne $ExpectedOracleHarnessSha256) {
            throw 'Baseline overlay requires the exact independently pinned portable oracle test'
        }
        $changes = @(Get-Content -LiteralPath $statusLog | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
        if ($changes.Count -gt 1 -or ($changes.Count -eq 1 -and $changes[0] -notin @(" M $harness", "M  $harness", "MM $harness"))) {
            throw 'Baseline may overlay only the pinned oracle test, never application source'
        }
        $null = Invoke-RecordedCommand 'git' @('diff', 'HEAD', '--', $harness) 'portable-oracle-test-overlay'
        $script:receipt.oracle_test_overlay_sha256 = $ExpectedOracleHarnessSha256
    } elseif (-not [string]::IsNullOrWhiteSpace($initialStatus)) {
        throw 'Release build requires a clean tracked checkout'
    }
    $toolchainText = Get-Content -LiteralPath 'rust-toolchain.toml' -Raw
    if ($toolchainText -notmatch 'channel\s*=\s*"1\.92\.0"') {
        throw 'Workflow toolchain must be reviewed with the changed repository pin'
    }
    $files = [ordered]@{
        'Cargo.lock' = 'Cargo.lock'
        'Cargo.toml' = 'Cargo.toml'
        'crates/studio-native/Cargo.lock' = 'native-Cargo.lock'
        'crates/studio-native/Cargo.toml' = 'native-Cargo.toml'
        'rust-toolchain.toml' = 'rust-toolchain.toml'
    }
    if ($Component -eq 'oracle') { $files[$harness] = 'oracle-test.rs' }
    if ($Component -eq 'oracle' -and $OracleControl -eq 'Enabled') {
        $files['crates/modeling-agent/tests/create_part_controlled.rs'] = 'controlled-oracle-test.rs'
        $files['crates/modeling-agent/Cargo.toml'] = 'agent-Cargo.toml'
    }
    foreach ($path in $files.Keys) {
        $script:receipt.source_files[$path] = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
        Copy-Item -LiteralPath $path -Destination (Join-Path $artifact ('source/' + $files[$path]))
    }
    Write-Receipt
    $null = Invoke-RecordedCommand 'rustup' @('toolchain', 'install', $toolchain, '--profile', 'minimal') 'install-pinned-toolchain'
    $rustcLog = Invoke-RecordedCommand 'rustc' @("+$toolchain", '-Vv') 'rustc-version'
    if ((Get-Content -LiteralPath $rustcLog -Raw) -notmatch 'host: x86_64-pc-windows-msvc') {
        throw 'Expected x86_64 MSVC host compiler'
    }
    $null = Invoke-RecordedCommand 'cargo' @("+$toolchain", '-Vv') 'cargo-version'
    $manifest = if ($Component -eq 'native') { 'crates/studio-native/Cargo.toml' } else { 'Cargo.toml' }
    $null = Invoke-RecordedCommand 'cargo' @("+$toolchain", 'fetch', '--locked', '--manifest-path', $manifest) 'fetch-locked'
    $arguments = @("+$toolchain", 'build', '--release', '--locked', '--offline', '--manifest-path', $manifest,
        '--target-dir', $targetDirectory, '--message-format=json-render-diagnostics')
    switch ($Component) {
        'native' { $arguments += @('--bin', 'agq-studio-native') }
        'helpers' { $arguments += @('-p', 'agq-studio-platform', '--example', 'profile_open', '--example', 'source_recovery', '--test', 'project_creation') }
        'oracle' { $arguments += @('-p', 'agq-modeling-agent', '--features', 'agq-modeling-agent/verification', '--test', $oracleTest) }
    }
    $buildLog = Invoke-RecordedCommand 'cargo' $arguments 'release-build'
    # Use Cargo's actual executable paths, including the hashed test harness;
    # stale files or a guessed filename cannot enter a successful artifact.
    $expected = switch ($Component) {
        'native' { @('agq-studio-native') }
        'helpers' { @('profile_open', 'source_recovery', 'project_creation') }
        'oracle' { @($oracleTest) }
    }
    $executables = @{}
    foreach ($line in Get-Content -LiteralPath $buildLog) {
        if (-not $line.StartsWith('{')) { continue }
        $message = $line | ConvertFrom-Json
        if ($message.reason -eq 'compiler-artifact' -and $message.executable -and $message.target.name -in $expected) {
            if ($executables.ContainsKey($message.target.name)) {
                throw "Multiple executable artifacts for $($message.target.name)"
            }
            $executables[$message.target.name] = $message.executable
        }
    }
    foreach ($name in $expected) {
        if (-not $executables.ContainsKey($name)) { throw "Cargo did not report executable $name" }
        Save-Binary $executables[$name] "$name.exe"
    }
    foreach ($path in $files.Keys) {
        if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $script:receipt.source_files[$path]) {
            throw "Build changed its pinned input: $path"
        }
    }
    $finalStatus = Invoke-RecordedCommand 'git' @('status', '--porcelain', '--untracked-files=no') 'final-tracked-status'
    if ([string](Get-Content -LiteralPath $finalStatus -Raw) -ne $initialStatus) {
        throw 'Build modified tracked source or the pinned test overlay'
    }
    $script:receipt.binaries | ForEach-Object { "$($_.sha256)  $($_.path)" } |
        Set-Content -LiteralPath (Join-Path $artifact 'SHA256SUMS') -Encoding utf8
    $script:receipt.outcome = 'passed'
}
catch {
    $script:receipt.outcome = 'failed'
    $script:receipt.error = $_.Exception.Message
    throw
}
finally {
    Write-Receipt
    Pop-Location
}
