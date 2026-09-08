# Vanguarda Automóvel

Repositório operacional da **VANGUARDA AUTOMÓVEL, UNIPESSOAL LDA** (NIPC 515461385).

Serviço premium de pintura e restauro de interiores automóveis em Portugal (tecidos, vinil, plásticos e pele), com aerógrafo profissional e sistema Stardust Artistic Pro (WPU). Pintar em vez de substituir.

Este repositório é o sítio vivo dos planos, transcrições, checklists **e do programa de secretária**. Não é o arquivo legal nem o baú de média.

## Como começar

Código e planos: [github.com/ElegantVW/vanguarda-automovel](https://github.com/ElegantVW/vanguarda-automovel) (source-available; ver [LICENSE-SOFTWARE.md](LICENSE-SOFTWARE.md)). Fichas de clientes **não** estão no git.

```bash
git clone git@github.com:ElegantVW/vanguarda-automovel.git
cd vanguarda-automovel
```

Ler: [docs/00-indice.md](docs/00-indice.md) → [plano mestre](docs/plano/plano-mestre.md) → checklist da fase. Manuais da app em [docs/software/](docs/software/).

## Mapa rápido

| Pasta | O que é |
|---|---|
| [docs/legal/](docs/legal/) | Extrato da certidão permanente (sem dump integral) |
| [docs/operacao/](docs/operacao/) | Correio (Purelymail + árvore de endereços) |
| [docs/transcricao/](docs/transcricao/) | Histórico consolidado e tabela de conflitos das conversas antigas |
| [docs/plano/](docs/plano/) | Plano mestre das fases 1–6 |
| [docs/checklists/](docs/checklists/) | Checklist acionável por fase |
| [docs/software/](docs/software/) | Manuais da app (utilizar, Design, PII, compilar, diagnóstico, preços) |
| [tools/ficha-cliente/](tools/ficha-cliente/) | App Windows: Front/Back, Pessoas, Relatório, Care, Trabalho, Agenda, Stock |
| [tools/diagnostico/](tools/diagnostico/) | Crate do vLinker FS — OBD/UDS público, mock no CI |

## Decisões fechadas

- Firma oficial: **VANGUARDA AUTOMÓVEL, UNIPESSOAL LDA** (Insc. 2, 27/07/2026).
- Sede registada: Rua Padre António Vieira nº 7, 4A, 2660-231 Santo António dos Cavaleiros (Loures — freguesia Santo António dos Cavaleiros e Frielas).
- CAE principal (R4): reparação e manutenção de veículos automóveis. Objeto de interiores / oficina / unidade móvel **registado**.
- Ficha legal: [docs/legal/registo-comercial.md](docs/legal/registo-comercial.md).
- Tintas oficiais: **Stardust Artistic Pro Series (WPU)**. SEM fica só como referência histórica.
- Marketing: **100 % orgânico**. Zero anúncios pagos.

## Casa (uma pasta)

Tudo o que a equipa abre no dia-a-dia vive em **`C:\Users\ruela\Documents\Vanguarda`** (disco local, **não** OneDrive — as fichas não podem ir para a nuvem).

| Pasta | O que é | Git? |
|---|---|---|
| `Educacao\` | Curso único (Design, electrónica, mecânica, Rust, faeOS, …) | Índice em [docs/educacao.md](docs/educacao.md); lições no disco |
| `Software\` | Este repositório | Sim |
| `Dados\Clientes\` | Fichas, PII, vault | **Nunca** |
| `Design\` | Ícones, fundos, flyers | Assets; não PII |
| `Empresa\` | Documento Geral, logótipos | Não commitar senhas / `emails.txt` |
| `App\` | atalhos / cópia do exe | Binários fora do git |

O atalho do Ambiente de Trabalho e «Vanguarda» no menu Iniciar abrem `App\Vanguarda.exe`. `alha.toml` ao lado do exe aponta para a casa (`home` + `clientes`). `%LOCALAPPDATA%\Vanguarda` é leftover da instalação antiga — ver `CASA.txt`.

Pessoas da empresa (Gil Salvador, Rodrigo Sousa) ≠ conta do **software** (sysadmin). Front = Escritório + Design; Back = Care + Oficina; Interiores é Back reservado (ainda não à venda).

Clone de trabalho extra (não é a casa): `Projects\vanguarda-automovel`. Editar **um** dos dois.

ElegantVW (faeOS, bulwark, goblin, fairy-lantern): ver [docs/educacao.md](docs/educacao.md) faculdade 12. Não misturar PII da oficina nesses repos.

## Onde vive o resto

| Tipo | Sítio | Não meter no git |
|---|---|---|
| Docs vivos (este repo) | GitHub `ElegantVW/vanguarda-automovel` / `Documents\Vanguarda\Software` | — |
| Academia (lições) | `Documents\Vanguarda\Educacao` | PII |
| Transcripts originais e média | Google Drive → Elegant Vanguard | Logos, vídeos, fotos de viaturas |
| Arquivo legal R&M / fiscal | OneDrive (conta da empresa) | NIF, SS, IBAN, senhas, contratos assinados |

Pasta Drive para exportações futuras do plano: [Plano Mestre 2026](https://drive.google.com/drive/folders/1NnEsJ9HivsgfxgCKOwIHks3zMxQ8Gs9x) (dentro de Elegant Vanguard / Working).

## Regras

- Documentos em português europeu.
- Escrever para a **equipa** (papéis: Gerente, Atendimento, Oficina, Design, Contabilista). Não assumir que o leitor é o pintor.
- Nunca commitar senhas, scans fiscais ou `emails.txt`.
- Ver [CONTRIBUTING.md](CONTRIBUTING.md).
