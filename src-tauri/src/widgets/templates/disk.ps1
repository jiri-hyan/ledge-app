# @ledge.widget Místo na disku
# @ledge.description Volné místo na pevných discích (klik otevře disk v Průzkumníkovi)
# @ledge.icon 💽
# @ledge.refresh 10m
# @ledge.show both

$disky = @(Get-CimInstance Win32_LogicalDisk -Filter 'DriveType=3' | Where-Object { $_.Size -gt 0 })
$items = @($disky | ForEach-Object {
    $plno = 1 - $_.FreeSpace / $_.Size
    @{
        label = "$($_.DeviceID) $($_.VolumeName)".Trim()
        value = '{0:N0} z {1:N0} GB volno' -f ($_.FreeSpace / 1GB), ($_.Size / 1GB)
        color = $(if ($plno -gt 0.9) { 'red' } elseif ($plno -gt 0.75) { 'amber' } else { 'green' })
        open = "$($_.DeviceID)\"
    }
})
$c = $disky | Where-Object { $_.DeviceID -eq $env:SystemDrive } | Select-Object -First 1
if (-not $c) { $c = $disky | Select-Object -First 1 }
$plnoC = 1 - $c.FreeSpace / $c.Size

@{
    strip = ('💽 {0} {1:N0} GB' -f $c.DeviceID, ($c.FreeSpace / 1GB))
    progress = $plnoC
    text = ('Disk {0} je zaplněný z {1:N0} %' -f $c.DeviceID, ($plnoC * 100))
    items = $items
} | ConvertTo-Json -Depth 5 -Compress
