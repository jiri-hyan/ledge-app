# @ledge.widget Místo na disku (Python)
# @ledge.description Volné místo na pevných discích (klik otevře disk v Průzkumníkovi)
# @ledge.icon 💽
# @ledge.refresh 10m
# @ledge.show both
import ctypes
import json
import os
import shutil
import string

PEVNY = 3  # GetDriveTypeW: DRIVE_FIXED
items, system = [], None
for p in string.ascii_uppercase:
    koren = f"{p}:\\"
    if ctypes.windll.kernel32.GetDriveTypeW(koren) != PEVNY:
        continue
    u = shutil.disk_usage(koren)
    plno = u.used / u.total
    items.append({
        "label": f"{p}:",
        "value": f"{u.free / 2**30:.0f} z {u.total / 2**30:.0f} GB volno",
        "color": "red" if plno > 0.9 else "amber" if plno > 0.75 else "green",
        "open": koren,
    })
    if koren.lower().startswith(os.environ.get("SystemDrive", "C:").lower()):
        system = (p, u)

p, u = system or ("C", shutil.disk_usage("C:\\"))
print(json.dumps({
    "strip": f"💽 {p}: {u.free / 2**30:.0f} GB",
    "progress": u.used / u.total,
    "text": f"Disk {p}: je zaplněný z {u.used / u.total * 100:.0f} %",
    "items": items,
}, ensure_ascii=False))
