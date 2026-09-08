use crate::auth::Departamento;
use crate::paths::interno_root;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn guias_root(root: &Path) -> PathBuf {
    interno_root(root).join("Guias")
}

pub fn guia_path(root: &Path, desk: Departamento, interno: bool) -> PathBuf {
    guias_root(root)
        .join(desk.slug())
        .join(if interno { "interno.md" } else { "cliente.md" })
}

pub fn load(root: &Path, desk: Departamento, interno: bool) -> String {
    let p = guia_path(root, desk, interno);
    fs::read_to_string(&p).unwrap_or_else(|_| seed_text(desk, interno).to_string())
}

pub fn save(root: &Path, desk: Departamento, interno: bool, body: &str) -> Result<PathBuf> {
    let p = guia_path(root, desk, interno);
    if let Some(dir) = p.parent() {
        fs::create_dir_all(dir).with_context(|| format!("{}", dir.display()))?;
    }
    fs::write(&p, body)?;
    Ok(p)
}

/// Write missing files only. Never clobber Design edits.
pub fn seed(root: &Path) -> Result<usize> {
    let mut n = 0usize;
    for desk in Departamento::all() {
        for interno in [false, true] {
            let p = guia_path(root, desk, interno);
            if p.is_file() {
                continue;
            }
            save(root, desk, interno, seed_text(desk, interno))?;
            n += 1;
        }
    }
    Ok(n)
}

pub fn seed_text(desk: Departamento, interno: bool) -> &'static str {
    match (desk, interno) {
        (Departamento::Care, false) => CARE_CLIENTE,
        (Departamento::Care, true) => CARE_INTERNO,
        (Departamento::Escritorio, false) => ESC_CLIENTE,
        (Departamento::Escritorio, true) => ESC_INTERNO,
        (Departamento::Oficina, false) => OFI_CLIENTE,
        (Departamento::Oficina, true) => OFI_INTERNO,
        (Departamento::Design, false) => DES_CLIENTE,
        (Departamento::Design, true) => DES_INTERNO,
        (Departamento::Interiores, false) => INT_CLIENTE,
        (Departamento::Interiores, true) => INT_INTERNO,
    }
}

const CARE_CLIENTE: &str = r#"# Care — Menu de serviço
Vanguarda Automóvel · IVA incluído

## Pacotes

**Basic · 20€ – 30€**
Refresh essencial. Lavagem exterior à mão, jantes, aspiração interior básica e pó no tablier.

**Classic · 35€ – 55€**
Tudo do Basic, mais aspiração da mala, vidros interiores, dressing nos pneus e condicionador de plásticos.

**Premium · 90€ – 160€**
Tudo do Classic, mais shampoo profundo de estofos, descontaminação e cera da pintura, compartimento do motor e tratamento de odores.

## À la carte
Podes juntar a qualquer pacote, ou marcar só o extra.

- Estofos profundos · 40€ – 70€
- Descontaminação e cera · 30€ – 60€
- Motor (desengordurante) · 20€ – 40€
- Restauro de óticas · 40€ – 65€
- Pelo de animal · 10€ – 25€
- Tratamento de odores · 10€ – 20€

## Condições
- Preços para carro pequeno/médio. SUV, carrinha e 7 lugares: suplemento 5€ a 30€.
- Lama, gordura ou interior muito sujo: taxa «heavy soil» à parte, falada antes de começar.
- Todos os preços incluem IVA.
"#;

const CARE_INTERNO: &str = r#"# Care — Guia interno
O «Basic» tem de ser o mesmo para toda a gente.

## Exterior
- Lavagem: dois baldes, enxaguar, secar a microfibra.
- Jantes: produto de jantes + escova, pó de travão e flanco do pneu.
- Dressing: gel no flanco, preto profundo, sem sujar a jante.
- Descontaminação/cera: clay bar e cera (UV).

## Interior
- Aspiração básica: tapetes da frente e de trás + bancos.
- Mala: incluindo debaixo do tapete.
- Tablier: pano seco ou húmido, só o que se vê.
- Plásticos: UV no tablier e painéis, brilho discreto.
- Vidros: sem riscas, todos os vidros e espelhos.
- Shampoo: extractor nos tecidos e alcatifas.

## Fluxo Basic (30–60 min)
1. Jantes  2. Lavagem e seco  3. Aspiração  4. Pó no tablier  5. Vidros.

## Fluxo Classic (1–2 h)
Basic + mala + vidros a fundo + dressing + plásticos.

## Fluxo Premium (4–8 h)
Classic + clay/cera + extractor + motor + cheiro.

## Preço
- Basic com bancos manchados → Premium ou extra de estofos.
- SUV: suplemento logo na cotação, não no fim.
- Lixo ou lama a sério: avisa o Escritório / responsável antes de começar (heavy soil).
"#;

const ESC_CLIENTE: &str = r#"# Escritório — Como te atendemos
Vanguarda Automóvel

## A visita
1. Deixas a viatura (matrícula, contacto, o que se passa).
2. A oficina lê a viatura e, se fizer falta, um relatório.
3. Recebes orçamento antes de qualquer reparação grande.
4. Quando está pronta, avisamos. Podes pedir Care (lavagem / detalhe) no mesmo dia.

## O que fica na ficha
Nome, contacto, NIF se quiseres fatura, a viatura (matrícula, VIN, km).
Não vendemos os teus dados. Consentimento de marketing é à parte.

## Fatura
Fatura com NIF a pedido. Care: preços do menu já com IVA.
"#;

const ESC_INTERNO: &str = r#"# Escritório — SOP

## Ficha de cliente
Tu e o Design editam pessoa: nome, NIF, morada, consents, conta.
A oficina vê e não grava PII. Não criar cliente a partir da oficina.

## Fila
Orçamento → peças / reparação (oficina) → aguardar cliente → pronto → entregue.
Care: «A aguardar Care» → «Em Care» → «Care pronto». Não mistures as duas filas.

## Orçamento
Confirma tamanho da viatura (SUV) antes de cotar Care.
Heavy soil: fala com o responsável antes de a Care começar.

## Relatório
Só leitura. Pré-visualizar e Abrir PDF. Não importar Autocom nem gravar diagnóstico.

## DJ xona bwé
Pasta bloqueada. Não regravar, não apagar.
"#;

const OFI_CLIENTE: &str = r#"# Oficina — O que fazemos à viatura

## Diagnóstico
Lemos a viatura (Autocom em PDF, quando há). O relatório Vanguarda é nosso: códigos, o que o mecânico viu, próximo passo. Não é o papel da Autocom.

## Reparação
Estado na fila: peças, em reparação, pronto. Recados internos não saem no PDF.

## Pintura de interiores
Plásticos, vinil e tejadilho com o sistema da casa (Stardust / WPU). Tempos de cura: não montar antes de 72 h; cura total 7 dias.
"#;

const OFI_INTERNO: &str = r#"# Oficina — SOP

## Relatório
Escolhe a viatura. Autocom é opcional (larga o PDF). Grava o relatório Vanguarda. Estilo do PDF é do Design (Documentos), não mexas.

## Fila
→ avança: orçamento → peças → reparação → cliente → pronto → entregue.
Não uses os estados Care.

## Saúde
Estrelas 5 = em ordem, 1 = grave. Escritório não edita estrelas.

## Recado
Interno, não vai para o PDF do cliente. Autosave ao sair da caixa.

## Pintura interiores (Documento Geral)
Preparo WPU501: 1:1 plásticos; 1:1,2 tejadilho em névoa. Secar 10–15 min mate. Não lixar o preparo. Pintar nas 2–4 h.
Tinta Stardust Pro PU: 1:0,7 plásticos (ex. 300 ml + 210 ml). Tejadilho mais fluido 1:1 a 1:1,2.
Verniz PU: 1:0,5 plásticos; 1:0,6 tejadilho. 2–3 camadas finas, flash 5–8 min.
Flash cor: 3–5 min; tejadilho 8–10 min. EPI: máscara ABEK, macacão, luvas. 20–25°C, humidade <60%.
Montagem só após 72 h. Checklist completo na pasta Guias (este texto) e no Documento Geral.
"#;

const DES_CLIENTE: &str = r#"# Documentos da casa
Os PDFs da Vanguarda (ficha, relatório, menu Care) usam o estilo da casa: Formal (papel branco, tinta preta) ou Estilizado (metal, ouro, contorno sem fundo preto).
O logótipo e os ícones vão sem caixa preta, sobre o fundo.
"#;

const DES_INTERNO: &str = r#"# Design — SOP

## Documentos
Separador Documentos: Formal / Estilizado, fundo (auto, branco, imagem), título, espaço, tinta, contorno, cantos, logo, corpo, ícones.

## Ícones
Documents\Vanguarda\Design\Icones\ — PNG sem fundo, nomes motor.png, travoes.png, …
O PDF mistura o PNG com o fundo da página (Adobe não quer /SMask).

## Fundos
Design\Fundos\ — JPEG. No Documentos clica a miniatura.

## Utilizadores
Sistema é da **conta do software** (sysadmin), não do Design e não da pessoa Gil. Care e Oficina são Back; Escritório e Design são Front. Interiores é Back reservado (ainda não à venda).

## Guias
Editas os .md em Interno/Guias\. A app não regrava ficheiros que já existem.
"#;

const INT_CLIENTE: &str = r#"# Interiores — serviço em preparação

Back · **ainda não à venda**.

A Vanguarda vai pintar interiores (tejadilho, consola, portas) em vez de substituir. Esse serviço **não está à venda** hoje — não orçamentar packs Bronze/Silver/Gold ao cliente.

Quando o Escritório abrir a mesa, o menu de preços sai daqui. Até lá: anotar o interesse no recado da viatura, sem valor.
"#;

const INT_INTERNO: &str = r#"# Interiores — SOP (reservado)

Mesa Back / Interiores. Serviço ainda não à venda.

A conta do software (sysadmin) cria o login de ensaio. Não é uma mesa Front.

Fluxo (ensaio): pack → extras → desmontar → limpar → WPU → cor → verniz → cura.

Não publiques preços ao cliente. O Escritório não cita Gold a partir desta guia.
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_writes_eight_and_does_not_clobber() {
        let root = std::env::temp_dir().join(format!("v-guias-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        assert_eq!(seed(&root).unwrap(), 10);
        let p = guia_path(&root, Departamento::Care, false);
        assert!(fs::read_to_string(&p).unwrap().contains("IVA incluído"));
        fs::write(&p, "EDITADO").unwrap();
        assert_eq!(seed(&root).unwrap(), 0);
        assert_eq!(fs::read_to_string(&p).unwrap(), "EDITADO");
        fs::remove_dir_all(&root).unwrap();
    }
}
