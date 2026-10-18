# @ledge.widget Odpočet
# @ledge.description Kolik dní zbývá do data — uprav $Nazev, $Mesic a $Den
# @ledge.icon ⏳
# @ledge.refresh 1h
# @ledge.show both

$Nazev = 'Vánoce'
$Mesic = 12
$Den = 24

$dnes = (Get-Date).Date
$cil = (Get-Date -Year $dnes.Year -Month $Mesic -Day $Den).Date
if ($cil -lt $dnes) { $cil = $cil.AddYears(1) }
$dni = ($cil - $dnes).Days

@{
    strip = $(if ($dni -eq 0) { "🎉 $Nazev" } else { "⏳ $dni d · $Nazev" })
    text = "$Nazev $($cil.ToString('d. M. yyyy'))"
    progress = 1 - $dni / 365
} | ConvertTo-Json -Compress
