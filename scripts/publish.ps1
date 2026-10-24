# Vydání Ledge do VEŘEJNÉHO repa (výchozí jiri-hyan/ledge-app) z vývojového (soukromého) repa.
#
# Veřejné repo nedostává vývojovou historii: každé vydání = JEDEN nový commit „snapshot“ stromu `main`
# (`git archive HEAD` — vynechá cesty s `export-ignore` v .gitattributes: interní poznámky, CI, tenhle
# seznam zakázaných slov), autor jiri-hyan s GitHub noreply e-mailem. Před commitem se strom i seznam
# změn prověří proti `scripts/private/denylist.txt` (jména klientů, strojů…) — jediný nález = konec.
# Pak lokální build (stejný tvar jako dřív) a GitHub Release vX.Y.<Z+1> (nebo -Version) ve veřejném repu s řádkem
# `SHA256 (<instalátor>) = <hex>`, který čte Ledge (update.rs `sha_for`).
#
# Podpis (kolo 21): instalátor se podepíše soukromým klíčem MIMO repo (`-SigningKey`, proměnná LEDGE_SIGNING_KEY,
# výchozí %USERPROFILE%\.ledge-signing\ledge-update.key; vytvoří ho jednou scripts/signing-setup.ps1) přes
# `tauri signer sign` → `<instalátor>.sig`, který Ledge ověří zapečeným veřejným klíčem (updsign.rs). Heslo ke klíči
# (bez člověka, nic se neptá): TAURI_SIGNING_PRIVATE_KEY_PASSWORD, jinak DPAPI soubor vedle klíče
# (`ledge-update.pass.dpapi`, `-PasswordFile`; vytvoří ho `signing-setup.ps1 -NonInteractive`). Heslo se nikdy
# nevypisuje; proměnnou dostane jen podproces podpisu a hned po něm zmizí. Bez klíče/hesla, s cizím klíčem nebo
# bez podpisu se nevydává.
# Authenticode (podpis Windows): na stroji není certifikát pro podepisování kódu → zatím ne (docs/podpis-vydani.md).
#
# Neměnná vydání (immutable releases na GitHubu): po publikaci už nejde nic nahrát ani přepsat. Proto vždy
# koncept (draft) → nahrát VŠECHNY soubory (instalátor, .sig, SHA256SUMS.txt, pro-claude.md) → zkontrolovat
# seznam → teprve pak publikovat. Koncept z nepovedeného běhu se při dalším běhu doplní (`--clobber`), publikované
# vydání se nikdy nepřepisuje (vydej vyšší verzi).
#
# Použití: powershell -NoProfile -File scripts/publish.ps1 [-Public jiri-hyan/ledge-app]
#          [-Private jiri-hyan/ledge] [-AlsoPrivate] [-Clone <složka klonu veřejného repa>] [-DryRun] [-Version x.y.z]
#   -AlsoPrivate  stejný instalátor vydá i v soukromém repu (přechod: Ledge ≤ 0.2.8 hledá aktualizace tam)
#   -DryRun       jen snapshot do klonu + kontrola denylistu, nic necommituje, nebuilduje ani nevydá
# Podmínky: větev main, čistý strom, HEAD pushnutý do soukromého repa.
param(
  [string]$Public = 'jiri-hyan/ledge-app',
  [string]$Private = 'jiri-hyan/ledge',
  [switch]$AlsoPrivate,
  [string]$Clone = (Join-Path $env:LOCALAPPDATA 'ledge-public'),
  [switch]$DryRun,
  [string]$Version,
  [string]$SigningKey = $(if ($env:LEDGE_SIGNING_KEY) { $env:LEDGE_SIGNING_KEY } else { Join-Path $env:USERPROFILE '.ledge-signing\ledge-update.key' }),
  [string]$PasswordFile
)
# 'Continue': git/gh píšou průběh na stderr, PowerShell 5.1 by to pod 'Stop' bral jako chybu —
# návratové kódy se kontrolují výslovně ($LASTEXITCODE) a chyby končí `exit 1`
$ErrorActionPreference = 'Continue'
# výstup gitu / gh je UTF-8 už od začátku (hlášky denylistu s diakritikou); skript sám zůstává UTF-8 s BOM kvůli PS 5.1
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$root = Split-Path -Parent $PSScriptRoot
# build nesmí dusit ostatní práci na stroji — potomci (pnpm, cargo, rustc) prioritu zdědí
(Get-Process -Id $PID).PriorityClass = 'BelowNormal'

function Fail([string]$msg) { Write-Error $msg; exit 1 }

# Soukromý klíč existuje, je mimo repo a jeho veřejná půlka je ta zapečená v src-tauri/signing/update.pub
# (jinak by vydání nešlo v Ledge ověřit). Kontroluje se PŘED snapshotem a buildem — nic se nezačne napůl.
function Assert-SigningKey {
  $setupHint = 'jednou spusť: powershell -NoProfile -File scripts/signing-setup.ps1 (nebo nastav LEDGE_SIGNING_KEY / -SigningKey)'
  if (-not (Test-Path -LiteralPath $SigningKey -PathType Leaf)) { Fail "chybí soukromý klíč pro podpis vydání: $SigningKey — $setupHint" }
  $full = [IO.Path]::GetFullPath($SigningKey)
  if ($full.StartsWith([IO.Path]::GetFullPath($root) + '\', [StringComparison]::OrdinalIgnoreCase)) { Fail "soukromý klíč $full leží v repu — přesuň ho mimo (např. %USERPROFILE%\.ledge-signing)" }
  $pubFile = "$full.pub"
  if (-not (Test-Path -LiteralPath $pubFile)) { Fail "chybí $pubFile (veřejná půlka klíče ze signing-setup.ps1)" }
  $embedded = Get-Content (Join-Path $root 'src-tauri\signing\update.pub') -Raw -Encoding UTF8
  if (-not $embedded -or -not $embedded.Trim()) { Fail "src-tauri/signing/update.pub je prázdný — Ledge by podpis neověřil; $setupHint a commitni update.pub" }
  if ($embedded.Trim() -ne (Get-Content -LiteralPath $pubFile -Raw -Encoding UTF8).Trim()) { Fail "src-tauri/signing/update.pub nepatří ke klíči $full — vydání by v Ledge neprošlo ověřením" }
  # heslo: z proměnné (a hned pryč z prostředí — build ani gh ho nevidí), jinak DPAPI soubor; drží se jen jako SecureString
  if ($env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) {
    $script:signPw = ConvertTo-SecureString -String $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -AsPlainText -Force
  } else {
    $pf = if ($PasswordFile) { $PasswordFile } else { $full -replace '\.key$', '.pass.dpapi' }
    if (-not (Test-Path -LiteralPath $pf -PathType Leaf)) { Fail "chybí heslo ke klíči: ani TAURI_SIGNING_PRIVATE_KEY_PASSWORD, ani DPAPI soubor $pf — $setupHint" }
    try { $script:signPw = Get-Content -LiteralPath $pf -Raw -Encoding ASCII | ForEach-Object { $_.Trim() } | ConvertTo-SecureString -ErrorAction Stop }
    catch { Fail "DPAPI soubor $pf nejde odemknout (je vázaný na účet a stroj, kde vznikl) — heslo obnov ze zálohy do TAURI_SIGNING_PRIVATE_KEY_PASSWORD" }
  }
  Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
}

# Podpis souboru: heslo jen v prostředí podprocesu podpisu a hned po něm pryč (i při chybě); výstup tauri hesla neobsahuje.
function Invoke-Sign([string]$file) {
  $bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($script:signPw)
  try {
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr)
    & node (Join-Path $root 'node_modules\@tauri-apps\cli\tauri.js') signer sign -f $SigningKey --app-version $v $file | Out-Host  # výstup do konzole, ne do návratové hodnoty
    return $LASTEXITCODE
  } finally {
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
    [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr)
  }
}

# Neměnné vydání: koncept → všechny soubory → kontrola seznamu → publikovat. Po publikaci se nic nenahrává.
function Publish-Release([string]$repo, [string]$target) {
  $tag = "v$v"
  $isDraft = gh release view $tag --repo $repo --json isDraft --jq .isDraft 2>$null
  if ($LASTEXITCODE -eq 0) {
    if ("$isDraft".Trim() -ne 'true') { Fail "vydání $tag v $repo už je publikované — neměnné vydání se nepřepisuje; vydej vyšší verzi (-Version)" }
    "koncept:    $tag v $repo z dřívějšího běhu — doplňuji soubory"
    gh release edit $tag --repo $repo --title "Ledge $v" --notes-file $notesFile --target $target
    if ($LASTEXITCODE -ne 0) { Fail "úprava konceptu $tag ($repo) selhala" }
    gh release upload $tag @assets --repo $repo --clobber
  } else {
    gh release create $tag @assets --repo $repo --draft --title "Ledge $v" --notes-file $notesFile --target $target
  }
  if ($LASTEXITCODE -ne 0) { Fail "nahrání konceptu $tag do $repo selhalo — koncept zůstal neveřejný, spusť publish.ps1 znovu" }
  $want = @($assets | ForEach-Object { Split-Path -Leaf $_ } | Sort-Object)
  # bez --jq s uvozovkami: Windows PowerShell 5.1 vnitřní " u nativních příkazů zahodí (jq pak hlásí „uploaded/0“)
  $json = gh release view $tag --repo $repo --json assets
  $viewOk = $LASTEXITCODE -eq 0
  $got = @(($json | ConvertFrom-Json).assets | Where-Object { $_.state -eq 'uploaded' } | ForEach-Object { $_.name } | Sort-Object)
  if (-not $viewOk -or ($want -join '|') -ne ($got -join '|')) { Fail "koncept $tag v $repo nemá přesně soubory $($want -join ', ') (má: $($got -join ', ')) — nepublikuji" }
  gh release edit $tag --repo $repo --draft=false
  if ($LASTEXITCODE -ne 0) { Fail "publikace konceptu $tag ($repo) selhala — koncept je úplný: gh release edit $tag --repo $repo --draft=false" }
}

# Řádky denylistu = regulární výrazy (bez ohledu na velikost písmen); # = komentář.
function Get-Denylist {
  $f = Join-Path $root 'scripts\private\denylist.txt'
  if (-not (Test-Path $f)) { Fail "chybí $f — bez něj se veřejně nevydává" }
  @(Get-Content $f -Encoding UTF8 | ForEach-Object { $_.Trim() } | Where-Object { $_ -and -not $_.StartsWith('#') })
}

function Find-Denied([string]$dir, [string[]]$deny) {
  $hits = @()
  Get-ChildItem -Path $dir -Recurse -File -Force | Where-Object { $_.FullName -notmatch '\\\.git\\' } | ForEach-Object {
    $rel = $_.FullName.Substring($dir.Length + 1)
    foreach ($d in $deny) { if ($rel -match "(?i)$d") { $hits += "jméno souboru: $rel ~ $d" } }
    if ($_.Extension -notin '.png', '.ico', '.icns', '.jpg', '.webp', '.bmp') {
      $text = [IO.File]::ReadAllText($_.FullName)
      foreach ($d in $deny) { if ($text -match "(?i)$d") { $hits += "$rel ~ $d" } }
    }
  }
  $hits
}

Push-Location $root
try {
  $branch = git rev-parse --abbrev-ref HEAD
  if ($branch -ne 'main') { Fail "jsem na větvi '$branch', vydává se jen main" }
  if (git status --porcelain) { Fail 'strom není čistý — commitni nebo odlož změny' }
  git fetch -q origin main
  $head = git rev-parse HEAD
  if ($head -ne (git rev-parse origin/main)) { Fail 'HEAD není shodný s origin/main — nejdřív push/pull' }
  $deny = Get-Denylist
  if (-not $DryRun) { Assert-SigningKey }

  # další verze = nejvyšší vydaná vX.Y.Z v OBOU repech, patch + 1 (jedna řada verzí; Ledge porovnává čísla);
  # -Version x.y.z vydá zadanou verzi (musí být vyšší než poslední vydaná) — např. skok na 0.3.0
  $last = [version]'0.0.0'
  foreach ($r in @($Public, $Private)) {
    # koncepty (nepovedený běh) se nepočítají — další běh tak vydá stejnou verzi a koncept doplní
    $tags = @(gh release list --repo $r --limit 200 --json tagName,isDraft --jq '.[] | select(.isDraft | not) | .tagName' 2>$null)
    foreach ($tg in $tags) { if ($tg -match '^v(\d+\.\d+\.\d+)$' -and [version]$Matches[1] -gt $last) { $last = [version]$Matches[1] } }
  }
  if ($Version) {
    if ($Version -notmatch '^\d+\.\d+\.\d+$') { Fail "-Version '$Version' není x.y.z" }
    if ([version]$Version -le $last) { Fail "-Version $Version není vyšší než poslední vydaná $last" }
    $v = $Version
  } else { $v = "$($last.Major).$($last.Minor).$($last.Build + 1)" }
  "verze:      $v (poslední vydaná: $last)"

  # ---- snapshot do klonu veřejného repa ----
  if (-not (Test-Path (Join-Path $Clone '.git'))) {
    gh repo clone $Public $Clone -- -q
    if ($LASTEXITCODE -ne 0) { Fail "klon $Public selhal" }
  }
  git -C $Clone fetch -q origin 2>$null
  $hasMain = git -C $Clone ls-remote --heads origin main
  if ($hasMain) { git -C $Clone checkout -q -B main origin/main } else { git -C $Clone checkout -q --orphan main 2>$null }
  # celý strom klonu (kromě .git) pryč a nahradit archivem HEAD — smazané soubory tak zmizí i veřejně
  Get-ChildItem -Path $Clone -Force | Where-Object { $_.Name -ne '.git' } | Remove-Item -Recurse -Force
  $tar = Join-Path $env:TEMP "ledge-snapshot-$v.tar"
  git archive --format=tar -o $tar HEAD
  if ($LASTEXITCODE -ne 0) { Fail 'git archive selhal' }
  # tar z Windows (Git Bash v PATH by podstrčil GNU tar, který `C:\…` bere jako vzdálený stroj)
  & (Join-Path $env:SystemRoot 'System32/tar.exe') -xf $tar -C $Clone
  if ($LASTEXITCODE -ne 0) { Fail 'rozbalení snapshotu selhalo' }
  Remove-Item $tar

  $hits = @(Find-Denied $Clone $deny)
  $prevTag = "v$last"
  git rev-parse -q --verify "refs/tags/$prevTag" *> $null
  # seznam změn jen z commitů, které mění něco, co jde ven (ne čistě interní export-ignore cesty)
  $pubPaths = @('--', '.', ':(exclude)scripts/private', ':(exclude).github')
  # git píše UTF-8; bez tohohle ho PS 5.1 dekóduje jako OEM 852 a poznámky vydání mají rozbitou češtinu
  [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
  $log =if ($LASTEXITCODE -eq 0) { @(git log --no-decorate --format='%H - %s' "$prevTag..HEAD" @pubPaths) } else { @(git log --no-decorate -n 30 --format='%H - %s' @pubPaths) }
  # commity vynechané z poznámek vydání (plný SHA na řádek, # = komentář) — pro zprávy, které by jinak
  # zastavily denylist a pushnutou historii nepřepisujeme; snapshot stromu se kontroluje dál celý
  $skipFile = Join-Path $root 'scripts\private\notes-skip.txt'
  $skip = if (Test-Path $skipFile) { @(Get-Content $skipFile -Encoding UTF8 | ForEach-Object { ($_ -split '#')[0].Trim() } | Where-Object { $_ }) } else { @() }
  $log = @($log | Where-Object { $skip -notcontains $_.Substring(0, 40) } | ForEach-Object { $_.Substring(41) })
  foreach ($line in $log) { foreach ($d in $deny) { if ($line -match "(?i)$d") { $hits += "seznam změn: $line ~ $d" } } }
  if ($hits.Count) { $hits | ForEach-Object { "  ZAKÁZÁNO: $_" }; Fail "snapshot obsahuje zakázaná slova ($($hits.Count)) — oprav na main a vydej znovu" }
  "snapshot:   $Clone čistý (denylist $($deny.Count) vzorů)"
  if ($DryRun) { 'DryRun: konec bez commitu, buildu a vydání'; exit 0 }

  # autor i committer = jiri-hyan s noreply e-mailem GitHubu (žádná osobní ani pracovní adresa)
  $uid = gh api user --jq .id
  $who = @('-c', 'user.name=jiri-hyan', '-c', "user.email=$uid+jiri-hyan@users.noreply.github.com")
  git -C $Clone add -A
  # opakovaný běh po selhání vydání: snapshot už je commitnutý a strom se nezměnil → použije se HEAD
  git -C $Clone diff --cached --quiet HEAD 2>$null
  if ($LASTEXITCODE -eq 0) { "snapshot:   beze změny proti veřejnému main — beru existující commit" }
  else {
    git -C $Clone @who commit -q -m "Ledge $v" -m "Snapshot vývojového stromu (bez historie)."
    if ($LASTEXITCODE -ne 0) { Fail 'commit snapshotu selhal' }
  }
  $pubHead = git -C $Clone rev-parse HEAD

  # ---- build (verze se zapíše jen dočasně, necommituje se) ----
  $verFiles = 'src-tauri/tauri.conf.json', 'package.json', 'src-tauri/Cargo.toml', 'src-tauri/Cargo.lock'
  try {
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'ci-version.ps1') -Version $v -Root $root  # v PS 5.1 je $PSScriptRoot ve výchozí hodnotě param prázdný
    if ($LASTEXITCODE -ne 0) { Fail 'ci-version.ps1 selhal' }
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'release.ps1')
    if ($LASTEXITCODE -ne 0) { Fail 'release.ps1 (build) selhal' }
  } finally {
    git checkout -q -- @verFiles
  }
  $targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root 'src-tauri\target' }
  $setup = Get-ChildItem -Path (Join-Path $targetDir 'release\bundle\nsis') -Filter "*_$($v)_*-setup.exe" | Select-Object -First 1
  if (-not $setup) { Fail "nenašel jsem instalátor pro $v" }
  $sha = (Get-FileHash -Algorithm SHA256 -Path $setup.FullName).Hash.ToLowerInvariant()

  # ---- podpis (minisign přes tauri signer; soubor `<instalátor>.sig`, ověřuje updsign.rs) ----
  $sigPath = "$($setup.FullName).sig"
  Remove-Item -LiteralPath $sigPath -ErrorAction SilentlyContinue
  Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue  # -f a -k se vylučují
  $signExit = Invoke-Sign $setup.FullName
  if ($signExit -ne 0 -or -not (Test-Path -LiteralPath $sigPath)) { Fail 'podpis instalátoru selhal (špatné heslo ke klíči? DPAPI soubor / TAURI_SIGNING_PRIVATE_KEY_PASSWORD) — nic se nevydalo' }
  # důvěryhodný komentář musí jmenovat tenhle soubor a verzi — jinak by ho Ledge odmítl (updsign::comment_names)
  $sigText = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String((Get-Content -LiteralPath $sigPath -Raw).Trim()))
  if ($sigText -notmatch "(?m)^trusted comment: .*\tfile:$([regex]::Escape($setup.Name))\tversion:$([regex]::Escape($v))\s*$") { Fail "podpis $sigPath nejmenuje $($setup.Name) / verzi $v — nevydávám" }
  "podpis:     $sigPath"

  # řádek SHA256 čte Ledge (update.rs `sha_for`) — tvar neměnit
  if ($log.Count -eq 0) { $log = @('- (žádné nové změny)') }
  # sekce „Nastav s Claudem“ (kolo 9): odkaz na návod pro Clauda + zadání, které stačí vložit do Clauda.
  # Řádky nesmí začínat „SHA256“ (update::notes_of takové řádky z poznámek v UI vypouští)
  $guideUrl = "https://github.com/$Public/releases/latest/download/pro-claude.md"
  $claudeSection = @(
    '', '## Nastav s Claudem / Set up with Claude', '',
    'Pošli Claudovi odkaz na návod a on Ledge nainstaluje a přes MCP nastaví podle toho, co používáš. Připojení smí zapnout jen člověk (první průchod nebo Nastavení → Claude (MCP)) a jde kdykoli vypnout.',
    'Send Claude the link to the guide: it installs Ledge and sets it up over MCP for the way you work. Only you can turn the connection on (setup guide or Settings → Claude (MCP)), and you can turn it off any time.',
    '', '```', "Nainstaluj a nastav mi Ledge podle $guideUrl", '```',
    '', "Návod pro Clauda / Guide for Claude: [pro-claude.md]($guideUrl). Kontrolní součty instalátoru jsou v souboru ``SHA256SUMS.txt`` (checksums in ``SHA256SUMS.txt``)."
  )
  $notes = @("SHA256 ($($setup.Name)) = $sha", '', 'Změny:', '') + $log + $claudeSection
  $notesFile = Join-Path $env:TEMP "ledge-release-notes-$v.md"
  [IO.File]::WriteAllLines($notesFile, [string[]]$notes, (New-Object System.Text.UTF8Encoding($false)))
  # přílohy vedle instalátoru: SHA256SUMS.txt (formát `sha256sum`: <hex>  <soubor>) a návod pro Clauda z docs/
  $assetDir = Join-Path $env:TEMP "ledge-release-assets-$v"
  New-Item -ItemType Directory -Force $assetDir | Out-Null
  $sumsFile = Join-Path $assetDir 'SHA256SUMS.txt'
  # jen instalátor: návod pro Clauda (docs/pro-claude.md) hledá řádek podle jména a `.sig` by se mu přimíchal
  [IO.File]::WriteAllText($sumsFile, "$sha  $($setup.Name)`n", (New-Object System.Text.UTF8Encoding($false)))
  $guideSrc = Join-Path $root 'docs\pro-claude.md'
  if (-not (Test-Path $guideSrc)) { Fail 'chybí docs/pro-claude.md — návod pro Clauda patří ke každému vydání' }
  $guideFile = Join-Path $assetDir 'pro-claude.md'
  Copy-Item $guideSrc $guideFile -Force
  $assets = @($setup.FullName, $sigPath, $sumsFile, $guideFile)

  git -C $Clone push -q origin main
  if ($LASTEXITCODE -ne 0) { Fail 'push snapshotu do veřejného repa selhal' }
  Publish-Release $Public $pubHead
  if ($AlsoPrivate) { Publish-Release $Private $head }
  if (-not $AlsoPrivate) {
    # tag i ve vývojovém repu — příští vydání z něj počítá seznam změn
    git tag "v$v" $head
    git push -q origin "v$v"
  }
  git fetch -q --tags origin
  "vydáno:     v$v · $($setup.Name) · SHA256 $sha · https://github.com/$Public/releases/tag/v$v"
} finally {
  Pop-Location
}
