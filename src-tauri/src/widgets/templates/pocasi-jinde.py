# @ledge.widget Počasí jinde (Python)
# @ledge.description Teplota a obloha v jiném městě z Open-Meteo (bez klíče) — uprav MESTO, LAT, LON
# @ledge.icon 🌍
# @ledge.refresh 30m
# @ledge.show both
# @ledge.timeout 15
# Síť: jen api.open-meteo.com.
import json
import urllib.request

MESTO, LAT, LON = "Lisabon", 38.72, -9.14

url = (f"https://api.open-meteo.com/v1/forecast?latitude={LAT}&longitude={LON}"
       "&current=temperature_2m,weather_code,wind_speed_10m"
       "&daily=temperature_2m_max,temperature_2m_min&forecast_days=1&timezone=auto")
with urllib.request.urlopen(url, timeout=10) as r:
    d = json.load(r)

kod = d["current"]["weather_code"]
ikona = next(i for hranice, i in [(0, "☀"), (3, "⛅"), (48, "🌫"), (67, "🌧"), (77, "❄"), (82, "🌦"), (999, "⛈")] if kod <= hranice)
t = round(d["current"]["temperature_2m"])
dnes = f"{round(d['daily']['temperature_2m_min'][0])} až {round(d['daily']['temperature_2m_max'][0])} °C"
print(json.dumps({
    "title": f"Počasí · {MESTO}",
    "strip": f"{ikona} {t} °C {MESTO}",
    "items": [
        {"label": "Teď", "value": f"{ikona} {t} °C"},
        {"label": "Dnes", "value": dnes},
        {"label": "Vítr", "value": f"{round(d['current']['wind_speed_10m'])} km/h"},
        {"label": "Data: Open-Meteo", "icon": "🔗", "url": "https://open-meteo.com/"},
    ],
}, ensure_ascii=False))
