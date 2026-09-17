# Run from the repository root. Results are observations, not semantic conformance.
$ErrorActionPreference = 'Continue'
$verificationDir = Join-Path (Get-Location) 'verification/kerml-descriptors-v1'
$checks = @(
    @{ id = 'rust-format'; executable = 'cargo'; arguments = @('fmt', '--all', '--', '--check') },
    @{ id = 'rust-clippy'; executable = 'cargo'; arguments = @('clippy', '--workspace', '--all-targets', '--', '-D', 'warnings') },
    @{ id = 'rust-tests'; executable = 'cargo'; arguments = @('test', '--workspace') },
    @{ id = 'frontend-types'; executable = 'npm.cmd'; arguments = @('run', 'check') },
    @{ id = 'frontend-build'; executable = 'npm.cmd'; arguments = @('run', 'build') },
    @{ id = 'engineering-tests'; executable = 'npm.cmd'; arguments = @('test') },
    @{ id = 'browser-tests'; executable = 'npm.cmd'; arguments = @('run', 'test:e2e') },
    @{ id = 'generated-current'; executable = 'cargo'; arguments = @('run', '--locked', '--offline', '-p', 'agq-metamodel-gen', '--', '--check') },
    @{ id = 'standards-integrity'; executable = 'npm.cmd'; arguments = @('run', 'standards:check') },
    @{ id = 'runtime-dependencies'; executable = 'cargo'; arguments = @('tree', '--locked', '--offline', '-p', 'agq-kerml', '--edges', 'normal') }
)
$results = @()
foreach ($check in $checks) {
    $started = [DateTime]::UtcNow
    $arguments = $check.arguments
    $executable = $check.executable
    $log = Join-Path $verificationDir ($check.id + '.txt')
    Write-Output ('Running ' + $check.id)
    $captured = @(& $executable @arguments 2>&1 | ForEach-Object { $_.ToString() })
    $code = $LASTEXITCODE
    [IO.File]::WriteAllText($log, ($captured -join "`n") + "`n", [Text.UTF8Encoding]::new($false))
    $results += [ordered]@{
        id = $check.id
        command = ($executable + ' ' + ($arguments -join ' '))
        started_at = $started.ToString('o')
        duration_ms = [int]([DateTime]::UtcNow - $started).TotalMilliseconds
        exit_code = $code
        result = $(if ($code -eq 0) { 'pass' } else { 'fail' })
        log = ('verification/kerml-descriptors-v1/' + $check.id + '.txt')
    }
    [ordered]@{
        format = 'agentique-verification/1'
        baseline_commit = (git rev-parse HEAD)
        branch = (git branch --show-current)
        records = $results
    } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $verificationDir 'results.json') -Encoding utf8
    Write-Output ($check.id + ': exit ' + $code)
}
if (Test-Path -LiteralPath 'verification/browser-results.json') {
    Copy-Item -LiteralPath 'verification/browser-results.json' -Destination (Join-Path $verificationDir 'browser-results.json')
}
if ($results | Where-Object { $_.exit_code -ne 0 }) { exit 1 }
