# @ledge.widget Kurz ČNB (Python)
# @ledge.description Kurz eura, dolaru a libry z denního kurzovního lístku ČNB (veřejná data, bez klíče)
# @ledge.icon 💶
# @ledge.refresh 1h
# @ledge.show both
# @ledge.timeout 15
# Síť: jen www.cnb.cz (stáhne kurzovní lístek).
import json
import urllib.request

MENY = ("EUR", "USD", "GBP")
WEB = "https://www.cnb.cz/cs/financni-trhy/devizovy-trh/kurzy-devizoveho-trhu/kurzy-devizoveho-trhu/"

with urllib.request.urlopen(WEB + "denni_kurz.txt", timeout=10) as r:
    radky = r.read().decode("utf-8").splitlines()

# řádek: země|měna|množství|kód|kurz
items = []
for radek in radky[2:]:
    p = radek.split("|")
    if len(p) == 5 and p[3] in MENY:
        kurz = float(p[4].replace(",", ".")) / float(p[2])
        items.append({"label": p[3], "value": f"{kurz:.3f} Kč".replace(".", ",")})

eur = next((i for i in items if i["label"] == "EUR"), None)
items.append({"label": "Kurzovní lístek na cnb.cz", "icon": "🔗", "url": WEB})
print(json.dumps({
    "title": f"Kurz ČNB · {radky[0].split()[0]}",
    "strip": f"€ {eur['value']}" if eur else None,
    "items": items,
}, ensure_ascii=False))
