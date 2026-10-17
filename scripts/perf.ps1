# Měří Ledge proti výkonovému rozpočtu: ledge.exe + celý jeho strom (WebView2 procesy).
# Použití: powershell -File scripts/perf.ps1 [-Seconds 30]
param([int]$Seconds = 30)

function Get-Tree([int]$root) {
  $all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name
  $ids = @($root); $frontier = @($root)
  while ($frontier.Count) {
    $next = @($all | Where-Object { $frontier -contains $_.ParentProcessId } | ForEach-Object { $_.ProcessId })
    $ids += $next; $frontier = $next
  }
  $ids | Select-Object -Unique
}

$main = Get-Process ledge -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $main) { Write-Error "Ledge neběží"; exit 1 }
$ids = Get-Tree $main.Id
$procs = $ids | ForEach-Object { Get-Process -Id $_ -ErrorAction SilentlyContinue } | Where-Object { $_ }
$cpu0 = [TimeSpan]::Zero
foreach ($p in $procs) { $cpu0 += $p.TotalProcessorTime }
$t0 = Get-Date
Start-Sleep -Seconds $Seconds
$procs = $ids | ForEach-Object { Get-Process -Id $_ -ErrorAction SilentlyContinue } | Where-Object { $_ }
$cpu1 = [TimeSpan]::Zero
foreach ($p in $procs) { $cpu1 += $p.TotalProcessorTime }
$elapsed = ((Get-Date) - $t0).TotalSeconds
$cores = [Environment]::ProcessorCount
$cpuPct = [Math]::Round((($cpu1 - $cpu0).TotalSeconds / $elapsed / $cores) * 100, 3)

$coreMb = [Math]::Round($main.PrivateMemorySize64 / 1MB, 1)
$totalMb = [Math]::Round((($procs | Measure-Object -Property PrivateMemorySize64 -Sum).Sum) / 1MB, 1)
$wsMb = [Math]::Round((($procs | Measure-Object -Property WorkingSet64 -Sum).Sum) / 1MB, 1)

"procesy:            $($procs.Count) ($((($procs | ForEach-Object Name) | Group-Object | ForEach-Object { "$($_.Name)×$($_.Count)" }) -join ', '))"
"CPU průměr ($Seconds s): $cpuPct % celého stroje ($cores jader)   rozpočet < 0.1 %"
"jádro ledge.exe:    $coreMb MB private                          rozpočet < 25 MB"
"celkem se WebView2: $totalMb MB private ($wsMb MB working set)   rozpočet < 90 MB"
