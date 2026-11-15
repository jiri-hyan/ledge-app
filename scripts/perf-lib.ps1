# Sdílené čisté funkce pro perf.ps1 / perf-wall.ps1 (dot-source). Test: scripts/perf-lib.test.mjs.

# Rozdělí procesy na skupiny: 'core' (ledge.exe bez --ext a jeho WebView2) a 'ext:<id>' (ledge.exe --ext <id>
# a CELÝ jeho strom, tedy i WebView2 rozšíření). Vstup: řádky {ProcessId, ParentProcessId, Name, CommandLine, CreationDate}.
# Výstup: hashtable pid -> skupina. Proces mimo strom ledge.exe se vynechá. Dítě musí být mladší než rodič
# (recyklované PID rodiče = cizí proces).
function Get-ExtId([string]$cmd) {
  if ($cmd -match '(?:^|\s)--ext(?:\s+|=)"?([A-Za-z0-9_-]+)"?') { return $Matches[1] }
  return $null
}

function Get-LedgeGroups($rows) {
  $byPid = @{}; foreach ($r in $rows) { $byPid[[int]$r.ProcessId] = $r }
  $groups = @{}
  $coreRoot = $null
  foreach ($r in $rows) {
    if ($r.Name -notmatch '^ledge(\.exe)?$') { continue }
    $id = Get-ExtId $r.CommandLine
    if ($id) { $groups[[int]$r.ProcessId] = "ext:$id" }
    elseif (-not $coreRoot) { $coreRoot = [int]$r.ProcessId; $groups[$coreRoot] = 'core' }
  }
  # potomci: opakovat, dokud přibývají (pořadí řádků není zaručeno)
  do {
    $added = $false
    foreach ($r in $rows) {
      $p = [int]$r.ProcessId
      if ($groups.ContainsKey($p)) { continue }
      $par = [int]$r.ParentProcessId
      if (-not $groups.ContainsKey($par)) { continue }
      if ($r.CreationDate -lt $byPid[$par].CreationDate) { continue }
      $groups[$p] = $groups[$par]; $added = $true
    }
  } while ($added)
  return $groups
}

# Úplné ledge.exe rozšíření (bez WebView2) — pro výpis „kolik jich je“.
function Get-GroupNames($groups) { $groups.Values | Select-Object -Unique | Sort-Object }

# Idle guard (jen pro měření tapety): vrací $null = smí, jinak důvod. Stav 2/3/4 = fullscreen, hra, prezentace.
function Test-MeasureGuard([int]$MinIdleMs = 20000) {
  Add-Type -ErrorAction SilentlyContinue -TypeDefinition @'
using System; using System.Runtime.InteropServices;
public static class LedgePerfGuard {
  [StructLayout(LayoutKind.Sequential)] struct LII { public uint cbSize; public uint dwTime; }
  [DllImport("user32.dll")] static extern bool GetLastInputInfo(ref LII p);
  [DllImport("shell32.dll")] static extern int SHQueryUserNotificationState(out int s);
  public static int Notif() { int s; return SHQueryUserNotificationState(out s) == 0 ? s : 0; }
  public static long IdleMs() { LII i = new LII(); i.cbSize = 8; if (!GetLastInputInfo(ref i)) return -1; return (uint)(Environment.TickCount - (int)i.dwTime); }
}
'@
  $n = [LedgePerfGuard]::Notif()
  if ($n -in 2, 3, 4) { return "neměřeno: stav oznámení $n (hra/fullscreen/prezentace)" }
  $idle = [LedgePerfGuard]::IdleMs()
  if ($idle -lt 0) { return 'neměřeno: nečinnost nezjistitelná' }
  if ($idle -lt $MinIdleMs) { return "neměřeno: uživatel aktivní (nečinnost $idle ms < $MinIdleMs ms)" }
  return $null
}
