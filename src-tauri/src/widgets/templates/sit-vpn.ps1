# @ledge.widget Síť a VPN
# @ledge.description Aktivní připojení a jestli běží VPN (jen údaje z Windows, nic se nestahuje)
# @ledge.icon 🛜
# @ledge.refresh 1m
# @ledge.show both

$VpnVzor = 'VPN|WireGuard|TAP-|Tunnel|OpenVPN|AnyConnect|Fortinet|Wintun|ZeroTier|Tailscale'

$aktivni = @(Get-NetAdapter | Where-Object Status -eq 'Up')
$vpn = @($aktivni | Where-Object { $_.InterfaceDescription -match $VpnVzor -or $_.Name -match $VpnVzor } | ForEach-Object { $_.Name })
$items = @($aktivni | ForEach-Object {
    @{ label = $_.Name; value = $_.LinkSpeed; icon = $(if ($vpn -contains $_.Name) { '🔒' } else { '🔌' }) }
})

@{
    strip = $(if ($vpn.Count) { '🔒 VPN' } elseif ($aktivni.Count) { '🛜 bez VPN' } else { '⚠ offline' })
    color = $(if ($vpn.Count) { 'green' } elseif ($aktivni.Count) { 'muted' } else { 'red' })
    text = $(if ($aktivni.Count) { "Připojení: $($aktivni.Count)" } else { 'Žádné aktivní připojení.' })
    items = $items
} | ConvertTo-Json -Depth 5 -Compress
