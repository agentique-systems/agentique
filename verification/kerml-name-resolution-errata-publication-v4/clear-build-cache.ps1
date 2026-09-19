$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path -LiteralPath "$PSScriptRoot/../..").Path
$expected = [IO.Path]::GetFullPath((Join-Path $workspace 'target/debug/incremental'))
$resolved = (Resolve-Path -LiteralPath $expected).Path
if ($resolved -ne $expected -or -not $resolved.StartsWith($workspace + [IO.Path]::DirectorySeparatorChar)) {
    throw 'Refusing to remove a path outside the exact workspace incremental cache'
}
Write-Output "Removing regenerable Rust incremental cache: $resolved"
Remove-Item -LiteralPath $resolved -Recurse -Force
Get-PSDrive C | Select-Object Free | ConvertTo-Json -Compress
