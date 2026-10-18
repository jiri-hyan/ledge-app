# @ledge.widget Uptime
# @ledge.description Jak dlouho počítač běží od posledního startu
# @ledge.icon ⏻
# @ledge.refresh 10m
# @ledge.show both

$start = (Get-CimInstance Win32_OperatingSystem).LastBootUpTime
$d = (Get-Date) - $start

@{
    strip = "⏻ $($d.Days) d $($d.Hours) h"
    text = "Běží od $($start.ToString('d. M. yyyy H:mm'))"
    items = @(@{ label = 'Doba běhu'; value = ('{0} d {1} h {2} min' -f $d.Days, $d.Hours, $d.Minutes) })
} | ConvertTo-Json -Depth 5 -Compress
