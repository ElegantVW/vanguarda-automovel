# Academia Vanguarda

Curso único da casa. Lições em disco:

`C:\Users\ruela\Documents\Vanguarda\Educacao\`

Começar por `00-CAMPUS.txt` (também: `00-TRILHOS-POR-MESA.txt`, `00-ORDEM.txt`, `00-GLOSSARIO.txt`, `00-CADERNO.txt`).

Isto não é o IDDA (interiores — Fase 2). É o resto: Design, electrónica, máquinas, código, diagnóstico **nosso**, empresa, e o computador/OS (faeOS).

**Língua:** 01 Design em inglês (curso Inkscape). Campus e faculdades 02–12 em português europeu.

## Faculdades

| # | Pasta | Estado |
|---|--------|--------|
| 01 | Design (Inkscape) | Completo (módulos 1–20) |
| 02 | Electrónica | Completo (12) |
| 03 | Mecânica (oficina + genset + kart) | Completo (14) |
| 04 | Matemática / estatística | Completo (10) |
| 05 | Algoritmos | Completo (10) |
| 06 | Rust | Completo (12) |
| 07 | Base de dados (SQLite) | Completo (10) — ship sqlite depois |
| 08 | Assembly | Completo (10) |
| 09 | Diagnóstico (OBD/UDS/CAN, legal) | Completo (10) — crate em `tools/diagnostico` |
| 10 | Finanças (PT) | Completo (8) |
| 11 | Recursos humanos (PT) | Completo (8) |
| 12 | Computadores + faeOS | Completo (12) |

## ElegantVW (não é a oficina)

GitHub: [ElegantVW](https://github.com/ElegantVW)

| Repo | Função | Relação com a Academia |
|------|--------|-------------------------|
| [faeOS](https://github.com/ElegantVW/faeOS) | OS / ambiente terminal offline-first (Linux) | Faculdade 12 — o sistema que a máquina Vanguarda deve correr |
| [bulwark](https://github.com/ElegantVW/bulwark) | Ward do host (Aegis / Purity / Sentinel) | 12 + 06 Rust |
| [goblin](https://github.com/ElegantVW/goblin) | Correio (IMAP/SMTP TLS) | 12; MX da empresa mais tarde — ver `docs/operacao/goblin-windows.md` |
| [fairy-lantern](https://github.com/ElegantVW/fairy-lantern) | Emulador GBA (ARM7TDMI from scratch) | 08 Assembly + 12 |

Não meter fichas de clientes, NIF, ou senhas de mail nesses repos.

## Diagnóstico

Objectivo: ferramenta **nossa** em Rust, protocolos públicos (OBD-II, ISO 15765, UDS), adaptador vLinker FS. Crate: `tools/diagnostico`. O Relatório importa PDF Autocom, log FORScan (`.txt`) e leitura ao vivo.

Manual da casa: [docs/software/diagnostico.md](software/diagnostico.md).

Proibido na Academia: dump do Autocom, keygen, copiar a base de dados deles.

## PII

`Dados\Clientes` fica no disco local. Nunca OneDrive. Nunca git.
