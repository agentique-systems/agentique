param([ValidateSet('egui','slint','gpui')][string]$Candidate, [string]$Profile='debug')
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class BakeoffWindow {
 [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left,Top,Right,Bottom; }
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr handle, out Rect rect);
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr handle);
 [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr handle, int state);
 private delegate bool EnumWindow(IntPtr handle, IntPtr argument);
 [DllImport("user32.dll")] private static extern bool EnumWindows(EnumWindow callback, IntPtr argument);
 [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr handle, out uint processId);
 [DllImport("user32.dll")] private static extern bool IsWindowVisible(IntPtr handle);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] private static extern int GetWindowText(IntPtr handle, StringBuilder title, int length);
 public static IntPtr Largest(int processId) {
   IntPtr best=IntPtr.Zero; long bestArea=0;
   EnumWindows((h,a)=>{uint p; GetWindowThreadProcessId(h,out p); Rect r;
     var title=new StringBuilder(512); GetWindowText(h,title,title.Capacity);
     if(p==processId && title.ToString().StartsWith("Agentique framework bakeoff") && GetWindowRect(h,out r)) {
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
$capturePath=Join-Path $out "$Candidate-$Profile.png"
if(Test-Path -LiteralPath $capturePath){Remove-Item -LiteralPath $capturePath}
$exe=Join-Path $PSScriptRoot "$Candidate/target/$Profile/agq-bakeoff-$Candidate.exe"
$p=Start-Process -FilePath $exe -ArgumentList '--bench' -WindowStyle Hidden -PassThru -RedirectStandardOutput "$out/$Candidate-$Profile-run.txt" -RedirectStandardError "$out/$Candidate-$Profile-errors.txt"
$null=$p.Handle
Start-Sleep -Seconds 2
$p.Refresh()
$window=[BakeoffWindow]::Largest($p.Id)
for($attempt=0;$attempt -lt 50 -and $window -eq 0 -and !$p.HasExited;$attempt++) {
    Start-Sleep -Milliseconds 100
    $window=[BakeoffWindow]::Largest($p.Id)
}
if (!$p.HasExited -and $window -ne 0) {
    # The launcher stays hidden. Reveal only the exact probe's native UI for its
    # requested visual review; GPUI respects inherited SW_HIDE at first creation.
    [BakeoffWindow]::ShowWindowAsync($window,5) | Out-Null
    [BakeoffWindow]::SetForegroundWindow($window) | Out-Null
    Start-Sleep -Milliseconds 700
    [BakeoffWindow]::SetForegroundWindow($window) | Out-Null
    $rect=New-Object BakeoffWindow+Rect
    [BakeoffWindow]::GetWindowRect($window,[ref]$rect) | Out-Null
    $bitmap=New-Object Drawing.Bitmap(($rect.Right-$rect.Left),($rect.Bottom-$rect.Top))
    $graphics=[Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($rect.Left,$rect.Top,0,0,$bitmap.Size)
    $bitmap.Save($capturePath,[Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose(); $bitmap.Dispose()
}
if(!$p.WaitForExit(55000)){Write-Output "pending_pid=$($p.Id)";exit 2}
Get-Content "$out/$Candidate-$Profile-run.txt"
Get-Content "$out/$Candidate-$Profile-errors.txt"
"exit=$($p.ExitCode)" | Tee-Object "$out/$Candidate-$Profile-exit.txt"
exit $p.ExitCode
