# Vyfotí jen pás u horního okraje uprostřed hlavního monitoru (tam žije Ledge) — ne celou plochu.
# Použití: powershell -File scripts/shot.ps1 -Out shot.png [-Height 600] [-Width 1400]
param([string]$Out = "shot.png", [int]$Height = 600, [int]$Width = 1400)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class Dpi { [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr v); }
"@
[Dpi]::SetProcessDpiAwarenessContext([IntPtr]-4) | Out-Null
Add-Type -AssemblyName System.Windows.Forms
$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$w = [Math]::Min($Width, $b.Width)
$x = $b.X + [int](($b.Width - $w) / 2)
$bmp = New-Object System.Drawing.Bitmap $w, $Height
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($x, $b.Y, 0, 0, (New-Object System.Drawing.Size $w, $Height))
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Output "$Out ${w}x$Height @ $x,$($b.Y) (monitor $($b.Width)x$($b.Height))"
