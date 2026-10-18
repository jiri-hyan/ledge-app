# @ledge.widget Docker
# @ledge.description Kontejnery z docker ps — které běží a které stojí (klik zkopíruje ID)
# @ledge.icon 🐳
# @ledge.refresh 1m
# @ledge.show both
# @ledge.timeout 15

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
    @{ text = 'Docker není nainstalovaný (příkaz docker není na PATH).' } | ConvertTo-Json -Compress
    exit 0
}
$radky = & docker ps -a --format '{{json .}}' 2>$null
if ($LASTEXITCODE -ne 0) {
    @{ text = 'Docker neběží — spusť Docker Desktop.'; strip = '🐳 neběží'; color = 'muted' } | ConvertTo-Json -Compress
    exit 0
}

$kontejnery = @($radky | Where-Object { $_ } | ForEach-Object { $_ | ConvertFrom-Json })
$bezi = @($kontejnery | Where-Object { $_.State -eq 'running' })
$items = @($kontejnery | ForEach-Object {
    @{ label = $_.Names; value = $_.Status; copy = $_.ID; color = $(if ($_.State -eq 'running') { 'green' } else { 'muted' }) }
})

@{
    title = 'Docker'
    badge = "$($bezi.Count)/$($kontejnery.Count)"
    strip = "🐳 $($bezi.Count) běží"
    text = $(if ($kontejnery.Count -eq 0) { 'Žádné kontejnery.' } else { $null })
    items = $items
} | ConvertTo-Json -Depth 5 -Compress
