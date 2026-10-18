# @ledge.widget Uptime (Python)
# @ledge.description Jak dlouho počítač běží od posledního startu
# @ledge.icon ⏻
# @ledge.refresh 10m
# @ledge.show both
import ctypes
import datetime
import json

k32 = ctypes.windll.kernel32
k32.GetTickCount64.restype = ctypes.c_ulonglong
d = datetime.timedelta(milliseconds=k32.GetTickCount64())
start = datetime.datetime.now() - d
h, rest = divmod(d.seconds, 3600)

print(json.dumps({
    "strip": f"⏻ {d.days} d {h} h",
    "text": f"Běží od {start:%d. %m. %Y %H:%M}",
    "items": [{"label": "Doba běhu", "value": f"{d.days} d {h} h {rest // 60} min"}],
}, ensure_ascii=False))
