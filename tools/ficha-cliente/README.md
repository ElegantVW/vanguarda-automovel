# Vanguarda

Programa de secretária: Front/Back, Pessoas, baía, Care, Relatório (visita: Autocom / FORScan txt / vLinker), Trabalho (orçamento com Desc. %), guias. A conta do software (sysadmin) é um login à parte das pessoas.

Dados: `Documents\Vanguarda\Dados\Clientes` via `alha.toml` ao lado do exe (não OneDrive). Pen `X:\Vanguarda\Office\Clientes` ainda é reconhecida se existir.

Gera, por pessoa, `.md`, `.txt`, `.docx` e `.pdf`. O JSON reabre o formulário — **não vai para o git**.

Manuais da casa: `docs/software/`.

## Instalar nesta máquina

```text
target/release/FichaCliente.exe     → Documents\Vanguarda\App\Vanguarda.exe
```

Um só `Vanguarda.exe`. Fechar a app antes de copiar. `vanguarda_setup.exe` só para instalar noutro PC.

Noutro Windows: Setup (não precisa de Rust). Desinstalar em Definições → Aplicações. Backup zip no botão da app (admin).

## Linha de comando

```text
Vanguarda.exe --scaffold
Vanguarda.exe --resave
Vanguarda.exe --exemplos
```

`--scaffold` cria fichas vazias nas pastas que ainda não têm ficheiros. Não substitui pastas com `NAO-REGRAVAR.txt`.

`--resave` volta a gerar documentos a partir dos JSON.

`--exemplos` escreve amostras em `Exemplos\` ao lado do exe.

## Compilar

Toolchain `stable-x86_64-pc-windows-gnu`. Ver `docs/software/compilar.md`.

```text
cd tools\ficha-cliente
cargo test --lib
cargo build --release
```

## Separadores

| Modo | Quem | Pasta |
|---|---|---|
| Início | todos | baía |
| Cliente | Escritório / Design / admin | `Clientes\<Nome>\` |
| Carro | todos (editar segundo mesa) | `…\Carros\<slug>\` |
| Relatório | mesa da loja (não a conta do software) | `Media\Diagnosticos\` |
| Care / Interiores | mesa respectiva | estado na viatura |
| Trabalho / Agenda | Escritório edita | `Interno\ops.sqlite` + `Interno\Operacao\` |
| Stock | Escritório edita | Stardust + peças Focus |
| Guias | todos (editar segundo mesa) | `Interno\Guias\` |
| Pessoas | Front lê; software/admin edita | `Interno\<Nome>\` |
| Documentos / Sistema | Design / conta do software | estilo + logins |

Não copiar NIF, telefones nem JSON de clientes para fora de `Dados\`.
