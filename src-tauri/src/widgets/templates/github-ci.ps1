# @ledge.widget GitHub CI
# @ledge.description Poslední commit a běhy GitHub Actions jednoho repa přes gh (přihlášení: gh auth login)
# @ledge.icon 🐙
# @ledge.refresh 5m
# @ledge.show both
# @ledge.timeout 30
# Síť: api.github.com přes gh (přihlášení zůstává v gh, Ledge ho nevidí).

# Sem napiš repo ve tvaru owner/name:
$Repo = ''

if (-not $Repo) {
    @{ text = 'Otevři skript (⋯ → Upravit skript) a doplň $Repo = ''owner/name''.' } | ConvertTo-Json -Compress
    exit 0
}
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    @{ text = 'Chybí GitHub CLI (gh) — nainstaluj ho a přihlas se: gh auth login.' } | ConvertTo-Json -Compress
    exit 0
}

$commit = gh api "repos/$Repo/commits?per_page=1" --jq '.[0] | {sha: .sha[0:7], msg: (.commit.message | split("\n")[0]), url: .html_url}' | ConvertFrom-Json
$runs = @(gh run list -R $Repo --limit 5 --json name,status,conclusion,url,headBranch | ConvertFrom-Json)
$barvy = @{ success = 'green'; failure = 'red'; cancelled = 'muted'; skipped = 'muted' }

$items = @()
if ($commit) { $items += @{ label = $commit.msg; value = $commit.sha; icon = '📝'; url = $commit.url } }
foreach ($r in $runs) {
    $hotovo = $r.status -eq 'completed'
    $barva = if (-not $hotovo) { 'amber' } elseif ($barvy[$r.conclusion]) { $barvy[$r.conclusion] } else { 'muted' }
    $stavRunu = if ($hotovo) { $r.conclusion } else { $r.status }
    $items += @{ label = $r.name; value = "$stavRunu · $($r.headBranch)"; color = $barva; url = $r.url }
}
$prvni = $runs | Select-Object -First 1
$stav = if (-not $prvni) { 'bez běhů' } elseif ($prvni.status -ne 'completed') { 'běží' } else { $prvni.conclusion }

@{
    title = $Repo
    strip = "🐙 CI $stav"
    color = $(if ($prvni -and $prvni.conclusion -eq 'failure') { 'red' } elseif ($prvni -and $prvni.status -ne 'completed') { 'amber' } else { 'green' })
    items = $items
} | ConvertTo-Json -Depth 5 -Compress
