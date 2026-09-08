# Compilar o Vanguarda

Toolchain **obrigatória:** `stable-x86_64-pc-windows-gnu` (`tools/ficha-cliente/rust-toolchain.toml`). Não misturar com o MSVC do Goblin/ElegantVW neste repo.

```text
cd Documents\Vanguarda\Software\tools\ficha-cliente
cargo test --lib
cargo build --release
```

A janela abre maximizada. O primeiro arranque desta cópia regista o exe no início de sessão (HKCU Run).

Fechar o Vanguarda se estiver aberto, depois um só exe:

```text
target\release\FichaCliente.exe  →  Documents\Vanguarda\App\Vanguarda.exe
```

Sem `Vanguarda-novo.exe` nem cópias extra. Atalho do Ambiente de Trabalho: o mesmo ficheiro, pasta de trabalho = `App\` (lê `alha.toml`).

Manter `App\alha.toml`, ícones e fundos. O `vanguarda_setup.exe` só se for instalar noutro Windows.

Diagnóstico (crate ao lado):

```text
cd ..\diagnostico
cargo test
cargo run --release --bin vanguarda-scan -- --mock
```

`--mock` não precisa de dongle. Live: `vanguarda-scan --port COM3` (ou sem `--port` para auto-detectar FTDI).

Testes: `cargo test --lib` no ficha-cliente **antes** de fundir alterações de código. Fixtures usam matrículas/VIN de exemplo, nunca PII real.
