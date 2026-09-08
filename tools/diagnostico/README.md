# Vanguarda Diagnóstico

Crate da casa para o **vLinker FS USB**. Protocolos públicos (ELM AT, STN `STP`, SAE J1979, ISO 14229). O Relatório da app consome o mesmo JSON que o PDF Autocom.

**Não** copia FORScan, Autocom, as-built, PATS, FEPS, nem keygens. Ver `docs/software/diagnostico.md`.

```text
cargo test
cargo run --release --bin vanguarda-scan -- --mock
cargo run --release --bin vanguarda-scan -- --port COM3
```

Sem `--port`, tenta o FTDI cujo serial é `D3C662V2` (dongle da casa).

HS-CAN: `STP 33`. MS-CAN: `STP 53`. Mapa nosso: `data/ford-modules.vanguarda.json`.

A v1 aborta abaixo de 11 V (`ATRV`), lê VIN + DTCs 03/07, sonda o mapa Ford com UDS `19 02 AF`, e só apaga (`04`) quando a app já gravou o PDF.
