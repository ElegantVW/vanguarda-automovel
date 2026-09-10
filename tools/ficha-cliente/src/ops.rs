use serde::{Deserialize, Serialize};

pub const JOB_TIPOS: &[&str] = &["pintura", "oficina", "care", "diagnostico", "pecas"];
pub const JOB_ESTADOS: &[&str] = &[
    "rascunho",
    "agendado",
    "em_curso",
    "pronto",
    "entregue",
    "cancelado",
];
pub const QUOTE_ESTADOS: &[&str] = &["rascunho", "enviado", "aceite", "recusado", "expirado"];
pub const CONTA_ESTADOS: &[&str] = &["rascunho", "emitida", "paga", "anulada"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line {
    #[serde(default)]
    pub desc: String,
    #[serde(default = "one")]
    pub qty: f64,
    #[serde(default)]
    pub cents: i64,
    /// `"peca"` or `"mao_de_obra"`. Empty (old rows) counts as peça.
    #[serde(default)]
    pub kind: String,
    /// Courtesy on this line, 0–100. Unit `cents` stays the list price.
    #[serde(default)]
    pub disc_pct: f64,
    /// Supplier unit cost in euro cents. House only — never on the guest PDF.
    #[serde(default)]
    pub custo_cents: i64,
    /// Markup on custo, 0–100. House only. `cents` = custo × (1 + margem/100).
    #[serde(default)]
    pub margem_pct: f64,
    /// What the operator is typing in Desc. %. Not stored.
    #[serde(skip)]
    pub disc_edit: String,
    #[serde(skip)]
    pub custo_edit: String,
    #[serde(skip)]
    pub margem_edit: String,
}

fn one() -> f64 {
    1.0
}

pub fn clamp_disc_pct(p: f64) -> f64 {
    if !p.is_finite() {
        0.0
    } else {
        p.clamp(0.0, 100.0)
    }
}

/// Empty → 0. Comma or dot. Trailing `%` ok. None if the box is mid-keystroke.
pub fn parse_disc_pct(s: &str) -> Option<f64> {
    let t = s
        .trim()
        .trim_end_matches('%')
        .trim()
        .replace(',', ".")
        .replace(' ', "");
    if t.is_empty() {
        return Some(0.0);
    }
    t.parse::<f64>().ok().map(clamp_disc_pct)
}

pub fn fmt_disc_edit(p: f64) -> String {
    let d = clamp_disc_pct(p);
    if d <= 0.0 {
        String::new()
    } else if (d - d.round()).abs() < 0.05 {
        format!("{}", d.round() as i64)
    } else {
        format!("{d:.1}").replace('.', ",")
    }
}

pub fn gross_cents(qty: f64, unit_cents: i64) -> i64 {
    let q = if qty <= 0.0 { 1.0 } else { qty };
    (unit_cents as f64 * q).round() as i64
}

pub fn net_cents(qty: f64, unit_cents: i64, disc_pct: f64) -> i64 {
    let gross = gross_cents(qty, unit_cents);
    let d = clamp_disc_pct(disc_pct);
    if d <= 0.0 {
        gross
    } else {
        (gross as f64 * (1.0 - d / 100.0)).round() as i64
    }
}

pub fn sell_from_cost(custo_cents: i64, margem_pct: f64) -> i64 {
    if custo_cents <= 0 {
        return 0;
    }
    let m = clamp_disc_pct(margem_pct);
    (custo_cents as f64 * (1.0 + m / 100.0)).round() as i64
}

pub fn looks_usd(s: &str) -> bool {
    let l = s.to_ascii_lowercase();
    l.contains('$') || l.contains("usd")
}

/// Supplier cost. `$` / `usd` uses `usd_eur` (€ per 1 USD). 0 rate + `$` → 0 (caller errors).
pub fn parse_custo_cents(s: &str, usd_eur: f64) -> i64 {
    if looks_usd(s) {
        if usd_eur <= 0.0 {
            return 0;
        }
        let dollars = parse_cents(s) as f64 / 100.0;
        (dollars * usd_eur * 100.0).round() as i64
    } else {
        parse_cents(s)
    }
}

impl Default for Line {
    fn default() -> Self {
        Self {
            desc: String::new(),
            qty: 1.0,
            cents: 0,
            kind: String::new(),
            disc_pct: 0.0,
            custo_cents: 0,
            margem_pct: 0.0,
            disc_edit: String::new(),
            custo_edit: String::new(),
            margem_edit: String::new(),
        }
    }
}

impl Line {
    pub fn peca(desc: impl Into<String>, qty: f64, cents: i64) -> Self {
        Self {
            desc: desc.into(),
            qty: if qty <= 0.0 { 1.0 } else { qty },
            cents,
            kind: "peca".into(),
            disc_pct: 0.0,
            custo_cents: 0,
            margem_pct: 0.0,
            disc_edit: String::new(),
            custo_edit: String::new(),
            margem_edit: String::new(),
        }
    }

    pub fn mao(desc: impl Into<String>, qty: f64, cents: i64) -> Self {
        Self {
            desc: desc.into(),
            qty: if qty <= 0.0 { 1.0 } else { qty },
            cents,
            kind: "mao_de_obra".into(),
            disc_pct: 0.0,
            custo_cents: 0,
            margem_pct: 0.0,
            disc_edit: String::new(),
            custo_edit: String::new(),
            margem_edit: String::new(),
        }
    }

    pub fn is_mao(&self) -> bool {
        self.kind == "mao_de_obra"
    }

    pub fn total_cents(&self) -> i64 {
        net_cents(self.qty, self.cents, self.disc_pct)
    }

    pub fn discount_cents(&self) -> i64 {
        gross_cents(self.qty, self.cents) - self.total_cents()
    }

    pub fn apply_margin(&mut self) {
        if self.is_mao() || self.custo_cents <= 0 {
            return;
        }
        self.cents = sell_from_cost(self.custo_cents, self.margem_pct);
    }

    pub fn peca_custo_total(&self) -> i64 {
        if self.is_mao() {
            0
        } else {
            gross_cents(self.qty, self.custo_cents)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub id: i64,
    pub numero: String,
    pub estado: String,
    pub cliente: String,
    pub matricula: String,
    pub vin: String,
    pub tipo: String,
    pub linhas: Vec<Line>,
    pub total_cents: i64,
    pub iva_incluido: bool,
    pub notas: String,
    pub job_id: Option<i64>,
    pub created: String,
    pub valid_until: String,
    #[serde(default)]
    pub colaborador: String,
    /// € per 1 USD. 0 = unused. House only.
    #[serde(default)]
    pub usd_eur: f64,
    /// Source orçamento when this row is a conta. Unused on quotes.
    #[serde(default)]
    pub parent_quote_id: Option<i64>,
}

impl Default for Quote {
    fn default() -> Self {
        Self {
            id: 0,
            numero: String::new(),
            estado: "rascunho".into(),
            cliente: String::new(),
            matricula: String::new(),
            vin: String::new(),
            tipo: "oficina".into(),
            linhas: Vec::new(),
            total_cents: 0,
            iva_incluido: false,
            notas: String::new(),
            job_id: None,
            created: String::new(),
            valid_until: String::new(),
            colaborador: String::new(),
            usd_eur: 0.0,
            parent_quote_id: None,
        }
    }
}

impl Quote {
    pub fn recompute(&mut self) {
        self.total_cents = self.linhas.iter().map(|l| l.total_cents()).sum();
    }

    pub fn pecas_cents(&self) -> i64 {
        self.linhas
            .iter()
            .filter(|l| !l.is_mao())
            .map(|l| l.total_cents())
            .sum()
    }

    pub fn mao_cents(&self) -> i64 {
        self.linhas
            .iter()
            .filter(|l| l.is_mao())
            .map(|l| l.total_cents())
            .sum()
    }

    pub fn discount_cents(&self) -> i64 {
        self.linhas.iter().map(|l| l.discount_cents()).sum()
    }

    pub fn pecas_custo_cents(&self) -> i64 {
        self.linhas.iter().map(|l| l.peca_custo_total()).sum()
    }

    pub fn lucro_cents(&self) -> i64 {
        self.total_cents - self.pecas_custo_cents()
    }

    pub fn label(&self) -> String {
        let tot = euro(self.total_cents);
        format!("{}  ·  {}  ·  {}  ·  {}", self.numero, self.cliente, self.estado, tot)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: i64,
    pub tipo: String,
    pub estado: String,
    pub cliente: String,
    pub matricula: String,
    pub vin: String,
    pub pack: String,
    pub linhas: Vec<Line>,
    pub previsao_cents: i64,
    pub custo_cents: i64,
    pub agendado: String,
    pub iniciado: String,
    pub entregue: String,
    pub garantia_meses: i64,
    pub notas: String,
    pub quote_id: Option<i64>,
    pub created: String,
    pub updated: String,
    #[serde(default)]
    pub pago: bool,
}

impl Default for Job {
    fn default() -> Self {
        Self {
            id: 0,
            tipo: "oficina".into(),
            estado: "rascunho".into(),
            cliente: String::new(),
            matricula: String::new(),
            vin: String::new(),
            pack: String::new(),
            linhas: Vec::new(),
            previsao_cents: 0,
            custo_cents: 0,
            agendado: String::new(),
            iniciado: String::new(),
            entregue: String::new(),
            garantia_meses: 0,
            notas: String::new(),
            quote_id: None,
            created: String::new(),
            updated: String::new(),
            pago: false,
        }
    }
}

impl Job {
    pub fn from_quote(q: &Quote) -> Self {
        let mut j = Job {
            tipo: q.tipo.clone(),
            estado: "agendado".into(),
            cliente: q.cliente.clone(),
            matricula: q.matricula.clone(),
            vin: q.vin.clone(),
            linhas: q.linhas.clone(),
            previsao_cents: q.total_cents,
            quote_id: Some(q.id),
            garantia_meses: if q.tipo == "pintura" { 24 } else { 0 },
            notas: format!("De {}", q.numero),
            ..Default::default()
        };
        if let Some(l) = q.linhas.first() {
            j.pack = l.desc.clone();
        }
        j
    }

    pub fn label(&self) -> String {
        let plate = if self.matricula.trim().is_empty() {
            "—"
        } else {
            self.matricula.trim()
        };
        format!(
            "#{}  ·  {}  ·  {}  ·  {}  ·  {}",
            self.id,
            self.cliente,
            plate,
            self.tipo,
            self.estado
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sku {
    pub id: i64,
    pub codigo: String,
    pub nome: String,
    pub tipo: String,
    pub qty: f64,
    pub unidade: String,
    pub pedir_cents: i64,
    pub chao_cents: i64,
    pub estado: String,
    pub notas: String,
}

impl Default for Sku {
    fn default() -> Self {
        Self {
            id: 0,
            codigo: String::new(),
            nome: String::new(),
            tipo: "consumo".into(),
            qty: 0.0,
            unidade: "un".into(),
            pedir_cents: 0,
            chao_cents: 0,
            estado: "em_stock".into(),
            notas: String::new(),
        }
    }
}

impl Sku {
    pub fn low(&self) -> bool {
        self.tipo == "stardust" && self.qty > 0.0 && self.qty < 0.25
            || (self.tipo == "peca" && self.estado == "em_stock" && self.qty < 1.0)
    }

    pub fn label(&self) -> String {
        format!(
            "{}  ·  {}  ·  {} {}  ·  {}",
            self.codigo,
            self.nome,
            trim_qty(self.qty),
            self.unidade,
            self.estado
        )
    }
}

fn trim_qty(q: f64) -> String {
    if (q - q.round()).abs() < 0.001 {
        format!("{}", q.round() as i64)
    } else {
        format!("{q:.2}").replace('.', ",")
    }
}

pub fn euro(cents: i64) -> String {
    let neg = cents < 0;
    let c = cents.abs();
    let s = format!("{},{:02} €", c / 100, c % 100);
    if neg {
        format!("−{s}")
    } else {
        s
    }
}

pub fn parse_cents(s: &str) -> i64 {
    let t: String = s
        .chars()
        .map(|c| if c == ',' { '.' } else { c })
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if t.is_empty() {
        return 0;
    }
    if let Some((a, b)) = t.split_once('.') {
        let whole: i64 = a.parse().unwrap_or(0);
        let frac = format!("{:0<2}", &b.chars().take(2).collect::<String>());
        let frac: i64 = frac.parse().unwrap_or(0);
        whole * 100 + frac
    } else {
        t.parse::<i64>().unwrap_or(0) * 100
    }
}

pub fn tipo_label(t: &str) -> &'static str {
    match t {
        "pintura" => "Interiores",
        "oficina" => "Oficina",
        "care" => "Care",
        "diagnostico" => "Diagnóstico",
        "pecas" => "Peças",
        _ => "Outro",
    }
}

pub fn estado_label(e: &str) -> &str {
    match e {
        "rascunho" => "Rascunho",
        "enviado" => "Enviado",
        "aceite" => "Aceite",
        "recusado" => "Recusado",
        "expirado" => "Expirado",
        "agendado" => "Agendado",
        "em_curso" => "Em curso",
        "pronto" => "Pronto",
        "entregue" => "Entregue",
        "cancelado" => "Cancelado",
        "emitida" => "Emitida",
        "paga" => "Paga",
        "anulada" => "Anulada",
        _ => e,
    }
}

pub fn pack_lines(tipo: &str, pack_id: &str) -> Vec<Line> {
    match (tipo, pack_id) {
        ("pintura", "bronze") => vec![Line::mao("Bronze — tejadilho + pilares", 1.0, 29000)],
        ("pintura", "silver") => {
            vec![Line::mao(
                "Silver — tejadilho + pilares + consola + portas",
                1.0,
                49000,
            )]
        }
        ("pintura", "gold") => vec![Line::mao("Gold — Full Black", 1.0, 69000)],
        ("care", "basic") => vec![Line::mao("Care Basic", 1.0, 2500)],
        ("care", "classic") => vec![Line::mao("Care Classic", 1.0, 4500)],
        ("care", "premium") => vec![Line::mao("Care Premium", 1.0, 12500)],
        _ => Vec::new(),
    }
}

pub fn extra_line(id: &str) -> Option<Line> {
    match id {
        "volante" => Some(line("Volante + manete", 9000)),
        "tapetes" => Some(line("Tapetes em tecido", 12000)),
        "cor" => Some(line("Cor personalizada", 12000)),
        "chapa" => Some(line("Chapa-certificado", 5000)),
        "estofos" => Some(line("Estofos profundos", 5500)),
        "cera" => Some(line("Descontaminação e cera", 4500)),
        "motor" => Some(line("Motor (desengordurante)", 3000)),
        "farois" => Some(line("Restauro de óticas", 5000)),
        _ => None,
    }
}

fn line(desc: &str, cents: i64) -> Line {
    Line::peca(desc, 1.0, cents)
}

pub fn format_qty(q: f64) -> String {
    trim_qty(q)
}

pub fn stardust_seed() -> Vec<Sku> {
    const ITEMS: &[(&str, &str)] = &[
        ("WPU001", "White 1 L"),
        ("WPU011", "Pure Black 1 L"),
        ("WPU028", "Vivid Red 1 L"),
        ("WPU007", "Umber 1 L"),
        ("WPU012", "Anthracite Grey 1 L"),
        ("VERNIZ-B", "Verniz brilhante / acetinado 1 L"),
        ("WPU162", "Verniz mate 1 L"),
        ("WPU501", "Preparo de aderência 1 L"),
        ("S4", "Diluente S4"),
        ("S5", "Diluente S5"),
        ("S6", "Diluente S6"),
    ];
    ITEMS
        .iter()
        .map(|(c, n)| Sku {
            codigo: (*c).into(),
            nome: (*n).into(),
            tipo: "stardust".into(),
            qty: 0.0,
            unidade: "L".into(),
            estado: "em_stock".into(),
            notas: "seed Fase 3 — conferir prateleira".into(),
            ..Default::default()
        })
        .collect()
}

/// Parse PRECOS-E-VENDA.txt (âncora vs chão). No PII.
pub fn parse_precos_pecas(text: &str) -> Vec<Sku> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') && line.chars().nth(1) != Some(' ') {
            // keep "12  EGR" rows which start with digits
        }
        let mut parts = line.split_whitespace();
        let Some(num) = parts.next() else {
            continue;
        };
        if num.len() > 2 || !num.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let rest = line[num.len()..].trim();
        let mut euro_bits = rest.split('€');
        let Some(left) = euro_bits.next() else {
            continue;
        };
        let after = euro_bits.next().unwrap_or("").trim();
        let toks: Vec<&str> = left.split_whitespace().collect();
        if toks.len() < 2 {
            continue;
        }
        let pedir = parse_cents(toks[toks.len() - 1]);
        let nome = toks[..toks.len() - 1].join(" ");
        if nome.is_empty() || pedir <= 0 {
            continue;
        }
        let (chao, estado, notas) = if after.to_lowercase().contains("fechado") {
            (pedir, "vendido".into(), "fechado no catálogo".into())
        } else {
            (parse_cents(after), "em_stock".into(), String::new())
        };
        out.push(Sku {
            codigo: format!("FOCUS-{num:0>2}"),
            nome,
            tipo: "peca".into(),
            qty: if estado == "vendido" { 0.0 } else { 1.0 },
            unidade: "un".into(),
            pedir_cents: pedir,
            chao_cents: chao,
            estado,
            notas,
            ..Default::default()
        });
    }
    out
}

pub fn quotes_csv(quotes: &[Quote]) -> String {
    let mut s = String::from("numero;cliente;tipo;estado;total_eur;matricula;data\n");
    for q in quotes {
        s.push_str(&format!(
            "{};{};{};{};{};{};{}\n",
            q.numero,
            q.cliente.replace(';', ","),
            q.tipo,
            q.estado,
            euro(q.total_cents).replace(';', ","),
            q.matricula.replace(';', ","),
            q.created
        ));
    }
    s
}

pub fn saldo_cents(jobs: &[Job], cliente: &str) -> i64 {
    let c = cliente.trim();
    if c.is_empty() {
        return 0;
    }
    jobs.iter()
        .filter(|j| {
            j.cliente.eq_ignore_ascii_case(c)
                && j.estado == "entregue"
                && !j.pago
        })
        .map(|j| j.previsao_cents)
        .sum()
}

pub fn jobs_csv(jobs: &[Job]) -> String {
    let mut s = String::from("id;cliente;tipo;estado;previsao_eur;agendado;matricula\n");
    for j in jobs {
        s.push_str(&format!(
            "{};{};{};{};{};{};{}\n",
            j.id,
            j.cliente.replace(';', ","),
            j.tipo,
            j.estado,
            euro(j.previsao_cents).replace(';', ","),
            j.agendado,
            j.matricula.replace(';', ",")
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cents_roundtrip() {
        assert_eq!(parse_cents("290 €"), 29000);
        assert_eq!(parse_cents("20,50"), 2050);
        assert_eq!(euro(29000), "290,00 €");
    }

    #[test]
    fn line_kind_default_is_peca() {
        let l: Line = serde_json::from_str(r#"{"desc":"Filtro","qty":1,"cents":1200}"#).unwrap();
        assert!(!l.is_mao());
        assert!(l.kind.is_empty());
    }

    #[test]
    fn quote_default_oficina_sem_iva() {
        let q = Quote::default();
        assert_eq!(q.tipo, "oficina");
        assert!(!q.iva_incluido);
    }

    #[test]
    fn from_quote_keeps_plate() {
        let q = Quote {
            numero: "ORC-2026-0002".into(),
            cliente: "Joana".into(),
            matricula: "60-HU-86".into(),
            vin: "WVGZZZ1TZ9W034244".into(),
            tipo: "oficina".into(),
            total_cents: 26128,
            ..Default::default()
        };
        let j = Job::from_quote(&q);
        assert_eq!(j.cliente, "Joana");
        assert_eq!(j.matricula, "60-HU-86");
        assert_eq!(j.vin, "WVGZZZ1TZ9W034244");
        assert_eq!(j.previsao_cents, 26128);
        assert_eq!(j.estado, "agendado");
        assert_eq!(j.tipo, "oficina");
        assert_eq!(Job::default().tipo, "oficina");
    }

    #[test]
    fn quote_splits_pecas_e_mao() {
        let mut q = Quote::default();
        q.linhas = vec![
            Line::peca("Filtro óleo", 1.0, 1200),
            Line::mao("Substituição", 1.0, 3500),
            Line {
                desc: "antiga".into(),
                qty: 2.0,
                cents: 1000,
                kind: String::new(),
                ..Default::default()
            },
        ];
        q.recompute();
        assert_eq!(q.pecas_cents(), 3200);
        assert_eq!(q.mao_cents(), 3500);
        assert_eq!(q.total_cents, 6700);
        let mao_qty = Line {
            desc: "Mão de obra".into(),
            qty: 4.0,
            cents: 12000,
            kind: "mao_de_obra".into(),
            ..Default::default()
        };
        assert_eq!(mao_qty.total_cents(), 48000);
        let tres_horas = Line::mao("Mão de obra", 3.0, 4000);
        assert_eq!(tres_horas.total_cents(), 12000);
    }

    #[test]
    fn line_disc_pct_cuts_total_keeps_unit() {
        let mut l = Line::peca("Kit", 2.0, 2897);
        assert_eq!(l.total_cents(), 5794);
        l.disc_pct = 10.0;
        assert_eq!(l.cents, 2897);
        assert_eq!(l.total_cents(), 5215);
        assert_eq!(l.discount_cents(), 579);
        let old: Line = serde_json::from_str(r#"{"desc":"Filtro","qty":2,"cents":1000}"#).unwrap();
        assert_eq!(old.disc_pct, 0.0);
        assert_eq!(old.total_cents(), 2000);
        l.disc_pct = 150.0;
        assert_eq!(l.total_cents(), 0);
        assert_eq!(parse_disc_pct(""), Some(0.0));
        assert_eq!(parse_disc_pct("10 %"), Some(10.0));
        assert_eq!(parse_disc_pct("10,5"), Some(10.5));
        assert_eq!(parse_disc_pct("."), None);
        assert_eq!(sell_from_cost(2000, 40.0), 2800);
        assert_eq!(parse_custo_cents("$20", 0.92), 1840);
        assert_eq!(parse_custo_cents("$20", 0.0), 0);
        let mut belt = Line::peca("Multi-V", 1.0, 0);
        belt.custo_cents = 2000;
        belt.margem_pct = 40.0;
        belt.apply_margin();
        assert_eq!(belt.cents, 2800);
        assert_eq!(belt.total_cents(), 2800);
    }

    #[test]
    fn saldo_entregue_nao_pago() {
        let mut j = Job {
            cliente: "Ana".into(),
            estado: "entregue".into(),
            previsao_cents: 29000,
            pago: false,
            ..Default::default()
        };
        assert_eq!(saldo_cents(&[j.clone()], "Ana"), 29000);
        j.pago = true;
        assert_eq!(saldo_cents(&[j], "Ana"), 0);
    }

    #[test]
    fn parse_focus_precos() {
        let t = "  12  EGR Ford                          60 €      45 €\n  03  Capas distribuição (par)          12 €     fechado\n";
        let v = parse_precos_pecas(t);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].codigo, "FOCUS-12");
        assert!(v[0].nome.contains("EGR"));
        assert_eq!(v[0].pedir_cents, 6000);
        assert_eq!(v[0].chao_cents, 4500);
        assert_eq!(v[1].estado, "vendido");
    }
}
