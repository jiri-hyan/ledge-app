# Porovná paměť WebView2 šuplíku pro různé argumenty Chromia (release build, šuplík otevřen a zavřen).
# Argumenty se předávají přes WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS, takže není potřeba rekompilace.
param([string]$Exe = "$PSScriptRoot\..\src-tauri\target\release\ledge.exe")
$base = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --enable-features=NetworkServiceInProcess2 --disable-background-networking --disable-component-update --disable-sync --disable-default-apps --no-first-run --disable-breakpad --renderer-process-limit=1 --disable-gpu-shader-disk-cache --disable-gpu --in-process-gpu"
$variants = [ordered]@{
  # = BROWSER_ARGS v drawer.rs
  "zaklad"         = $base
  "gpu-proces"     = $base -replace ' --in-process-gpu', ''
  "sit-proces"     = $base -replace ' --enable-features=NetworkServiceInProcess2', ''
}
foreach ($k in $variants.Keys) {
  Get-Process ledge -ErrorAction SilentlyContinue | Stop-Process -Force
  Start-Sleep 2
  $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $variants[$k]
  Start-Process $Exe
  Start-Sleep 3
  $sw = [Diagnostics.Stopwatch]::StartNew()
  Invoke-RestMethod -Method Post http://127.0.0.1:47800/ctl/open -Body '{}' | Out-Null
  do { Start-Sleep -Milliseconds 100; $d = (Invoke-RestMethod http://127.0.0.1:47800/status).drawer } until ($d.lastOpenMs -gt 0 -or $sw.ElapsedMilliseconds -gt 20000)
  $cold = $d.lastOpenMs
  Start-Sleep 1
  Invoke-RestMethod -Method Post http://127.0.0.1:47800/ctl/close -Body '{}' | Out-Null
  Start-Sleep 1
  Invoke-RestMethod -Method Post http://127.0.0.1:47800/ctl/open -Body '{}' | Out-Null
  Start-Sleep -Milliseconds 600
  $warm = (Invoke-RestMethod http://127.0.0.1:47800/status).drawer.lastOpenMs
  Invoke-RestMethod -Method Post http://127.0.0.1:47800/ctl/close -Body '{}' | Out-Null
  Start-Sleep 6
  $main = Get-Process ledge | Select-Object -First 1
  $all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId
  $ids = @($main.Id); $f = @($main.Id)
  while ($f.Count) { $n = @($all | Where-Object { $f -contains $_.ParentProcessId } | ForEach-Object ProcessId); $ids += $n; $f = $n }
  $ps = $ids | ForEach-Object { Get-Process -Id $_ -ErrorAction SilentlyContinue }
  $tot = [Math]::Round((($ps | Measure-Object PrivateMemorySize64 -Sum).Sum) / 1MB, 1)
  "{0,-16} procesů {1}  celkem {2,6} MB private  studené otevření {3} ms  teplé {4} ms" -f $k, $ps.Count, $tot, $cold, $warm
}
Remove-Item Env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
Get-Process ledge -ErrorAction SilentlyContinue | Stop-Process -Force
