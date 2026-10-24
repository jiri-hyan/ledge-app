# Záloha klíče pro podpis vydání Ledge — pro šéfa, ručně v okně konzole (ne z automatizace).
#
# Záloha = soukromý klíč + heslo. DPAPI soubor s heslem (`ledge-update.pass.dpapi`) jde odemknout jen tímto
# účtem na tomto stroji, takže sám zálohou NENÍ: heslo je potřeba mít i jinde (správce hesel).
#
# - Zkopíruje `ledge-update.key` + `.pub` do `-Dest` (USB, šifrovaný disk). Existující soubory nepřepíše.
# - Heslo (z DPAPI souboru) volitelně:
#     -ShowPassword        ukáže ho JEDNOU v tomhle okně po výslovném potvrzení; po Enteru se obrazovka smaže.
#                          Jen v interaktivní konzoli bez přesměrování výstupu (nepatří do logu).
#     -PasswordOut <soubor> uloží ho do souboru (mimo repo, nepřepisuje, práva jen pro tvůj účet). POZOR: je to
#                          PROSTÝ TEXT — vlož ho do správce hesel a soubor hned smaž (Shift+Delete).
#
# Použití: powershell -NoProfile -File scripts/signing-backup.ps1 -Dest E:\ledge-zaloha [-ShowPassword | -PasswordOut <soubor>]
#          [-Dir <složka klíče, výchozí %USERPROFILE%\.ledge-signing>]
param(
  [Parameter(Mandatory = $true)][string]$Dest,
  [string]$Dir = (Join-Path $env:USERPROFILE '.ledge-signing'),
  [switch]$ShowPassword,
  [string]$PasswordOut
)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$root = Split-Path -Parent $PSScriptRoot

function Fail([string]$msg) { Write-Error $msg; exit 1 }
function Test-InRepo([string]$p) { ([IO.Path]::GetFullPath($p) + '\').StartsWith([IO.Path]::GetFullPath($root) + '\', [StringComparison]::OrdinalIgnoreCase) }

if ($ShowPassword -and $PasswordOut) { Fail 'zvol buď -ShowPassword, nebo -PasswordOut' }
$key = Join-Path $Dir 'ledge-update.key'
$passFile = Join-Path $Dir 'ledge-update.pass.dpapi'
if (-not (Test-Path -LiteralPath $key)) { Fail "klíč $key neexistuje — nejdřív scripts/signing-setup.ps1" }
if (Test-InRepo $Dest) { Fail "záloha nesmí ležet v repu ($Dest)" }

# ---- klíč ----
New-Item -ItemType Directory -Force $Dest | Out-Null
foreach ($f in @($key, "$key.pub")) {
  $to = Join-Path $Dest (Split-Path -Leaf $f)
  if (Test-Path -LiteralPath $to) { Fail "$to už existuje — nepřepisuji (zvol jinou složku)" }
  Copy-Item -LiteralPath $f -Destination $to
  if ((Get-FileHash -LiteralPath $f).Hash -ne (Get-FileHash -LiteralPath $to).Hash) { Fail "kopie $to nesedí s originálem" }
  "zkopírováno: $to"
}

if (-not $ShowPassword -and -not $PasswordOut) {
  ''
  'Heslo se nezálohovalo. Bez hesla je klíč k ničemu — spusť znovu s -ShowPassword nebo -PasswordOut <soubor>'
  '(nebo ho máš ze signing-setup.ps1 bez -NonInteractive v hlavě / ve správci hesel).'
  exit 0
}

# ---- heslo (DPAPI → jen do okna nebo do souboru, nikdy do výstupu skriptu) ----
if (-not (Test-Path -LiteralPath $passFile)) { Fail "DPAPI soubor $passFile neexistuje (klíč nevznikl s -NonInteractive) — heslo zálohuj ze svého správce hesel" }
try { $secure = Get-Content -LiteralPath $passFile -Raw -Encoding ASCII | ForEach-Object { $_.Trim() } | ConvertTo-SecureString -ErrorAction Stop }
catch { Fail "DPAPI soubor $passFile nejde odemknout (jiný účet nebo stroj, než kde vznikl)" }

$bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
try {
  if ($PasswordOut) {
    if (Test-InRepo $PasswordOut) { Fail "soubor s heslem nesmí ležet v repu ($PasswordOut)" }
    if (Test-Path -LiteralPath $PasswordOut) { Fail "$PasswordOut už existuje — nepřepisuji" }
    $outFull = [IO.Path]::GetFullPath($PasswordOut)
    # nejdřív prázdný soubor s právy jen pro aktuálního uživatele, teprve pak do něj heslo zapiš
    [IO.File]::WriteAllText($outFull, '', (New-Object System.Text.UTF8Encoding($false)))
    & icacls $outFull /inheritance:r /grant:r "$($env:USERDOMAIN)\$($env:USERNAME):F" | Out-Null
    if ($LASTEXITCODE -ne 0) { Remove-Item -LiteralPath $outFull -Force; Fail "icacls nenastavil práva souboru $outFull — heslo do něj nezapisuji" }
    [IO.File]::WriteAllText($outFull, [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr), (New-Object System.Text.UTF8Encoding($false)))
    "heslo uloženo: $outFull (jen pro tvůj účet) — je to PROSTÝ TEXT: vlož ho do správce hesel a soubor ihned smaž (Shift+Delete)"
  } else {
    if (-not [Environment]::UserInteractive -or [Console]::IsOutputRedirected -or [Console]::IsInputRedirected) {
      Fail 'heslo ukážu jen v interaktivní konzoli bez přesměrování (aby neskončilo v logu) — použij -PasswordOut'
    }
    $answer = Read-Host 'Heslo se teď ukáže na obrazovce. Nikdo se nedívá? Napiš ZOBRAZIT'
    if ($answer -cne 'ZOBRAZIT') { Fail 'nezobrazeno' }
    [Console]::WriteLine('')
    [Console]::WriteLine('Heslo ke klíči (přepiš do správce hesel):')
    [Console]::WriteLine([Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr))
    [Console]::WriteLine('')
    [void](Read-Host 'Enter = smazat obrazovku')
    Clear-Host
    'heslo ukázáno a obrazovka smazána'
  }
} finally {
  [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr)
}
