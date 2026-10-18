# @ledge.widget Počasí jinde
# @ledge.description Teplota a obloha v jiném městě z Open-Meteo (bez klíče) — uprav $Mesto, $Lat, $Lon
# @ledge.icon 🌍
# @ledge.refresh 30m
# @ledge.show both
# @ledge.timeout 15
# Síť: jen api.open-meteo.com.

$Mesto = 'Lisabon'
$Lat = 38.72
$Lon = -9.14

$url = "https://api.open-meteo.com/v1/forecast?latitude=$Lat&longitude=$Lon&current=temperature_2m,weather_code,wind_speed_10m&daily=temperature_2m_max,temperature_2m_min&forecast_days=1&timezone=auto"
$d = Invoke-RestMethod -Uri $url -TimeoutSec 10
$kod = [int]$d.current.weather_code
$ikona = if ($kod -eq 0) { '☀' } elseif ($kod -le 3) { '⛅' } elseif ($kod -le 48) { '🌫' } elseif ($kod -le 67) { '🌧' } elseif ($kod -le 77) { '❄' } elseif ($kod -le 82) { '🌦' } else { '⛈' }
$t = [math]::Round($d.current.temperature_2m)
$min = [math]::Round($d.daily.temperature_2m_min[0])
$max = [math]::Round($d.daily.temperature_2m_max[0])

@{
    title = "Počasí · $Mesto"
    strip = "$ikona $t °C $Mesto"
    items = @(
        @{ label = 'Teď'; value = "$ikona $t °C" },
        @{ label = 'Dnes'; value = "$min až $max °C" },
        @{ label = 'Vítr'; value = "$([math]::Round($d.current.wind_speed_10m)) km/h" },
        @{ label = 'Data: Open-Meteo'; icon = '🔗'; url = 'https://open-meteo.com/' }
    )
} | ConvertTo-Json -Depth 5 -Compress
