param([ValidateSet('egui','slint','gpui')][string]$Candidate, [string]$Profile='debug')
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class BakeoffWindow {
 [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left,Top,Right,Bottom; }
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr handle, out Rect rect);
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr handle);
 private delegate bool EnumWindow(IntPtr handle, IntPtr argument);
 [DllImport("user32.dll")] private static extern bool EnumWindows(EnumWindow callback, IntPtr argument);
 [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr handle, out uint processId);
 [DllImport("user32.dll")] private static extern bool IsWindowVisible(IntPtr handle);
 public static IntPtr Largest(int processId) {
   IntPtr best=IntPtr.Zero; long bestArea=0;
   EnumWindows((h,a)=>{uint p; GetWindowThreadProcessId(h,out p); Rect r;
     if(p==processId && IsWindowVisible(h) && GetWindowRect(h,out r)) {
       long area=(long)(r.Right-r.Left)*(r.Bottom-r.Top);
       if(area>bestArea){best=h;bestArea=area;}
     } return true;},IntPtr.Zero);
   return best;
 }
}
'@
$base=Resolve-Path "$PSScriptRoot/../.."
$out=Join-Path $base 'verification/generated/native-bakeoff'
New-Item -ItemType Directory -Force $out | Out-Null
$exe=Join-Path $PSScriptRoot "$Candidate/target/$Profile/agq-bakeoff-$Candidate.exe"
$p=Start-Process -FilePath $exe -ArgumentList '--bench' -WindowStyle Hidden -PassThru -RedirectStandardOutput "$out/$Candidate-$Profile-run.txt" -RedirectStandardError "$out/$Candidate-$Profile-errors.txt"
$null=$p.Handle
Start-Sleep -Seconds 2
$p.Refresh()
$window=[BakeoffWindow]::Largest($p.Id)
if (!$p.HasExited -and $window -ne 0) {
    [BakeoffWindow]::SetForegroundWindow($window) | Out-Null
    Start-Sleep -Milliseconds 150
    $rect=New-Object BakeoffWindow+Rect
    [BakeoffWindow]::GetWindowRect($window,[ref]$rect) | Out-Null
    $bitmap=New-Object Drawing.Bitmap(($rect.Right-$rect.Left),($rect.Bottom-$rect.Top))
    $graphics=[Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($rect.Left,$rect.Top,0,0,$bitmap.Size)
    $bitmap.Save("$out/$Candidate-$Profile.png",[Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose(); $bitmap.Dispose()
}
if(!$p.WaitForExit(55000)){Write-Output "pending_pid=$($p.Id)";exit 2}
Get-Content "$out/$Candidate-$Profile-run.txt"
Get-Content "$out/$Candidate-$Profile-errors.txt"
"exit=$($p.ExitCode)" | Tee-Object "$out/$Candidate-$Profile-exit.txt"
exit $p.ExitCode
