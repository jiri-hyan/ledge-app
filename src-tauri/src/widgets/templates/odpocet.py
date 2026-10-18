# @ledge.widget Odpočet (Python)
# @ledge.description Kolik dní zbývá do data — uprav NAZEV, MESIC a DEN. Ukázka jednoduchého textového formátu.
# @ledge.icon ⏳
# @ledge.refresh 1h
# @ledge.show both
import datetime

NAZEV = "Vánoce"
MESIC, DEN = 12, 24

dnes = datetime.date.today()
cil = datetime.date(dnes.year, MESIC, DEN)
if cil < dnes:
    cil = cil.replace(year=dnes.year + 1)
dni = (cil - dnes).days

# Textový formát: 1. řádek = proužek, další řádky = text karty, za --- položky (parametry za |)
print(f"⏳ {dni} d · {NAZEV}" if dni else f"🎉 {NAZEV}")
print(f"{NAZEV} {cil.day}. {cil.month}. {cil.year}")
print("---")
print(f"Zbývá dní | value={dni} color={'green' if dni > 7 else 'amber'}")
print(f"Zkopírovat datum | copy={cil.isoformat()}")
