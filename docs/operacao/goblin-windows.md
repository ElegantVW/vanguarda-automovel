# Goblin on this Windows client

**Removed from this Windows PC on 19 Aug 2026** (Inno uninstaller + `Projects\goblin` clone).  
Mail server is **goblind on Linux**; Windows GUI work continues in another session.  
Do not reinstall the client here unless that session says so.

Mail **MX is still Purelymail**. Do not flip Squarespace to goblind until `goblin steal` works against the Linux server.

## What this machine can do

| Action | How |
|---|---|
| Fetch / send (client) | `goblin steal` `peek` `read` `send` after `summon` |
| GUI | Desktop / `goblin-gui.exe` (you click; Grok cannot drive the GUI) |
| Build (after rustup) | clone `ElegantVW/goblin`, `cargo test`, `cargo build --release` |
| Push | only after `gh auth login` on this PC (same GitHub as Linux) |

Grok **cannot** reach the Linux `goblind` host from here (no SSH). Grok **must not** put mailbox passwords, `design.password`, or DKIM keys in git.

## First summon (Purelymail — design tests)

In PowerShell (you type the mailbox password when asked):

```powershell
$env:Path += ";$env:LOCALAPPDATA\Programs\Goblin"
goblin summon --preset purelymail
goblin who
goblin steal
goblin peek
```

Use a real User (`designer@vanguardaautomovel.com` or `ceo@…`), not an Identidade.

## Status 19 Aug 2026 (this session)

- Clone: `C:\Users\ruela\Projects\goblin` → `origin` `https://github.com/ElegantVW/goblin.git` (`c0a5450`)
- rustup + rustc 1.97.1 + cargo **installed**
- `cargo test` **failed**: no `link.exe` (need Visual Studio Build Tools / C++ workload)
- `gh auth login` **not done** — push will fail until you log in
- `goblin who`: no accounts yet — you must `goblin summon --preset purelymail` and type the mailbox password

## GitHub (both machines, same account)

Windows today: `gh` installed, **not logged in**; no `~/.ssh`; no global git name/email.

1. You: `gh auth login` (GitHub.com, HTTPS or SSH, repo write).
2. `gh auth setup-git`
3. `git config --global user.name` / `user.email` (noreply `@users.noreply.github.com` is fine)
4. Linux: `gh auth status` must show the **same** user/org
5. Remote: `https://github.com/ElegantVW/goblin.git` unless you say otherwise

## Migration later (not this pass)

Linux `goblind` → prove steal/send → then Squarespace MX to `mail.vanguardaautomovel.com`. See Goblin `docs/MIGRATE-PURELYMAIL.md`.
