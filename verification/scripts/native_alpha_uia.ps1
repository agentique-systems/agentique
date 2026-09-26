param([Parameter(Mandatory=$true)][string]$Report, [switch]$WithNarrator)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$auditRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$auditBinary = Join-Path $auditRoot 'target/native-alpha/release/agq-studio-native.exe'
$auditReport = [IO.Path]::GetFullPath((Join-Path $auditRoot $Report))
if (Test-Path -LiteralPath $auditReport) { throw 'Refusing to overwrite prior evidence' }
[IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($auditReport)) | Out-Null
$auditResult = [ordered]@{
  format = 'agentique-native-uia/1'; started_utc = [DateTime]::UtcNow.ToString('o')
  binary = $auditBinary; binary_sha256 = (Get-FileHash -LiteralPath $auditBinary -Algorithm SHA256).Hash.ToLower()
  scope = 'Actual Windows UI Automation on an explicit visual fixture. No screen-reader speech, IME or mixed-DPI claim.'
  passed = $false; assertions = @(); error = $null
}
function Nodes { $script:auditWindow.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition) }
function Button([string]$name) {
  @(Nodes) | Where-Object { $_.Current.Name -eq $name -and $_.Current.ControlType -eq [System.Windows.Automation.ControlType]::Button } | Select-Object -First 1
}
function Invoke-Button($node) {
  if ($null -eq $node) { throw 'Expected accessible button is absent' }
  ([System.Windows.Automation.InvokePattern]$node.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)).Invoke()
  Start-Sleep -Milliseconds 350
}
function Assert-Audit([bool]$pass, [string]$name, $observed) {
  $script:auditResult.assertions += [ordered]@{ name=$name; passed=$pass; observed=$observed }
  if (-not $pass) { throw "Assertion failed: $name" }
}
$auditProcess = $null
$auditNarrator = $null
try {
  if ($WithNarrator) {
    if (Get-Process -Name Narrator -ErrorAction SilentlyContinue) { throw 'An existing Narrator session is in use; leaving it untouched' }
    $auditNarrator = Start-Process -FilePath (Join-Path $env:WINDIR 'System32/Narrator.exe') -WindowStyle Hidden -PassThru
    Start-Sleep -Milliseconds 1500
    $auditNarrator.Refresh()
    if ($auditNarrator.HasExited) { throw 'The owned Narrator process exited before the smoke check' }
    $auditResult.narrator = [ordered]@{ pid=$auditNarrator.Id; spoken_output_qualified=$false; scope='Process coexistence and native accessibility actions while Narrator is running; speech has not been listened to or assessed.' }
  }
  $auditProcess = Start-Process -FilePath $auditBinary -ArgumentList @('--fixture','architecture','--no-restore','--frames','18000') -WorkingDirectory $auditRoot -WindowStyle Hidden -PassThru -RedirectStandardError ($auditReport + '.stderr.txt') -RedirectStandardOutput ($auditReport + '.stdout.txt')
  $auditResult.pid = $auditProcess.Id
  for ($attempt=0; $attempt -lt 30; $attempt++) {
    Start-Sleep -Milliseconds 200
    $auditProcess.Refresh()
    if ($auditProcess.MainWindowHandle -ne 0) { break }
  }
  $script:auditWindow = [System.Windows.Automation.AutomationElement]::FromHandle($auditProcess.MainWindowHandle)
  for ($attempt=0; $attempt -lt 30; $attempt++) {
    $processCondition = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ProcessIdProperty, $auditProcess.Id)
    $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll([System.Windows.Automation.TreeScope]::Children, $processCondition)
    $nativeWindow = @($windows) | Where-Object { $_.Current.Name -like '*Native Studio*' } | Select-Object -First 1
    if ($null -ne $nativeWindow) { $script:auditWindow = $nativeWindow }
    if ($null -ne (Button 'Commands  Ctrl K')) { break }
    Start-Sleep -Milliseconds 200
  }
  $auditResult.initial_nodes = @(Nodes | ForEach-Object { [ordered]@{ name=$_.Current.Name; type=$_.Current.ControlType.ProgrammaticName } })
  $auditResult.window = [ordered]@{ name=$auditWindow.Current.Name; handle=$auditWindow.Current.NativeWindowHandle; process_id=$auditWindow.Current.ProcessId }
  $search = @(Nodes) | Where-Object { $_.Current.ControlType -eq [System.Windows.Automation.ControlType]::Edit } | Select-Object -First 1
  Assert-Audit ($null -ne $search -and $search.Current.Name -ne '') 'Explorer search has an accessible name' $search.Current.Name
  Invoke-Button (Button 'ModelingPlatform')
  Invoke-Button (Button 'Focus selection')
  Start-Sleep -Milliseconds 500
  $ports = @(Nodes) | Where-Object { $_.Current.Name -like '*, Port in *, direction *' }
  Assert-Audit ($ports.Count -ge 6) 'Focused subsystem exposes its actual ports' @($ports | ForEach-Object { $_.Current.Name })
  $port = $ports | Select-Object -First 1
  $patterns = @($port.GetSupportedPatterns() | ForEach-Object { $_.ProgrammaticName })
  Assert-Audit ($patterns -contains 'InvokePatternIdentifiers.Pattern') 'Port supports accessible invocation' $patterns
  Invoke-Button $port
  $portName = $port.Current.Name.Split(',')[0]
  $heading = @(Nodes) | Where-Object { $_.Current.ControlType -eq [System.Windows.Automation.ControlType]::Text -and $_.Current.Name -eq $portName }
  Assert-Audit ($heading.Count -gt 0) 'Port invocation selects its Inspector' $portName
  $port.SetFocus()
  Start-Sleep -Milliseconds 200
  $focused = [System.Windows.Automation.AutomationElement]::FocusedElement.Current.Name
  Assert-Audit ($focused -like '*, Port in *, direction *') 'Port accepts keyboard focus' $focused
  $windowPattern = [System.Windows.Automation.WindowPattern]$auditWindow.GetCurrentPattern([System.Windows.Automation.WindowPattern]::Pattern)
  $windowPattern.SetWindowVisualState([System.Windows.Automation.WindowVisualState]::Minimized)
  Start-Sleep -Milliseconds 350
  $windowPattern.SetWindowVisualState([System.Windows.Automation.WindowVisualState]::Normal)
  Start-Sleep -Milliseconds 500
  $auditProcess.Refresh()
  Assert-Audit (-not $auditProcess.HasExited -and $null -ne (Button 'Commands  Ctrl K')) 'Minimize/restore retains an interactive native window' $windowPattern.Current.WindowVisualState.ToString()
  $transform = $null
  if ($auditWindow.TryGetCurrentPattern([System.Windows.Automation.TransformPattern]::Pattern, [ref]$transform) -and $transform.Current.CanResize) {
    foreach ($size in @(@(1280.0,800.0), @(1600.0,1000.0), @(1100.0,760.0), @(1600.0,1000.0))) {
      $transform.Resize($size[0],$size[1])
      Start-Sleep -Milliseconds 200
    }
    Invoke-Button (Button 'Commands  Ctrl K')
    $focused = [System.Windows.Automation.AutomationElement]::FocusedElement
    Assert-Audit ($focused.Current.ControlType -eq [System.Windows.Automation.ControlType]::Edit -and $focused.Current.Name -ne '') 'Resize sequence retains named command-palette input' $focused.Current.Name
  } else {
    $auditResult.resize = 'Unavailable through this UIA window provider; not qualified'
  }
  if ($WithNarrator) {
    $auditNarrator.Refresh()
    Assert-Audit (-not $auditNarrator.HasExited) 'Native accessibility action sequence completes while Narrator remains running' $auditNarrator.Id
  }
  $auditResult.passed = $true
} catch {
  $auditResult.passed = $false
  $auditResult.error = $_.ToString()
} finally {
  if ($null -ne $auditProcess) {
    $auditProcess.Refresh()
    if (-not $auditProcess.HasExited) {
      $auditProcess.CloseMainWindow() | Out-Null
      if (-not $auditProcess.WaitForExit(10000)) {
        Stop-Process -Id $auditProcess.Id
        $auditResult.forced_test_process_close = $true
      }
      $auditProcess.Refresh()
    }
    $auditResult.process_exit_code = $auditProcess.ExitCode
  }
  if ($null -ne $auditNarrator) {
    $auditNarrator.Refresh()
    if (-not $auditNarrator.HasExited) {
      $auditNarrator.CloseMainWindow() | Out-Null
      if (-not $auditNarrator.WaitForExit(3000)) {
        Stop-Process -Id $auditNarrator.Id
        $auditResult.narrator_forced_close = $true
      }
    }
    $auditNarrator.Refresh()
    $auditResult.narrator_exit_code = $auditNarrator.ExitCode
  }
  $auditResult.finished_utc = [DateTime]::UtcNow.ToString('o')
  $auditResult | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $auditReport -Encoding UTF8
}
$auditResult | ConvertTo-Json -Depth 8
if (-not $auditResult.passed) { exit 1 }
