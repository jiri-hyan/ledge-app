# Měří tapetu (`ledge.exe --ext wall`) ve scénářích klid / pauza a odpovídá, jestli Job drží celý strom (WebView2).
# Použití: powershell -ExecutionPolicy Bypass -File scripts/perf-wall.ps1 [-Exe cesta\ledge.exe] [-Seconds 30] [-MinIdleSec 20]
# Pouští okno tapety NA PLOŠE, proto: jen s guardem (SHQueryUserNotificationState ∉ 2/3/4, nečinnost >= MinIdleSec),
# guard se kontroluje před každým scénářem; proces se vždy ukončí (Job KILL_ON_JOB_CLOSE) a ověří se, že nic nezůstalo.
# Nespouštět z bran. Žádné zásahy do cizích oken, tapety ani Průzkumníka.
param([string]$Exe = '', [int]$Seconds = 30, [int]$MinIdleSec = 20)
. "$PSScriptRoot\perf-lib.ps1"
$ErrorActionPreference = 'Stop'

if (-not $Exe) {
  # worktree leží uvnitř repa: cíl buildu hledat i v master složce (git common dir) a v CARGO_TARGET_DIR
  $roots = @($env:CARGO_TARGET_DIR, (Join-Path $PSScriptRoot '..\src-tauri\target-agents'))
  $master = (& git -C $PSScriptRoot rev-parse --path-format=absolute --git-common-dir 2>$null)
  if ($master) { $roots += (Join-Path (Split-Path $master -Parent) 'src-tauri\target-agents') }
  $Exe = foreach ($r in $roots) { if ($r) { foreach ($k in 'release', 'debug') { $c = Join-Path $r "$k\ledge.exe"; if (Test-Path $c) { $c; break } } } }
  $Exe = $Exe | Select-Object -First 1
}
if (-not $Exe -or -not (Test-Path $Exe)) { "neměřeno: chybí ledge.exe (zadej -Exe)"; exit 2 }
$Exe = (Resolve-Path $Exe).Path
# Starší build nezná `--ext` a spustil by celý Ledge — proto nejdřív zkontrolovat, že binár obsahuje kód rozšíření.
$bytes = [IO.File]::ReadAllBytes($Exe)
if ([Text.Encoding]::GetEncoding(28591).GetString($bytes).IndexOf('Local\ledge-ext-') -lt 0) { "neměřeno: $Exe je starší build bez --ext (postav ho přes frontu)"; exit 2 }
$why = Test-MeasureGuard ($MinIdleSec * 1000)
if ($why) { $why; exit 2 }
$already = Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'ledge.exe' -and (Get-ExtId $_.CommandLine) -eq 'wall' }
if ($already) { "neměřeno: tapeta už běží (pid $($already.ProcessId)) — vypni ji v Nastavení"; exit 2 }

Add-Type -TypeDefinition @'
using System; using System.Runtime.InteropServices;
public static class LedgeJob {
  [DllImport("kernel32.dll")] static extern IntPtr CreateJobObjectW(IntPtr a, IntPtr n);
  [DllImport("kernel32.dll")] static extern bool SetInformationJobObject(IntPtr j, int c, IntPtr i, int l);
  [DllImport("kernel32.dll")] static extern bool QueryInformationJobObject(IntPtr j, int c, IntPtr i, int l, IntPtr r);
  [DllImport("kernel32.dll")] static extern bool AssignProcessToJobObject(IntPtr j, IntPtr p);
  [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr h);
  public static IntPtr Create() {
    IntPtr j = CreateJobObjectW(IntPtr.Zero, IntPtr.Zero);
    IntPtr b = Marshal.AllocHGlobal(144);
    for (int i = 0; i < 144; i++) Marshal.WriteByte(b, i, 0);
    Marshal.WriteInt32(b, 16, 0x2000); // JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
    bool ok = SetInformationJobObject(j, 9, b, 144);
    Marshal.FreeHGlobal(b);
    return ok ? j : IntPtr.Zero;
  }
  public static bool Assign(IntPtr j, IntPtr proc) { return AssignProcessToJobObject(j, proc); }
  public static uint[] Pids(IntPtr j) {
    IntPtr b = Marshal.AllocHGlobal(4096);
    try {
      if (!QueryInformationJobObject(j, 3, b, 4096, IntPtr.Zero)) return new uint[0];
      int n = Marshal.ReadInt32(b, 4);
      uint[] r = new uint[n];
      for (int i = 0; i < n; i++) r[i] = (uint)Marshal.ReadInt64(b, 8 + i * 8);
      return r;
    } finally { Marshal.FreeHGlobal(b); }
  }
  // celkový CPU čas Jobu (100 ns) vč. už skončených procesů
  public static long CpuTicks(IntPtr j) {
    IntPtr b = Marshal.AllocHGlobal(64);
    try {
      if (!QueryInformationJobObject(j, 1, b, 48, IntPtr.Zero)) return -1;
      return Marshal.ReadInt64(b, 0) + Marshal.ReadInt64(b, 8);
    } finally { Marshal.FreeHGlobal(b); }
  }
  public static void Close(IntPtr j) { CloseHandle(j); }
}
'@

$job = [LedgeJob]::Create()
if ($job -eq [IntPtr]::Zero) { "chyba: Job se nepodařilo vytvořit"; exit 1 }
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $Exe; $psi.Arguments = '--ext wall'
$psi.UseShellExecute = $false; $psi.CreateNoWindow = $true
$psi.RedirectStandardInput = $true; $psi.RedirectStandardOutput = $true
$proc = $null
$result = [ordered]@{}
try {
  $proc = [System.Diagnostics.Process]::Start($psi)
  # dítě se přiřadí do Jobu dřív než vznikne WebView2 (to čeká na `init`), podprocesy dědí Job
  [void][LedgeJob]::Assign($job, $proc.Handle)

  function Read-Line([int]$ms) {
    $t = $proc.StandardOutput.ReadLineAsync()
    if ($t.Wait($ms)) { return $t.Result } else { return $null }
  }
  function Send([string]$json) { $proc.StandardInput.WriteLine($json); $proc.StandardInput.Flush() }

  $hello = Read-Line 10000
  if (-not $hello -or $hello -notmatch '"hello"') { throw "žádný hello od rozšíření ($hello)" }
  Send '{"v":1,"t":"init","cfg":{},"lang":"cs","dead":false}'
  $ready = $false
  for ($i = 0; $i -lt 20 -and -not $ready; $i++) {
    $l = Read-Line 2000
    if ($l -match '"ready"') { $ready = $true }
  }
  if (-not $ready) { throw 'tapeta do 40 s nehlásí ready' }

  function Measure-Scenario([string]$name) {
    $g = Test-MeasureGuard ($MinIdleSec * 1000)
    if ($g) { return [pscustomobject]@{ Scenario = $name; Note = $g } }
    Start-Sleep -Seconds 5   # usazení po přepnutí stavu
    $c0 = [LedgeJob]::CpuTicks($job); $t0 = Get-Date
    Start-Sleep -Seconds $Seconds
    $c1 = [LedgeJob]::CpuTicks($job); $el = ((Get-Date) - $t0).TotalSeconds
    $cores = [Environment]::ProcessorCount
    $jobPids = @([LedgeJob]::Pids($job) | ForEach-Object { [int]$_ })
    $rows = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name, CommandLine, CreationDate
    $groups = Get-LedgeGroups $rows
    $treePids = @($groups.Keys | Where-Object { $groups[$_] -eq 'ext:wall' } | ForEach-Object { [int]$_ })
    $mb = { param($ids) [Math]::Round((($ids | ForEach-Object { (Get-Process -Id $_ -ErrorAction SilentlyContinue).PrivateMemorySize64 } | Measure-Object -Sum).Sum) / 1MB, 1) }
    [pscustomobject]@{
      Scenario = $name
      CpuPct = [Math]::Round((($c1 - $c0) / 1e7) / $el / $cores * 100, 3)
      JobProcs = $jobPids.Count; JobMb = (& $mb $jobPids)
      TreeProcs = $treePids.Count; TreeMb = (& $mb $treePids)
      OutsideJob = @($treePids | Where-Object { $jobPids -notcontains $_ }).Count
      Note = ''
    }
  }

  $result.klid = Measure-Scenario 'klid'
  Send '{"v":1,"t":"state","pause":true,"why":"perf"}'
  $result.pauza = Measure-Scenario 'pauza'
  Send '{"v":1,"t":"state","pause":false}'
  Send '{"v":1,"t":"quit"}'
  [void]$proc.WaitForExit(5000)
} finally {
  if ($proc -and -not $proc.HasExited) { try { $proc.Kill() } catch {} }
  [LedgeJob]::Close($job)   # KILL_ON_JOB_CLOSE vezme i WebView2
}

Start-Sleep -Seconds 2
$left = Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -match '--ext\s+wall' -or $_.CommandLine -match 'Ledge\\ext\\wall' }
$result.Values | Format-Table -AutoSize | Out-String -Width 200 | Write-Host
$out = @($result.Values | Where-Object { $_.PSObject.Properties.Name -contains 'OutsideJob' })
if ($out.Count) {
  $o = ($out | Measure-Object -Property OutsideJob -Sum).Sum
  "WebView2 podprocesy v jobu: " + $(if ($o -eq 0) { 'ANO (strom PID == Job)' } else { "NE ($o procesů stromu mimo Job)" })
}
if ($left) { "POZOR: zůstaly procesy: " + (($left | ForEach-Object { "$($_.Name)#$($_.ProcessId)" }) -join ', ') } else { 'úklid: žádný proces nezůstal' }
