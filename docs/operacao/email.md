# Correio da empresa — Vanguarda Automóvel

Sistema escolhido: **Purelymail** no domínio existente **vanguardaautomovel.com**.  
O Google (ou o registador atual) **mantém a propriedade do nome**. Só o correio (registos MX / SPF / DKIM / DMARC) muda para a Purelymail.

Custo: ~10 USD / ano (~0,80 €/mês). Não se paga por endereço.  
Marketing continua orgânico: a Purelymail **não** serve para newsletters.

## Árvore

```
                    partners@ ──────────────────────────┐
                                                        │
geral@ ── cliente? ──► orcamentos@ ─────────────────────┤
   │                                                    ▼
   └── resto ──────► rh@ ─────────────────────────► board@ ──► ceo@
                      ▲                               ▲          ▲
         mechanic@ ───┤                               │          │
         painting@ ───┤                               │          │
         washing@  ───┤                               │          │
         design@  ────┴───────────────────────────────┴──────────┘
```

- Porta pública: `geral@`
- Pedido de cliente / preço: `orcamentos@` (formulário ou encaminhamento humano)
- Interno e equipas: `rh@`
- Design fala com RH **e** com o CEO
- Parceiros e orçamentos sobem a `board@` e depois a `ceo@`

Endereços em ASCII: `orcamentos@`, nunca `orçamentos@`.

## Quem é caixa real vs alias (arrancar)

| Endereço | Tipo agora | Encaminha / lê | Mais tarde |
|---|---|---|---|
| `ceo@` | Caixa real (gerente) | — | mantém |
| `board@` | Alias | `ceo@` | grupo ceo + partners |
| `partners@` | Alias | `board@` | caixa quando houver parceiro |
| `rh@` | Alias → `ceo@` se só uma pessoa; caixa se houver segunda | também recebe as equipas | login RH |
| `geral@` | Alias ou caixa partilhada | quem faz a porta | inbox partilhada |
| `orcamentos@` | Alias / partilhada | cópia para `board@` | login comercial |
| `orcamento@` | Alias legado | `orcamentos@` | manter para contactos antigos |
| `suporte@` | Alias legado | `geral@` | manter |
| `design@` | Alias → `ceo@` + pasta, ou caixa se o Design tiver pessoa | cópia RH e CEO | login Design |
| `mechanic@` `painting@` `washing@` | Aliases | `rh@` | caixas quando houver pessoa |

Não criar caixas vazias “para parecer empresa”. Alias até haver quem as leia.

Triagem «é pedido de cliente?»:

1. Bio / site: «Orçamento → orcamentos@vanguardaautomovel.com»
2. O que cair em `geral@` é encaminhado à mão em segundos. Não há classificador automático neste orçamento.

## Cutover (DNS)

Uma só hospedagem de correio. Não deixar MX no Google **e** na Purelymail.

1. Conta Purelymail → adicionar `vanguardaautomovel.com` → **ainda não** mudar MX.
2. Criar as caixas/aliases da tabela.
3. Exportar o que ainda estiver nas caixas Google (`rh@`, `orcamento@`, `suporte@`).
4. Publicar MX, SPF, DKIM, DMARC (`p=none` na primeira semana).
5. Testar com um telemóvel a enviar para `geral@`.
6. Desligar o correio Google desse domínio.

O site e o Drive não mudam. Só os quatro tipos de registo de correio.

IMAP/SMTP da Purelymail no telemóvel ou Thunderbird/Outlook. A webmail deles é simples; não é o Gmail.

## Fora de âmbito

Helpdesk pago, AI a triar mail, anúncios, segundo domínio (`.ai` / `.pt`) para o correio.
