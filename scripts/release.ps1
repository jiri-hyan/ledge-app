# Vydání Ledge: release build + NSIS instalátor (installMode currentUser → %LOCALAPPDATA%\Ledge).
# Vypíše cestu, velikost a SHA256 vzniklého *-setup.exe.
# Použití: powershell -NoProfile -File scripts/release.ps1 [-Install]
#   -Install  když Ledge (ani starý Notch) neběží, nainstaluje tiše (/S) a nainstalovanou kopii spustí.
#             Když běží, jen řekne „zavři Ledge“ a skončí — nic nezabíjí (tichý instalátor
#             Tauri by běžící kopii sám násilně ukončil přes Restart Manager).
param([switch]$Install)
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$tauriDir = Join-Path $root 'src-tauri'

# --- verze: tauri.conf.json a Cargo.toml se musí shodovat ---
$confVer = (Get-Content (Join-Path $tauriDir 'tauri.conf.json') -Raw -Encoding UTF8 | ConvertFrom-Json).version
$cargoVer = $null
$section = ''
foreach ($line in Get-Content (Join-Path $tauriDir 'Cargo.toml') -Encoding UTF8) {
  if ($line -match '^\s*\[(.+)\]\s*$') { $section = $Matches[1]; continue }
  if ($section -eq 'package' -and $line -match '^\s*version\s*=\s*"([^"]+)"') { $cargoVer = $Matches[1]; break }
}
if (-not $confVer -or -not $cargoVer) { Write-Error "nenašel jsem verzi (tauri.conf.json: '$confVer', Cargo.toml: '$cargoVer')"; exit 1 }
if ($confVer -ne $cargoVer) { Write-Error "verze nesouhlasí: tauri.conf.json $confVer ≠ Cargo.toml $cargoVer"; exit 1 }
"verze:      $confVer"

# --- kam cargo staví (respektuje CARGO_TARGET_DIR) ---
$targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $tauriDir 'target' }
$releaseExe = [IO.Path]::GetFullPath((Join-Path $targetDir 'release\ledge.exe'))

# běžící vývojová binárka z target\release by shodila link („přístup odepřen“) — raději to říct hned
$devRunning = Get-Process ledge -ErrorAction SilentlyContinue | Where-Object { $_.Path -and ($_.Path -ieq $releaseExe) }
if ($devRunning) { Write-Error "běží vývojový Ledge z $releaseExe — zavři ho, jinak build neprojde"; exit 1 }

# --- build ---
$started = Get-Date
Push-Location $root
try {
  # pnpm nemusí být na PATH (druhé PC). Tauri ho volá i samo (`beforeBuildCommand`), proto shimy
  # corepacku (součást Node) do dočasné složky a ta na PATH jen pro tenhle proces.
  if (-not (Get-Command pnpm -ErrorAction SilentlyContinue)) {
    $shim = Join-Path $env:TEMP 'ledge-corepack'
    New-Item -ItemType Directory -Force $shim | Out-Null
    & corepack enable --install-directory $shim pnpm
    if ($LASTEXITCODE -ne 0) { Write-Error 'pnpm chybí a corepack enable selhal'; exit 1 }
    $env:PATH = "$shim;$env:PATH"
  }
  & pnpm tauri build --bundles nsis
  if ($LASTEXITCODE -ne 0) { Write-Error "pnpm tauri build selhal (exit $LASTEXITCODE)"; exit 1 }
} finally {
  Pop-Location
}

# --- najdi instalátor z tohoto běhu (ne starý z dřívějška) ---
$nsisDir = Join-Path $targetDir 'release\bundle\nsis'
$setup = Get-ChildItem -Path $nsisDir -Filter "*_$($confVer)_*-setup.exe" -ErrorAction SilentlyContinue |
  Where-Object { $_.LastWriteTime -ge $started } |
  Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $setup) { Write-Error "v $nsisDir není čerstvý *-setup.exe pro verzi $confVer"; exit 1 }

$hash = (Get-FileHash -Algorithm SHA256 -Path $setup.FullName).Hash
"instalátor: $($setup.FullName)"
"velikost:   $([Math]::Round($setup.Length / 1MB, 2)) MB ($($setup.Length) B)"
"SHA256:     $hash"

if (-not $Install) { exit 0 }

# --- instalace ---
if (Get-Process ledge -ErrorAction SilentlyContinue) {
  "zavři Ledge (běží) a spusť skript znovu s -Install — nic neinstaluji"
  exit 1
}
# starý Notch (před přejmenováním) drží port mostu 47800 a nová instalace by s ním nešla dohromady
if (Get-Process notch -ErrorAction SilentlyContinue) {
  "zavři starý Notch (běží, drží port 47800) a spusť skript znovu s -Install — nic neinstaluji"
  exit 1
}
"instaluji tiše…"
$p = Start-Process -FilePath $setup.FullName -ArgumentList '/S' -Wait -PassThru
if ($p.ExitCode -ne 0) { Write-Error "instalátor skončil s kódem $($p.ExitCode)"; exit 1 }

# kam se instalovalo: NSIS šablona Tauri zapisuje HKCU\…\Uninstall\<productName> InstallLocation (v uvozovkách)
# + MainBinaryName; výchozí je $LOCALAPPDATA\<productName> (viz doc-komentář v src-tauri/src/autostart.rs)
$installed = $null
$uninst = Get-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Ledge' -ErrorAction SilentlyContinue
if ($uninst -and $uninst.InstallLocation) {
  $bin = if ($uninst.MainBinaryName) { $uninst.MainBinaryName } else { 'ledge.exe' }
  $installed = Join-Path $uninst.InstallLocation.Trim('"') $bin
}
if (-not $installed -or -not (Test-Path $installed)) { $installed = Join-Path $env:LOCALAPPDATA 'Ledge\ledge.exe' }
if (-not (Test-Path $installed)) { Write-Error "instalace doběhla, ale $installed neexistuje"; exit 1 }

"spouštím:   $installed"
Start-Process -FilePath $installed
