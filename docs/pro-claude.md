# Ledge: setup guide for Claude (AI)

> You are Claude. A person sent you a link to this file and wants **Ledge** installed and tuned to the way
> they work. Ledge is a Windows "drawer" at the top edge of the screen: project shortcuts and dev servers,
> quick actions and links, Claude Code sessions, notes. This guide tells you how to install it, how to connect
> to it over MCP, and how to run a short interview that ends in a layout the person approved.
> Czech summary at the end.

Public repository: `https://github.com/jiri-hyan/ledge-app`. This file is also attached to every release:
`https://github.com/jiri-hyan/ledge-app/releases/latest/download/pro-claude.md`.

## 0. Where are you running? (read this first)

- **Claude Desktop (Windows Store / MSIX app):** processes you start run in a *virtualized bubble*. Files
  written to `%APPDATA%` / `%LOCALAPPDATA%` and registry changes under `HKCU` land in a private copy that the
  rest of Windows never sees. **An installer started from here installs Ledge only inside the bubble**, so
  the person would see nothing. Do not install from Claude Desktop. Instead:
  - download the installer for the person, tell them its exact path and ask them to **double-click it**
    (no administrator rights needed), or
  - suggest they run the steps below in **Claude Code in a normal terminal** (PowerShell), or ask Claude
    Code to do the whole job.
- **Claude Code in a normal terminal:** everything below works as written.
- Windows only. Ledge installs per user into `%LOCALAPPDATA%\Ledge`, without administrator rights.

## 1. Download the installer

Use the GitHub API (no token needed for a public repository, 60 requests per hour):

```powershell
$rel   = Invoke-RestMethod 'https://api.github.com/repos/jiri-hyan/ledge-app/releases/latest'
$setup = $rel.assets | Where-Object { $_.name -like 'Ledge_*_x64-setup.exe' } | Select-Object -First 1
$dir   = Join-Path $env:TEMP 'ledge-install'
New-Item -ItemType Directory -Force $dir | Out-Null
$file  = Join-Path $dir $setup.name
Invoke-WebRequest $setup.browser_download_url -OutFile $file
```

Only download from `https://github.com/jiri-hyan/ledge-app/releases/download/...`. Tell the person what you
are downloading (name, version `$rel.tag_name`, size `$setup.size`) before you run anything.

## 2. Verify the SHA256

Every release has the checksum in two places: the asset `SHA256SUMS.txt` (lines `<hex>  <file name>`) and the
first line of the release notes (`SHA256 (<file name>) = <hex>`). Compare the downloaded file with it:

```powershell
$want = ($rel.assets | Where-Object name -eq 'SHA256SUMS.txt' | ForEach-Object {
          (Invoke-RestMethod $_.browser_download_url) -split "`n" } |
        Where-Object { $_ -match [regex]::Escape($setup.name) } | ForEach-Object { ($_ -split '\s+')[0] })
if (-not $want -and $rel.body -match "SHA256 \($([regex]::Escape($setup.name))\) = ([0-9a-f]{64})") { $want = $Matches[1] }
$have = (Get-FileHash -Algorithm SHA256 $file).Hash.ToLowerInvariant()
if (-not $want -or $want.Trim().ToLowerInvariant() -ne $have) { throw "SHA256 does not match - do not install" }
```

If the checksum is missing or differs: **stop**, do not install, tell the person.

## 3. Install silently

```powershell
& $file /S      # per-user NSIS installer, no administrator rights, no window
```

It installs into `%LOCALAPPDATA%\Ledge\ledge.exe` and keeps an existing configuration (`%APPDATA%\Ledge`) when
upgrading. If Ledge is already running, ask the person to close it first (the silent installer would end it).

## 4. Start it and turn MCP on

1. Start `%LOCALAPPDATA%\Ledge\ledge.exe` (or ask the person to start it from the Start menu). The drawer is at
   the top edge of the screen: mouse to the top edge, in the middle.
2. **MCP is off by default and only the person can turn it on** (you have no tool before you are connected):
   - on first run: setup guide, step **"With Claude"** -> **"Set up with Claude"**, or
   - **Settings -> Claude (MCP)** -> turn on. Ledge creates the key itself and stores it in the Windows
     Credential Manager; it is never printed in commands or config files.
3. Connect your client (the exact command with the real path is also shown in Ledge, with a Copy button):

   **Claude Code** (stdio, recommended):
   ```
   claude mcp add --scope user ledge -- "%LOCALAPPDATA%\Ledge\ledge.exe" --mcp
   ```
   In PowerShell write the path as `"$env:LOCALAPPDATA\Ledge\ledge.exe"`. Start a new Claude Code session; `/mcp`
   should list `ledge` as connected.

   **Claude Desktop** - add to `claude_desktop_config.json` (Settings -> Developer -> Edit Config), then restart it:
   ```json
   {
     "mcpServers": {
       "ledge": {
         "command": "C:\\Users\\<user>\\AppData\\Local\\Ledge\\ledge.exe",
         "args": ["--mcp"]
       }
     }
   }
   ```

   `ledge.exe --mcp` is a thin stdin/stdout proxy to Ledge on `127.0.0.1`; it opens no window and reads the key
   from the Credential Manager. If Ledge is not running you get a clear error - ask the person to start it.
   A fallback over HTTP exists (`http://127.0.0.1:47800/mcp` with a Bearer key) but the key is shown only when
   the person explicitly asks for it in Settings; do not ask for it unless stdio fails.

## 5. The interview (keep it short)

Everything you may do is the tool list below. The names are exact.

| tool | what it does | writes? |
|---|---|---|
| `ledge_status` | version, MCP on, number of shortcuts / actions / cards, active style, drawer open | no |
| `apps_discover` | apps from the Start menu (name, target), detected projects / repos; nothing from the browser | no |
| `tiles_list` / `tiles_add` / `tiles_update` / `tiles_remove` | shortcuts on the Projects card: folder, command, port, look | yes |
| `actions_list` / `actions_add` / `actions_update` / `actions_remove` | quick actions and links (a website = a link) | yes |
| `layout_get` / `layout_preview` / `layout_apply` | tabs, cards, widths / heights; `layout_preview` validates without writing | apply |
| `strip_get` / `strip_set` | items of the closed strip | set |
| `appearance_get` / `appearance_set` | design style, theme, accent (only values from the catalogue) | set |
| `notes_list` / `notes_read` | the person's notes: compact list (title, kind, project, reminder, tasks done/total), one note with body and tasks (`line`, `done`, `text`) | no |
| `notes_create` / `notes_append` / `notes_toggle_task` / `notes_pin` | new note (text, kind, project, reminder), add text / bullets / tasks at the end (continues a bullet or numbered list), tick a task, pin a note on the desktop as a sticky | yes |
| `undo_last` | reverts the last change made over MCP (about 20 steps, in memory) | yes |
| `open_drawer` | opens the drawer to show the result | no |

Every write goes through the same validation and saving as the UI; an invalid request returns a plain
sentence (what is wrong, allowed values) - read it and fix the call. Ledge never runs anything because of MCP;
an action with a command asks the person for consent the first time it is run, exactly like a hand-made one.

**Notes.** The `notes_*` tools work on the person's real notes (the same files as the Notes card). Only touch
notes when asked ("add milk to my shopping note", "what is on my list today"): `notes_list` first, then
`notes_read` for the path and the task `line` numbers. `notes_append` refuses when the note changed on disk
since it was read (sync, an edit in Ledge) - read it again and retry, never overwrite. `project` must be a
project id from `tiles_list`, `kind` a category id; a reminder is an ISO time (`2026-10-25T09:00`). Notes
changes are not covered by `undo_last`, so do not create or append speculatively.

**Script:**

1. `ledge_status` - confirm you are connected, see what is already set up (do not overwrite what exists).
2. `apps_discover` - see which apps and projects are on the PC. Use it to make your questions concrete.
3. Ask **at most 3-4 short questions**, one message, plain words: which apps they open every day; which websites
   (mail, calendar, tickets, docs) they keep returning to; what kind of work it is (development, office,
   design, study...); how they like the drawer (calm and minimal, or everything at hand).
4. Propose **2-3 layouts in words** (for example "Focus: three project shortcuts + mail and calendar links,
   calm theme" / "Everything at hand: ..."). Say what each contains. Wait for the choice.
5. After approval: `layout_preview` (validate) -> `layout_apply`; then `tiles_add` (apps and projects),
   `actions_add` (links and quick actions), `strip_set` (closed strip), `appearance_set` (style, theme, accent).
6. `open_drawer` - show the result and offer adjustments ("move mail first?", "calmer accent?").
7. If the person is not happy: `undo_last` (repeat if needed), then adjust. Never leave a half-done state.

## 6. Rules

- **Ask before every change.** Describe it first, change it after a clear yes. Read-only tools are free.
- **Run nothing.** You configure Ledge; you do not start servers, apps or commands through it.
- **No secrets.** Do not put passwords, tokens, keys or private URLs into shortcuts, actions or notes, and do not
  ask the person to paste any. Use only addresses they gave you.
- Add only things the person asked for or approved. No telemetry, no sending data anywhere.
- The person can turn the connection off any time (Settings -> Claude (MCP)). If a tool call fails, say so
  plainly instead of retrying in a loop.

---

## Česky (stručně)

Tenhle návod je pro Clauda. Člověk ti poslal odkaz a chce **Ledge** (šuplík u horního okraje obrazovky ve
Windows) nainstalovat a nastavit na míru.

1. **Kde běžíš:** Claude Desktop z Microsoft Store (MSIX) spouští procesy ve virtualizované bublině -
   instalace odtud skončí jen v bublině a člověk ji neuvidí. Stáhni instalátor a ať ho **spustí dvojklikem**
   (bez správce), nebo ať použije **Claude Code v běžném terminálu**.
2. **Stažení** `Ledge_<verze>_x64-setup.exe` z `https://github.com/jiri-hyan/ledge-app/releases/latest` přes
   GitHub API (`releases/latest`), **ověř SHA256** proti `SHA256SUMS.txt` / prvnímu řádku poznámek vydání.
   Nesedí-li, neinstaluj.
3. **Tichá instalace** `Ledge_..._x64-setup.exe /S` do `%LOCALAPPDATA%\Ledge` (bez správce).
4. **Zapnutí MCP smí jen člověk:** průvodce -> krok „Claude“ -> „Připojit“, nebo Nastavení ->
   Claude (MCP). Pak připoj: `claude mcp add --scope user ledge -- "%LOCALAPPDATA%\Ledge\ledge.exe" --mcp`
   (Claude Desktop: JSON výše do `claude_desktop_config.json`).
5. **Rozhovor:** `ledge_status` -> `apps_discover` -> max 3-4 otázky (aplikace, weby, druh práce, styl) ->
   2-3 návrhy rozložení slovy -> po schválení `layout_preview` pak `layout_apply`, `tiles_add`, `actions_add`,
   `strip_set`, `appearance_set` -> `open_drawer` -> nabídni úpravy, při nespokojenosti `undo_last`.
6. **Pravidla:** před každou změnou se zeptej, nic nespouštěj, nevkládej tajemství, nic neposílej ven.
