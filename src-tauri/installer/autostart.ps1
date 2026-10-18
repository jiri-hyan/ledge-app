# Instalátor Ledge (hooks.nsh): přečte / nastaví `autostart` v %APPDATA%\Ledge\config.json, aby
# zaškrtávátko „Spouštět s Windows“ a přepínač v Nastavení Ledge říkaly totéž. Pravdou je config:
# Ledge při každém startu (`autostart::apply`) podle něj zapíše nebo smaže hodnotu Run, takže instalátor
# musí změnit config, jinak by ho appka při dalším startu přepsala zpátky. Hodnotu Run zapisuje hooks.nsh.
#
#   -Mode get  → exit 0 = zapnuto (i bez configu: výchozí je zapnuto), exit 1 = config říká vypnuto
#   -Mode on   → `"autostart": false` v configu přepne na true (bez configu nic — výchozí je zapnuto)
#   -Mode off  → `"autostart"` na false; bez configu založí čerstvý `{"autostart": false, "onboarding":
#                {"done": false}}` (= config nového uživatele, průvodce prvním spuštěním proběhne)
# Exit 2 = config nejde bezpečně upravit (nečitelný JSON …) → nic se nezapsalo. Exit 3 = configu Ledge
# není, ale je starý %APPDATA%\Notch\config.json — ten při prvním startu převezme migrace (`migrate.rs`),
# založený config by ji zablokoval, takže se nic nezakládá.
# Soubor se přepisuje jen změnou té jedné hodnoty (formát, pořadí a ostatní klíče zůstanou), UTF-8 bez BOM.
param(
  [Parameter(Mandatory)][ValidateSet('get', 'on', 'off')][string]$Mode,
  [string]$Dir = (Join-Path $env:APPDATA 'Ledge'),
  [string]$LegacyDir = (Join-Path $env:APPDATA 'Notch')
)
$ErrorActionPreference = 'Stop'
$cfg = Join-Path $Dir 'config.json'

function Read-Autostart([string]$text) {
  $v = $text.TrimStart([char]0xFEFF) | ConvertFrom-Json
  if ($null -eq $v -or $v -isnot [pscustomobject]) { throw 'config není objekt' }
  if ($v.PSObject.Properties.Name -contains 'autostart') { return [bool]$v.autostart }
  return $true
}

if (-not (Test-Path -LiteralPath $cfg -PathType Leaf)) {
  if ($Mode -ne 'off') { exit 0 }
  if (Test-Path -LiteralPath (Join-Path $LegacyDir 'config.json') -PathType Leaf) { exit 3 }
  New-Item -ItemType Directory -Force -Path $Dir | Out-Null
  [IO.File]::WriteAllText($cfg, "{`n  `"autostart`": false,`n  `"onboarding`": {`n    `"done`": false`n  }`n}`n", (New-Object Text.UTF8Encoding($false)))
  exit 0
}

try {
  $text = [IO.File]::ReadAllText($cfg)
  $now = Read-Autostart $text
} catch {
  if ($Mode -eq 'get') { exit 0 }  # nečitelný config: Ledge jede s výchozím (zapnuto)
  exit 2
}
if ($Mode -eq 'get') { if ($now) { exit 0 } else { exit 1 } }

$want = $Mode -eq 'on'
if ($now -eq $want) { exit 0 }
$lit = if ($want) { 'true' } else { 'false' }
$re = [regex]'"autostart"\s*:\s*(true|false)'
if ($re.IsMatch($text)) {
  $new = $re.Replace($text, ('"autostart": ' + $lit), 1)
} elseif ($text -match '^\s*\{\s*\}\s*$') {
  $new = "{`n  `"autostart`": $lit`n}`n"
} else {
  # klíč chybí (= výchozí zapnuto) → vložit hned za úvodní `{`
  $new = ([regex]'\{').Replace($text, "{`n  `"autostart`": $lit,", 1)
}
# pojistka: první shoda musí být klíč nejvyšší úrovně a výsledek platný JSON
try { if ((Read-Autostart $new) -ne $want) { exit 2 } } catch { exit 2 }
[IO.File]::WriteAllText($cfg, $new, (New-Object Text.UTF8Encoding($false)))
exit 0
