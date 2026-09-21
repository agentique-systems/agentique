param(
    [string]$ProcessName = 'canonical_publication',
    [string]$OutputName = 'full-resource-observations.jsonl'
)
# Optional Windows resource observations. These never change semantic acceptance.
$workspacePath = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../../..')).Path
$binaryPath = Join-Path $workspacePath "target/release/examples/$ProcessName.exe"
$outputDirectory = Join-Path $workspacePath 'verification/generated/kerml-complete-publication'
if ([IO.Path]::GetFileName($OutputName) -ne $OutputName) { throw 'OutputName must be a filename' }
$outputPath = Join-Path $outputDirectory $OutputName
New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
$observedProcess = $null
while ($null -eq $observedProcess) {
    $observedProcess = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -eq $binaryPath } | Select-Object -First 1
    if ($null -eq $observedProcess) { Start-Sleep -Seconds 5 }
}
$processId = $observedProcess.Id
while ($null -ne $observedProcess) {
    [ordered]@{
        observed_utc = [DateTime]::UtcNow.ToString('o')
        process_id = $processId
        cpu_seconds = $observedProcess.CPU
        working_set_bytes = $observedProcess.WorkingSet64
        peak_working_set_bytes = $observedProcess.PeakWorkingSet64
        private_bytes = $observedProcess.PrivateMemorySize64
    } | ConvertTo-Json -Compress | Add-Content -LiteralPath $outputPath -Encoding UTF8
    Start-Sleep -Seconds 30
    $observedProcess = Get-Process -Id $processId -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -eq $binaryPath }
}
Write-Output "Resource observations: $outputPath"
