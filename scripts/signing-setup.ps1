# Jednorázově: klíč pro podpis vydání Ledge (minisign / Ed25519 přes `tauri signer generate`).
#
# - Soukromý klíč vznikne MIMO repo: výchozí %USERPROFILE%\.ledge-signing\ledge-update.key (+ .pub vedle).
#   Složka dostane práva jen pro aktuálního uživatele.
# - `-NonInteractive` (pro vydávání bez člověka): silné náhodné heslo (CSPRNG, 256 bitů), klíč bez dotazů
#   a heslo uložené přes DPAPI (CurrentUser) do `ledge-update.pass.dpapi` vedle klíče — `publish.ps1` si ho
#   načte sám. Heslo se nikde nevypisuje; zálohu klíče + hesla udělá šéf přes scripts/signing-backup.ps1.
#   Pozor: `tauri signer generate` bere heslo jen jako argument (-p), takže je po dobu generování (sekundy)
#   vidět v příkazové řádce procesu pro tentýž účet.
# - Bez přepínače (interaktivně): heslo zadáš 2× při výzvě tauri, DPAPI soubor nevznikne — publish.ps1 pak
#   chce heslo v TAURI_SIGNING_PRIVATE_KEY_PASSWORD.
# - Veřejný klíč se zkopíruje do src-tauri/signing/update.pub — ten commitni; zapeče se do binárky a Ledge
#   jím ověřuje každé stažené vydání (src-tauri/src/updsign.rs).
# - Existující klíč ani heslo nikdy nepřepíše (ztráta klíče = viz docs/podpis-vydani.md). Nic neposílá do sítě.
#
# - Když je src-tauri/signing/update.pub v repu už NEPRÁZDNÝ, skript odmítne běžet (jiný stroj, účet nebo
#   složka by klíč přepsala a nainstalované kopie by přestaly přijímat aktualizace). Výměna klíče jen
#   s výslovným -Rotate.
#
# Použití: powershell -NoProfile -File scripts/signing-setup.ps1 [-NonInteractive] [-Dir <složka mimo repo>] [-Rotate]
param(
  [string]$Dir = (Join-Path $env:USERPROFILE '.ledge-signing'),
  [switch]$NonInteractive,
  [switch]$Rotate
)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$root = Split-Path -Parent $PSScriptRoot

function Fail([string]$msg) { Write-Error $msg; exit 1 }

$full = [IO.Path]::GetFullPath($Dir)
if (($full + '\').StartsWith([IO.Path]::GetFullPath($root) + '\', [StringComparison]::OrdinalIgnoreCase)) { Fail "klíč nesmí ležet v repu ($full) — zvol složku mimo něj" }
$destPub = Join-Path $root 'src-tauri\signing\update.pub'
$embedded = if (Test-Path -LiteralPath $destPub) { Get-Content -LiteralPath $destPub -Raw -Encoding UTF8 } else { '' }
if ($embedded -and $embedded.Trim()) {
  if (-not $Rotate) {
    Fail "src-tauri/signing/update.pub už obsahuje veřejný klíč — nový nevytvářím (přepsal by ho a nainstalované Ledge by přestaly přijímat aktualizace). Klíč patří na stroji vydavatele do %USERPROFILE%\.ledge-signing; na jiném stroji ho obnov ze zálohy (docs/podpis-vydani.md). Výměna klíče vědomě: -Rotate."
  }
  Write-Warning 'ROTACE KLÍČE: po vydání s novým klíčem nainstalované verze Ledge aktualizaci ODMÍTNOU. Uživatel (i ty) musí první vydání s novým klíčem nainstalovat RUČNĚ ze stránky vydání (docs/podpis-vydani.md).'
}
$key = Join-Path $full 'ledge-update.key'
$passFile = Join-Path $full 'ledge-update.pass.dpapi'
foreach ($f in @($key, "$key.pub", $passFile)) {
  if (Test-Path -LiteralPath $f) { Fail "$f už existuje — nepřepisuji. Výměna klíče: docs/podpis-vydani.md (Ztráta nebo vyzrazení klíče)." }
}
$tauriJs = Join-Path $root 'node_modules\@tauri-apps\cli\tauri.js'
if (-not (Test-Path -LiteralPath $tauriJs)) { Fail "chybí $tauriJs — nejdřív corepack pnpm install" }

New-Item -ItemType Directory -Force $full | Out-Null
# práva jen pro aktuálního uživatele (bez dědění z profilu); soubory uvnitř je zdědí
& icacls $full /inheritance:r /grant:r "$($env:USERDOMAIN)\$($env:USERNAME):(OI)(CI)F" | Out-Null
if ($LASTEXITCODE -ne 0) {
  if ($NonInteractive) { Fail "icacls nenastavil práva složky $full — heslo do ní neukládám" }
  Write-Warning "icacls nenastavil práva složky $full — zkontroluj je ručně"
}

if ($NonInteractive) {
  # 32 bajtů z CSPRNG → base64url (43 znaků, bez znaků, které by bylo nutné escapovat)
  $bytes = New-Object byte[] 32
  $rng = [Security.Cryptography.RandomNumberGenerator]::Create()
  try { $rng.GetBytes($bytes) } finally { $rng.Dispose() }
  $pw = [Convert]::ToBase64String($bytes).TrimEnd('=').Replace('+', '-').Replace('/', '_')
  [Array]::Clear($bytes, 0, $bytes.Length)
  try {
    # výstup tauri heslo neobsahuje (jen cesty a návod); s -p se nic neptá
    & node $tauriJs signer generate -w $key -p $pw | Out-Host
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath "$key.pub")) { Fail 'generování klíče selhalo — nic se nezměnilo v repu' }
    # DPAPI CurrentUser: odemkne jen tentýž účet na tomtéž stroji
    $blob = ConvertTo-SecureString -String $pw -AsPlainText -Force | ConvertFrom-SecureString
    [IO.File]::WriteAllText($passFile, $blob, (New-Object System.Text.ASCIIEncoding))
  } finally {
    $pw = $null
  }
} else {
  # bez CI=… se tauri zeptá na heslo (s CI by vytvořil klíč bez hesla)
  Remove-Item Env:CI -ErrorAction SilentlyContinue
  & node $tauriJs signer generate -w $key
  if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath "$key.pub")) { Fail 'generování klíče selhalo — nic se nezměnilo v repu' }
}

$pub = (Get-Content -LiteralPath "$key.pub" -Raw -Encoding UTF8).Trim()
$dest = $destPub
[IO.File]::WriteAllText($dest, "$pub`n", (New-Object System.Text.UTF8Encoding($false)))

$lines = @(
  '',
  "hotovo:     soukromý klíč $key",
  "            veřejný klíč $key.pub → $dest"
)
if ($NonInteractive) { $lines += "            heslo (DPAPI, jen tento účet a stroj) $passFile" }
$lines += @(
  '',
  'Další kroky:',
  '  1. Commitni veřejný klíč:  git add src-tauri/signing/update.pub; git commit -m "feat(update): veřejný klíč podpisu vydání"',
  '  2. Zálohu klíče + hesla udělá šéf: powershell -NoProfile -File scripts/signing-backup.ps1 -Dest <USB složka>',
  '     (DPAPI soubor sám zálohou NENÍ — jde odemknout jen tímto účtem na tomto stroji).'
)
if ($NonInteractive) { $lines += '  3. Vydávej jako dřív: publish.ps1 si heslo načte z DPAPI souboru, nic se neptá.' }
else { $lines += '  3. Vydávej s heslem v $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD (jen pro tu jednu konzoli).' }
$lines | ForEach-Object { $_ }
