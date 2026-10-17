# Vydání Ledge do VEŘEJNÉHO repa (výchozí jiri-hyan/ledge-app) z vývojového (soukromého) repa.
#
# Veřejné repo nedostává vývojovou historii: každé vydání = JEDEN nový commit „snapshot“ stromu `main`
# (`git archive HEAD` — vynechá cesty s `export-ignore` v .gitattributes: interní poznámky, CI, tenhle
# seznam zakázaných slov), autor jiri-hyan s GitHub noreply e-mailem. Před commitem se strom i seznam
# změn prověří proti `scripts/private/denylist.txt` (jména klientů, strojů…) — jediný nález = konec.
# Pak lokální build (stejný tvar jako dřív) a GitHub Release v0.2.<N+1> ve veřejném repu s řádkem
# `SHA256 (<instalátor>) = <hex>`, který čte Ledge (update.rs `sha_for`).
#
# Použití: powershell -NoProfile -File scripts/publish.ps1 [-Public jiri-hyan/ledge-app]
#          [-Private jiri-hyan/ledge] [-AlsoPrivate] [-Clone <složka klonu veřejného repa>] [-DryRun]
#   -AlsoPrivate  stejný instalátor vydá i v soukromém repu (přechod: Ledge ≤ 0.2.8 hledá aktualizace tam)
#   -DryRun       jen snapshot do klonu + kontrola denylistu, nic necommituje, nebuilduje ani nevydá
# Podmínky: větev main, čistý strom, HEAD pushnutý do soukromého repa.
param(
  [string]$Public = 'jiri-hyan/ledge-app',
  [string]$Private = 'jiri-hyan/ledge',
  [switch]$AlsoPrivate,
  [string]$Clone = (Join-Path $env:LOCALAPPDATA 'ledge-public'),
  [switch]$DryRun
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
    if ($_.Extension -notin '.png', '.ico', '.icns', '.jpg', '.webp') {
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

  # další verze = nejvyšší vydaná 0.2.x v OBOU repech + 1 (jedna řada verzí; Ledge porovnává čísla)
  $max = 0
  foreach ($r in @($Public, $Private)) {
    $tags = @(gh release list --repo $r --limit 200 --json tagName --jq '.[].tagName' 2>$null)
    foreach ($tg in $tags) { if ($tg -match '^v0\.2\.(\d+)$' -and [int]$Matches[1] -gt $max) { $max = [int]$Matches[1] } }
  }
  $v = "0.2.$($max + 1)"
  "verze:      $v (poslední vydaná: 0.2.$max)"

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
  $prevTag = "v0.2.$max"
  git rev-parse -q --verify "refs/tags/$prevTag" *> $null
  # seznam změn jen z commitů, které mění něco, co jde ven (ne čistě interní export-ignore cesty)
  $public = @('--', '.', ':(exclude)scripts/private', ':(exclude).github')
  $log = if ($LASTEXITCODE -eq 0) { @(git log --no-decorate --format='- %s' "$prevTag..HEAD" @public) } else { @(git log --no-decorate -n 30 --format='- %s' @public) }
  foreach ($line in $log) { foreach ($d in $deny) { if ($line -match "(?i)$d") { $hits += "seznam změn: $line ~ $d" } } }
  if ($hits.Count) { $hits | ForEach-Object { "  ZAKÁZÁNO: $_" }; Fail "snapshot obsahuje zakázaná slova ($($hits.Count)) — oprav na main a vydej znovu" }
  "snapshot:   $Clone čistý (denylist $($deny.Count) vzorů)"
  if ($DryRun) { 'DryRun: konec bez commitu, buildu a vydání'; exit 0 }

  # autor i committer = jiri-hyan s noreply e-mailem GitHubu (žádná osobní ani pracovní adresa)
  $uid = gh api user --jq .id
  $who = @('-c', 'user.name=jiri-hyan', '-c', "user.email=$uid+jiri-hyan@users.noreply.github.com")
  git -C $Clone add -A
  git -C $Clone @who commit -q -m "Ledge $v" -m "Snapshot vývojového stromu (bez historie)."
  if ($LASTEXITCODE -ne 0) { Fail 'commit snapshotu selhal (nic se nezměnilo?)' }
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
  $notes = @("SHA256 ($($setup.Name)) = $sha", '', 'Změny:', '') + $log
  $notesFile = Join-Path $env:TEMP "ledge-release-notes-$v.md"
  [IO.File]::WriteAllLines($notesFile, [string[]]$notes, (New-Object System.Text.UTF8Encoding($false)))

  git -C $Clone push -q origin main
  if ($LASTEXITCODE -ne 0) { Fail 'push snapshotu do veřejného repa selhal' }
  gh release create "v$v" $setup.FullName --repo $Public --title "Ledge $v" --notes-file $notesFile --target $pubHead
  if ($LASTEXITCODE -ne 0) { Fail "gh release create ($Public) selhal" }
  if ($AlsoPrivate) {
    gh release create "v$v" $setup.FullName --repo $Private --title "Ledge $v" --notes-file $notesFile --target $head
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
