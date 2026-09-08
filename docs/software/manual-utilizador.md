# Manual — usar o Vanguarda no dia-a-dia

Programa de secretária. Dados em `Documents\Vanguarda\Dados\Clientes` (disco local, **não** OneDrive).

Atalho no Ambiente de Trabalho e em `App\Vanguarda.exe`. `alha.toml` ao lado do exe aponta para a casa. A janela abre maximizada. Depois do primeiro arranque, o Vanguarda entra no início de sessão do Windows (esta conta).

## Duas contas, duas coisas

| Conta | O que é |
|---|---|
| **Pessoa** | Trabalho da empresa. Uma pasta + um login em SQLite (`Interno\ops.sqlite`, tabela `pessoa`). Começa por **Gil Salvador**. As outras pessoas adicionam-se em Sistema. |
| **Software** (sysadmin) | O programa. Não é uma mesa. Cria-se em Sistema (nome `Vanguarda`). **Não** é o login de trabalho do Gil. |

Gil trabalha no login **Gil Salvador** (muda a mesa do dia no cabeçalho). A conta do software fica à parte.

## Empresa: Front e Back

| Ala | Mesa | Trabalho |
|---|---|---|
| **Front** | Escritório | Clientes, orçamentos, agenda, stock, CSV |
| **Front** | Design | Identidade, estilo dos PDF, guias internas |
| **Back** | Oficina | Relatório, baía mecânica, diagnóstico |
| **Back** | Care | Detalhe, checklist Care |
| **Back** | Interiores | Reservado. **Ainda não à venda.** Não orçamentar Gold ao cliente. |

A mesa do dia está no cabeçalho. Mudar a mesa muda os separadores e o Início. Não precisas de segundo login para passar de Escritório a Care.

## Início

Mesa do dia, não uma grelha vazia da semana.

- **Front:** orçamentos aceite/enviado/rascunho, ordens sem data, fichas a meio, recados, só os dias da semana que têm OS.
- **Oficina:** fila, recados, últimos relatórios.
- **Care:** fila + recados.
- **Interiores:** reservado.
- **Software:** Sistema, Pessoas, backup. Sem baía.

Um toque abre a ficha. Consumo próprio: botão no Início (não há separador Consumo). O histórico vive em **Pessoas**.

Atalhos: `Ctrl+S` grava o separador; `F5` (e **Actualizar** no cabeçalho) recarrega listas. Há guarda se houver alterações por gravar.

## Cliente / Carro / Relatório / Care / Trabalho / Agenda / Stock / Guias

Nas fichas (Cliente, Carro, Relatório, Trabalho) o primeiro passo é **Abrir / Novo / Apagar**. Procurar fica em Abrir.

Permissões da **mesa**:

- Relatório (editar): qualquer mesa da loja (não a conta do software).
- Care (editar): **Care**.
- Stock (editar) e orçamentos: **Escritório** (e Design).
- Documentos: **Design** (ou conta do software).

## Relatório

É uma **visita**, não um formulário de 6 passos. Qualquer mesa da loja (não a conta do software).

**Novo:** até escolheres o carro, só a lista. Depois: identidade + histórico à esquerda; Autocom, códigos por sistema, carta e 1–5★ à direita. **Abrir PDF Autocom…** (ou larga o PDF na janela). FORScan / vLinker em *Outras origens*.

**Abrir** um relatório gravado abre a mesma visita. **Apagar** pede confirmação.

Rodapé: **Gravar**, **Pré-visualizar**, **Abrir PDF**, **Exportar PDF**. Depois de gravar, **Fazer orçamento** (Escritório/Design) leva o cliente, o carro e a carta para Trabalho. **Apagar DTCs** no adaptador só depois de gravar, e só em vLinker.

## Trabalho (orçamento)

Uma tabela: **Mão de obra** (horas × €/h) e **Peças** (Peça · Qtd · Custo · Margem · Venda · % · Valor). **Contas da casa** fica no cartão à direita (custo, venda, lucro, câmbio USD) e **não sai no PDF**. Venda = custo × (1 + margem %). A coluna **%** na linha é cortesia ao cliente; no PDF **Desc. %** só aparece se houver corte. Margem começa em 40. `$` no custo usa **1 USD = … €**. Sem IVA — não é fatura AT.

**Gravar**, **Exportar PDF** e **Aceitar - trabalho** ficam debaixo do cliente/viatura (sempre visíveis). Aceitar copia linhas e previsão para a ordem. Interiores ainda não à venda.

## Pessoas

Único sítio para humanos. O escritório **pede**; o administrador do software **aprova ou recusa** (com motivo), ou corrige a ficha e aprova.

1. Escritório: **Novo pedido** — nome, função, contactos, mesa. Sem palavra-passe.
2. Admin: em **Pessoas**, no topo da ficha — **Aprovar** (mesa + passe inicial) ou **Recusar** (motivo). A caixa «Activo» em Sistema **não** liga o login.
3. Só depois da aprovação a pessoa aparece no login.

A conta do software não passa por aqui. Sistema já não cria pessoas.

## O que isto não faz

Não emite fatura certificada AT. Recibos na pasta Media são comprovativos. TVDE fica no arquivo OneDrive, fora desta app.
