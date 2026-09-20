# Collect final measurements only after the complete captured audit exits.
$ErrorActionPreference = 'Stop'
while (-not (Test-Path -LiteralPath (Join-Path $PSScriptRoot 'v5-quality-final-2/result.json'))) {
    Start-Sleep -Seconds 10
}
function Invoke-Recorded {
    param([string]$Label, [string]$Script, [int]$Expected = 0, [string[]]$Options = @())
    & python -B -X utf8 (Join-Path $PSScriptRoot 'capture.py') $Label python -B -X utf8 (Join-Path $PSScriptRoot $Script) @Options
    if ($LASTEXITCODE -ne $Expected) { throw "$Label exited $LASTEXITCODE; expected $Expected" }
}
Invoke-Recorded -Label 'complete-corpus-audits' -Script 'audits.py'
Invoke-Recorded -Label 'all-constraint-coverage-gate' -Script 'coverage.py' -Expected 1 -Options @('--gate')
Invoke-Recorded -Label 'all-constraint-inventory-check' -Script 'coverage.py' -Options @('--check')
Invoke-Recorded -Label 'authority-frozen-check' -Script 'authority.py' -Options @('--check')
Invoke-Recorded -Label 'profile-frozen-check' -Script 'profile-review.py' -Options @('--check')
Invoke-Recorded -Label 'preservation-closing' -Script 'preservation.py'
Invoke-Recorded -Label 'closing-final' -Script 'closing.py'
Invoke-Recorded -Label 'closing-check' -Script 'closing.py' -Options @('--check')
