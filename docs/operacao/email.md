# Correio da empresa — Vanguarda Automóvel

Sistema escolhido: **Purelymail** no domínio existente **vanguardaautomovel.com**.  
O Google (ou o registador atual) **mantém a propriedade do nome**. Só o correio (registos MX / SPF / DKIM / DMARC) muda para a Purelymail.

Custo: ~10 USD / ano (~0,80 €/mês). Não se paga por endereço.  
Marketing continua orgânico: a Purelymail **não** serve para newsletters.

Endereços em ASCII: `orcamentos@`, nunca `orçamentos@`.

## Equipa atual — 4 caixas reais

Sim: criar **já** estas quatro pessoas na Purelymail (Users / Mailboxes). Cada uma tem password própria. Não partilhar a password da conta admin.

| Pessoa | Caixa (login Thunderbird) |
|---|---|
| CEO / gerente | `ceo@vanguardaautomovel.com` |
| Designer | `designer@vanguardaautomovel.com` |
| RH | `rh@vanguardaautomovel.com` |
| Parceiro Ricardo | `ricardo@vanguardaautomovel.com` |

Aliases de função (não são logins):

| Alias | Entrega em |
|---|---|
| `design@` | `designer@` |
| `partners@` | `ricardo@` |
| `board@` | `ceo@` e `ricardo@` (os dois) |
| `geral@` | `rh@` (porta; RH encaminha) |
| `orcamentos@` e `orcamento@` | `rh@` + cópia `ceo@` até haver comercial |
| `suporte@` | `geral@` → `rh@` |
| `mechanic@` `painting@` `washing@` | `rh@` — só quando houver pessoa na função |

Árvore com nomes reais:

```
geral@ / orcamentos@ ──► rh@ ──► board@ (ceo@ + ricardo@)
designer@ ──► rh@  e  ceo@
ricardo@  ──► board@
```

Podes criar as 4 caixas **antes** de mudar o MX. O Thunderbird liga à Purelymail na mesma. Correio **de fora** (Gmail, clientes) só chega a estes endereços depois do MX apontar para a Purelymail. Até lá, as caixas Google antigas (`rh@`, `orcamento@`, `suporte@`) continuam a receber — exporta-as antes do corte.

## Thunderbird — sim

A Purelymail é IMAP/SMTP normal. O Thunderbird é a escolha certa (e grátis).

Em cada PC: Thunderbird → Conta → Configuração manual:

| | Servidor | Porta | Segurança | Autenticação |
|---|---|---|---|---|
| Receber (IMAP) | `imap.purelymail.com` | **993** | SSL/TLS | Palavra-passe normal |
| Enviar (SMTP) | `smtp.purelymail.com` | **465** | SSL/TLS | Palavra-passe normal |

- **Nome de utilizador:** o endereço completo (`ceo@vanguardaautomovel.com`, não só `ceo`).
- **Palavra-passe:** a da *caixa*, não a da conta de faturação Purelymail.
- Nome a mostrar: `Vanguarda Automóvel` ou o nome da pessoa.

O CEO pode ter as 4 contas no mesmo Thunderbird (Conta → Adicionar conta de correio) para testes. Depois cada pessoa fica só com a sua.

Telemóvel (opcional): mesma tabela IMAP/SMTP na app de Mail.

## Como criar as 4 pessoas na Purelymail

1. Painel Purelymail → domínio `vanguardaautomovel.com` já adicionado.
2. Users / Addresses → criar utilizador: `ceo`, `designer`, `rh`, `ricardo`.
3. Password forte e diferente para cada um. Entregar em mão / gestor de palavras-passe — **não** enviar a password por Gmail.
4. Routing / aliases: `design@` → designer; `partners@` → ricardo; `geral@` → rh; etc.
5. Só depois: MX/SPF/DKIM (ver cutover). Não mudes o MX no mesmo minuto em que crias as contas se ainda precisas do correio antigo no Google.

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
