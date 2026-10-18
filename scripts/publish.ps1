# Vydání Ledge do VEŘEJNÉHO repa (výchozí jiri-hyan/ledge-app) z vývojového (soukromého) repa.
#
# Veřejné repo nedostává vývojovou historii: každé vydání = JEDEN nový commit „snapshot“ stromu `main`
# (`git archive HEAD` — vynechá cesty s `export-ignore` v .gitattributes: interní poznámky, CI, tenhle
# seznam zakázaných slov), autor jiri-hyan s GitHub noreply e-mailem. Před commitem se strom i seznam
# změn prověří proti `scripts/private/denylist.txt` (jména klientů, strojů…) — jediný nález = konec.
# Pak lokální build (stejný tvar jako dřív) a GitHub Release vX.Y.<Z+1> (nebo -Version) ve veřejném repu s řádkem
# `SHA256 (<instalátor>) = <hex>`, který čte Ledge (update.rs `sha_for`).
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
  [string]$Version
)
# 'Continue': git/gh píšou průběh na stderr, PowerShell 5.1 by to pod 'Stop' bral jako chybu —
# návratové kódy se kontrolují výslovně ($LASTEXITCODE) a chyby končí `exit 1`
$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
# build nesmí dusit ostatní práci na stroji — potomci (pnpm, cargo, rustc) prioritu zdědí
(Get-Process -Id $PID).PriorityClass = 'BelowNormal'

function Fail([string]$msg) { Write-Error $msg; exit 1 }

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

  # další verze = nejvyšší vydaná vX.Y.Z v OBOU repech, patch + 1 (jedna řada verzí; Ledge porovnává čísla);
  # -Version x.y.z vydá zadanou verzi (musí být vyšší než poslední vydaná) — např. skok na 0.3.0
  $last = [version]'0.0.0'
  foreach ($r in @($Public, $Private)) {
    $tags = @(gh release list --repo $r --limit 200 --json tagName --jq '.[].tagName' 2>$null)
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
  [IO.File]::WriteAllText($sumsFile, "$sha  $($setup.Name)`n", (New-Object System.Text.UTF8Encoding($false)))
  $guideSrc = Join-Path $root 'docs\pro-claude.md'
  if (-not (Test-Path $guideSrc)) { Fail 'chybí docs/pro-claude.md — návod pro Clauda patří ke každému vydání' }
  $guideFile = Join-Path $assetDir 'pro-claude.md'
  Copy-Item $guideSrc $guideFile -Force
  $assets = @($setup.FullName, $sumsFile, $guideFile)

  git -C $Clone push -q origin main
  if ($LASTEXITCODE -ne 0) { Fail 'push snapshotu do veřejného repa selhal' }
  gh release create "v$v" @assets --repo $Public --title "Ledge $v" --notes-file $notesFile --target $pubHead
  if ($LASTEXITCODE -ne 0) { Fail "gh release create ($Public) selhal" }
  if ($AlsoPrivate) {
    gh release create "v$v" @assets --repo $Private --title "Ledge $v" --notes-file $notesFile --target $head
    if ($LASTEXITCODE -ne 0) { Fail "gh release create ($Private) selhal" }
  }
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
