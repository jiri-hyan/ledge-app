# Ikona Ledge: src-tauri/icons/ledge.svg → icon-1024.png (headless Edge, průhledné rohy) → `pnpm tauri icon`
# vygeneruje celou sadu (ico, icns, png, Square*Logo). Bez okna na obrazovce.
# Použití: powershell -NoProfile -File scripts/icon.ps1
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$icons = Join-Path $root 'src-tauri\icons'
$svg = Join-Path $icons 'ledge.svg'
$png = Join-Path $icons 'icon-1024.png'

$edge = @("${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe", "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe") |
  Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $edge) { Write-Error 'nenašel jsem msedge.exe'; exit 1 }

# HTML obal: SVG přesně 1024×1024 bez okrajů
$html = Join-Path $env:TEMP 'ledge-icon.html'
$svgText = Get-Content $svg -Raw -Encoding UTF8
"<!doctype html><html><head><style>html,body{margin:0;background:transparent}svg{display:block}</style></head><body>$svgText</body></html>" |
  Set-Content $html -Encoding UTF8

$profile = Join-Path $env:TEMP 'ledge-icon-edge'
# Edge hlásí „… bytes written“ na stderr — pod 'Stop' by to PowerShell 5.1 vzal jako chybu
$ErrorActionPreference = 'Continue'
& $edge --headless=new --disable-gpu --hide-scrollbars --user-data-dir="$profile" --default-background-color=00000000 `
  --window-size=1024,1024 --screenshot="$png" "file:///$($html -replace '\\','/')" 2>&1 | Out-Null
$ErrorActionPreference = 'Stop'
Start-Sleep -Milliseconds 500
if (-not (Test-Path $png)) { Write-Error "Edge nevytvořil $png"; exit 1 }
"png: $png ($((Get-Item $png).Length) B)"

Push-Location $root
try { & pnpm tauri icon $png; if ($LASTEXITCODE -ne 0) { Write-Error 'tauri icon selhal'; exit 1 } } finally { Pop-Location }

# Ledge je jen pro Windows — mobilní sady z `tauri icon` neuchováváme
Remove-Item -Recurse -Force (Join-Path $icons 'android'), (Join-Path $icons 'ios') -ErrorAction SilentlyContinue
