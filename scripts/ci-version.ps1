# Zapíše verzi do tauri.conf.json, [package] v Cargo.toml (+ odpovídající záznam v Cargo.lock)
# a package.json — v CI checkoutu před buildem (necommituje se). Mění jen řádky s verzí, zbytek
# souborů nechá beze změny. Když některé místo nenajde, skončí chybou (žádné tiché vydání se starou verzí).
# Použití: powershell -NoProfile -File scripts/ci-version.ps1 -Version 0.2.14 [-Root <kořen repa>]
param(
  [Parameter(Mandatory)][string]$Version,
  [string]$Root = (Split-Path -Parent $PSScriptRoot)
)
$ErrorActionPreference = 'Stop'
if ($Version -notmatch '^\d+\.\d+\.\d+$') { Write-Error "verze '$Version' není x.y.z"; exit 1 }

$utf8 = New-Object System.Text.UTF8Encoding($false)
function Set-Once([string]$path, [string]$pattern, [string]$label) {
  $text = [IO.File]::ReadAllText($path, $utf8)
  $re = [regex]::new($pattern, 'Multiline')
  if (-not $re.IsMatch($text)) { Write-Error "$label`: v $path jsem nenašel verzi"; exit 1 }
  $new = $re.Replace($text, { param($m) $m.Groups[1].Value + '"' + $Version + '"' }, 1)
  [IO.File]::WriteAllText($path, $new, $utf8)
  "$label -> $Version"
}

$tauri = Join-Path $Root 'src-tauri'
# první "version" v souboru = kořenový klíč (tauri.conf.json i package.json ho mají nahoře)
Set-Once (Join-Path $tauri 'tauri.conf.json') '^(\s*"version"\s*:\s*)"[^"]*"' 'tauri.conf.json'
Set-Once (Join-Path $Root 'package.json') '^(\s*"version"\s*:\s*)"[^"]*"' 'package.json'
# version přímo v sekci [package] (ne v závislostech)
Set-Once (Join-Path $tauri 'Cargo.toml') '(?s)\A(.*?^\[package\]\s*$.*?^version\s*=\s*)"[^"]*"' 'Cargo.toml [package]'
# Cargo.lock: záznam balíčku ledge, jinak `cargo test --locked` odmítne zastaralý lockfile
Set-Once (Join-Path $tauri 'Cargo.lock') '(^name = "ledge"\r?\nversion = )"[^"]*"' 'Cargo.lock ledge'
