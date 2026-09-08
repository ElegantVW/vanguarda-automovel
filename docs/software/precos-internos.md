# Preços internos (uma lista)

A Fase 6 ainda não fechou a tabela pública. Até o Gerente escolher, a app mostra **esta** lista (plano de negócios + Care já na app). Não publicar as duas tabelas do transcript.

IVA: o Care está marcado «IVA incluído». Os packs de interiores abaixo estão como no plano de negócios (o transcript avulso era sem IVA). Até haver um «sim» datado, o Escritório fala os dois lados ao cliente e anota no orçamento.

## Interiores (pintar em vez de substituir)

| Pack | Âmbito | Preço |
|---|---|---|
| Bronze | Tejadilho + pilares | 290 € |
| Silver | Tejadilho + pilares + consola + portas | 490 € |
| Gold | Full Black (peças visíveis) | 690 € |

Extras: volante + manete +90 €; tapetes tecido +120 €; cor personalizada +120 €; chapa-certificado 50 €.

## Care (já na app, IVA incluído)

Basic 20–30 € · Classic 35–55 € · Premium 90–160 €. SUV/carrinha suplemento 5–30 €. Heavy soil à parte.

À la carte: estofos 40–70 €; cera 30–60 €; motor 20–40 €; óticas 40–65 €; pelo 10–25 €; odores 10–20 €.

## Oficina / diagnóstico / peças

Oficina: orçamento por linhas (qtd × €, mão de obra horas × €/h, **Desc. %** por linha). Ainda não há tabela pública fechada. Peças Focus: `Dados\Clientes\Interno\Catálogos\PRECOS-E-VENDA.txt` — âncora de pedido vs chão de haggle, só interno. Packs de Interiores **não se vendem** até o Gerente abrir essa mesa.

Quando o Gerente fechar Fase 6, actualizar **este** ficheiro e as constantes em `model.rs` no mesmo PR. Os orçamentos da app já têm custo de fornecedor + margem % (casa) e Desc. % (cliente, só no PDF se houver corte).
