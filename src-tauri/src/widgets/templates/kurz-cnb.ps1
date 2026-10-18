# @ledge.widget Kurz ČNB
# @ledge.description Kurz eura, dolaru a libry z denního kurzovního lístku ČNB (veřejná data, bez klíče)
# @ledge.icon 💶
# @ledge.refresh 1h
# @ledge.show both
# @ledge.timeout 15
# Síť: jen www.cnb.cz (stáhne kurzovní lístek).

$Meny = @('EUR', 'USD', 'GBP')
$web = 'https://www.cnb.cz/cs/financni-trhy/devizovy-trh/kurzy-devizoveho-trhu/kurzy-devizoveho-trhu/'
$radky = (Invoke-WebRequest -Uri ($web + 'denni_kurz.txt') -UseBasicParsing -TimeoutSec 10).Content -split "`n"

# řádek: země|měna|množství|kód|kurz
$items = @()
foreach ($r in ($radky | Select-Object -Skip 2)) {
    $p = $r.Trim() -split '\|'
    if ($p.Count -eq 5 -and $Meny -contains $p[3]) {
        $kurz = [double]($p[4] -replace ',', '.') / [double]$p[2]
        $items += @{ label = $p[3]; value = ('{0:N3} Kč' -f $kurz) }
    }
}
$eur = $items | Where-Object { $_.label -eq 'EUR' } | Select-Object -First 1
$items += @{ label = 'Kurzovní lístek na cnb.cz'; icon = '🔗'; url = $web }

@{
    title = "Kurz ČNB · $(($radky[0] -split ' ')[0])"
    strip = $(if ($eur) { "€ $($eur.value)" } else { $null })
    items = $items
} | ConvertTo-Json -Depth 5 -Compress
