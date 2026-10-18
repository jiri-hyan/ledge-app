# Lokální instalace Ledge z Claude session MIMO bublinu MSIX.
#
# Proč: Claude desktop je balíček MSIX. Všechno, co z jeho session spustíš (terminál, instalátor, Ledge),
# zapisuje do virtualizovaného %APPDATA% / %LOCALAPPDATA% / HKCU (fyzicky
# %LOCALAPPDATA%\Packages\Claude_pzs8sxrjxfjjc\LocalCache, soukromý hive). Instalátor spuštěný „normálně“
# by Ledge nainstaloval jen do bubliny a Explorer by po přihlášení nic nespustil.
# Jak: instalátor spustí WMI (Win32_Process.Create — proces vytvoří služba WMI, mimo bublinu) tiše (/S)
# a bez okna (ShowWindow = 0), počká na jeho PID, a výsledek ověří zase přes WMI, které vidí skutečný
# disk a registr: soubor přes CIM_DataFile, klíče Uninstall a Run přes StdRegProv.
# Hodnotu Run nezapisuje — zapíše ji Ledge sám, až poběží mimo bublinu (`autostart.rs`).
#
# Použití: powershell -NoProfile -File scripts/install-local.ps1 [-Setup <cesta k *-setup.exe>] [-Start]
#   -Setup     instalátor; výchozí nejnovější src-tauri\target\release\bundle\nsis\Ledge_*_x64-setup.exe
#   -Start     po instalaci spustit Ledge mimo bublinu (jednorázová úloha Plánovače v přihlášené relaci,
#              s --autostart), počkat na jeho proces a znovu ověřit Run
#   -TimeoutS  jak dlouho nejvýš čekat na instalátor (výchozí 180 s)
# Pozor: tichý instalátor zavře běžící Ledge ze stejné složky (Restart Manager). Běží-li jiná instance
# (např. z bubliny), -Start skončí chybou — nová instance by se jen ohlásila té staré.
param(
  [string]$Setup,
  [switch]$Start,
  [int]$TimeoutS = 180
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$HKCU = [uint32]2147483649   # 0x80000001 pro StdRegProv
$TaskName = 'Ledge-outside-relaunch'   # stejné jméno jako `outside.rs` → Ledge zbytek uklidí sám

function Fail([string]$msg) { Write-Error $msg; exit 1 }

# Řetězec do filtru WQL (zpětná lomítka a apostrofy zdvojit).
function Wql([string]$s) { $s.Replace('\', '\\').Replace("'", "\'") }

# Hodnota z SKUTEČNÉHO HKCU (StdRegProv přes WMI — ne virtualizovaný hive bubliny); $null = chybí.
function Get-RealReg([string]$key, [string]$name) {
  $r = Invoke-CimMethod -Namespace root/default -ClassName StdRegProv -MethodName GetStringValue `
    -Arguments @{ hDefKey = $HKCU; sSubKeyName = $key; sValueName = $name }
  if ($r.ReturnValue -eq 0) { $r.sValue } else { $null }
}

# Soubor na SKUTEČNÉM disku (CIM_DataFile přes WMI); $null = neexistuje.
function Get-RealFile([string]$path) {
  Get-CimInstance -ClassName CIM_DataFile -Filter ("Name='{0}'" -f (Wql $path.ToLowerInvariant())) -ErrorAction SilentlyContinue |
    Select-Object -First 1
}

function Get-LedgeProcs {
  @(Get-CimInstance -ClassName Win32_Process -Filter "Name='ledge.exe'" -ErrorAction SilentlyContinue)
}

# ---------- 1. instalátor ----------
if (-not $Setup) {
  $dir = Join-Path $root 'src-tauri\target\release\bundle\nsis'
  $cand = Get-ChildItem -Path $dir -Filter 'Ledge_*_x64-setup.exe' -File -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
  if (-not $cand) { Fail "V $dir není žádný Ledge_*_x64-setup.exe — nejdřív build (corepack pnpm tauri build), nebo -Setup <cesta>." }
  $Setup = $cand.FullName
}
$Setup = [IO.Path]::GetFullPath($Setup)
if (-not (Test-Path -LiteralPath $Setup -PathType Leaf)) { Fail "Instalátor $Setup neexistuje." }
# WMI vidí skutečný disk: instalátor uložený do virtualizovaného AppData by tam nenašlo
if (-not (Get-RealFile $Setup)) {
  Fail "Instalátor $Setup existuje jen v bublině (WMI ho nevidí). Zkopíruj ho mimo AppData (např. na Plochu) a spusť znovu s -Setup."
}
Write-Host "instalátor: $Setup"

# ---------- 2. tichá instalace mimo bublinu ----------
$startup = New-CimInstance -ClassName Win32_ProcessStartup -ClientOnly -Property @{ ShowWindow = [uint16]0 }
$created = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{
  CommandLine               = ('"{0}" /S' -f $Setup)
  CurrentDirectory          = (Split-Path -Parent $Setup)
  ProcessStartupInformation = $startup
}
if ($created.ReturnValue -ne 0) { Fail "Win32_Process.Create vrátil $($created.ReturnValue) — instalátor neběží." }
$setupPid = [int]$created.ProcessId
Write-Host "instalátor běží mimo bublinu (PID $setupPid), čekám…"
$deadline = (Get-Date).AddSeconds($TimeoutS)
while (Get-CimInstance -ClassName Win32_Process -Filter "ProcessId=$setupPid" -ErrorAction SilentlyContinue) {
  if ((Get-Date) -gt $deadline) { Fail "Instalátor (PID $setupPid) neskončil do $TimeoutS s — nechávám ho běžet, zkontroluj ho ručně." }
  Start-Sleep -Milliseconds 500
}

# ---------- 3. ověření skutečné instalace ----------
$uninst = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\Ledge'
$loc = Get-RealReg $uninst 'InstallLocation'
$bin = Get-RealReg $uninst 'MainBinaryName'
$ver = Get-RealReg $uninst 'DisplayVersion'
if (-not $loc) { Fail "Ve skutečném HKCU chybí $uninst\InstallLocation — instalace neproběhla (nebo šla do bubliny)." }
if (-not $bin) { $bin = 'ledge.exe' }
$exe = Join-Path $loc.Trim().Trim('"') $bin
$file = Get-RealFile $exe
if (-not $file) { Fail "Skutečný $exe neexistuje — instalace neproběhla." }
Write-Host ("nainstalováno: {0} (verze {1}, upraveno {2})" -f $exe, $(if ($ver) { $ver } else { $file.Version }), $file.LastModified)

# ---------- 4. skutečný Run ----------
$runKey = 'Software\Microsoft\Windows\CurrentVersion\Run'
$want = '"{0}" --autostart' -f $exe
function Show-Run {
  $run = Get-RealReg $runKey 'Ledge'
  if ($run -eq $want) { Write-Host "Run: OK ($run)" }
  elseif ($run) { Write-Warning "Run míří jinam: $run (čekám $want) — opraví ho Ledge spuštěný mimo bublinu." }
  else { Write-Warning "Run Ledge chybí — zapíše ho Ledge spuštěný mimo bublinu (se zapnutým autostartem)." }
  $run
}
$run = Show-Run

# ---------- 5. volitelně spustit mimo bublinu ----------
if ($Start) {
  $others = Get-LedgeProcs
  if ($others.Count -gt 0) {
    $list = ($others | ForEach-Object { "PID $($_.ProcessId): $($_.ExecutablePath)" }) -join '; '
    Fail "Už běží Ledge ($list) — nová instance by se jen ohlásila té staré. Ukonči ho (Nastavení → Restartovat mimo, nebo ručně) a spusť -Start znovu."
  }
  $action = New-ScheduledTaskAction -Execute $exe -Argument '--autostart' -WorkingDirectory (Split-Path -Parent $exe)
  $principal = New-ScheduledTaskPrincipal -UserId ("{0}\{1}" -f $env:USERDOMAIN, $env:USERNAME) -LogonType Interactive -RunLevel Limited
  # bez limitu běhu (výchozí 72 h by Ledge zabilo), i na baterii, normální priorita (výchozí 7 = snížená)
  $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit ([TimeSpan]::Zero) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -Priority 5 -MultipleInstances Parallel
  Register-ScheduledTask -TaskName $TaskName -Action $action -Principal $principal -Settings $settings -Force | Out-Null
  try {
    Start-ScheduledTask -TaskName $TaskName
    $deadline = (Get-Date).AddSeconds(15)
    do {
      Start-Sleep -Milliseconds 300
      $mine = Get-LedgeProcs | Where-Object { $_.ExecutablePath -and ($_.ExecutablePath -ieq $exe) }
    } until ($mine -or (Get-Date) -gt $deadline)
  } finally {
    # smazání úlohy běžící Ledge nezastaví
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
  }
  if (-not $mine) { Fail "Ledge se do 15 s nespustil ($exe)." }
  Write-Host ("Ledge běží mimo bublinu: PID {0}" -f (@($mine)[0].ProcessId))
  # Ledge zapisuje Run při startu (autostart.rs) — chvíli mu dát
  Start-Sleep -Seconds 5
  $run = Show-Run
}
exit 0
