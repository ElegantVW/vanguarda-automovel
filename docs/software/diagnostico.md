# Diagnóstico nosso (vLinker FS)

Hardware da casa: **Vgate vLinker FS USB**, chip STN/ELM, FTDI `D3C662V2` (visto no log do Focus 1.5 TDCI). Não está trancado ao FORScan.

A app fala com ele em `tools/diagnostico`. O Relatório continua a aceitar PDF Autocom e `.txt` FORScan.

## O que a v1 faz

- Identificar (`ATI`).
- Tensão (`ATRV`); **aborta se < 11 V**.
- OBD-II J1979: VIN (`0902`), DTCs guardados (`03`) e pendentes (`07`), freeze-frame (`0202` se existir), rpm (`010C`).
- Apagar DTCs (`04`) **só depois** do PDF da casa gravado (botão no Relatório).
- Sonda o mapa Ford da casa (`data/ford-modules.vanguarda.json`) com UDS `19 02 AF` em HS (`STP 33`) e MS (`STP 53`). Sem security-access.

## O que nunca entra

- As-built FORScan, coding, PATS, FEPS 18 V, dumps Autocom, keygens.
- Imobilizador como produto. Scan «na rua». Cabos HV de EV.

FORScan grátis / FORScan Lite podem ficar no banco como ferramenta de terceiros. O PDF que o cliente vê é sempre o da Vanguarda.

## Na app

Relatório é a visita: **Novo** → lista de carros a preencher a janela → Autocom (botão ouro ou larga o PDF) → códigos agrupados por sistema → carta → 1–5★ → **Gravar** / **Exportar**. O PDF do cliente: Cinzel nos títulos, Rajdhani no corpo (português com ç ã õ), estrelas desenhadas, ícones sem placa preta, sem bloco Notas. Depois de gravar, **Fazer orçamento** (Escritório/Design). Qualquer mesa da loja (não a conta do software).

«Ler vLinker» e «Simular» ficam em **Outras origens**. Sem dongle, Simular usa o mock (Academia 06) e não toca no carro. Arrastar um `.txt` FORScan preenche o mesmo `Scan`.

## Legal

Protocolos públicos. O mapa Ford cresce com scans **nossos** (Focus da casa primeiro). Não ingerir XML/as-built da pasta Documentos\FORScan.
