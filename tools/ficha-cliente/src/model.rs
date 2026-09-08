use serde::{Deserialize, Serialize};

pub const EMPRESA: &str = "Vanguarda Automóvel";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientFicha {
    #[serde(default)]
    pub nome_completo: String,
    #[serde(default)]
    pub nome_preferido: String,
    #[serde(default)]
    pub nif: String,
    #[serde(default)]
    pub data_nascimento: String,
    #[serde(default = "portugues")]
    pub idioma: String,
    #[serde(default)]
    pub telemovel: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub morada: String,
    #[serde(default)]
    pub codigo_postal: String,
    #[serde(default)]
    pub localidade: String,
    #[serde(default)]
    pub consentimento_contacto: String,
    #[serde(default)]
    pub consentimento_marketing: String,
    #[serde(default)]
    pub veiculo_marca: String,
    #[serde(default)]
    pub veiculo_modelo: String,
    #[serde(default)]
    pub veiculo_versao: String,
    #[serde(default)]
    pub veiculo_matricula: String,
    #[serde(default)]
    pub veiculo_ano: String,
    #[serde(default)]
    pub veiculo_cor: String,
    #[serde(default)]
    pub veiculo_vin: String,
    #[serde(default)]
    pub veiculo_km: String,
    #[serde(default)]
    pub data_ultima_visita: String,
    #[serde(default)]
    pub pneus: String,
    #[serde(default)]
    pub observacoes_veiculo: String,
    #[serde(default)]
    pub preferencias: String,
    #[serde(default = "zero_euro")]
    pub saldo_aberto: String,
    #[serde(default = "zero_euro")]
    pub credito: String,
    #[serde(default)]
    pub forma_pagamento: String,
    #[serde(default)]
    pub fatura_com_nif: String,
    #[serde(default)]
    pub notas: String,
    #[serde(default)]
    pub estado: String,
}

impl Default for ClientFicha {
    fn default() -> Self {
        Self {
            nome_completo: String::new(),
            nome_preferido: String::new(),
            nif: String::new(),
            data_nascimento: String::new(),
            idioma: portugues(),
            telemovel: String::new(),
            email: String::new(),
            morada: String::new(),
            codigo_postal: String::new(),
            localidade: String::new(),
            consentimento_contacto: String::new(),
            consentimento_marketing: String::new(),
            veiculo_marca: String::new(),
            veiculo_modelo: String::new(),
            veiculo_versao: String::new(),
            veiculo_matricula: String::new(),
            veiculo_ano: String::new(),
            veiculo_cor: String::new(),
            veiculo_vin: String::new(),
            veiculo_km: String::new(),
            data_ultima_visita: String::new(),
            pneus: String::new(),
            observacoes_veiculo: String::new(),
            preferencias: String::new(),
            saldo_aberto: zero_euro(),
            credito: zero_euro(),
            forma_pagamento: String::new(),
            fatura_com_nif: String::new(),
            notas: String::new(),
            estado: "Rascunho — preencher".to_string(),
        }
    }
}

impl ClientFicha {
    pub fn blank(nome: &str) -> Self {
        let mut f = Self::default();
        f.nome_completo = nome.to_string();
        f.nome_preferido = first_name(nome);
        f
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaffFicha {
    #[serde(default)]
    pub nome: String,
    #[serde(default)]
    pub funcao: String,
    #[serde(default)]
    pub telemovel: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub data_inicio: String,
    #[serde(default)]
    pub notas: String,
}

impl Default for StaffFicha {
    fn default() -> Self {
        Self {
            nome: String::new(),
            funcao: String::new(),
            telemovel: String::new(),
            email: String::new(),
            data_inicio: String::new(),
            notas: String::new(),
        }
    }
}

impl StaffFicha {
    pub fn blank(nome: &str) -> Self {
        let mut f = Self::default();
        f.nome = nome.to_string();
        f
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConsumoEntry {
    #[serde(default)]
    pub data: String,
    #[serde(default)]
    pub tipo: String,
    #[serde(default)]
    pub descricao: String,
    #[serde(default)]
    pub veiculo: String,
    #[serde(default)]
    pub custo_interno: String,
    #[serde(default)]
    pub quem_fez: String,
    #[serde(default)]
    pub notas: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConsumoLog {
    #[serde(default)]
    pub nome: String,
    #[serde(default)]
    pub entradas: Vec<ConsumoEntry>,
}

impl ConsumoLog {
    pub fn blank(nome: &str) -> Self {
        Self {
            nome: nome.to_string(),
            entradas: Vec::new(),
        }
    }
}

pub const DONO_INTERNO: &str = "Interno";

pub const ESTADOS_OFICINA: &[&str] = &[
    "A aguardar orçamento",
    "A aguardar peças",
    "Em reparação",
    "A aguardar cliente",
    "Pronto",
    "Entregue",
    "A aguardar Care",
    "Em Care",
    "Care pronto",
    "A aguardar Interiores",
    "Em Interiores",
    "Interiores pronto",
];

pub const ESTADOS_CARE: &[&str] = &[
    "A aguardar Care",
    "Em Care",
    "Care pronto",
];

pub const ESTADOS_INTERIORES: &[&str] = &[
    "A aguardar Interiores",
    "Em Interiores",
    "Interiores pronto",
];

const OFICINA_CHAIN: &[&str] = &[
    "A aguardar orçamento",
    "A aguardar peças",
    "Em reparação",
    "A aguardar cliente",
    "Pronto",
    "Entregue",
];

/// Next shop-floor state. Stays on «Entregue». Empty → first state.
pub fn next_estado(current: &str) -> &'static str {
    let cur = current.trim();
    if cur.is_empty() {
        return OFICINA_CHAIN[0];
    }
    match OFICINA_CHAIN.iter().position(|e| *e == cur) {
        Some(i) if i + 1 < OFICINA_CHAIN.len() => OFICINA_CHAIN[i + 1],
        Some(_) => OFICINA_CHAIN[OFICINA_CHAIN.len() - 1],
        None => OFICINA_CHAIN[0],
    }
}

pub fn next_estado_care(current: &str) -> &'static str {
    let cur = current.trim();
    if cur.is_empty() {
        return ESTADOS_CARE[0];
    }
    match ESTADOS_CARE.iter().position(|e| *e == cur) {
        Some(i) if i + 1 < ESTADOS_CARE.len() => ESTADOS_CARE[i + 1],
        Some(_) => ESTADOS_CARE[ESTADOS_CARE.len() - 1],
        None => ESTADOS_CARE[0],
    }
}

pub fn next_estado_interiores(current: &str) -> &'static str {
    let cur = current.trim();
    if cur.is_empty() {
        return ESTADOS_INTERIORES[0];
    }
    match ESTADOS_INTERIORES.iter().position(|e| *e == cur) {
        Some(i) if i + 1 < ESTADOS_INTERIORES.len() => ESTADOS_INTERIORES[i + 1],
        Some(_) => ESTADOS_INTERIORES[ESTADOS_INTERIORES.len() - 1],
        None => ESTADOS_INTERIORES[0],
    }
}

/// Packs do plano de negócios. Uma lista — ver docs/software/precos-internos.md.
pub const PINTURA_PACOTES: &[(&str, &str, &str)] = &[
    ("bronze", "Bronze", "290 € — tejadilho + pilares"),
    ("silver", "Silver", "490 € — + consola + portas"),
    ("gold", "Gold", "690 € — Full Black"),
];

pub const PINTURA_EXTRAS: &[(&str, &str, &str)] = &[
    ("volante", "Volante + manete", "+90 €"),
    ("tapetes", "Tapetes em tecido", "+120 €"),
    ("cor", "Cor personalizada", "+120 €"),
    ("chapa", "Chapa-certificado", "50 €"),
];

pub const PINTURA_CHECKS: &[&str] = &[
    "desmontar",
    "limpar",
    "mascarar",
    "aderencia",
    "cor",
    "verniz",
    "cura",
    "montar",
    "fotos",
];

pub fn pintura_check_label(id: &str) -> &'static str {
    match id {
        "desmontar" => "Desmontagem",
        "limpar" => "Limpeza",
        "mascarar" => "Mascaramento",
        "aderencia" => "WPU501 (aderência)",
        "cor" => "Cor",
        "verniz" => "Verniz",
        "cura" => "Cura (72 h / 7 dias)",
        "montar" => "Montagem",
        "fotos" => "Fotos before/after",
        _ => "Passo",
    }
}

pub const CARE_PACOTES: &[(&str, &str, &str)] = &[
    ("basic", "Basic", "20€ – 30€"),
    ("classic", "Classic", "35€ – 55€"),
    ("premium", "Premium", "90€ – 160€"),
];

pub const CARE_EXTRAS: &[(&str, &str, &str)] = &[
    ("estofos", "Estofos profundos", "40€ – 70€"),
    ("cera", "Descontaminação e cera", "30€ – 60€"),
    ("motor", "Motor (desengordurante)", "20€ – 40€"),
    ("farois", "Restauro de óticas", "40€ – 65€"),
    ("pelo", "Pelo de animal", "10€ – 25€"),
    ("cheiro", "Tratamento de odores", "10€ – 20€"),
];

pub const CARE_CHECKS_BASIC: &[&str] = &["wheels", "wash", "vacuum", "dash", "windows"];
pub const CARE_CHECKS_CLASSIC: &[&str] = &["boot", "glass", "tyre", "plastic"];
pub const CARE_CHECKS_PREMIUM: &[&str] = &["clay", "extract", "engine", "scent"];

pub fn care_check_label(id: &str) -> &'static str {
    match id {
        "wheels" => "Jantes limpas",
        "wash" => "Lavagem exterior e seco",
        "vacuum" => "Aspiração interior",
        "dash" => "Pó no tablier",
        "windows" => "Vidros (visto final)",
        "boot" => "Aspiração da mala",
        "glass" => "Todos os vidros polidos",
        "tyre" => "Dressing nos pneus",
        "plastic" => "Condicionador de plásticos",
        "clay" => "Clay bar e cera",
        "extract" => "Extracção de estofos",
        "engine" => "Compartimento do motor",
        "scent" => "Tratamento de cheiro",
        _ => "Passo",
    }
}

pub fn care_checks_for(pacote: &str) -> Vec<&'static str> {
    let mut v: Vec<&str> = CARE_CHECKS_BASIC.to_vec();
    if pacote == "classic" || pacote == "premium" {
        v.extend(CARE_CHECKS_CLASSIC);
    }
    if pacote == "premium" {
        v.extend(CARE_CHECKS_PREMIUM);
    }
    v
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SistemaNota {
    #[serde(default)]
    pub categoria: String,
    #[serde(default)]
    pub estrelas: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Carro {
    #[serde(default)]
    pub dono: String,
    #[serde(default)]
    pub matricula: String,
    #[serde(default)]
    pub vin: String,
    #[serde(default)]
    pub marca: String,
    #[serde(default)]
    pub modelo: String,
    #[serde(default)]
    pub versao: String,
    #[serde(default)]
    pub ano: String,
    #[serde(default)]
    pub cor: String,
    #[serde(default)]
    pub km: String,
    #[serde(default)]
    pub pneus: String,
    #[serde(default)]
    pub notas: String,
    #[serde(default)]
    pub estado_oficina: String,
    #[serde(default)]
    pub pecas_pendentes: String,
    #[serde(default)]
    pub sistemas: Vec<SistemaNota>,
    /// Internal bay note — never printed on the client PDF.
    #[serde(default)]
    pub recado: String,
    #[serde(default)]
    pub recado_quem: String,
    #[serde(default)]
    pub recado_quando: String,
    #[serde(default)]
    pub care_pacote: String,
    #[serde(default)]
    pub care_extras: Vec<String>,
    #[serde(default)]
    pub care_suv: bool,
    #[serde(default)]
    pub care_heavy: bool,
    #[serde(default)]
    pub care_checks: Vec<String>,
    #[serde(default)]
    pub interiores_pacote: String,
    #[serde(default)]
    pub interiores_extras: Vec<String>,
    #[serde(default)]
    pub interiores_checks: Vec<String>,
}

impl Carro {
    pub fn blank(dono: &str) -> Self {
        let mut c = Self::default();
        c.dono = dono.to_string();
        c.ensure_sistemas();
        c
    }

    pub fn ensure_sistemas(&mut self) {
        use crate::diag_parse::CATEGORY_ORDER;
        for cat in CATEGORY_ORDER {
            if !self.sistemas.iter().any(|s| s.categoria == *cat) {
                self.sistemas.push(SistemaNota {
                    categoria: (*cat).to_string(),
                    estrelas: 0,
                });
            }
        }
        self.sistemas.sort_by_key(|s| {
            CATEGORY_ORDER
                .iter()
                .position(|c| *c == s.categoria)
                .unwrap_or(99)
        });
    }

    pub fn label(&self) -> String {
        let plate = self.matricula.trim();
        let vin = self.vin.trim();
        let key = if !plate.is_empty() {
            plate
        } else if !vin.is_empty() {
            vin
        } else {
            "sem matrícula"
        };
        if self.dono.trim().is_empty() {
            key.to_string()
        } else {
            format!("{key}  —  {}", self.dono.trim())
        }
    }
}

fn portugues() -> String {
    "Português".to_string()
}

fn zero_euro() -> String {
    "0,00 €".to_string()
}

fn first_name(nome: &str) -> String {
    nome.split_whitespace()
        .next()
        .unwrap_or(nome)
        .to_string()
}

pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

pub fn now_stamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()
}

/// Internal spend for oficina, escritório and design — wide buckets so a
/// new item almost always fits an existing line.
pub const CONSUMO_TIPOS: &[&str] = &[
    "Peças de viatura",
    "Ferramentas e equipamento",
    "Consumíveis de oficina",
    "EPI e higiene",
    "Café, água e refeições",
    "Papel, toner e escritório",
    "Informática e software",
    "Mobiliário e espaço",
    "Combustível e viatura da casa",
    "Comunicações e envios",
    "Imagem e merchandising",
    "Formação e deslocações",
    "Outro",
];
