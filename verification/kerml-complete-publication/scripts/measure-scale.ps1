# Capture this short synthetic executable's peak without a polling startup race.
$workspacePath = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../../..')).Path
$binaryPath = Join-Path $workspacePath 'target/release/examples/shared_derivation_scale.exe'
$outputDirectory = Join-Path $workspacePath 'verification/generated/kerml-complete-publication'
$process = Start-Process -FilePath $binaryPath -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $outputDirectory 'synthetic-scale.stdout.log') -RedirectStandardError (Join-Path $outputDirectory 'synthetic-scale.stderr.log')
# Retain the Windows process handle before it exits so ExitCode stays available.
$retainedHandle = $process.Handle
$maximumPrivate = 0L
$maximumWorking = 0L
while (-not $process.HasExited) {
    $process.Refresh()
    $maximumPrivate = [Math]::Max($maximumPrivate, $process.PrivateMemorySize64)
    $maximumWorking = [Math]::Max($maximumWorking, $process.PeakWorkingSet64)
    Start-Sleep -Milliseconds 50
}
$process.WaitForExit()
if ($null -eq $process.ExitCode) { throw 'Missing child exit code' }
Get-Content -LiteralPath (Join-Path $outputDirectory 'synthetic-scale.stdout.log')
Get-Content -LiteralPath (Join-Path $outputDirectory 'synthetic-scale.stderr.log')
[ordered]@{
    exit_code = $process.ExitCode
    maximum_observed_private_bytes = $maximumPrivate
    peak_working_set_bytes = $maximumWorking
    correctness_threshold = $null
} | ConvertTo-Json | Tee-Object -FilePath (Join-Path $outputDirectory 'synthetic-scale-resources.json')
exit $process.ExitCode
