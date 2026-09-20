# Observe the running audit without changing its context or memoization boundary.
param(
    [string]$AuditExecutable = 'target\kerml-v7\release\examples\library_quality.exe',
    [string]$OutputName = 'memory-samples.json'
)
$ErrorActionPreference = 'Stop'
$expected = Join-Path (Get-Location).Path $AuditExecutable
$auditProcess = Get-Process library_quality | Where-Object { $_.Path -eq $expected }
if (@($auditProcess).Count -ne 1) { throw 'Expected exactly one final v5 audit process' }
$samples = [System.Collections.Generic.List[object]]::new()
while (-not $auditProcess.HasExited) {
    $auditProcess.Refresh()
    $samples.Add([ordered]@{
        timestamp = [DateTime]::UtcNow.ToString('o')
        process_id = $auditProcess.Id
        working_set_bytes = $auditProcess.WorkingSet64
        peak_working_set_bytes = $auditProcess.PeakWorkingSet64
        cpu_seconds = $auditProcess.TotalProcessorTime.TotalSeconds
    })
    $samples | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $PSScriptRoot $OutputName) -Encoding UTF8
    Start-Sleep -Seconds 30
}
Write-Output "Final audit memory observations: $($samples.Count) samples; OS lifetime peak retained in samples."
