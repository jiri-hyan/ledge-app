# Pomocník pro skripty: hlavička `Authorization` k lokálnímu mostu (`/status`, `/ctl/*` chtějí klíč hooků, L7).
# Klíč je ve Správci pověření (`Ledge/hooks`) a zároveň v hlavičkách hooků v ~/.claude/settings.json — čteme odtud.
# Použití: `. "$PSScriptRoot\ledgeapi.ps1"; Invoke-RestMethod http://127.0.0.1:47800/status -Headers (Get-LedgeAuth)`
function Get-LedgeAuth {
  $f = Join-Path $env:USERPROFILE '.claude\settings.json'
  $j = Get-Content $f -Raw | ConvertFrom-Json
  foreach ($ev in $j.hooks.PSObject.Properties) {
    foreach ($g in $ev.Value) {
      foreach ($h in $g.hooks) {
        if ($h.url -like 'http://127.0.0.1:*' -and $h.headers.Authorization) { return @{ Authorization = $h.headers.Authorization } }
      }
    }
  }
  throw 'v ~/.claude/settings.json nejsou hooky Ledge s klíčem (Nastavení → Claude → Zapojit hooky)'
}
