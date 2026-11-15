# Měří Ledge proti výkonovému rozpočtu: ledge.exe (bez --ext) + jeho WebView2. Rozšíření (`ledge.exe --ext <id>`
# i s podprocesy) se do rozpočtu Ledge NEpočítají a vypíšou se zvlášť (RAM/CPU per id).
# Použití: powershell -ExecutionPolicy Bypass -File scripts/perf.ps1 [-Seconds 30]
param([int]$Seconds = 30)
. "$PSScriptRoot\perf-lib.ps1"

function Get-Rows {
  Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name, CommandLine, CreationDate
}
function Sample([hashtable]$groups, [string]$g) {
  $ids = @($groups.Keys | Where-Object { $groups[$_] -eq $g })
  $procs = $ids | ForEach-Object { Get-Process -Id $_ -ErrorAction SilentlyContinue } | Where-Object { $_ }
  $cpu = [TimeSpan]::Zero; foreach ($p in $procs) { $cpu += $p.TotalProcessorTime }
  [pscustomobject]@{ Procs = @($procs); Cpu = $cpu }
}

$groups = Get-LedgeGroups (Get-Rows)
if (-not ($groups.Values -contains 'core')) { Write-Error "Ledge neběží"; exit 1 }
$names = @(Get-GroupNames $groups)
$s0 = @{}; foreach ($g in $names) { $s0[$g] = Sample $groups $g }
$t0 = Get-Date
Start-Sleep -Seconds $Seconds
$elapsed = ((Get-Date) - $t0).TotalSeconds
$cores = [Environment]::ProcessorCount
$main = (Sample $groups 'core').Procs | Where-Object { $_.ProcessName -eq 'ledge' } | Select-Object -First 1

foreach ($g in $names) {
  $s1 = Sample $groups $g
  $cpuPct = [Math]::Round((($s1.Cpu - $s0[$g].Cpu).TotalSeconds / $elapsed / $cores) * 100, 3)
  $privMb = [Math]::Round((($s1.Procs | Measure-Object -Property PrivateMemorySize64 -Sum).Sum) / 1MB, 1)
  $wsMb = [Math]::Round((($s1.Procs | Measure-Object -Property WorkingSet64 -Sum).Sum) / 1MB, 1)
  $kinds = ($s1.Procs | ForEach-Object Name | Group-Object | ForEach-Object { "$($_.Name)×$($_.Count)" }) -join ', '
  if ($g -eq 'core') {
    $coreMb = if ($main) { [Math]::Round($main.PrivateMemorySize64 / 1MB, 1) } else { 'n/a' }
    "procesy Ledge:      $($s1.Procs.Count) ($kinds)"
    "CPU průměr ($Seconds s): $cpuPct % celého stroje ($cores jader)   rozpočet < 0.1 %"
    "jádro ledge.exe:    $coreMb MB private                          rozpočet < 25 MB"
    "celkem se WebView2: $privMb MB private ($wsMb MB working set)   rozpočet < 90 MB"
  } else {
    "rozšíření $($g.Substring(4)): $($s1.Procs.Count) procesů ($kinds), $privMb MB private ($wsMb MB WS), CPU $cpuPct %   (mimo rozpočet Ledge)"
  }
}
if ($names.Count -eq 1) { "rozšíření:         žádné neběží" }
