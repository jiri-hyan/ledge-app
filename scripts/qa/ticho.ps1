# Tichy "prehravac" pro QA: prehrava ticho (hlasitost 0) pres WinRT MediaPlayer, aby v systemu byla
# hrajici medialni relace (GSMTC) bez okna a bez zvuku. Bezi -Seconds sekund a skonci. ASCII only.
param([int]$Seconds = 240, [string]$Dir = $PSScriptRoot)
$ErrorActionPreference = 'Stop'
$wav = Join-Path $Dir 'ticho.wav'
if (-not (Test-Path $wav)) {
  # 8 kHz, mono, 8 bit, 300 s ticha (0x80)
  $n = 8000 * 300
  $ms = New-Object System.IO.MemoryStream
  $bw = New-Object System.IO.BinaryWriter($ms)
  $bw.Write([Text.Encoding]::ASCII.GetBytes('RIFF')); $bw.Write([int](36 + $n)); $bw.Write([Text.Encoding]::ASCII.GetBytes('WAVEfmt '))
  $bw.Write([int]16); $bw.Write([int16]1); $bw.Write([int16]1); $bw.Write([int]8000); $bw.Write([int]8000); $bw.Write([int16]1); $bw.Write([int16]8)
  $bw.Write([Text.Encoding]::ASCII.GetBytes('data')); $bw.Write([int]$n)
  $buf = New-Object byte[] $n; for ($i = 0; $i -lt $n; $i++) { $buf[$i] = 0x80 }
  $bw.Write($buf); $bw.Flush()
  [IO.File]::WriteAllBytes($wav, $ms.ToArray())
}
[Windows.Media.Playback.MediaPlayer, Windows.Media.Playback, ContentType = WindowsRuntime] | Out-Null
[Windows.Media.Core.MediaSource, Windows.Media.Core, ContentType = WindowsRuntime] | Out-Null
$p = New-Object Windows.Media.Playback.MediaPlayer
$p.CommandManager.IsEnabled = $false
$p.Volume = 0
$p.IsLoopingEnabled = $true
$p.Source = [Windows.Media.Core.MediaSource]::CreateFromUri([Uri]$wav)
$smtc = $p.SystemMediaTransportControls
$smtc.IsEnabled = $true; $smtc.IsPlayEnabled = $true; $smtc.IsPauseEnabled = $true
$smtc.DisplayUpdater.Type = [Windows.Media.MediaPlaybackType]::Music
$smtc.DisplayUpdater.MusicProperties.Title = 'Ledge QA (ticho)'
$smtc.DisplayUpdater.MusicProperties.Artist = 'QA'
$smtc.DisplayUpdater.Update()
$p.Play()
$smtc.PlaybackStatus = [Windows.Media.MediaPlaybackStatus]::Playing
"prehravac bezi: $($p.PlaybackSession.PlaybackState)"
# stav Opening muze trvat; ostrovek se ma otevirat az kdyz prehravac OPRAVDU hraje (QA ceka na radek "prehravac hraje")
$sw = [Diagnostics.Stopwatch]::StartNew()
while ($sw.ElapsedMilliseconds -lt 15000 -and [string]$p.PlaybackSession.PlaybackState -ne 'Playing') { Start-Sleep -Milliseconds 100 }
"prehravac hraje: $($p.PlaybackSession.PlaybackState) po $($sw.ElapsedMilliseconds) ms"
Start-Sleep -Seconds $Seconds
$p.Pause(); $p.Dispose()
"prehravac konec"
