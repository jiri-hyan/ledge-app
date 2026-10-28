<div align="center">

<img src="src-tauri/icons/icon.png" width="96" height="96" alt="Ledge icon">

# Ledge

**Your whole workday, one mouse move away.**
An always-on drawer at the top edge of your Windows screen — projects, dev servers, notes, quick actions
and every Claude Code session, with approvals, questions and usage limits you can handle without hunting
for the terminal.

[![Download for Windows](https://img.shields.io/badge/Download-for%20Windows-7c3aed?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/jiri-hyan/ledge-app/releases/latest)

[![Latest release](https://img.shields.io/github/v/release/jiri-hyan/ledge-app?label=release&color=6366f1)](https://github.com/jiri-hyan/ledge-app/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/jiri-hyan/ledge-app/total?color=22d3ee)](https://github.com/jiri-hyan/ledge-app/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011%20x64-0078d4)](#install)
[![License: MIT](https://img.shields.io/github/license/jiri-hyan/ledge-app?color=a78bfa)](LICENSE)

[Website](https://jiri-hyan.github.io/ledge-app/) · [Česky](README.cs.md) · [Install](#install) · [FAQ](#faq) · [For agents](AGENTS.md)

<img src="docs/media/open.gif" width="860" alt="The Ledge drawer unfolding from the closed island at the top edge of the screen">

<sub>The real Ledge interface rendered with sample data (made-up projects, no personal content) on a neutral background.
The drawer slides down when you touch the top edge of the screen.</sub>

<img src="docs/screenshots/drawer.png" width="860" alt="The Ledge drawer: Claude sessions with an approval and rate limits, project tiles, repos and notes">

</div>

## Why Ledge

Ledge is built by one developer for his own daily work on Windows with Claude Code, and shared as open
source (MIT). It doesn't try to fit everyone out of the box: it has a lot of settings, and if it still isn't
quite right for you, [fork it and make it yours](#make-it-yours).

- **Out of your way.** A thin strip at the top edge until you need it. Ledge never takes keyboard focus
  from what you're typing, goes quiet while a fullscreen game runs, and is built to an idle CPU budget
  below 0.1 %.
- **Claude Code, finally visible on Windows.** Every session at a glance, approvals and questions answered
  from the drawer, 5-hour and weekly limits always in sight — no more digging through terminal windows.
- **Yours, local and open.** No account, no cloud, no telemetry. Notes are plain Markdown files in your
  folder, config is one JSON file, and the code is MIT-licensed.

## Claude Code on Windows

Ledge connects to Claude Code through its official hooks and status line. When Claude needs you, a peek
slides down from the top edge — without stealing focus from your editor.

<img src="docs/media/approve-click.gif" width="640" alt="Approving a Claude Code edit with one click in the Ledge drawer">
<img src="docs/screenshots/question.png" width="760" alt="A question from Claude Code in the Ledge drawer with multiple-choice answers">
<img src="docs/screenshots/limits.png" width="760" alt="Claude 5-hour and weekly limits with a pace marker in the Ledge drawer">

<sub>Sample data.</sub>

| | |
|---|---|
| **Approvals** | Permission requests show the command or diff; **Allow** or **Deny** with a click. If you don't answer in time, the request goes back to the terminal. |
| **Questions** | `AskUserQuestion` prompts with single choice, multiple choice or your own text. |
| **Plans** | Review a plan when Claude wants to leave plan mode: approve, or send it back with a note. |
| **Sessions** | Every session on the PC — desktop app and CLI, with subagents — and its state: waiting for you, working, idle, unfinished work. A click switches to that window or resumes a finished session. |
| **Limits and pace** | 5-hour and weekly usage with a reset countdown (from Claude Code's status line; your own status line keeps working), an optional pace marker and a reminder when a full window resets. |
| **Activity and cost** | What Claude is doing right now, and a cost estimate at API prices computed from your local transcripts. |
| **Unfinished work** | Files a session changed but didn't commit, and commits that aren't pushed anywhere. |
| **Set up by Claude** | Send Claude the [setup guide](docs/pro-claude.md); over MCP it adds your projects, links and layout — after you turn the connection on. |

> **Only a human can approve.** There is deliberately no API, MCP tool or setting that lets anything
> approve on your behalf.

## Features

| | |
|---|---|
| **Projects & dev servers** | One tile per project: a click starts its dev server (`npm run dev`, port handling, log) or launches its app. Logos are detected from the project. Running servers get Open / Stop; projects missing on this PC can be cloned with one click. |
| **Notes & tasks** | Quick Markdown notes with categories, checklists (`- [ ]`), images, reminders and per-project filing. Plain `.md` files; optional sync between PCs through your own private git repo. |
| **Command palette** | Websites, folders, files, apps and commands, searchable with `Ctrl+K`. Clipboard history that skips passwords and tokens. Commands with arguments always ask before running until you allow them. |
| **File shelf** | Drop files on the strip to park them, drag them out where you need them. |
| **Personal tab** | Clock, weather (Open-Meteo), calendar (ICS, Outlook, Microsoft or Google account), now playing with volume, microphone mute and audio output — separate from work. |
| **Widgets** | Pin any card to the desktop or a second monitor, or write your own widgets as scripts (xbar-style). GitHub card (via `gh`) and a System card included. |
| **Your layout** | Cards on your own tabs in S/M/L/XL sizes; drag cards between tabs, resize by the edge, undo with `Ctrl+Z`. Bento view for Projects, Repos and Notes. |
| **Setup guide** | On first run it finds your repos, picks apps from the Start menu, connects Claude Code hooks and sets language and theme. Nothing is pre-filled with anyone else's data. |

## The notch, for Windows

NotchNook, Boring Notch and Alcove are macOS-only. Ledge brings the Dynamic Island idea to Windows: choose
how the **closed** Ledge looks at the top edge, and it only grows when something actually happens.

- **Closed-state styles:** Classic strip, Island (black capsule that expands on events), Notch, Dots, Line,
  Icons, Clock, Corner, Ghost (invisible until Claude needs you), Bar, Progress, Ticker, Minimum, Trio — or
  compose your own.
- **On hover:** the full drawer, a compact preview, or an expanded island with the single most important thing.
- **Looks:** 13 design styles, 16 colour themes plus your own, accent colour, density, font size, corner
  radius, translucency and 9 opening animations — including a liquid, Dynamic Island–style capsule.
- **Czech and English** UI.

<img src="docs/screenshots/looks-closed.png" width="860" alt="Eight looks of the closed Ledge at the top edge of the screen">
<img src="docs/screenshots/looks-open.png" width="860" alt="Six design styles of the open drawer with the same data">
<img src="docs/media/styles.gif" width="640" alt="Switching between design styles of the open drawer">

<img src="docs/screenshots/studio-strip.png" width="860" alt="Ledge Studio: the Strip section with a live preview of the closed look">
<img src="docs/screenshots/personal.png" width="560" alt="Personal tab: weather, now playing, calendar, links and notes">

<sub>Sample data. Ledge Studio is the settings window: every change applies at once and can be undone.</sub>

## Make it yours

- **Settings first.** Cards and tabs, the closed-state look, design styles and your own colour themes,
  shortcuts, quick actions and script widgets are all settings — in the app, or in one
  `%APPDATA%\Ledge\config.json` with a generated JSON schema.
- **Fork it.** The code is MIT-licensed and the repo is set up for Claude Code: [`CLAUDE.md`](CLAUDE.md) is its
  entry point (where to start for a new card, a config key, a translation or a settings panel, plus the
  project rules), [`docs/codemap/`](docs/codemap/README.md) maps each area to its files, and
  `node scripts/kde.mjs <symbol>` finds where a symbol lives. Open your fork in Claude Code and ask for the
  change you want. These docs are written in Czech; Claude reads them fine.
- **Agents can configure it too.** [`AGENTS.md`](AGENTS.md) describes the config, note format, local API and
  MCP, so Claude can set up your projects, links and layout without touching the code.

## Install

1. **Download** `Ledge_<version>_x64-setup.exe` from the
   [latest release](https://github.com/jiri-hyan/ledge-app/releases/latest) and run it. It installs for the
   current user into `%LOCALAPPDATA%\Ledge` — no admin rights needed.
2. **Get past SmartScreen.** Windows will probably say *"Windows protected your PC"* because the installer
   isn't code-signed yet. Click **More info → Run anyway**.
3. **Touch the top edge** of the screen with the mouse. The setup guide opens on first run.

**Requirements:** Windows 10 or 11 (x64) with the Microsoft Edge WebView2 runtime (preinstalled on
Windows 11). Claude Code is optional — without it Ledge is a project launcher, notes and quick actions.

### Why the SmartScreen warning — and how to check the installer

SmartScreen warns about any installer that isn't signed with a paid Authenticode certificate; "unknown
publisher" doesn't mean anything was detected. Ledge doesn't have that certificate yet (it's on the
roadmap). What you can verify today:

- **SHA256.** Each release lists the installer's hash in its notes and in `SHA256SUMS.txt`:

  ```powershell
  Get-FileHash .\Ledge_*_x64-setup.exe -Algorithm SHA256
  ```

- **Signature (optional).** Each installer has a `.sig` file — a [minisign](https://jedisct1.github.io/minisign/)
  signature, base64-encoded. With minisign installed:

  ```powershell
  $f = Get-ChildItem .\Ledge_*_x64-setup.exe | Select-Object -First 1
  [IO.File]::WriteAllBytes("$($f.FullName).minisig", [Convert]::FromBase64String((Get-Content "$($f.FullName).sig" -Raw)))
  minisign -Vm $f.FullName -x "$($f.FullName).minisig" -P RWTR3r+4gAkl14XCUqBP8h3nZK8o0eW0JR08g+dOEMiMsZsk1UF90rCQ
  ```

**Updates.** Ledge checks this repo's GitHub releases and shows a "New version" chip. It downloads the
installer over HTTPS, verifies the SHA256 **and** the publisher's signature against a key built into Ledge,
and only installs when you click. Without a valid signature nothing runs.

**Set up with Claude.** Send Claude the link
`https://github.com/jiri-hyan/ledge-app/releases/latest/download/pro-claude.md` ("Install and set up Ledge for me
using this guide"). Claude installs Ledge, connects to it over MCP, asks what you use and sets up shortcuts,
links and layout — after you approve. The connection is off until you turn it on in the setup guide or in
Settings → Claude (MCP), and you can turn it off any time.

**Uninstall.** Windows Settings → Apps → Ledge. Your config (`%APPDATA%\Ledge`) and notes stay.

## Privacy

- **No telemetry, no account, no cloud.** Ledge goes online only for:
  - the update check (`api.github.com`, this repo's releases, every 6 hours by default — can be turned off);
  - weather (`api.open-meteo.com`, only when you set a location);
  - a one-time favicon download for a website shortcut;
  - things you set up yourself: calendar feeds, a Microsoft account (calendar, mail) or Google account
    (calendar), git sync of notes to your own remote;
  - the Anthropic status check in the Claude card — **off by default**.
- Claude Code sessions, hooks, limits and cost estimates are read **locally**. The local API listens on
  `127.0.0.1` only and rejects browser requests.
- Account sign-in happens in your browser; Ledge never sees your password and keeps tokens in Windows
  Credential Manager.
- Notes are plain files in your folder (`%USERPROFILE%\Ledge\notes` by default).

## FAQ

**Is Ledge free?** Yes — free and open source under the MIT license. No paid tier, no account.

**Do I need Claude Code?** No. Without it, Ledge is a project launcher, dev-server manager, notes app and
quick-action drawer. With it, you also get sessions, approvals, questions and limits.

**Can Ledge approve something for Claude on its own?** No, by design. Only your click approves. If you
don't respond in time, the request returns to the terminal.

**Will it get in the way while I type or play?** Ledge never takes keyboard focus when it opens on its own,
and it goes quiet while a fullscreen game is running.

**Is it NotchNook / Dynamic Island for Windows?** In spirit: a drawer and status area at the top of the
screen, with Island and Notch looks and a liquid opening animation — built for Windows and developer
workflows rather than as a clone.

**macOS or Linux?** Not today. Ledge is Windows-only (10 and 11, x64).

**Where are my settings?** In Settings (gear icon) — or as one file, `%APPDATA%\Ledge\config.json`, with a
generated `config.schema.json` next to it. An invalid file is never applied and never silently replaced.

## Roadmap

Work in progress, no dates promised:

- **Ledge Studio** — rebuilt settings for looks and layout (in progress).
- **winget and Scoop** packages — install with one command (in progress).
- **Windows code signing** (Authenticode) — so SmartScreen stops warning (planned).
- **Claude activity** — clearer phases and errors in the live activity card (planned).

Have an idea? [Open an issue](https://github.com/jiri-hyan/ledge-app/issues/new).

## For agents and contributors

Ledge is designed to be configured by AI agents as well as by hand. [`AGENTS.md`](AGENTS.md) describes the
config, the note file format, the local API and the rules (agents may add projects, actions and notes, but
approvals are human-only). Code maps live in [`docs/codemap/`](docs/codemap/README.md).

```
corepack pnpm install
corepack pnpm tauri dev          # development
cd src-tauri && cargo test       # Rust tests
corepack pnpm test:ui            # UI logic tests
powershell -File scripts/release.ps1   # NSIS installer
```

Stack: Tauri 2, Rust, SolidJS, TypeScript. Performance budget: idle CPU below 0.1 %, `ledge.exe` below
25 MB private memory (measured with `scripts/perf.ps1`).

## Support Ledge

If Ledge saves you a few context switches a day, **[give it a star](https://github.com/jiri-hyan/ledge-app)** —
it helps other Windows and Claude Code users find it. Bugs and ideas are welcome in
[Issues](https://github.com/jiri-hyan/ledge-app/issues).

## License

[MIT](LICENSE). Ledge is an independent project, not affiliated with Anthropic or Microsoft.
