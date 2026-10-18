# @ledge.widget Ping (Python)
# @ledge.description Odezva serveru (jeden ping) — uprav SERVER
# @ledge.icon 📡
# @ledge.refresh 1m
# @ledge.show both
# @ledge.timeout 10
# Síť: jeden ICMP ping na SERVER.
import json
import re
import subprocess

SERVER = "1.1.1.1"

r = subprocess.run(["ping", "-n", "1", "-w", "2000", SERVER], capture_output=True, text=True, errors="replace")
m = re.search(r"[=<](\d+)\s*ms", r.stdout)
if r.returncode == 0 and m:
    ms = int(m.group(1))
    out = {"strip": f"📡 {ms} ms", "text": f"{SERVER} odpovídá za {ms} ms", "color": "green" if ms < 80 else "amber"}
else:
    out = {"strip": "📡 nedostupný", "text": f"{SERVER} neodpovídá", "color": "red"}
print(json.dumps(out, ensure_ascii=False))
