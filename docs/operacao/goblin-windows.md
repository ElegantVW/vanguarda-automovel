# Goblin / goblind

**goblind** is the mail daemon on **Linux**. This Windows PC does not run it. Grok here has no SSH to that host.

The Goblin **Windows client was uninstalled** on 19 Aug 2026. Do not reinstall unless a later session says so.

Mail **MX is still Purelymail** (`docs/operacao/email.md`). Do not point Squarespace at `mail.vanguardaautomovel.com` until `goblin steal` works against the Linux goblind.

## What lives where

| Piece | Where |
|---|---|
| goblind (server) | Linux — not this PC |
| Goblin client (steal / peek / send) | Uninstalled here; source [ElegantVW/goblin](https://github.com/ElegantVW/goblin) |
| Thunderbird / Purelymail | Day-to-day mail until goblind is proven |
| Desk app | `tools/ficha-cliente` — not mail |

Never commit mailbox passwords, `design.password`, or DKIM keys.

## Later (not this pass)

Linux goblind → prove steal/send → then MX. See Goblin `docs/MIGRATE-PURELYMAIL.md` in that repo.
