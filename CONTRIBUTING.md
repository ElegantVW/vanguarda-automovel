# Como contribuir

Este repositório é o cérebro partilhado da Vanguarda Automóvel. Várias pessoas, várias máquinas.

## Papéis

| Papel | Responsabilidade típica neste repo |
|---|---|
| Gerente | Fecha decisões de empresa, preços e investidor |
| Oficina | Atualiza estado de equipamento, testes e demos |
| Atendimento | Contactos, agenda, indicações |
| Design | Identidade, conteúdos, templates |
| Contabilista | Fora do git — só recebe pedidos e devolve comprovativos |

Assina as alterações com o teu papel no commit (`docs: fase 3 — compressor recebido (Oficina)`).

## Fluxo

1. Trabalha numa branch: `fase-3/rececao-stardust`, `plano/precos`, etc.
2. Um assunto por branch. Não mistures Fase 1 legal com reels da Fase 5.
3. Abre um pull request (ou pede revisão) antes de fundir em `main` se a alteração muda uma decisão fechada.
4. Atualiza o estado da checklist (`[ ]` → `[x]`) no mesmo PR.

Remote: [ElegantVW/vanguarda-automovel](https://github.com/ElegantVW/vanguarda-automovel). Sítio unificado no disco: `Documents\Vanguarda\Software`.

Alterações em `tools/`: `cargo test --lib` em `tools/ficha-cliente` (e `cargo test` em `tools/diagnostico` se o crate mudou) antes de fundir.

## O que não entra aqui

- Palavras-passe, chaves, `.env`, `emails.txt`
- PDFs de Finanças, Segurança Social, IBAN, DUAs, ATAs assinadas
- Vídeos, PSD, lotes de fotografias (vão para Elegant Vanguard / Media)
- O arquivo TVDE antigo em `Documentos Empresa`

## Decisões vs ideias

- **Decisão fechada** (tintas Stardust, marketing orgânico, firma, sede) só muda com acordo explícito da equipa e uma linha em [fontes-e-conflitos.md](docs/transcricao/fontes-e-conflitos.md).
- Ideias (pacotes premium, nomes de campanha) ficam marcadas como *proposta* até o Gerente as fechar.

## Idioma e tom

Português europeu. Frases curtas. Checklists no infinitivo (`Confirmar…`, `Enviar…`, `Arquivar…`). Sem “tu” dirigido a uma só pessoa.
