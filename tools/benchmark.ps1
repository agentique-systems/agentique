param([string]$Output = 'verification/benchmark.json')
$ErrorActionPreference = 'Stop'
$rootPath = Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $rootPath
$runName = 'benchmark-' + [guid]::NewGuid().ToString()
$database = Join-Path $rootPath ('.workspaces/' + $runName + '.db')
$stdoutPath = Join-Path $rootPath ('.cache/' + $runName + '.json')
$stderrPath = Join-Path $rootPath ('.cache/' + $runName + '.log')
New-Item -ItemType Directory -Force -Path '.cache','.workspaces' | Out-Null
$machine = Get-CimInstance Win32_ComputerSystem
$installedMemory = (Get-CimInstance Win32_PhysicalMemory | Measure-Object -Property Capacity -Sum).Sum
$os = Get-CimInstance Win32_OperatingSystem
$process = Start-Process -FilePath (Join-Path $rootPath 'target/release/examples/benchmark.exe') -ArgumentList @($database) -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
# Retain the native handle: Windows PowerShell otherwise can lose ExitCode after polling HasExited.
$processHandle = $process.Handle
$peakBytes = 0L
$samples = 0
while (!$process.HasExited) {
    $process.Refresh()
    $peakBytes = [Math]::Max($peakBytes, $process.PeakWorkingSet64)
    $samples += 1
    Start-Sleep -Milliseconds 100
}
$process.WaitForExit()
if ($process.ExitCode -ne 0) { Get-Content -LiteralPath $stderrPath; throw 'Benchmark failed' }
$measurement = Get-Content -LiteralPath $stdoutPath -Raw -Encoding UTF8 | ConvertFrom-Json
$report = [ordered]@{
    measured_at = [DateTime]::UtcNow.ToString('o')
    host = @{os=$os.Caption; os_version=$os.Version; architecture=$env:PROCESSOR_ARCHITECTURE; logical_processors=$machine.NumberOfLogicalProcessors; physical_memory_bytes=$machine.TotalPhysicalMemory; installed_dimm_bytes=$installedMemory; memory_basis='Win32_PhysicalMemory.Capacity sum for installed RAM; Win32_ComputerSystem.TotalPhysicalMemory for OS-usable RAM'; rustc=(& rustc --version); node=(& node --version)}
    process = @{exit_code=$process.ExitCode; peak_working_set_bytes=$peakBytes; samples=$samples; sampling_period_ms=100; method='Windows Process.PeakWorkingSet64 (actual operating-system high-water mark)'}
    measurement=$measurement
    acceptance=@{host_sufficient=($machine.NumberOfLogicalProcessors -ge 4 -and $installedMemory -ge 16GB); input_size_matches=($measurement.user_elements -eq 1000); pause_under_one_second=($measurement.contended_pause_ack_ms -lt 1000); stop_under_one_second=($measurement.contended_stop_ack_ms -lt 1000)}
}
$report | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $Output -Encoding UTF8
Get-Content -LiteralPath $Output -Encoding UTF8
