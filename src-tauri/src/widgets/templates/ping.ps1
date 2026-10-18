# @ledge.widget Ping
# @ledge.description Odezva serveru (jeden ping) — uprav $Server
# @ledge.icon 📡
# @ledge.refresh 1m
# @ledge.show both
# @ledge.timeout 10
# Síť: jeden ICMP ping na $Server.

$Server = '1.1.1.1'

$p = Test-Connection -ComputerName $Server -Count 1 -ErrorAction SilentlyContinue
$out = if ($p) {
    $ms = $p.ResponseTime
    @{ strip = "📡 $ms ms"; text = "$Server odpovídá za $ms ms"; color = $(if ($ms -lt 80) { 'green' } else { 'amber' }) }
} else {
    @{ strip = '📡 nedostupný'; text = "$Server neodpovídá"; color = 'red' }
}
$out | ConvertTo-Json -Compress
