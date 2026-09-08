//! One-shot: orçamento n.º 1 — Joana, Volkswagen Touran 60-HU-86.
use anyhow::Result;
use ficha_cliente::paths;
use ficha_cliente::writers::{self, OrcamentoComercial, OrcamentoLinha};
use std::path::PathBuf;

fn main() -> Result<()> {
    let dest = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            paths::vanguarda_home()
                .join("Dados")
                .join("Clientes")
                .join("Joana")
                .join("Carros")
                .join("60_HU_86")
                .join("Orcamento_1_04-09-2026.pdf")
        });

    let orc = OrcamentoComercial {
        numero: "1".into(),
        data: "04/09/2026".into(),
        hora: "20:46".into(),
        colaborador: "Gil Salvador".into(),
        departamento: "Oficina".into(),
        cliente_n: "1".into(),
        cliente: "Joana".into(),
        nif: String::new(),
        veiculo: "Volkswagen Touran [03-10] 2009".into(),
        matricula: "60-HU-86".into(),
        vin: "WVGZZZ1TZ9W034244".into(),
        linhas: vec![
            OrcamentoLinha::peca(
                "Kit de rótulas de suspensão (apoio do triângulo)",
                2.0,
                28.974,
            ),
            OrcamentoLinha::peca("Kit de ponteiras de direcção", 2.0, 21.1854),
            OrcamentoLinha::peca(
                "Depósito de expansão do líquido de refrigeração",
                1.0,
                20.986,
            ),
            OrcamentoLinha::mao(
                "Reparação (direcção, triângulo, depósito)",
                3.0,
                40.0,
            ),
        ],
        notas: vec![
            "Diagnóstico 04/09/2026: ponteiras de direcção, apoios do triângulo, depósito de expansão e tubo de saída.".into(),
            "Fora deste orçamento: fecho centralizado e sensores do habitáculo.".into(),
            "Válido até 04/10/2026.".into(),
        ],
    };

    writers::write_orcamento_comercial_pdf(&dest, &orc)?;
    println!("{}", dest.display());
    Ok(())
}
