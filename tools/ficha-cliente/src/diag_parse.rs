use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dtc {
    pub sistema: String,
    #[serde(default)]
    pub categoria: String,
    #[serde(default)]
    pub zona: String,
    pub codigo: String,
    pub descricao: String,
    pub estado: String,
}

pub const CATEGORY_ORDER: &[&str] = &[
    "Motor",
    "Travões e estabilidade",
    "Painel de instrumentos",
    "Climatização e aquecimento",
    "Segurança (airbag)",
    "Direcção",
    "Portas e fechos",
    "Sistema eléctrico",
    "Conforto e habitáculo",
    "Rede da viatura",
    "Ajuda à condução",
    "Imobilizador",
    "Rádio",
    "Outros sistemas",
];

/// House bucket for a DTC: categoria, else sistema, else Outros.
pub fn dtc_bucket(d: &Dtc) -> String {
    let c = d.categoria.trim();
    if !c.is_empty() {
        return c.to_string();
    }
    let s = d.sistema.trim();
    if !s.is_empty() {
        return s.to_string();
    }
    "Outros sistemas".into()
}

/// Indices of `dtcs` grouped in house category order, then extras.
pub fn group_dtc_indices(dtcs: &[Dtc]) -> Vec<(String, Vec<usize>)> {
    let mut buckets: Vec<(String, Vec<usize>)> = Vec::new();
    for (i, d) in dtcs.iter().enumerate() {
        let cat = dtc_bucket(d);
        if let Some((_, v)) = buckets.iter_mut().find(|(c, _)| *c == cat) {
            v.push(i);
        } else {
            buckets.push((cat, vec![i]));
        }
    }
    buckets.sort_by(|(a, _), (b, _)| {
        let ia = CATEGORY_ORDER
            .iter()
            .position(|c| *c == a.as_str())
            .unwrap_or(99);
        let ib = CATEGORY_ORDER
            .iter()
            .position(|c| *c == b.as_str())
            .unwrap_or(99);
        ia.cmp(&ib).then_with(|| a.cmp(b))
    });
    buckets
}

pub fn grouped_dtcs(dtcs: &[Dtc]) -> Vec<(String, Vec<&Dtc>)> {
    group_dtc_indices(dtcs)
        .into_iter()
        .map(|(cat, idxs)| (cat, idxs.into_iter().map(|i| &dtcs[i]).collect()))
        .collect()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scan {
    pub data: String,
    pub vin: String,
    pub matricula: String,
    pub veiculo: String,
    pub km: String,
    pub dtcs: Vec<Dtc>,
    pub source_name: String,
    #[serde(default)]
    pub titulo: String,
    #[serde(default)]
    pub sumario: String,
    #[serde(default)]
    pub mecanico: String,
    /// `vanguarda` (cards) or `simples` (fundo visível). Empty = vanguarda.
    #[serde(default)]
    pub estilo: String,
    /// `pdf` (Autocom), `live` (vLinker), `forscan-log`. Empty = pdf.
    #[serde(default)]
    pub source: String,
}

pub fn parse_diag_file(path: &Path) -> Result<Scan> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "txt" | "log" => parse_forscan_txt(path),
        _ => parse_autocom_pdf(path),
    }
}

pub fn parse_autocom_pdf(path: &Path) -> Result<Scan> {
    let text = extract_pdf_text(path)
        .with_context(|| format!("não li {}", path.display()))?;
    let mut scan = parse_text(&text);
    if scan.source.is_empty() {
        scan.source = "pdf".into();
    }
    scan.source_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if let Some(parent) = path.parent() {
        let folder = parent
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if scan.matricula.trim().is_empty() && looks_like_plate(&folder) {
            scan.matricula = format_plate(&folder);
        }
    }
    if scan.titulo.trim().is_empty() {
        scan.titulo = default_titulo(&scan);
    }
    Ok(scan)
}

/// Log de texto FORScan (exportação da casa). Não lê XML/as-built.
pub fn parse_forscan_txt(path: &Path) -> Result<Scan> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("não li {}", path.display()))?;
    let mut scan = parse_forscan_text(&text);
    scan.source = "forscan-log".into();
    scan.source_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if scan.titulo.trim().is_empty() {
        scan.titulo = default_titulo(&scan);
    }
    Ok(scan)
}

pub fn parse_forscan_text(text: &str) -> Scan {
    let mut scan = Scan {
        source: "forscan-log".into(),
        ..Default::default()
    };
    for raw in text.lines() {
        let line = raw.trim();
        if let Some(rest) = after_label(line, "Veículo:") {
            parse_forscan_vehicle(&mut scan, rest);
        } else if let Some(rest) = after_label(line, "Veiculo:") {
            parse_forscan_vehicle(&mut scan, rest);
        }
        if let Some(idx) = line.find("VIN:") {
            let v = line[idx + 4..].trim();
            let vin: String = v
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '*')
                .collect();
            if vin.len() >= 11 && scan.vin.is_empty() {
                scan.vin = vin;
            }
        }
        if let Some(rest) = line.split("Módulo encontrado:").nth(1) {
            let _ = rest; // módulo sem DTC ainda assim identifica o carro
        }
        if let Some(rest) = line.split("DTCs dentro ").nth(1) {
            if let Some((mod_name, codes)) = rest.split_once(':') {
                for token in codes.split(',') {
                    if let Some(code) = forscan_code(token) {
                        let sistema = forscan_module_label(mod_name.trim());
                        let (categoria, zona) = classify(&sistema, "");
                        if scan.dtcs.iter().any(|d| d.codigo == code && d.sistema == sistema)
                        {
                            continue;
                        }
                        scan.dtcs.push(Dtc {
                            sistema: sistema.clone(),
                            categoria,
                            zona,
                            codigo: code,
                            descricao: "sem texto público".into(),
                            estado: "Presente".into(),
                        });
                    }
                }
            }
        }
    }
    scan
}

fn after_label<'a>(line: &'a str, label: &str) -> Option<&'a str> {
    let idx = line.find(label)?;
    Some(line[idx + label.len()..].trim())
}

fn parse_forscan_vehicle(scan: &mut Scan, rest: &str) {
    let rest = rest.trim();
    if rest.is_empty() {
        return;
    }
    let veiculo = rest
        .split(", VIN:")
        .next()
        .unwrap_or(rest)
        .trim()
        .to_string();
    if scan.veiculo.is_empty() {
        scan.veiculo = veiculo;
    }
}

fn forscan_module_label(mod_name: &str) -> String {
    let key = mod_name
        .split_whitespace()
        .next()
        .unwrap_or(mod_name)
        .trim()
        .to_ascii_uppercase();
    match key.as_str() {
        "PCM" | "OBD2_PCM" => "PCM — motor".into(),
        "ABS" => "ABS — travões".into(),
        "RCM" => "RCM — airbag".into(),
        "BCM" | "BCMII" => "BCM — sistema eléctrico".into(),
        "IPC" => "IPC — instrumentos".into(),
        "HVAC" => "HVAC — climatização".into(),
        "SASM" | "PSCM" => format!("{key} — direcção"),
        "PAM" => "PAM — ajuda à condução".into(),
        "RFA" => "RFA — conforto".into(),
        other => other.to_string(),
    }
}

fn forscan_code(token: &str) -> Option<String> {
    let t = token.trim().to_ascii_uppercase();
    let alnum: String = t
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    if alnum.len() >= 5 {
        let head = &alnum[..5];
        if matches!(head.as_bytes().first(), Some(b'P' | b'B' | b'C' | b'U'))
            && head[1..].chars().all(|c| c.is_ascii_hexdigit())
        {
            return Some(head.to_string());
        }
    }
    None
}

pub fn default_titulo(scan: &Scan) -> String {
    let who = if !scan.matricula.trim().is_empty() {
        scan.matricula.trim().to_string()
    } else if !scan.veiculo.trim().is_empty() {
        scan.veiculo
            .split(['-', '['])
            .next()
            .unwrap_or(&scan.veiculo)
            .trim()
            .to_string()
    } else {
        "Diagnóstico".into()
    };
    let day = if !scan.data.trim().is_empty() {
        scan.data.trim().to_string()
    } else {
        crate::model::today()
    };
    format!("{who} — {day}")
}

fn looks_like_plate(s: &str) -> bool {
    let t: String = s.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    t.len() >= 5 && t.len() <= 8
}

fn format_plate(s: &str) -> String {
    let t: String = s.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if t.len() == 6 {
        format!("{}-{}-{}", &t[0..2], &t[2..4], &t[4..6])
    } else {
        t
    }
}

fn extract_pdf_text(path: &Path) -> Result<String> {
    let pe = pdf_extract::extract_text(path).unwrap_or_default();
    if looks_like_report(&pe) {
        return Ok(pe);
    }
    #[cfg(windows)]
    {
        if let Ok(t) = extract_windows_ocr(path) {
            if looks_like_report(&t) || looks_useful(&t) {
                return Ok(t);
            }
        }
    }
    if looks_useful(&pe) {
        Ok(pe)
    } else {
        anyhow::bail!(
            "este PDF não tem texto extraível (relatório Autocom é imagem). OCR Windows falhou ou não está disponível."
        )
    }
}

fn looks_useful(t: &str) -> bool {
    t.chars().filter(|c| c.is_alphanumeric()).count() > 40
}

fn looks_like_report(t: &str) -> bool {
    let low = t.to_lowercase();
    [
        "códigos de erro",
        "codigos de erro",
        "matrícula do automóvel",
        "matricula do automovel",
        "sistema electrónico",
        "sistema electronico",
        "sem códigos de avaria",
        "sem codigos de avaria",
        "vela de incandesc",
        "volkswagen",
    ]
    .iter()
    .any(|k| low.contains(k))
}

#[cfg(windows)]
fn extract_windows_ocr(path: &Path) -> Result<String> {
    use windows::core::HSTRING;
    use windows::Data::Pdf::PdfDocument;
    use windows::Globalization::Language;
    use windows::Graphics::Imaging::BitmapDecoder;
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::StorageFile;
    use windows::Storage::Streams::InMemoryRandomAccessStream;

    // WinRT OCR wants STA. cargo test threads are not STA and can AV without this.
    unsafe {
        windows_sys::Win32::System::Com::CoInitializeEx(
            std::ptr::null(),
            windows_sys::Win32::System::Com::COINIT_APARTMENTTHREADED as u32,
        );
    }

    let full = std::fs::canonicalize(path)?;
    let full = full.to_string_lossy().replacen(r"\\?\", "", 1);
    let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(full))?.get()?;
    let pdf = PdfDocument::LoadFromFileAsync(&file)?.get()?;
    let ocr = match OcrEngine::TryCreateFromUserProfileLanguages() {
        Ok(e) => e,
        Err(_) => {
            let lang = Language::CreateLanguage(&HSTRING::from("en"))?;
            OcrEngine::TryCreateFromLanguage(&lang)?
        }
    };
    let mut out = String::new();
    let n = pdf.PageCount()?;
    for i in 0..n {
        let page = pdf.GetPage(i)?;
        let stream = InMemoryRandomAccessStream::new()?;
        page.RenderToStreamAsync(&stream)?.get()?;
        stream.Seek(0)?;
        let dec = BitmapDecoder::CreateAsync(&stream)?.get()?;
        let bmp = dec.GetSoftwareBitmapAsync()?.get()?;
        let r = ocr.RecognizeAsync(&bmp)?.get()?;
        out.push_str(&r.Text()?.to_string());
        out.push('\n');
        page.Close()?;
    }
    Ok(out)
}

/// First page of a PDF as JPEG (WinRT render). Used for the Relatório preview.
#[cfg(windows)]
pub fn render_pdf_first_page_jpeg(path: &Path) -> Result<Vec<u8>> {
    use windows::core::HSTRING;
    use windows::Data::Pdf::PdfDocument;
    use windows::Storage::StorageFile;
    use windows::Storage::Streams::{DataReader, InMemoryRandomAccessStream};

    unsafe {
        windows_sys::Win32::System::Com::CoInitializeEx(
            std::ptr::null(),
            windows_sys::Win32::System::Com::COINIT_APARTMENTTHREADED as u32,
        );
    }
    let full = std::fs::canonicalize(path)?;
    let full = full.to_string_lossy().replacen(r"\\?\", "", 1);
    let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(full.as_str()))?.get()?;
    let pdf = PdfDocument::LoadFromFileAsync(&file)?.get()?;
    if pdf.PageCount()? < 1 {
        anyhow::bail!("PDF sem páginas");
    }
    let page = pdf.GetPage(0)?;
    let stream = InMemoryRandomAccessStream::new()?;
    page.RenderToStreamAsync(&stream)?.get()?;
    stream.Seek(0)?;
    let sz = stream.Size()?;
    let input = stream.GetInputStreamAt(0)?;
    let reader = DataReader::CreateDataReader(&input)?;
    reader.LoadAsync(sz as u32)?.get()?;
    let mut buf = vec![0u8; sz as usize];
    reader.ReadBytes(&mut buf)?;
    page.Close()?;
    let img = ::image::load_from_memory(&buf).map_err(|e| anyhow::anyhow!("preview: {e}"))?;
    let mut jpeg = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut jpeg),
        ::image::ImageOutputFormat::Jpeg(82),
    )
    .map_err(|e| anyhow::anyhow!("preview jpeg: {e}"))?;
    Ok(jpeg)
}

#[cfg(not(windows))]
pub fn render_pdf_first_page_jpeg(_path: &Path) -> Result<Vec<u8>> {
    anyhow::bail!("pré-visualização só no Windows")
}

pub fn norm_id(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

pub fn ids_match(a: &str, b: &str) -> bool {
    let a = norm_id(a);
    let b = norm_id(b);
    !a.is_empty() && a == b
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VeiculoBits {
    pub marca: String,
    pub modelo: String,
    pub versao: String,
    pub ano: String,
}

impl VeiculoBits {
    pub fn label(&self) -> String {
        [&self.marca, &self.modelo, &self.versao, &self.ano]
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub fn parse_text(text: &str) -> Scan {
    let mut scan = Scan::default();
    if let Some(vin) = find_vin(text) {
        scan.vin = vin;
    }
    if let Some(v) = capture_field(text, "Veículo:") {
        scan.veiculo = v;
    } else if let Some(v) = capture_field(text, "Veiculo:") {
        scan.veiculo = v;
    }
    if let Some(v) = capture_field(text, "Data:") {
        scan.data = first_date(&v).unwrap_or(v);
    }
    if let Some(v) = capture_field(text, "Matrícula do automóvel:") {
        if !looks_like_vin_token(&v) {
            scan.matricula = v;
        }
    } else if let Some(v) = capture_field(text, "Matrícula:") {
        if !looks_like_vin_token(&v) {
            scan.matricula = v;
        }
    }
    if let Some(v) = capture_field(text, "Kilometragem:") {
        scan.km = crate::format::format_km(&v);
    } else if let Some(v) = capture_field(text, "Quilometragem:") {
        scan.km = crate::format::format_km(&v);
    }
    if scan.data.is_empty() {
        if let Some(d) = first_date(text) {
            scan.data = d;
        }
    }
    if let Some(idx) = text.find("Volkswagen") {
        let rest = &text[idx..];
        let header = cut_veiculo_header(rest);
        if header.len() > 5 {
            scan.veiculo = header;
        }
    }
    let bits = split_veiculo(&scan.veiculo);
    if !bits.label().is_empty() {
        scan.veiculo = bits.label();
    }
    scan.dtcs = enrich_dtcs(parse_dtcs(text));
    if scan.titulo.trim().is_empty() {
        scan.titulo = default_titulo(&scan);
    }
    scan
}

fn cut_veiculo_header(s: &str) -> String {
    let low = s.to_lowercase();
    let mut end = s.len();
    for needle in [
        "códigos de erro",
        "codigos de erro",
        "sistema electrónico",
        "sistema electronico",
        "sistema electrónico do motor",
        "dtc ",
        " presente",
        " permanente",
    ] {
        if let Some(i) = low.find(needle) {
            end = end.min(i);
        }
    }
    for (i, w) in s.char_indices() {
        if i + 5 <= s.len() {
            let slice = &s[i..];
            if looks_like_dtc_token(slice) {
                end = end.min(i);
                break;
            }
        }
        let _ = w;
    }
    s[..end]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|c: char| c == '-' || c.is_whitespace())
        .to_string()
}

fn looks_like_dtc_token(s: &str) -> bool {
    let t: String = s
        .chars()
        .take(6)
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    (t.len() == 5 && t.starts_with('P') && t[1..].chars().all(|c| c.is_ascii_hexdigit()))
        || (t.len() == 5 && t.chars().all(|c| c.is_ascii_digit()))
}

pub fn split_veiculo(raw: &str) -> VeiculoBits {
    let header = cut_veiculo_header(raw);
    let repaired = repair_generation(&header);
    let ano = extract_year(&repaired)
        .or_else(|| extract_year(raw))
        .unwrap_or_default();
    let mut work = repaired.clone();
    if !ano.is_empty() {
        work = work.replace(&ano, " ");
    }
    let mut versao = String::new();
    if let Some(a) = work.find('[') {
        let rest = &work[a..];
        let close = rest.find(']').unwrap_or(rest.len().min(12));
        versao = rest[..close.min(rest.len())].to_string();
        if !versao.ends_with(']') {
            versao.push(']');
        }
        let after = if close < rest.len() { &rest[close + 1..] } else { "" };
        work = format!("{} {}", &work[..a], after);
    }
    let parts: Vec<&str> = work
        .split(|c: char| c == '-' || c.is_whitespace())
        .map(str::trim)
        .filter(|p| !p.is_empty() && *p != "-" )
        .collect();
    let marca = parts.first().unwrap_or(&"").to_string();
    let modelo = parts
        .iter()
        .skip(1)
        .copied()
        .filter(|p| {
            let l = p.to_lowercase();
            l != "motor"
                && l != "clima"
                && l != "dtc"
                && !l.starts_with('p')
        })
        .collect::<Vec<_>>()
        .join(" ");
    VeiculoBits {
        marca,
        modelo,
        versao,
        ano,
    }
}

fn extract_year(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 4 <= bytes.len() {
        if bytes[i..i + 4].iter().all(|b| b.is_ascii_digit()) {
            let y = std::str::from_utf8(&bytes[i..i + 4]).ok()?;
            if y.starts_with("19") || y.starts_with("20") {
                return Some(y.to_string());
            }
        }
        i += 1;
    }
    None
}

fn repair_generation(s: &str) -> String {
    let Some(start) = s.find('[') else {
        return s.to_string();
    };
    let rest = &s[start + 1..];
    let inner_len = rest.find(']').unwrap_or(rest.len().min(16));
    let inner = &rest[..inner_len];
    let mapped: String = inner
        .chars()
        .map(|c| match c {
            'O' | 'o' => '0',
            'I' | 'l' | '|' => '1',
            c => c,
        })
        .filter(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    let digits: String = mapped.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 4 {
        return s.to_string();
    }
    let gen = format!("[{}-{}]", &digits[0..2], &digits[2..4]);
    let after = if let Some(j) = rest.find(']') {
        rest[j + 1..].to_string()
    } else {
        let skip = inner
            .chars()
            .take_while(|c| {
                matches!(*c, '0'..='9' | 'O' | 'o' | 'I' | 'l' | '|' | '-' | ' ')
            })
            .count();
        rest.get(skip..).unwrap_or("").to_string()
    };
    format!("{}{}{}", &s[..start], gen, after)
}

pub fn enrich_dtcs(dtcs: Vec<Dtc>) -> Vec<Dtc> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for mut d in dtcs {
        d.sistema = scrub_workshop_junk(&d.sistema);
        d.descricao = clean_descricao(&scrub_workshop_junk(&d.descricao));
        d.estado = client_estado(&d.estado);
        let (cat, zona) = classify(&d.sistema, &d.descricao);
        if d.categoria.trim().is_empty() {
            d.categoria = cat;
        }
        if d.zona.trim().is_empty() {
            d.zona = zona;
        }
        if d.codigo.trim().is_empty() || d.codigo == "—" {
            continue;
        }
        let key = format!(
            "{}|{}",
            d.codigo.to_ascii_uppercase(),
            d.descricao.to_lowercase()
        );
        if !seen.insert(key) {
            continue;
        }
        out.push(d);
    }
    out
}

fn client_estado(s: &str) -> String {
    let l = s.to_lowercase();
    if l.contains("intermitente") {
        "Intermitente (pode voltar)".into()
    } else if l.contains("permanente") || l.contains("presente") {
        "Presente".into()
    } else if s.trim().is_empty() {
        "Presente".into()
    } else {
        s.trim().to_string()
    }
}

fn clean_descricao(s: &str) -> String {
    let mut t = s.replace("  ", " ");
    let low = t.to_lowercase();
    if low.contains("sinal referência etapa") || low.contains("sinal referencia etapa") {
        t = t
            .split(" - ")
            .filter(|p| {
                let p = p.to_lowercase();
                !p.contains("sinal referência") && !p.contains("sinal referencia")
            })
            .collect::<Vec<_>>()
            .join(" - ");
    }
    t = t
        .replace("sinal implausivel", "sinal incorrecto")
        .replace("Sinal implausivel", "Sinal incorrecto")
        .replace("Circuito defeituoso", "Circuito com avaria")
        .replace("circuito defeituoso", "circuito com avaria")
        .replace("Curto circuito para positivo ou circuito aberto", "curto-circuito ou circuito aberto")
        .replace("Nenhuma comunicação", "sem comunicação");
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn classify(sistema: &str, descricao: &str) -> (String, String) {
    let zona = extract_zona(sistema);
    let blob = format!("{sistema} {descricao} {zona}").to_lowercase();
    let cat = if contains_any(&blob, &["porta "]) || blob.contains("fecho central") {
        "Portas e fechos"
    } else if contains_any(&blob, &["airbag", "restrições", "restricoes"]) {
        "Segurança (airbag)"
    } else if contains_any(&blob, &["travão", "travao", "abs", "esp"]) {
        "Travões e estabilidade"
    } else if contains_any(
        &blob,
        &[
            "climat",
            "clima (",
            "aquecimento",
            "heater",
            "combustível",
            "combustivel",
        ],
    ) {
        "Climatização e aquecimento"
    } else if contains_any(&blob, &["instrumento"]) {
        "Painel de instrumentos"
    } else if contains_any(&blob, &["direcção", "direcao", "volante"]) {
        "Direcção"
    } else if contains_any(&blob, &["gateway", "bus de dados"]) {
        "Rede da viatura"
    } else if contains_any(&blob, &["conveniência", "conveniencia", "habitáculo", "habitaculo"]) {
        "Conforto e habitáculo"
    } else if contains_any(&blob, &["alternador", "chapa de matrícula", "chapa de matricula"])
        || (blob.contains("electrónico central") || blob.contains("electronico central"))
    {
        "Sistema eléctrico"
    } else if contains_any(&blob, &["adas", "estacionar"]) {
        "Ajuda à condução"
    } else if blob.contains("imobilizador") {
        "Imobilizador"
    } else if contains_any(&blob, &["infotenimento", "radio", "rádio"]) {
        "Rádio"
    } else if contains_any(&blob, &["incandesc", "sistema electrónico do motor", "sistema electronico do motor"])
        || (blob.contains("motor") && !blob.contains("porta"))
    {
        "Motor"
    } else {
        "Outros sistemas"
    };
    (cat.into(), zona)
}

fn contains_any(hay: &str, keys: &[&str]) -> bool {
    keys.iter().any(|k| hay.contains(k))
}

fn extract_zona(sistema: &str) -> String {
    let low = sistema.to_lowercase();
    let mapped = [
        ("porta do lado do condutor", "porta do lado do condutor"),
        ("porta do lado do passageiro", "porta do lado do passageiro"),
        ("porta traseira direita", "porta traseira direita"),
        ("porta traseira esquerda", "porta traseira esquerda"),
        ("aquecimento suplementar", "aquecimento auxiliar"),
        ("aquecimento a combustível", "aquecimento auxiliar"),
        ("aquecimento a combustivel", "aquecimento auxiliar"),
        ("parking", "aquecimento auxiliar"),
    ];
    for (k, z) in mapped {
        if low.contains(k) {
            return z.into();
        }
    }
    String::new()
}

fn scrub_workshop_junk(s: &str) -> String {
    s.split_whitespace()
        .filter(|t| {
            let l = t.to_lowercase();
            !l.contains("autocom")
                && !l.contains("code4bin")
                && !l.contains("cdp+")
                && l != "vci"
                && !l.starts_with("vci:")
                && !l.contains("@gmail")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn ocr_vin_fix(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'I' => '1',
            'O' | 'Q' => '0',
            c => c,
        })
        .collect()
}

fn vin_candidate(raw: &str) -> Option<String> {
    let n = norm_id(raw);
    let fixed = ocr_vin_fix(&n);
    if is_vin(&fixed) {
        return Some(fixed);
    }
    if fixed.len() > 17 {
        let cand: String = fixed.chars().take(17).collect();
        if is_vin(&cand) {
            return Some(cand);
        }
    }
    None
}

fn find_vin(text: &str) -> Option<String> {
    let after = text.split("VIN:").nth(1).unwrap_or("");
    for hay in [after, text] {
        for tok in hay.split_whitespace() {
            if let Some(v) = vin_candidate(tok) {
                return Some(v);
            }
        }
    }
    if let Some(v) = capture_field(text, "VIN:") {
        if let Some(v) = vin_candidate(&v) {
            return Some(v);
        }
    }
    let compact = ocr_vin_fix(&norm_id(text));
    let chars: Vec<char> = compact.chars().collect();
    if chars.len() < 17 {
        return None;
    }
    let mut best: Option<(usize, String)> = None;
    for i in 0..=chars.len() - 17 {
        let s: String = chars[i..i + 17].iter().collect();
        if !is_vin(&s) {
            continue;
        }
        let digits = s.chars().filter(|c| c.is_ascii_digit()).count();
        if best.as_ref().map(|(d, _)| digits > *d).unwrap_or(true) {
            best = Some((digits, s));
        }
    }
    best.map(|(_, s)| s)
}

fn is_vin(s: &str) -> bool {
    if s.len() != 17 {
        return false;
    }
    if !s
        .chars()
        .all(|c| matches!(c, 'A'..='H' | 'J'..='N' | 'P' | 'R'..='Z' | '0'..='9'))
    {
        return false;
    }
    let digits = s.chars().filter(|c| c.is_ascii_digit()).count();
    if digits < 4 {
        return false;
    }
    // "RELEASE2021" glued to a word is 13 letters + year, not a VIN.
    let prefix: String = s.chars().take(13).collect();
    let suffix: String = s.chars().skip(13).collect();
    if prefix.chars().all(|c| c.is_ascii_alphabetic()) && suffix.chars().all(|c| c.is_ascii_digit())
    {
        return false;
    }
    true
}

fn looks_like_vin_token(s: &str) -> bool {
    is_vin(&norm_id(s)) || s.trim().eq_ignore_ascii_case("VIN")
}

fn capture_field(text: &str, label: &str) -> Option<String> {
    let idx = text.find(label)?;
    let rest = &text[idx + label.len()..];
    let stop = [
        "VIN:",
        "Veículo:",
        "Veiculo:",
        "Data:",
        "Matrícula do automóvel:",
        "Matrícula:",
        "Kilometragem:",
        "Quilometragem:",
        "Códigos de erro",
        "Codigos de erro",
        "Mecânico:",
        "Mecanico:",
        "Endereço:",
        "Endereco:",
        "Telefone:",
        "E-mail:",
        "Fax:",
    ];
    let mut collected = String::new();
    for line in rest.lines() {
        let t = line.trim();
        if t.is_empty() {
            if !collected.is_empty() {
                break;
            }
            continue;
        }
        if stop.iter().any(|l| t.starts_with(l)) {
            break;
        }
        if collected.is_empty() {
            collected = t.to_string();
        } else {
            break;
        }
    }
    let collected = collected
        .trim()
        .trim_matches('|')
        .trim()
        .to_string();
    if collected.is_empty() {
        None
    } else {
        Some(collected)
    }
}

fn first_date(s: &str) -> Option<String> {
    let re = regex_lite_date(s)?;
    Some(re)
}

fn regex_lite_date(s: &str) -> Option<String> {
    let chars: Vec<char> = s.chars().collect();
    for i in 0..chars.len().saturating_sub(9) {
        if chars[i].is_ascii_digit()
            && chars[i + 1].is_ascii_digit()
            && chars[i + 2] == '/'
            && chars[i + 3].is_ascii_digit()
            && chars[i + 4].is_ascii_digit()
            && chars[i + 5] == '/'
            && chars[i + 6].is_ascii_digit()
        {
            return Some(chars[i..i + 10].iter().collect());
        }
    }
    None
}

fn parse_dtcs(text: &str) -> Vec<Dtc> {
    let nonempty: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let blob = nonempty.len() < 8 || nonempty.iter().any(|l| l.len() > 180);
    if blob {
        parse_dtcs_tokens(text)
    } else {
        parse_dtcs_lines(text)
    }
}

fn parse_dtcs_lines(text: &str) -> Vec<Dtc> {
    let start = text
        .find("Códigos de erro")
        .or_else(|| text.find("Codigos de erro"))
        .unwrap_or(0);
    let body = &text[start..];
    let mut dtcs = Vec::new();
    let mut sistema = String::new();
    let mut i = 0;
    let lines: Vec<String> = body
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    while i < lines.len() {
        let line = &lines[i];
        if is_system_heading(line) {
            sistema = line
                .trim_start_matches('#')
                .trim()
                .to_string();
            i += 1;
            continue;
        }
        if line.eq_ignore_ascii_case("DTC")
            || line.eq_ignore_ascii_case("Descrição")
            || line.eq_ignore_ascii_case("Descricão")
            || line.contains("Códigos de erro")
            || line.contains("Codigos de erro")
        {
            i += 1;
            continue;
        }
        if let Some(code) = dtc_code(line) {
            let mut desc_parts = Vec::new();
            let mut estado = String::new();
            if line.len() > code.len() {
                let extra = line[code.len()..].trim();
                if !extra.is_empty() && extra != "-" {
                    desc_parts.push(extra.to_string());
                }
            }
            i += 1;
            while i < lines.len() {
                let n = &lines[i];
                if dtc_code(n).is_some() || is_system_heading(n) {
                    break;
                }
                if n.eq_ignore_ascii_case("DTC")
                    || n.eq_ignore_ascii_case("Descrição")
                    || n.eq_ignore_ascii_case("Descricão")
                {
                    i += 1;
                    continue;
                }
                let low = n.to_lowercase();
                if low.contains("sem códigos") || low.contains("sem codigos") {
                    i += 1;
                    continue;
                }
                let mut rest = n.trim_start_matches('-').trim().to_string();
                if low.contains("permanente") {
                    estado = "Permanente".into();
                    rest = rest.replace("Permanente", "").replace("permanente", "");
                } else if low.contains("intermitente") {
                    estado = "Intermitente".into();
                    rest = rest.replace("Intermitente", "").replace("intermitente", "");
                }
                let rest = rest.trim().trim_matches('-').trim();
                if !rest.is_empty() {
                    desc_parts.push(rest.to_string());
                }
                i += 1;
            }
            let descricao = desc_parts.join(" — ");
            if descricao.contains("Sem códigos") && code == "DTC" {
                continue;
            }
            dtcs.push(Dtc {
                sistema: sistema.clone(),
                codigo: code,
                descricao,
                estado,
                ..Default::default()
            });
            continue;
        }
        i += 1;
    }
    dtcs.retain(|d| d.codigo != "—" && !d.codigo.is_empty());
    if dtcs.is_empty()
        && (body.to_lowercase().contains("sem códigos")
            || body.to_lowercase().contains("sem codigos"))
    {
        dtcs.push(Dtc {
            sistema: String::new(),
            codigo: "—".into(),
            descricao: "Sem códigos de avaria".into(),
            estado: String::new(),
            ..Default::default()
        });
    }
    dtcs
}

fn heading_token(tok: &str) -> bool {
    let l = tok.to_lowercase();
    [
        "sistema",
        "travão",
        "travao",
        "instrumento",
        "clima",
        "restrições",
        "restricoes",
        "multifunções",
        "multifuncoes",
        "multifun",
        "gateway",
        "adas",
        "imobilizador",
        "infotenimento",
        "conveniência",
        "conveniencia",
    ]
    .iter()
    .any(|k| l.contains(k))
}

fn is_desc_label(tok: &str) -> bool {
    let l = tok.to_lowercase();
    l.starts_with("descric") || l.eq_ignore_ascii_case("dtc")
}

fn parse_dtcs_tokens(text: &str) -> Vec<Dtc> {
    let start = text
        .find("Códigos de erro")
        .or_else(|| text.find("Codigos de erro"))
        .unwrap_or(0);
    let body = &text[start..];
    let raw: Vec<String> = body.split_whitespace().map(|s| s.to_string()).collect();
    let mut dtcs = Vec::new();
    let mut sistema = String::new();
    let mut i = 0;
    while i < raw.len() {
        if heading_token(&raw[i]) {
            let mut parts = vec![raw[i].clone()];
            i += 1;
            while i < raw.len() {
                if dtc_code(&raw[i]).is_some() || is_desc_label(&raw[i]) {
                    break;
                }
                parts.push(raw[i].clone());
                i += 1;
                if parts.len() > 18 {
                    break;
                }
            }
            sistema = parts.join(" ");
            continue;
        }
        if is_desc_label(&raw[i]) {
            i += 1;
            continue;
        }
        if let Some(first) = dtc_code(&raw[i]) {
            let mut codes = vec![first];
            i += 1;
            while i < raw.len() {
                if let Some(c) = dtc_code(&raw[i]) {
                    codes.push(c);
                    i += 1;
                } else {
                    break;
                }
            }
            while i < raw.len() && is_desc_label(&raw[i]) {
                i += 1;
            }
            let mut chunks: Vec<(String, String)> = Vec::new();
            let mut cur: Vec<String> = Vec::new();
            while i < raw.len() {
                if dtc_code(&raw[i]).is_some() || heading_token(&raw[i]) {
                    break;
                }
                if is_desc_label(&raw[i]) {
                    i += 1;
                    continue;
                }
                let low = raw[i].to_lowercase();
                if low.contains("sem")
                    && i + 1 < raw.len()
                    && (raw[i + 1].to_lowercase().contains("código")
                        || raw[i + 1].to_lowercase().contains("codigo"))
                {
                    while i < raw.len()
                        && dtc_code(&raw[i]).is_none()
                        && !heading_token(&raw[i])
                    {
                        let done = raw[i].to_lowercase().contains("avaria");
                        i += 1;
                        if done {
                            break;
                        }
                    }
                    break;
                }
                if low.contains("permanente") {
                    let t = raw[i]
                        .replace("Permanente", "")
                        .replace("permanente", "");
                    let t = t.trim().trim_matches('-').trim();
                    if !t.is_empty() {
                        cur.push(t.to_string());
                    }
                    chunks.push((cur.join(" "), "Permanente".into()));
                    cur.clear();
                    i += 1;
                    if chunks.len() >= codes.len() {
                        break;
                    }
                    continue;
                }
                if low.contains("intermitente") {
                    let t = raw[i]
                        .replace("Intermitente", "")
                        .replace("intermitente", "");
                    let t = t.trim().trim_matches('-').trim();
                    if !t.is_empty() {
                        cur.push(t.to_string());
                    }
                    chunks.push((cur.join(" "), "Intermitente".into()));
                    cur.clear();
                    i += 1;
                    if chunks.len() >= codes.len() {
                        break;
                    }
                    continue;
                }
                let t = raw[i].trim_start_matches('-').trim();
                if !t.is_empty() && t != "-" {
                    cur.push(t.to_string());
                }
                i += 1;
            }
            if chunks.is_empty() && !cur.is_empty() {
                chunks.push((cur.join(" "), String::new()));
            }
            for (idx, code) in codes.into_iter().enumerate() {
                let (descricao, estado) = chunks.get(idx).cloned().unwrap_or_default();
                dtcs.push(Dtc {
                    sistema: sistema.clone(),
                    codigo: code,
                    descricao,
                    estado,
                    ..Default::default()
                });
            }
            continue;
        }
        i += 1;
    }
    dtcs.retain(|d| d.codigo != "—" && !d.codigo.is_empty());
    dtcs
}

fn is_system_heading(line: &str) -> bool {
    if dtc_code(line).is_some() {
        return false;
    }
    let l = line.to_lowercase();
    if l.contains("códigos de erro") || l.contains("autocom") || l.contains("página") {
        return false;
    }
    if line.len() < 8 {
        return false;
    }
    let keys = [
        "sistema",
        "travão",
        "travao",
        "instrumento",
        "clima",
        "restrições",
        "restricoes",
        "multifun",
        "gateway",
        "adas",
        "motor",
        "airbag",
        "electrónico",
        "electronico",
        "porta",
        "imobilizador",
        "infotenimento",
        "estacionar",
        "conveniência",
        "conveniencia",
        "volante",
        "radio",
        "aquecimento",
    ];
    keys.iter().any(|k| l.contains(k)) && !l.starts_with("-")
}

fn dtc_code(line: &str) -> Option<String> {
    let t = line.trim().trim_start_matches('-').trim();
    let token = t.split_whitespace().next().unwrap_or("");
    let alnum: String = token
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if alnum.len() == 5 && alnum.starts_with('P') && alnum[1..].chars().all(|c| c.is_ascii_hexdigit())
    {
        return Some(alnum);
    }
    if alnum.len() == 5 && alnum.starts_with('0') && alnum.chars().all(|c| c.is_ascii_digit()) {
        return Some(alnum);
    }
    None
}

/// Text for an orçamento notes box: date + sumário, or a few DTCs if there is no carta.
pub fn notes_from_scan(scan: &Scan) -> String {
    let mut out = String::new();
    let data = scan.data.trim();
    if !data.is_empty() {
        out.push_str("Relatório ");
        out.push_str(data);
        out.push('\n');
    }
    let sum = scan.sumario.trim();
    if !sum.is_empty() {
        out.push_str(sum);
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    #[test]
    fn dtc_bucket_prefers_categoria() {
        let d = Dtc {
            categoria: "Motor".into(),
            sistema: "ECM".into(),
            ..Default::default()
        };
        assert_eq!(dtc_bucket(&d), "Motor");
        let d = Dtc {
            sistema: "ABS".into(),
            ..Default::default()
        };
        assert_eq!(dtc_bucket(&d), "ABS");
        assert_eq!(dtc_bucket(&Dtc::default()), "Outros sistemas");
    }

    #[test]
    fn group_dtcs_follows_house_order() {
        let dtcs = vec![
            Dtc {
                categoria: "Portas e fechos".into(),
                codigo: "00096".into(),
                ..Default::default()
            },
            Dtc {
                categoria: "Motor".into(),
                codigo: "P161A".into(),
                ..Default::default()
            },
            Dtc {
                sistema: "Módulo raro".into(),
                codigo: "B0001".into(),
                ..Default::default()
            },
        ];
        let g = group_dtc_indices(&dtcs);
        assert_eq!(g[0].0, "Motor");
        assert_eq!(g[0].1, vec![1]);
        assert_eq!(g[1].0, "Portas e fechos");
        assert_eq!(g[1].1, vec![0]);
        assert_eq!(g[2].0, "Módulo raro");
        assert_eq!(g[2].1, vec![2]);
    }

    #[test]
    fn notes_from_scan_prefers_sumario() {
        let scan = Scan {
            data: "04/09/2026".into(),
            sumario: "Ponteiras e triângulo.".into(),
            dtcs: vec![Dtc {
                codigo: "P161A".into(),
                descricao: "circuito aberto".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let n = notes_from_scan(&scan);
        assert!(n.contains("Relatório 04/09/2026"));
        assert!(n.contains("Ponteiras e triângulo"));
        assert!(!n.contains("P161A"));
    }

    #[test]
    fn notes_from_scan_skips_dtc_wall() {
        let scan = Scan {
            data: "04/09/2026".into(),
            dtcs: vec![Dtc {
                codigo: "P161A".into(),
                descricao: "circuito aberto".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let n = notes_from_scan(&scan);
        assert!(n.contains("Relatório 04/09/2026"));
        assert!(!n.contains("P161A"));
    }

    #[test]
    fn parse_erros_text() {
        let t = include_str!("../tests/fixtures/erros.layout.txt");
        let scan = parse_text(t);
        assert_eq!(scan.vin, "WVGZZZ1TZ9W034244");
        assert_eq!(scan.data, "20/08/2026");
        assert!(scan.veiculo.contains("Touran"), "{}", scan.veiculo);
        assert!(scan.matricula.is_empty(), "empty plate must not steal VIN");
        let codes: Vec<_> = scan.dtcs.iter().map(|d| d.codigo.as_str()).collect();
        assert!(codes.contains(&"P161A"), "{codes:?}");
        assert!(codes.contains(&"P161D"), "{codes:?}");
        assert!(codes.contains(&"01415"), "{codes:?}");
        assert!(codes.contains(&"00096"), "{codes:?}");
        assert!(codes.contains(&"01117"), "{codes:?}");
        assert!(!codes.iter().any(|c| *c == "—"), "skip clean modules: {codes:?}");
        let glow = scan
            .dtcs
            .iter()
            .find(|d| d.codigo == "P161A")
            .unwrap();
        assert_eq!(glow.estado, "Presente");
        assert_eq!(glow.categoria, "Motor");
        assert!(glow.descricao.to_lowercase().contains("incandesc"));
        let door = scan.dtcs.iter().find(|d| d.codigo == "00096").unwrap();
        assert_eq!(door.categoria, "Portas e fechos");
        assert!(door.zona.contains("condutor"), "{}", door.zona);
        let heat = scan.dtcs.iter().find(|d| d.codigo == "01415").unwrap();
        assert_eq!(heat.categoria, "Climatização e aquecimento");
    }

    #[test]
    fn vin_not_kilometragem_blob() {
        let t = "KilometragemVeiculo Volkswagen VIN: WVGZZZ1TZ9W034244";
        assert_eq!(find_vin(t).as_deref(), Some("WVGZZZ1TZ9W034244"));
    }

    #[test]
    fn ids_match_plate() {
        assert!(ids_match("60-HU-86", "60HU86"));
        assert!(!ids_match("", "60HU86"));
    }

    #[test]
    #[ignore]
    fn parse_erros_pdf() {
        let scan = parse_autocom_pdf(&fixture("erros.pdf")).unwrap();
        assert_eq!(scan.vin, "WVGZZZ1TZ9W034244");
        let codes: Vec<_> = scan.dtcs.iter().map(|d| d.codigo.as_str()).collect();
        assert!(codes.contains(&"P161A"), "{codes:?}");
        assert!(codes.contains(&"P161D"), "{codes:?}");
    }

    #[test]
    fn parse_erros_ocr_blob() {
        let t = include_str!("../tests/fixtures/erros.ocr.txt");
        let scan = parse_text(t);
        assert_eq!(scan.vin, "WVGZZZ1TZ9W034244");
        assert_eq!(scan.data, "20/08/2026");
        assert!(scan.veiculo.contains("Touran"), "{}", scan.veiculo);
        let codes: Vec<_> = scan.dtcs.iter().map(|d| d.codigo.as_str()).collect();
        assert!(codes.contains(&"P161A"), "{codes:?}");
        assert!(codes.contains(&"P161D"), "{codes:?}");
        assert!(codes.contains(&"01415"), "{codes:?}");
        assert!(codes.contains(&"00096"), "{codes:?}");
        assert!(codes.contains(&"01117"), "{codes:?}");
        let glow = scan.dtcs.iter().find(|d| d.codigo == "P161A").unwrap();
        assert_eq!(glow.estado, "Presente");
        assert_eq!(glow.categoria, "Motor");
        assert!(glow.descricao.to_lowercase().contains("incandesc"));
        let door = scan.dtcs.iter().find(|d| d.codigo == "00096").unwrap();
        assert_eq!(door.categoria, "Portas e fechos");
        let bits = split_veiculo(&scan.veiculo);
        assert_eq!(bits.marca, "Volkswagen");
        assert_eq!(bits.ano, "2009");
        assert!(bits.versao.contains("03-10"), "{}", bits.versao);
        assert!(!scan.veiculo.to_lowercase().contains("circuito"));
        for d in &scan.dtcs {
            let blob = format!("{} {}", d.sistema, d.descricao).to_lowercase();
            assert!(!blob.contains("autocom"), "{blob}");
            assert!(!blob.contains("code4bin"), "{blob}");
        }
    }

    #[test]
    fn parse_forscan_focus_log() {
        let t = include_str!("../tests/fixtures/forscan-focus.txt");
        let scan = parse_forscan_text(t);
        assert_eq!(scan.vin, "WF0ZZZ0G0FGB63208");
        assert!(scan.veiculo.contains("Focus"), "{}", scan.veiculo);
        let codes: Vec<_> = scan.dtcs.iter().map(|d| d.codigo.as_str()).collect();
        assert!(codes.contains(&"P2598"), "{codes:?}");
        assert!(codes.contains(&"P2463"), "{codes:?}");
        assert!(codes.contains(&"U0401"), "{codes:?}");
        assert!(codes.contains(&"B1182"), "{codes:?}");
        assert_eq!(scan.source, "forscan-log");
        let pcm = scan
            .dtcs
            .iter()
            .find(|d| d.codigo == "P2463")
            .unwrap();
        assert_eq!(pcm.categoria, "Motor");
    }

    #[test]
    fn joana_dump_splits_into_fields() {
        let dump = "Volkswagen - Touran [03-1 OI Motor P161A Presente - 2009 Vela de incandescência do cilindro 1, circuito aberto. P161D Presente";
        let bits = split_veiculo(dump);
        assert_eq!(bits.marca, "Volkswagen", "{bits:?}");
        assert_eq!(bits.modelo, "Touran", "{bits:?}");
        assert_eq!(bits.ano, "2009", "{bits:?}");
        assert!(bits.versao.contains("03-10"), "{}", bits.versao);
        assert!(!bits.modelo.contains("P161A"));
        let scan = parse_text(dump);
        assert!(!scan.veiculo.contains("P161A"), "{}", scan.veiculo);
    }
}
