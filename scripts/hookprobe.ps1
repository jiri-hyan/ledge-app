# Sonda: co udělá Claude Code CLI, když HTTP hook PermissionRequest míří na port, kde nic neběží
# (= Ledge neběží). Očekávané (dokumentované) chování: neblokující chyba → normální tok oprávnění.
# Hlášený bug (anthropics/claude-code#46193, 2.1.100): tiché zamítnutí „The user doesn't want to proceed…“.
#
# Spustí dvakrát `claude -p` v dočasné složce (bez uživatelských hooků, takže ostrý Ledge nic nevidí):
#   1) bez hooku (základ), 2) s hookem na mrtvý port. Shodná zamítnutí = PASS, hookem navíc
#   „user doesn't want to proceed“ = FAIL. V -p režimu se nikdo nemá jak zeptat, takže zamítnutí
#   v základu je normální — rozhoduje rozdíl proti základu.
# Potřebuje přihlášené CLI (`claude auth status`). Použití: powershell -File scripts/hookprobe.ps1
param([int]$Port = 47999, [string]$Model = "haiku")
$ErrorActionPreference = "Stop"

# proměnné hostitelské session (desktop / jiný Claude) by CLI zmátly
Get-ChildItem Env: | Where-Object { $_.Name -like "CLAUDE*" -or $_.Name -like "ANTHROPIC*" } |
  ForEach-Object { [Environment]::SetEnvironmentVariable($_.Name, $null) }

$auth = (& claude auth status 2>$null) -join "`n"
if ($auth -notmatch '"loggedIn":\s*true') { Write-Host "CLI není přihlášené: spusť 'claude auth login'"; exit 2 }

$c = New-Object System.Net.Sockets.TcpClient
try { $c.Connect("127.0.0.1", $Port); Write-Host "Port $Port je obsazený, zvol jiný (-Port)"; exit 2 } catch {} finally { $c.Close() }

$dir = Join-Path $env:TEMP ("ledge-hookprobe-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
New-Item -ItemType Directory $dir | Out-Null
$settings = Join-Path $dir "probe-settings.json"
@{ hooks = @{ PermissionRequest = @(@{ hooks = @(@{ type = "http"; url = "http://127.0.0.1:$Port/permission"; timeout = 30 }) }) } } |
  ConvertTo-Json -Depth 8 | Set-Content -Encoding ascii $settings
$prompt = "Use your shell tool to run exactly this command and nothing else: echo probe > probe.txt . Then quote verbatim any error or denial you got."

function Invoke-Probe([string[]]$extra) {
  Push-Location $dir
  try {
    $t0 = Get-Date
    $out = & claude -p $prompt --model $Model --setting-sources project --permission-mode default --max-turns 3 --output-format json @extra 2>$null
    $secs = [Math]::Round(((Get-Date) - $t0).TotalSeconds, 1)
  } finally { Pop-Location }
  $j = ($out -join "`n") | ConvertFrom-Json
  $denials = @($j.permission_denials | ForEach-Object { $_.tool_name })
  [pscustomobject]@{ Denials = ($denials -join ","); Result = [string]$j.result; Secs = $secs }
}

$base = Invoke-Probe @()
Remove-Item (Join-Path $dir "probe.txt") -ErrorAction SilentlyContinue
$dead = Invoke-Probe @("--settings", $settings)

Write-Host "Claude Code: $((& claude --version) -join ' ')"
Write-Host ("základ:      zamítnuto [{0}] za {1} s" -f $base.Denials, $base.Secs)
Write-Host ("mrtvý hook:  zamítnuto [{0}] za {1} s" -f $dead.Denials, $dead.Secs)
$closed = "doesn't want to proceed"
$failClosed = ($dead.Result -match $closed) -and -not ($base.Result -match $closed)
Remove-Item -Recurse -Force $dir
if ($failClosed) { Write-Host "FAIL: hook na mrtvý port zamítá uzavřeně"; Write-Host $dead.Result; exit 1 }
if ($dead.Denials -ne $base.Denials) { Write-Host "NEJASNÉ: jiná zamítnutí než základ"; Write-Host $dead.Result; exit 3 }
Write-Host "PASS: mrtvý hook se chová jako bez hooku (neblokující chyba)"
