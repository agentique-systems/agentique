$ErrorActionPreference = 'Stop'
$v8Samples = [System.Collections.Generic.List[object]]::new()
$v8Audit = Get-CimInstance Win32_Process -Filter "Name='library_quality.exe'" |
    Where-Object { $_.CommandLine.Contains('--output=verification/kerml-semantic-closure-v8/baseline-quality.json') }
if (@($v8Audit).Count -ne 1) { throw 'Expected exactly one v8 quality audit process' }
$v8ProcessId = $v8Audit.ProcessId
while ($true) {
    $v8Process = Get-Process -Id $v8ProcessId -ErrorAction SilentlyContinue
    if ($null -eq $v8Process) { break }
    $v8Samples.Add([pscustomobject]@{
        sampled_at = [DateTime]::UtcNow.ToString('o')
        process_id = $v8ProcessId
        working_set = $v8Process.WorkingSet64
        peak_working_set_since_process_start = $v8Process.PeakWorkingSet64
        private_bytes = $v8Process.PrivateMemorySize64
    })
    ConvertTo-Json -InputObject @($v8Samples.ToArray()) -Depth 4 |
        Set-Content -LiteralPath "$PSScriptRoot/memory-samples.json" -Encoding UTF8
    Start-Sleep -Seconds 10
}
Write-Output "Recorded $($v8Samples.Count) samples for v8 audit process $v8ProcessId; sampling began after refinement."
