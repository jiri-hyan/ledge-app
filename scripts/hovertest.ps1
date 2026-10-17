# Skutečný test otevření najetím: kurzor na proužek → fotka „nažhaveného“ proužku → otevření →
# kurzor pryč → zavření. Hýbe myší (krátce), proto jen na vyžádání.
param([string]$Out = ".")
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class M {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr v);
  public struct POINT { public int X; public int Y; }
}
"@
[M]::SetProcessDpiAwarenessContext([IntPtr]-4) | Out-Null
Add-Type -AssemblyName System.Windows.Forms
$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$p = New-Object M+POINT; [M]::GetCursorPos([ref]$p) | Out-Null
$cx = $b.X + [int]($b.Width / 2)
$shot = "$PSScriptRoot\shot.ps1"
function St { (Invoke-RestMethod http://127.0.0.1:47800/status).drawer }
[M]::SetCursorPos($cx, $b.Y + 12) | Out-Null
Start-Sleep -Milliseconds 70
powershell -NoProfile -File $shot -Out "$Out\hot.png" -Height 70 -Width 1400 | Out-Null
$t0 = Get-Date
do { Start-Sleep -Milliseconds 20; $v = (St).view } until ($v -eq "full" -or ((Get-Date) - $t0).TotalMilliseconds -gt 2000)
"otevřeno najetím: view=$v za $([int]((Get-Date) - $t0).TotalMilliseconds) ms od fotky (prodleva 140 ms)"
Start-Sleep -Milliseconds 900
"animace: $((St).anim | ConvertTo-Json -Compress)"
[M]::SetCursorPos($cx, $b.Y + [int]($b.Height * 0.8)) | Out-Null
$t0 = Get-Date
do { Start-Sleep -Milliseconds 20; $v = (St).view } until ($v -eq "hidden" -or ((Get-Date) - $t0).TotalMilliseconds -gt 3000)
"zavřeno po odjetí: view=$v za $([int]((Get-Date) - $t0).TotalMilliseconds) ms (prodleva 350 ms + animace)"
Start-Sleep -Milliseconds 500
"animace zavření: $((St).anim | ConvertTo-Json -Compress)"
[M]::SetCursorPos($p.X, $p.Y) | Out-Null
