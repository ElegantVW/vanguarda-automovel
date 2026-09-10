use crate::diag_parse::{self, Dtc, Scan};
use crate::docstyle::DocStyle;
use crate::fonts;
use crate::media;
use crate::vault;
use crate::model::{today, Carro, ClientFicha, ConsumoLog, SistemaNota, StaffFicha, EMPRESA};
use crate::ops::{self, Job, Quote};
use crate::slug::slug;
use anyhow::{Context, Result};
use docx_rs::*;
use printpdf::path::{PaintMode, WindingOrder};
use printpdf::*;
use serde::Serialize;
use std::fs::{self, File};
use std::io::{BufWriter, Cursor, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
pub struct WriteOpts {
    pub skip_existing: bool,
}

impl Default for WriteOpts {
    fn default() -> Self {
        Self {
            skip_existing: false,
        }
    }
}

struct Spec {
    title: String,
    subtitle: String,
    meta: Vec<(String, String)>,
    sections: Vec<Section>,
    footer: String,
    logo: Option<PathBuf>,
    photos: Vec<(String, Vec<u8>)>,
}

struct Section {
    heading: String,
    rows: Vec<(String, String)>,
    bullets: Vec<String>,
    table: Option<(Vec<String>, Vec<Vec<String>>)>,
}

pub fn write_client(dir: &Path, ficha: &ClientFicha, opts: WriteOpts) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(dir)?;
    let _ = media::ensure_client_media(dir);
    let spec = spec_client(ficha, dir);
    let base = format!("Ficha_Cliente_{}", slug(&ficha.nome_completo));
    let mut written = write_spec(dir, &base, &spec, &opts)?;
    let mut disk = ficha.clone();
    if let Ok(key) = vault::load_or_create_key(&vault::data_root_guess(dir)) {
        vault::protect_client(&key, &mut disk);
    }
    if let Some(p) = write_json(dir, "ficha.json", &disk, opts.skip_existing)? {
        written.push(p);
    }
    Ok(written)
}

pub fn write_staff(dir: &Path, ficha: &StaffFicha, opts: WriteOpts) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(dir)?;
    let _ = media::ensure_staff_media(dir);
    let spec = spec_staff(ficha, dir);
    let base = format!("Ficha_Staff_{}", slug(&ficha.nome));
    let mut written = write_spec(dir, &base, &spec, &opts)?;
    let mut disk = ficha.clone();
    if let Ok(key) = vault::load_or_create_key(&vault::data_root_guess(dir)) {
        vault::protect_staff(&key, &mut disk);
    }
    if let Some(p) = write_json(dir, "staff.json", &disk, opts.skip_existing)? {
        written.push(p);
    }
    Ok(written)
}

pub fn write_consumo(dir: &Path, log: &ConsumoLog, opts: WriteOpts) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(dir)?;
    let _ = media::ensure_staff_media(dir);
    let spec = spec_consumo(log, dir);
    let base = format!("Consumo_Interno_{}", slug(&log.nome));
    let mut written = write_spec(dir, &base, &spec, &opts)?;
    if let Some(p) = write_json(dir, "consumo.json", log, opts.skip_existing)? {
        written.push(p);
    }
    Ok(written)
}

pub fn write_carro(dir: &Path, car: &Carro, opts: WriteOpts) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(dir)?;
    let _ = media::ensure_car_media(dir);
    let spec = spec_carro(car, dir);
    let base = format!("Ficha_Viatura_{}", slug(&car_key_label(car)));
    let mut written = write_spec(dir, &base, &spec, &opts)?;
    if let Some(p) = write_json(dir, "carro.json", car, opts.skip_existing)? {
        written.push(p);
    }
    Ok(written)
}

fn car_key_label(car: &Carro) -> String {
    if !car.matricula.trim().is_empty() {
        car.matricula.clone()
    } else if !car.vin.trim().is_empty() {
        car.vin.clone()
    } else {
        "Viatura".into()
    }
}

fn spec_carro(car: &Carro, dir: &Path) -> Spec {
    let mut meta = vec![("Empresa".into(), EMPRESA.into())];
    meta.extend(rows(&[
        ("Dono", &car.dono),
        ("Matrícula", &car.matricula),
        ("VIN", &car.vin),
        ("Marca", &car.marca),
        ("Modelo", &car.modelo),
        ("Versão", &car.versao),
        ("Ano", &car.ano),
        ("Cor", &car.cor),
        ("Quilometragem", &car.km),
        ("Pneus", &car.pneus),
        ("Estado oficina", &car.estado_oficina),
        ("Peças pendentes", &car.pecas_pendentes),
    ]));
    let mut sections = Vec::new();
    let star_rows: Vec<(String, String)> = car
        .sistemas
        .iter()
        .filter(|s| s.estrelas > 0)
        .map(|s| {
            let n = s.estrelas.min(5) as usize;
            let bar = format!("{}{}", "●".repeat(n), "○".repeat(5 - n));
            (s.categoria.clone(), bar)
        })
        .collect();
    if !star_rows.is_empty() {
        sections.push(Section {
            heading: "Estado da viatura (5 em ordem, 1 grave)".into(),
            rows: star_rows,
            bullets: vec![],
            table: None,
        });
    }
    push_sec(&mut sections, "Notas".into(), vec![], vec![car.notas.clone()]);
    push_sec(
        &mut sections,
        "Media na pasta".into(),
        vec![],
        media::media_bullets(dir),
    );
    Spec {
        title: "Ficha de viatura".into(),
        subtitle: car.label(),
        meta,
        sections,
        footer: format!("Última atualização: {}  ·  {}", today(), EMPRESA),
        logo: media::find_logo(),
        photos: vec![],
    }
}

pub fn diagnostic_basename(scan: &Scan) -> String {
    let key = if !scan.titulo.trim().is_empty() {
        slug(&scan.titulo)
    } else if !scan.matricula.trim().is_empty() {
        slug(&scan.matricula)
    } else if !scan.vin.trim().is_empty() {
        slug(&scan.vin)
    } else {
        "Scan".into()
    };
    let day = if !scan.data.trim().is_empty() {
        scan.data.replace('/', "-")
    } else {
        today()
    };
    format!("Diagnostico_{key}_{day}")
}

pub fn write_diagnostico(
    dest_dir: &Path,
    basename: &str,
    scan: &Scan,
    cliente: &str,
) -> Result<Vec<PathBuf>> {
    write_diagnostico_stars(dest_dir, basename, scan, cliente, &[])
}

pub fn write_diagnostico_stars(
    dest_dir: &Path,
    basename: &str,
    scan: &Scan,
    cliente: &str,
    stars: &[SistemaNota],
) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(dest_dir)?;
    let mut scan = scan.clone();
    scan.dtcs = diag_parse::enrich_dtcs(scan.dtcs);
    let spec = spec_diag(&scan, cliente, stars);
    let mut written = Vec::new();
    for (ext, write_fn) in [
        ("md", write_md as fn(&Path, &Spec) -> Result<()>),
        ("txt", write_txt),
        ("docx", write_docx),
    ] {
        let path = dest_dir.join(format!("{basename}.{ext}"));
        write_fn(&path, &spec).with_context(|| format!("a escrever {}", path.display()))?;
        written.push(path);
    }
    let pdf_path = dest_dir.join(format!("{basename}.pdf"));
    write_diag_pdf(&pdf_path, &scan, cliente, stars, &[])?;
    written.push(pdf_path);
    if let Some(p) = write_json(dest_dir, &format!("{basename}.json"), &scan, false)? {
        written.push(p);
    }
    Ok(written)
}

fn star_bar(n: u8) -> String {
    let n = n.min(5) as usize;
    format!("{}{}", "*".repeat(n), "-".repeat(5 - n))
}

fn meta_pair(label: &str, value: &str) -> Option<(String, String)> {
    let t = value.trim();
    if t.is_empty() || t == "[●]" {
        None
    } else {
        Some((label.into(), t.into()))
    }
}

fn grouped_dtcs(dtcs: &[Dtc]) -> Vec<(String, Vec<&Dtc>)> {
    diag_parse::grouped_dtcs(dtcs)
}

fn spec_diag(scan: &Scan, cliente: &str, stars: &[SistemaNota]) -> Spec {
    let mut sections = Vec::new();
    let star_rows: Vec<(String, String)> = stars
        .iter()
        .filter(|s| s.estrelas > 0)
        .map(|s| (s.categoria.clone(), star_bar(s.estrelas)))
        .collect();
    if !star_rows.is_empty() {
        sections.push(Section {
            heading: "Estado da viatura (5 em ordem, 1 grave)".into(),
            rows: star_rows,
            bullets: vec![],
            table: None,
        });
    }
    if scan.dtcs.is_empty() {
        sections.push(Section {
            heading: "Avarias".into(),
            rows: vec![],
            bullets: vec!["Nenhuma avaria listada.".into()],
            table: None,
        });
    } else {
        for (cat, items) in grouped_dtcs(&scan.dtcs) {
            let mut bullets = Vec::new();
            for d in items {
                let where_ = if d.zona.trim().is_empty() {
                    String::new()
                } else {
                    format!(" ({})", d.zona.trim())
                };
                bullets.push(format!(
                    "{}{}  ·  {}  —  {}",
                    d.codigo, where_, d.estado, d.descricao
                ));
            }
            sections.push(Section {
                heading: cat,
                rows: vec![],
                bullets,
                table: None,
            });
        }
    }
    sections.push(Section {
        heading: "Nota".into(),
        rows: vec![],
        bullets: vec![
            "Leitura das avarias da viatura.".into(),
            "A reparação deve ser confirmada pelo técnico.".into(),
        ],
        table: None,
    });
    let mut meta = Vec::new();
    if let Some(p) = meta_pair("Cliente", cliente) {
        meta.push(p);
    }
    if let Some(p) = meta_pair("Data", &scan.data) {
        meta.push(p);
    }
    if let Some(p) = meta_pair("Matrícula", &scan.matricula) {
        meta.push(p);
    }
    if let Some(p) = meta_pair("VIN", &scan.vin) {
        meta.push(p);
    }
    if let Some(p) = meta_pair("Veículo", &scan.veiculo) {
        meta.push(p);
    }
    if let Some(p) = meta_pair("Quilometragem", &scan.km) {
        meta.push(p);
    }
    if let Some(p) = meta_pair("Mecânico", &scan.mecanico) {
        meta.push(p);
    }
    if filled(&scan.sumario) {
        sections.insert(
            0,
            Section {
                heading: "Sumário do mecânico".into(),
                rows: vec![],
                bullets: wrap(&scan.sumario, 88),
                table: None,
            },
        );
    }
    Spec {
        title: "Relatório de diagnóstico".into(),
        subtitle: if cliente.trim().is_empty() {
            scan.veiculo.clone()
        } else {
            cliente.to_string()
        },
        meta,
        sections,
        footer: format!("{}  ·  Vanguarda Automóvel Unipessoal Lda", today()),
        logo: media::find_logo(),
        photos: vec![],
    }
}

fn pdf_gold() -> printpdf::Color {
    printpdf::Color::Rgb(Rgb::new(0.831, 0.690, 0.416, None))
}
#[allow(dead_code)]
fn pdf_gold_dim() -> printpdf::Color {
    printpdf::Color::Rgb(Rgb::new(0.72, 0.58, 0.32, None))
}
fn pdf_page_black() -> printpdf::Color {
    printpdf::Color::Rgb(Rgb::new(0.055, 0.047, 0.039, None))
}
fn pdf_page_white() -> printpdf::Color {
    printpdf::Color::Rgb(Rgb::new(1.0, 1.0, 1.0, None))
}

thread_local! {
    static PDF_STYLE: std::cell::RefCell<DocStyle> = std::cell::RefCell::new(DocStyle::default());
}

pub fn set_house_style(style: DocStyle) {
    PDF_STYLE.with(|c| *c.borrow_mut() = style);
}

pub fn house_style() -> DocStyle {
    PDF_STYLE.with(|c| c.borrow().clone())
}

fn pdf_simples() -> bool {
    !house_style().show_cards()
}
fn pdf_ink_black() -> printpdf::Color {
    printpdf::Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None))
}

const CARD_X0: f32 = 14.0;
const CARD_X1: f32 = 196.0;
const CARD_PAD: f32 = 4.5;
const CARD_GAP: f32 = 3.5;
#[cfg(test)]
const CARD_RADIUS: f32 = 3.0;
const PAGE_TOP: f32 = 282.0;
const PAGE_BOT: f32 = 16.0;
const ICON_MM: f32 = 8.0;
const HEAD_ROW: f32 = 10.0;

fn load_pdf_bg_jpeg() -> Option<Vec<u8>> {
    let st = house_style();
    if !st.use_image_bg() {
        return None;
    }
    let p = if st.fundo.trim().eq_ignore_ascii_case("imagem") {
        media::find_fundo_bg()
            .or_else(media::find_simples_bg)
            .or_else(media::find_pdf_bg)
    } else {
        media::find_pdf_bg().or_else(media::find_fundo_bg)
    }?;
    let b = fs::read(p).ok()?;
    if b.len() < 64 {
        None
    } else {
        Some(b)
    }
}

fn draw_page_metal(
    doc: &PdfDocumentReference,
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    bg: &Option<Vec<u8>>,
) {
    let fill = if house_style().white_paper() {
        pdf_page_white()
    } else {
        pdf_page_black()
    };
    fill_rect(doc, page, layer, 0.0, 0.0, 210.0, 297.0, fill);
    let Some(jpeg) = bg.as_ref() else {
        return;
    };
    let (px_w, px_h) = jpeg_dimensions(jpeg);
    if px_w < 2.0 || px_h < 2.0 {
        return;
    }
    let pdf_img = printpdf::Image::from(ImageXObject {
        width: Px(px_w as usize),
        height: Px(px_h as usize),
        color_space: ColorSpace::Rgb,
        bits_per_component: ColorBits::Bit8,
        interpolate: true,
        image_data: jpeg.clone(),
        image_filter: Some(ImageFilter::DCT),
        smask: None,
        clipping_bbox: None,
    });
    let native_w = px_w / 300.0 * 25.4;
    let native_h = px_h / 300.0 * 25.4;
    let current = doc.get_page(page).get_layer(layer);
    pdf_img.add_to_layer(
        current,
        printpdf::ImageTransform {
            translate_x: Some(Mm(0.0)),
            translate_y: Some(Mm(0.0)),
            scale_x: Some(210.0 / native_w.max(1.0)),
            scale_y: Some(297.0 / native_h.max(1.0)),
            ..Default::default()
        },
    );
}

#[derive(Clone)]
struct InkLine {
    text: String,
    size: f32,
    indent: f32,
    dim: bool,
    gap: f32,
}

impl InkLine {
    fn gold(text: impl Into<String>, size: f32, indent: f32, gap: f32) -> Self {
        Self {
            text: text.into(),
            size,
            indent,
            dim: false,
            gap,
        }
    }
    fn dim(text: impl Into<String>, size: f32, indent: f32, gap: f32) -> Self {
        Self {
            text: text.into(),
            size,
            indent,
            dim: true,
            gap,
        }
    }
}

fn ink_wrap(s: &str, max: usize, size: f32, indent: f32, dim: bool, gap: f32) -> Vec<InkLine> {
    wrap(s, max)
        .into_iter()
        .filter(|l| !l.is_empty())
        .map(|l| InkLine {
            text: l,
            size,
            indent,
            dim,
            gap,
        })
        .collect()
}

fn is_gold_pixel(r: u8, g: u8, b: u8) -> bool {
    r >= 140 && g >= 90 && b <= 190 && r >= g && (r as i16 - b as i16) >= 40
}

/// Keep gold line-art, punch the dark plate to alpha so it sits on the velvet.
fn gold_art_transparent(img: ::image::RgbaImage) -> ::image::RgbaImage {
    let mut out = img;
    for p in out.pixels_mut() {
        if is_gold_pixel(p.0[0], p.0[1], p.0[2]) {
            p.0[3] = 255;
            continue;
        }
        p.0[3] = 0;
    }
    out
}

fn page_patch(
    bg: &Option<Vec<u8>>,
    x_mm: f32,
    y_top_mm: f32,
    w_mm: f32,
    h_mm: f32,
    out_w: u32,
    out_h: u32,
) -> ::image::RgbImage {
    let white = house_style().white_paper();
    let fill = if white {
        ::image::Rgb([255, 255, 255])
    } else {
        ::image::Rgb([0x0E, 0x0C, 0x0A])
    };
    let out_w = out_w.max(1);
    let out_h = out_h.max(1);
    let mut out = ::image::RgbImage::from_pixel(out_w, out_h, fill);
    let Some(jpeg) = bg else {
        return out;
    };
    let Ok(src) = ::image::load_from_memory(jpeg) else {
        return out;
    };
    let src = src.to_rgb8();
    let sw = src.width().max(1) as f32;
    let sh = src.height().max(1) as f32;
    let x0 = (x_mm / 210.0 * sw).max(0.0);
    let y0 = ((297.0 - y_top_mm) / 297.0 * sh).max(0.0);
    let pw = (w_mm / 210.0 * sw).max(1.0);
    let ph = (h_mm / 297.0 * sh).max(1.0);
    for dy in 0..out_h {
        for dx in 0..out_w {
            let sx = (x0 + dx as f32 / out_w as f32 * pw) as u32;
            let sy = (y0 + dy as f32 / out_h as f32 * ph) as u32;
            if sx < src.width() && sy < src.height() {
                out.put_pixel(dx, dy, *src.get_pixel(sx, sy));
            }
        }
    }
    out
}

fn flatten_rgba_on_page(
    rgba: &::image::RgbaImage,
    bg: &Option<Vec<u8>>,
    x_mm: f32,
    y_top_mm: f32,
    w_mm: f32,
    h_mm: f32,
) -> Option<Vec<u8>> {
    flatten_rgba_on_page_max(rgba, bg, x_mm, y_top_mm, w_mm, h_mm, 256)
}

/// `max_px` is 256 for icons; Cinzel body lines need ~1800 or they smear in Adobe.
fn flatten_rgba_on_page_max(
    rgba: &::image::RgbaImage,
    bg: &Option<Vec<u8>>,
    x_mm: f32,
    y_top_mm: f32,
    w_mm: f32,
    h_mm: f32,
    max_px: u32,
) -> Option<Vec<u8>> {
    let (pw, ph) = ::image::GenericImageView::dimensions(rgba);
    let max = max_px.max(32);
    let (nw, nh) = if pw.max(ph) > max {
        let s = max as f32 / pw.max(ph) as f32;
        (
            ((pw as f32) * s).max(1.0) as u32,
            ((ph as f32) * s).max(1.0) as u32,
        )
    } else {
        (pw.max(1), ph.max(1))
    };
    let fg = if nw != pw || nh != ph {
        ::image::imageops::resize(rgba, nw, nh, ::image::imageops::FilterType::Triangle)
    } else {
        rgba.clone()
    };
    let mut rgb = page_patch(bg, x_mm, y_top_mm, w_mm, h_mm, nw, nh);
    for (x, y, p) in ::image::GenericImageView::pixels(&fg) {
        let a = p.0[3] as f32 / 255.0;
        if a <= 0.01 {
            continue;
        }
        let dst = rgb.get_pixel_mut(x, y);
        for i in 0..3 {
            dst.0[i] = (p.0[i] as f32 * a + dst.0[i] as f32 * (1.0 - a)).round() as u8;
        }
    }
    let mut jpeg = Vec::new();
    ::image::DynamicImage::ImageRgb8(rgb)
        .write_to(&mut Cursor::new(&mut jpeg), ::image::ImageOutputFormat::Jpeg(90))
        .ok()?;
    Some(jpeg)
}

fn load_icon_on_page(
    heading: &str,
    bg: &Option<Vec<u8>>,
    x_mm: f32,
    y_top_mm: f32,
    h_mm: f32,
) -> Option<Vec<u8>> {
    let stem = media::icon_stem(heading)?;
    let path = media::find_icon(stem)?;
    let dynimg = ::image::open(&path).ok()?;
    let rgba = gold_art_transparent(dynimg.to_rgba8());
    let (w, h) = ::image::GenericImageView::dimensions(&rgba);
    let w_mm = if h > 0 {
        h_mm * (w as f32 / h as f32)
    } else {
        h_mm
    };
    flatten_rgba_on_page(&rgba, bg, x_mm, y_top_mm, w_mm, h_mm)
}

fn rounded_card_points(x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> Vec<(Point, bool)> {
    let w = (x1 - x0).abs();
    let h = (y1 - y0).abs();
    let r = r.min(w / 2.0).min(h / 2.0).max(0.6);
    let c = r * 0.551915;
    let p = |x: f32, y: f32, bez: bool| (Point::new(Mm(x), Mm(y)), bez);
    vec![
        p(x0 + r, y1, false),
        p(x1 - r, y1, false),
        p(x1 - r, y1, true),
        p(x1 - r + c, y1, true),
        p(x1, y1 - r + c, true),
        p(x1, y1 - r, false),
        p(x1, y0 + r, false),
        p(x1, y0 + r, true),
        p(x1, y0 + r - c, true),
        p(x1 - r + c, y0, true),
        p(x1 - r, y0, false),
        p(x0 + r, y0, false),
        p(x0 + r, y0, true),
        p(x0 + r - c, y0, true),
        p(x0, y0 + r - c, true),
        p(x0, y0 + r, false),
        p(x0, y1 - r, false),
        p(x0, y1 - r, true),
        p(x0, y1 - r + c, true),
        p(x0 + r - c, y1, true),
        p(x0 + r, y1, false),
    ]
}

fn pdf_body_ink() -> printpdf::Color {
    if house_style().ink_black() {
        pdf_ink_black()
    } else {
        pdf_gold()
    }
}

#[allow(dead_code)]
fn pdf_body_ink_dim() -> printpdf::Color {
    if house_style().ink_black() {
        printpdf::Color::Rgb(Rgb::new(0.28, 0.26, 0.24, None))
    } else {
        pdf_gold_dim()
    }
}

fn gold_hairline(
    doc: &PdfDocumentReference,
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    y: f32,
) {
    gold_rule(doc, page, layer, CARD_X0, CARD_X1, y);
}

fn gold_rule(
    doc: &PdfDocumentReference,
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    x0: f32,
    x1: f32,
    y: f32,
) {
    let current = doc.get_page(page).get_layer(layer);
    current.set_outline_color(pdf_body_ink());
    current.set_outline_thickness(0.45);
    current.add_line(Line {
        points: vec![
            (Point::new(Mm(x0), Mm(y)), false),
            (Point::new(Mm(x1), Mm(y)), false),
        ],
        is_closed: false,
    });
}

fn gold_card(
    doc: &PdfDocumentReference,
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
) {
    if pdf_simples() {
        gold_hairline(doc, page, layer, y1);
        return;
    }
    let st = house_style();
    let r = st.raio_mm.clamp(0.0, 8.0);
    let w = st.contorno_mm.clamp(0.3, 1.8);
    let current = doc.get_page(page).get_layer(layer);
    current.set_outline_color(pdf_body_ink());
    current.set_outline_thickness(w);
    current.set_line_join_style(LineJoinStyle::Round);
    current.set_line_cap_style(LineCapStyle::Round);
    current.add_polygon(Polygon {
        rings: vec![rounded_card_points(x0, y0, x1, y1, r)],
        mode: PaintMode::Stroke,
        winding_order: WindingOrder::NonZero,
    });
}

fn body_pt() -> f32 {
    house_style().corpo_pt.clamp(8.0, 13.0)
}

fn body_gap() -> f32 {
    line_skip(body_pt(), (body_pt() * 0.5).clamp(4.0, 7.0))
}

fn line_skip(size_pt: f32, gap: f32) -> f32 {
    gap.max(size_pt * 0.3528 + 1.8)
}

fn ink_skip(line: &InkLine) -> f32 {
    line_skip(line.size, line.gap)
}

fn chars_for_width_mm(width_mm: f32, size_pt: f32) -> usize {
    let char_mm = (size_pt * 0.50 * 0.3528).max(1.1);
    ((width_mm / char_mm).floor() as usize).clamp(16, 90)
}

fn body_cols() -> usize {
    let icon = if house_style().show_icons() {
        ICON_MM + 2.5
    } else {
        0.0
    };
    let w = CARD_X1 - CARD_X0 - CARD_PAD * 2.0 - icon;
    chars_for_width_mm(w, body_pt())
}

fn body_h_mm(size_pt: f32) -> f32 {
    (size_pt * 0.42).clamp(3.4, 8.0)
}

fn paint_line(
    doc: &PdfDocumentReference,
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    _font: &IndirectFontRef,
    bg: &Option<Vec<u8>>,
    x: f32,
    y: f32,
    text: &str,
    size: f32,
    dim: bool,
) {
    if text.trim().is_empty() {
        return;
    }
    let h_mm = body_h_mm(size);
    let px = (h_mm * 12.0).clamp(28.0, 96.0);
    let Ok((rgba, w, h)) = fonts::raster_body_rgba(text, px, !dim) else {
        return;
    };
    let w_mm = if h > 0 {
        h_mm * (w as f32 / h as f32)
    } else {
        h_mm * 4.0
    };
    let y_top = y + h_mm * 0.58;
    if let Some(jpeg) = flatten_rgba_on_page_max(&rgba, bg, x, y_top, w_mm, h_mm, 1400) {
        place_jpeg(doc, page, layer, &jpeg, x, y_top, h_mm);
    }
}

fn body_width_mm(text: &str, size_pt: f32) -> f32 {
    let h_mm = body_h_mm(size_pt);
    let px = (h_mm * 12.0).clamp(28.0, 96.0);
    let Ok((_, w, h)) = fonts::raster_body_rgba(text, px, true) else {
        return text.chars().count() as f32 * (size_pt * 0.50 * 0.3528).max(1.05);
    };
    if h > 0 {
        h_mm * (w as f32 / h as f32)
    } else {
        h_mm * 4.0
    }
}

fn paint_right(
    doc: &PdfDocumentReference,
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    font: &IndirectFontRef,
    bg: &Option<Vec<u8>>,
    x_right: f32,
    y: f32,
    text: &str,
    size: f32,
    dim: bool,
) {
    let w = body_width_mm(text, size);
    paint_line(doc, page, layer, font, bg, x_right - w, y, text, size, dim);
}

fn draw_gold_star(
    doc: &PdfDocumentReference,
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    cx: f32,
    cy: f32,
    r: f32,
    on: bool,
) {
    let mut pts = Vec::new();
    for i in 0..10 {
        let a = std::f32::consts::PI / 2.0 + i as f32 * std::f32::consts::PI / 5.0;
        let rad = if i % 2 == 0 { r } else { r * 0.42 };
        pts.push((
            Point::new(Mm(cx + rad * a.cos()), Mm(cy + rad * a.sin())),
            false,
        ));
    }
    let current = doc.get_page(page).get_layer(layer);
    current.set_outline_color(pdf_body_ink());
    current.set_fill_color(pdf_body_ink());
    current.set_outline_thickness(0.28);
    current.add_polygon(Polygon {
        rings: vec![pts],
        mode: if on {
            PaintMode::FillStroke
        } else {
            PaintMode::Stroke
        },
        winding_order: WindingOrder::NonZero,
    });
}

fn draw_star_meter(
    doc: &PdfDocumentReference,
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    x: f32,
    cy: f32,
    n: u8,
) {
    let n = n.min(5);
    for i in 0..5 {
        draw_gold_star(doc, page, layer, x + i as f32 * 4.1, cy, 1.55, (i as u8) < n);
    }
}

fn turn_page(
    doc: &PdfDocumentReference,
    bg: &Option<Vec<u8>>,
    page: &mut PdfPageIndex,
    layer: &mut PdfLayerIndex,
    y: &mut f32,
) {
    let (p, l) = doc.add_page(Mm(210.0), Mm(297.0), "Page");
    *page = p;
    *layer = l;
    draw_page_metal(doc, *page, *layer, bg);
    *y = PAGE_TOP;
}

fn draw_card(
    doc: &PdfDocumentReference,
    page: &mut PdfPageIndex,
    layer: &mut PdfLayerIndex,
    y: &mut f32,
    font: &IndirectFontRef,
    bg: &Option<Vec<u8>>,
    heading: &str,
    aside: Option<&str>,
    lines: &[InkLine],
) {
    let has_head = !heading.trim().is_empty();
    let inner_x = CARD_X0 + CARD_PAD;
    let mut rest: &[InkLine] = lines;
    loop {
        let icon_jpeg = if has_head && house_style().show_icons() {
            load_icon_on_page(heading, bg, inner_x, *y - CARD_PAD, ICON_MM)
        } else {
            None
        };
        let text_x = if icon_jpeg.is_some() {
            inner_x + ICON_MM + 2.5
        } else {
            inner_x
        };
        let head_skip = if !has_head {
            0.0
        } else if icon_jpeg.is_some() {
            ICON_MM.max(6.0) + 1.0
        } else {
            (HEAD_ROW - 2.0).max(5.5)
        };
        let min_h = CARD_PAD + head_skip + 5.0 + CARD_PAD;
        if *y - min_h < PAGE_BOT {
            turn_page(doc, bg, page, layer, y);
        }
        let avail = (*y - PAGE_BOT).max(min_h);
        let mut used = CARD_PAD + head_skip;
        let mut n = 0usize;
        for line in rest {
            let skip = ink_skip(line);
            if used + skip + CARD_PAD > avail && n > 0 {
                break;
            }
            used += skip;
            n += 1;
        }
        if n == 0 && !rest.is_empty() {
            n = 1;
            used = CARD_PAD + head_skip + ink_skip(&rest[0]);
        }
        let h = (used + CARD_PAD).max(if has_head {
            CARD_PAD * 2.0 + ICON_MM
        } else {
            CARD_PAD * 2.0 + 6.0
        });
        gold_card(doc, *page, *layer, CARD_X0, *y - h, CARD_X1, *y);
        let mut cy = *y - CARD_PAD - 6.0;
        if has_head {
            if let Some(jpeg) = icon_jpeg.as_ref() {
                place_jpeg(
                    doc,
                    *page,
                    *layer,
                    jpeg,
                    inner_x,
                    *y - CARD_PAD,
                    ICON_MM,
                );
            }
            place_cinzel(doc, *page, *layer, bg, heading, text_x, *y - CARD_PAD, 6.0);
            if let Some(extra) = aside {
                if !extra.trim().is_empty() {
                    paint_line(
                        doc,
                        *page,
                        *layer,
                        font,
                        bg,
                        148.0,
                        cy,
                        extra,
                        body_pt(),
                        false,
                    );
                }
            }
            cy -= head_skip;
        }
        for line in rest.iter().take(n) {
            paint_line(
                doc,
                *page,
                *layer,
                font,
                bg,
                text_x + line.indent,
                cy,
                &line.text,
                line.size,
                line.dim,
            );
            cy -= ink_skip(line);
        }
        *y = cy - CARD_PAD - CARD_GAP;
        if n >= rest.len() {
            break;
        }
        rest = &rest[n..];
    }
}

fn place_title_raster(
    doc: &PdfDocumentReference,
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    bg: &Option<Vec<u8>>,
    text: &str,
    x: f32,
    y_top: f32,
    height_mm: f32,
    gold: bool,
    _on_white: bool,
) -> bool {
    let px = (height_mm * 8.0).clamp(28.0, 120.0);
    let Ok((rgba, w, h)) = fonts::raster_title_rgba(text, px, gold) else {
        return false;
    };
    let w_mm = if h > 0 {
        height_mm * (w as f32 / h as f32)
    } else {
        height_mm * 4.0
    };
    let Some(jpeg) = flatten_rgba_on_page(&rgba, bg, x, y_top, w_mm, height_mm) else {
        return false;
    };
    place_jpeg(doc, page, layer, &jpeg, x, y_top, height_mm);
    true
}

fn cinzel_box(text: &str, height_mm: f32) -> Option<(::image::RgbaImage, f32, f32)> {
    let px = (height_mm * 12.0).clamp(36.0, 160.0);
    let (rgba, w, h) = fonts::raster_title_rgba(text, px, true).ok()?;
    let w_mm = if h > 0 {
        height_mm * (w as f32 / h as f32)
    } else {
        height_mm * 4.0
    };
    Some((rgba, w_mm, height_mm))
}

fn place_cinzel(
    doc: &PdfDocumentReference,
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    bg: &Option<Vec<u8>>,
    text: &str,
    x: f32,
    y_top: f32,
    height_mm: f32,
) -> f32 {
    let Some((rgba, w_mm, h_mm)) = cinzel_box(text, height_mm) else {
        return 0.0;
    };
    if let Some(jpeg) = flatten_rgba_on_page_max(&rgba, bg, x, y_top, w_mm, h_mm, 1800) {
        place_jpeg(doc, page, layer, &jpeg, x, y_top, h_mm);
        return w_mm;
    }
    0.0
}

fn cinzel_width(text: &str, height_mm: f32) -> f32 {
    cinzel_box(text, height_mm)
        .map(|(_, w, _)| w)
        .unwrap_or(0.0)
}

fn cinzel_wrap_words(text: &str, max_w_mm: f32, height_mm: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let trial = if cur.is_empty() {
            word.to_string()
        } else {
            format!("{cur} {word}")
        };
        if cur.is_empty() || cinzel_width(&trial, height_mm) <= max_w_mm * 0.96 {
            cur = trial;
        } else {
            lines.push(std::mem::take(&mut cur));
            cur = word.to_string();
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

fn cinzel_wrap(text: &str, max_w_mm: f32, height_mm: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for para in text.split('\n') {
        if para.trim().is_empty() {
            if !lines.is_empty() {
                lines.push(String::new());
            }
            continue;
        }
        let wrapped = cinzel_wrap_words(para, max_w_mm, height_mm);
        if wrapped.is_empty() {
            lines.push(String::new());
        } else {
            lines.extend(wrapped);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn draw_header_card(
    doc: &PdfDocumentReference,
    page: &mut PdfPageIndex,
    layer: &mut PdfLayerIndex,
    y: &mut f32,
    font: &IndirectFontRef,
    bg: &Option<Vec<u8>>,
    title: &str,
    subtitle: &str,
    extra: &[InkLine],
    logo: Option<&Path>,
) {
    let st = house_style();
    let extra_mm = st.espaco_titulo.clamp(0.0, 14.0);
    let gold = !st.ink_black();
    let on_white = st.white_paper();
    let mark_h = 6.0;
    let title_h = (st.titulo_pt.max(14.0) * 0.45).clamp(8.0, 16.0);
    let mut body: Vec<InkLine> = Vec::new();
    if !subtitle.trim().is_empty() {
        body.push(InkLine::gold(subtitle, 12.0, 0.0, 5.5));
    }
    body.extend(extra.iter().cloned());
    let body_h: f32 = body.iter().map(|l| l.gap).sum();
    let title_block = if title.trim().is_empty() {
        0.0
    } else {
        title_h + 2.0 + extra_mm * 0.4
    };
    let text_h = mark_h + 2.0 + extra_mm * 0.4 + title_block + body_h;
    let logo_mm = st.logo_mm.clamp(8.0, 24.0);
    let logo_h: f32 = if logo.is_some() { logo_mm + 2.0 } else { 0.0 };
    let h = CARD_PAD + logo_h.max(text_h) + CARD_PAD + extra_mm + 2.0;
    if *y - h < PAGE_BOT {
        turn_page(doc, bg, page, layer, y);
    }
    gold_card(doc, *page, *layer, CARD_X0, *y - h, CARD_X1, *y);
    let mut text_x = CARD_X0 + CARD_PAD;
    if let Some(path) = logo {
        if let Ok(dynimg) = ::image::open(path) {
            let rgba = trim_rgba(dynimg.to_rgba8());
            let (pw, ph) = ::image::GenericImageView::dimensions(&rgba);
            let w_mm = if ph > 0 {
                logo_mm * (pw as f32 / ph as f32)
            } else {
                logo_mm
            };
            if let Some(jpeg) = flatten_rgba_on_page(
                &rgba,
                bg,
                CARD_X0 + CARD_PAD,
                *y - CARD_PAD,
                w_mm,
                logo_mm,
            ) {
                place_jpeg(
                    doc,
                    *page,
                    *layer,
                    &jpeg,
                    CARD_X0 + CARD_PAD,
                    *y - CARD_PAD,
                    logo_mm,
                );
                text_x = CARD_X0 + CARD_PAD + (w_mm + 4.0).clamp(24.0, 52.0);
            }
        }
    }
    let mut cy = *y - CARD_PAD - 1.0;
    if !place_title_raster(
        doc,
        *page,
        *layer,
        bg,
        "VANGUARDA AUTOMÓVEL",
        text_x,
        cy,
        mark_h,
        gold,
        on_white,
    ) {
        paint_line(
            doc,
            *page,
            *layer,
            font,
            bg,
            text_x,
            cy - 4.0,
            "VANGUARDA AUTOMÓVEL",
            11.0,
            false,
        );
    }
    cy -= mark_h + 1.5 + extra_mm * 0.35;
    if !title.trim().is_empty() {
        if !place_title_raster(doc, *page, *layer, bg, title, text_x, cy, title_h, gold, on_white) {
            paint_line(
                doc,
                *page,
                *layer,
                font,
                bg,
                text_x,
                cy - 5.0,
                title,
                st.titulo_pt.max(12.0),
                false,
            );
        }
        cy -= title_h + 2.5 + extra_mm * 0.35;
    }
    for line in &body {
        paint_line(
            doc,
            *page,
            *layer,
            font,
            bg,
            text_x,
            cy,
            &line.text,
            line.size,
            line.dim,
        );
        cy -= line.gap;
    }
    *y -= h + CARD_GAP;
}

fn place_jpeg(
    doc: &PdfDocumentReference,
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    jpeg: &[u8],
    x_mm: f32,
    y_top: f32,
    height_mm: f32,
) -> f32 {
    let (px_w, px_h) = jpeg_dimensions(jpeg);
    if px_w < 2.0 || px_h < 2.0 {
        return 0.0;
    }
    let pdf_img = printpdf::Image::from(ImageXObject {
        width: Px(px_w as usize),
        height: Px(px_h as usize),
        color_space: ColorSpace::Rgb,
        bits_per_component: ColorBits::Bit8,
        interpolate: true,
        image_data: jpeg.to_vec(),
        image_filter: Some(ImageFilter::DCT),
        smask: None,
        clipping_bbox: None,
    });
    let native_h = px_h / 300.0 * 25.4;
    let native_w = px_w / 300.0 * 25.4;
    let scale = height_mm / native_h.max(0.1);
    let used_h = native_h * scale;
    let current = doc.get_page(page).get_layer(layer);
    pdf_img.add_to_layer(
        current,
        printpdf::ImageTransform {
            translate_x: Some(Mm(x_mm)),
            translate_y: Some(Mm(y_top - used_h)),
            scale_x: Some(scale),
            scale_y: Some(scale),
            ..Default::default()
        },
    );
    let _ = native_w;
    used_h
}

fn fill_rect(
    doc: &PdfDocumentReference,
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    color: printpdf::Color,
) {
    let current = doc.get_page(page).get_layer(layer);
    current.set_fill_color(color);
    let mut rect = Rect::new(Mm(x0), Mm(y0), Mm(x1), Mm(y1));
    rect.mode = printpdf::path::PaintMode::Fill;
    current.add_rect(rect);
}

fn write_diag_pdf(
    path: &Path,
    scan: &Scan,
    cliente: &str,
    stars: &[SistemaNota],
    photos: &[(String, Vec<u8>)],
) -> Result<()> {
    let (doc, page0, layer0) =
        PdfDocument::new("Relatorio", Mm(210.0), Mm(297.0), "Layer 1");
    let font = load_font(&doc)?;
    let bg = load_pdf_bg_jpeg();
    let mut y = PAGE_TOP;
    let mut page = page0;
    let mut layer = layer0;
    draw_page_metal(&doc, page, layer, &bg);

    let sub = if cliente.trim().is_empty() {
        scan.veiculo.clone()
    } else {
        cliente.to_string()
    };
    draw_header_card(
        &doc,
        &mut page,
        &mut layer,
        &mut y,
        &font,
        &bg,
        if scan.dtcs.is_empty() {
            "Relatório"
        } else {
            "Relatório de diagnóstico"
        },
        &sub,
        &[],
        media::find_logo().filter(|p| p.is_file()).as_deref(),
    );

    let pairs = [
        ("Cliente", cliente),
        ("Data", scan.data.as_str()),
        ("Matrícula", scan.matricula.as_str()),
        ("VIN", scan.vin.as_str()),
        ("Veículo", scan.veiculo.as_str()),
        ("Km", scan.km.as_str()),
        ("Mecânico", scan.mecanico.as_str()),
    ];
    let mut id_lines = Vec::new();
    for (k, v) in pairs {
        if v.trim().is_empty() {
            continue;
        }
        id_lines.extend(ink_wrap(&format!("{k}: {v}"), body_cols(), body_pt(), 0.0, false, body_gap()));
    }
    if !id_lines.is_empty() {
        draw_card(
            &doc,
            &mut page,
            &mut layer,
            &mut y,
            &font,
            &bg,
            "Identificação",
            None,
            &id_lines,
        );
    }

    if filled(&scan.sumario) {
        let lines = ink_wrap(&scan.sumario, body_cols(), body_pt(), 0.0, false, body_gap());
        draw_card(
            &doc,
            &mut page,
            &mut layer,
            &mut y,
            &font,
            &bg,
            "Sumário do mecânico",
            None,
            &lines,
        );
    }

    let rated: Vec<&SistemaNota> = stars.iter().filter(|s| s.estrelas > 0).collect();
    if !rated.is_empty() {
        let row_h = 6.4_f32;
        let head = 14.0_f32;
        let h = CARD_PAD + head + 7.0 + rated.len() as f32 * row_h + CARD_PAD;
        if y - h < PAGE_BOT {
            turn_page(&doc, &bg, &mut page, &mut layer, &mut y);
        }
        gold_card(&doc, page, layer, CARD_X0, y - h, CARD_X1, y);
        let inner_x = CARD_X0 + CARD_PAD;
        if let Some(jpeg) = load_icon_on_page("Estado da viatura", &bg, inner_x, y - CARD_PAD, ICON_MM)
        {
            place_jpeg(&doc, page, layer, &jpeg, inner_x, y - CARD_PAD, ICON_MM);
        }
        place_cinzel(
            &doc,
            page,
            layer,
            &bg,
            "Estado da viatura",
            inner_x + ICON_MM + 2.5,
            y - CARD_PAD,
            6.0,
        );
        let mut cy = y - CARD_PAD - head;
        paint_line(
            &doc,
            page,
            layer,
            &font,
            &bg,
            inner_x,
            cy,
            "5 em ordem, 1 grave",
            9.0,
            true,
        );
        cy -= 6.0;
        for s in &rated {
            paint_line(
                &doc,
                page,
                layer,
                &font,
                &bg,
                inner_x,
                cy,
                &s.categoria,
                10.0,
                false,
            );
            draw_star_meter(&doc, page, layer, CARD_X1 - CARD_PAD - 22.0, cy + 1.2, s.estrelas);
            cy -= row_h;
        }
        y -= h + CARD_GAP;
    }

    if scan.dtcs.is_empty() {
        draw_card(
            &doc,
            &mut page,
            &mut layer,
            &mut y,
            &font,
            &bg,
            "Avarias",
            None,
            &[InkLine::dim("Nenhuma avaria listada.", 11.0, 0.0, 6.0)],
        );
    } else {
        for (cat, items) in grouped_dtcs(&scan.dtcs) {
            let aside: Option<&str> = None;
            let mut lines = Vec::new();
            let mut last_zona = String::from("\0");
            for d in items {
                let zona = d.zona.trim();
                if !zona.is_empty() && zona != last_zona {
                    lines.push(InkLine::dim(zona, body_pt(), 0.0, body_gap() + 0.5));
                    last_zona = zona.to_string();
                }
                lines.push(InkLine::gold(
                    format!("{}    {}", d.codigo, d.estado),
                    body_pt() + 1.0,
                    0.0,
                    body_gap() + 1.0,
                ));
                lines.extend(ink_wrap(&d.descricao, body_cols().saturating_sub(4), body_pt(), 4.0, true, body_gap()));
            }
            draw_card(
                &doc,
                &mut page,
                &mut layer,
                &mut y,
                &font,
                &bg,
                &cat,
                aside.as_deref(),
                &lines,
            );
        }
    }

    paint_line(
        &doc,
        page,
        layer,
        &font,
        &bg,
        CARD_X0 + CARD_PAD,
        PAGE_BOT + 5.0,
        &format!("{}  ·  {EMPRESA}", today()),
        body_pt() - 2.0,
        true,
    );
    add_photo_pages(&doc, &mut page, &mut layer, &font, &bg, photos);

    let mut raw = Vec::new();
    {
        let mut w = BufWriter::new(Cursor::new(&mut raw));
        doc.save(&mut w)?;
        w.flush()?;
    }
    fs::write(path, sanitize_printpdf(&raw))?;
    Ok(())
}

pub fn fill_missing_from_markdown(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut written = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else {
        return Ok(written);
    };
    let md = rd
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("md"))
                .unwrap_or(false)
                && p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("Ficha_Cliente_"))
                    .unwrap_or(false)
        });
    let Some(md_path) = md else {
        return Ok(written);
    };
    let stem = md_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Ficha_Cliente".to_string());
    let body = fs::read_to_string(&md_path)?;
    let txt = dir.join(format!("{stem}.txt"));
    if !txt.exists() {
        fs::write(&txt, &body)?;
        written.push(txt);
    }
    let docx = dir.join(format!("{stem}.docx"));
    if !docx.exists() {
        write_docx_from_text(&docx, "Ficha de Cliente", &body)?;
        written.push(docx);
    }
    Ok(written)
}

fn write_json<T: Serialize>(
    dir: &Path,
    name: &str,
    value: &T,
    skip_existing: bool,
) -> Result<Option<PathBuf>> {
    let path = dir.join(name);
    if skip_existing && path.exists() {
        return Ok(None);
    }
    let json = serde_json::to_string_pretty(value)?;
    fs::write(&path, json)?;
    Ok(Some(path))
}

fn write_spec(
    dir: &Path,
    base: &str,
    spec: &Spec,
    opts: &WriteOpts,
) -> Result<Vec<PathBuf>> {
    let mut written = Vec::new();
    for (ext, write_fn) in [
        (
            "md",
            write_md as fn(&Path, &Spec) -> Result<()>,
        ),
        ("txt", write_txt),
        ("docx", write_docx),
        ("pdf", write_pdf),
    ] {
        let path = dir.join(format!("{base}.{ext}"));
        if opts.skip_existing && path.exists() {
            continue;
        }
        write_fn(&path, spec).with_context(|| format!("a escrever {}", path.display()))?;
        written.push(path);
    }
    Ok(written)
}

fn filled(s: &str) -> bool {
    !crate::format::is_placeholder(s)
}

fn val(s: &str) -> String {
    s.trim().to_string()
}

fn row(k: &str, v: &str) -> Option<(String, String)> {
    if filled(v) {
        Some((k.to_string(), v.trim().to_string()))
    } else {
        None
    }
}

fn rows(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items.iter().filter_map(|(k, v)| row(k, v)).collect()
}

fn bullets_filled(items: Vec<String>) -> Vec<String> {
    items
        .into_iter()
        .filter(|b| filled(b) && !b.contains("ainda sem ficheiros"))
        .collect()
}

fn push_sec(out: &mut Vec<Section>, heading: String, rows: Vec<(String, String)>, bullets: Vec<String>) {
    let bullets = bullets_filled(bullets);
    if rows.is_empty() && bullets.is_empty() {
        return;
    }
    out.push(Section {
        heading,
        rows,
        bullets,
        table: None,
    });
}

fn spec_client(f: &ClientFicha, dir: &Path) -> Spec {
    let nome = if f.nome_completo.trim().is_empty() {
        "Cliente".to_string()
    } else {
        f.nome_completo.clone()
    };
    let mut sections = Vec::new();
    push_sec(
        &mut sections,
        "Identificação".into(),
        rows(&[
            ("Nome completo", &f.nome_completo),
            ("Nome preferido", &f.nome_preferido),
            ("NIF", &f.nif),
            ("Data de nascimento", &f.data_nascimento),
            ("Idioma de contacto", &f.idioma),
        ]),
        vec![],
    );
    push_sec(
        &mut sections,
        "Contactos".into(),
        rows(&[
            ("Telemóvel", &f.telemovel),
            ("Email", &f.email),
            ("Morada", &f.morada),
            ("Código postal", &f.codigo_postal),
            ("Localidade", &f.localidade),
            ("Consentimento de contacto", &f.consentimento_contacto),
            ("Consentimento marketing", &f.consentimento_marketing),
        ]),
        vec![],
    );
    push_sec(
        &mut sections,
        "Veículo".into(),
        rows(&[
            ("Marca", &f.veiculo_marca),
            ("Modelo", &f.veiculo_modelo),
            ("Versão / motor", &f.veiculo_versao),
            ("Matrícula", &f.veiculo_matricula),
            ("Ano", &f.veiculo_ano),
            ("Cor", &f.veiculo_cor),
            ("VIN", &f.veiculo_vin),
            ("Quilometragem", &f.veiculo_km),
            ("Data última visita", &f.data_ultima_visita),
            ("Pneus", &f.pneus),
            ("Observações", &f.observacoes_veiculo),
        ]),
        vec![],
    );
    let mut notes = Vec::new();
    if filled(&f.preferencias) {
        notes.push(f.preferencias.clone());
    }
    if filled(&f.notas) {
        notes.push(f.notas.clone());
    }
    push_sec(&mut sections, "Notas".into(), vec![], notes);
    push_sec(
        &mut sections,
        "Conta corrente".into(),
        rows(&[
            ("Saldo em aberto", &f.saldo_aberto),
            ("Crédito em conta", &f.credito),
            ("Forma de pagamento", &f.forma_pagamento),
            ("Fatura com NIF", &f.fatura_com_nif),
        ]),
        vec![],
    );
    sections.push(Section {
        heading: "RGPD / privacidade".into(),
        rows: vec![],
        bullets: vec![
            "Finalidade: gestão de clientes, viaturas, marcações e faturação.".into(),
            "Base legal: execução de medidas pré-contratuais/contrato e obrigação legal (faturação).".into(),
            "Prazo de conservação: duração da relação comercial + prazos legais fiscais.".into(),
            "Direitos: acesso, retificação, apagamento (quando aplicável), limitação, oposição.".into(),
        ],
        table: None,
    });
    push_sec(
        &mut sections,
        "Documentos na pasta".into(),
        vec![],
        media::media_bullets(dir),
    );
    let mut meta = vec![("Empresa".into(), EMPRESA.into())];
    if let Some(p) = row("Estado da ficha", &f.estado) {
        meta.push(p);
    }
    Spec {
        title: "Ficha de Cliente".into(),
        subtitle: nome,
        meta,
        sections,
        footer: format!("Última atualização: {}  ·  {}", today(), EMPRESA),
        logo: media::find_logo(),
        photos: vec![],
    }
}

fn spec_staff(f: &StaffFicha, dir: &Path) -> Spec {
    let nome = if f.nome.trim().is_empty() {
        "Staff".to_string()
    } else {
        f.nome.clone()
    };
    Spec {
        title: "Ficha de Staff (interno)".into(),
        subtitle: nome.clone(),
        meta: vec![
            ("Empresa".into(), EMPRESA.into()),
            ("Pasta".into(), format!("Clientes / Interno / {nome}")),
            ("Criada em".into(), today()),
            ("Tipo".into(), "Staff / consumo interno".into()),
        ],
        sections: {
            let mut sections = Vec::new();
            push_sec(
                &mut sections,
                "Identificação".into(),
                rows(&[
                    ("Nome", &f.nome),
                    ("Função / papel", &f.funcao),
                    ("Data de início", &f.data_inicio),
                ]),
                vec![],
            );
            push_sec(
                &mut sections,
                "Contactos".into(),
                rows(&[("Telemóvel", &f.telemovel), ("Email", &f.email)]),
                vec![],
            );
            push_sec(&mut sections, "Notas".into(), vec![], vec![f.notas.clone()]);
            push_sec(
                &mut sections,
                "Media na pasta".into(),
                vec![],
                media::media_bullets(dir),
            );
            sections
        },
        footer: format!("Última atualização: {}  ·  {}", today(), EMPRESA),
        logo: media::find_logo(),
        photos: vec![],
    }
}

fn spec_consumo(log: &ConsumoLog, dir: &Path) -> Spec {
    let nome = if log.nome.trim().is_empty() {
        "Interno".to_string()
    } else {
        log.nome.clone()
    };
    let headers = vec![
        "Data".into(),
        "Tipo".into(),
        "Descrição".into(),
        "Veículo".into(),
        "Custo interno (€)".into(),
        "Quem fez".into(),
        "Notas".into(),
    ];
    let rows: Vec<Vec<String>> = if log.entradas.is_empty() {
        Vec::new()
    } else {
        log.entradas
            .iter()
            .map(|e| {
                vec![
                    val(&e.data),
                    val(&e.tipo),
                    val(&e.descricao),
                    val(&e.veiculo),
                    val(&e.custo_interno),
                    val(&e.quem_fez),
                    val(&e.notas),
                ]
            })
            .collect()
    };
    Spec {
        title: "Consumo interno / reparações in-company".into(),
        subtitle: nome.clone(),
        meta: vec![
            ("Empresa".into(), EMPRESA.into()),
            ("Pessoa".into(), nome),
            ("Criada em".into(), today()),
            (
                "Registos".into(),
                log.entradas.len().to_string(),
            ),
        ],
        sections: vec![
            Section {
                heading: "Registo de uso, peças e reparações internas".into(),
                rows: vec![],
                bullets: vec![],
                table: Some((headers, rows)),
            },
            Section {
                heading: "Media na pasta".into(),
                rows: vec![],
                bullets: media::media_bullets(dir),
                table: None,
            },
        ],
        footer: format!("Última atualização: {}  ·  {}", today(), EMPRESA),
        logo: media::find_logo(),
        photos: vec![],
    }
}

fn write_md(path: &Path, spec: &Spec) -> Result<()> {
    let mut s = String::new();
    s.push_str(&format!("# {}\n## {}\n\n", spec.title, spec.subtitle));
    for (k, v) in &spec.meta {
        s.push_str(&format!("**{k}:** {v}  \n"));
    }
    s.push_str("\n---\n\n");
    for sec in &spec.sections {
        s.push_str(&format!("### {}\n", sec.heading));
        if !sec.rows.is_empty() {
            s.push_str("| Campo | Valor |\n|-------|-------|\n");
            for (k, v) in &sec.rows {
                s.push_str(&format!("| {} | {} |\n", k, v));
            }
            s.push('\n');
        }
        for b in &sec.bullets {
            s.push_str(&format!("- {b}\n"));
        }
        if !sec.bullets.is_empty() {
            s.push('\n');
        }
        if let Some((headers, rows)) = &sec.table {
            s.push('|');
            for h in headers {
                s.push_str(&format!(" {h} |"));
            }
            s.push('\n');
            s.push('|');
            for _ in headers {
                s.push_str("---|");
            }
            s.push('\n');
            for row in rows {
                s.push('|');
                for cell in row {
                    s.push_str(&format!(" {cell} |"));
                }
                s.push('\n');
            }
            s.push('\n');
        }
    }
    s.push_str(&format!("---\n\n*{footer}*\n", footer = spec.footer));
    fs::write(path, s)?;
    Ok(())
}

fn write_txt(path: &Path, spec: &Spec) -> Result<()> {
    let mut s = String::new();
    s.push_str(&format!("{}\n{}\n{}\n\n", spec.title, spec.subtitle, "=".repeat(40)));
    for (k, v) in &spec.meta {
        s.push_str(&format!("{k}: {v}\n"));
    }
    s.push('\n');
    for sec in &spec.sections {
        s.push_str(&format!("{}\n{}\n", sec.heading, "-".repeat(sec.heading.chars().count())));
        for (k, v) in &sec.rows {
            s.push_str(&format!("{k}: {v}\n"));
        }
        for b in &sec.bullets {
            s.push_str(&format!("- {b}\n"));
        }
        if let Some((headers, rows)) = &sec.table {
            s.push_str(&headers.join(" | "));
            s.push('\n');
            for row in rows {
                s.push_str(&row.join(" | "));
                s.push('\n');
            }
        }
        s.push('\n');
    }
    s.push_str(&spec.footer);
    s.push('\n');
    fs::write(path, s)?;
    Ok(())
}

fn write_docx(path: &Path, spec: &Spec) -> Result<()> {
    let mut doc = Docx::new();
    if let Some(logo) = spec.logo.as_ref().filter(|p| p.is_file()) {
        if let Ok(bytes) = fs::read(logo) {
            let (w, h) = logo_emu_size(&bytes, 28.0);
            let pic = Pic::new(&bytes).size(w, h);
            doc = doc.add_paragraph(
                Paragraph::new().add_run(Run::new().add_image(pic)),
            );
            doc = doc.add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text("VANGUARDA AUTOMÓVEL")
                        .bold()
                        .size(28)
                        .color("C4A35A"),
                ),
            );
        }
    }
    doc = doc
        .add_paragraph(heading(&spec.title, 32))
        .add_paragraph(heading(&spec.subtitle, 26));
    for (k, v) in &spec.meta {
        doc = doc.add_paragraph(kv_paragraph(k, v));
    }
    for sec in &spec.sections {
        doc = doc.add_paragraph(heading(&sec.heading, 24));
        if !sec.rows.is_empty() {
            doc = doc.add_table(kv_table(&sec.rows));
        }
        for b in &sec.bullets {
            doc = doc.add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(format!("- {b}")).size(22)),
            );
        }
        if let Some((headers, rows)) = &sec.table {
            doc = doc.add_table(grid_table(headers, rows));
        }
    }
    doc = doc.add_paragraph(
        Paragraph::new().add_run(Run::new().add_text(&spec.footer).italic().size(18)),
    );
    let file = File::create(path)?;
    doc.build()
        .pack(file)
        .map_err(|e| anyhow::anyhow!("docx: {e}"))?;
    Ok(())
}

fn write_docx_from_text(path: &Path, title: &str, body: &str) -> Result<()> {
    let mut doc = Docx::new().add_paragraph(heading(title, 32));
    for line in body.lines() {
        let t = line.trim();
        if t.is_empty() {
            doc = doc.add_paragraph(Paragraph::new());
            continue;
        }
        doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(t).size(22)));
    }
    let file = File::create(path)?;
    doc.build()
        .pack(file)
        .map_err(|e| anyhow::anyhow!("docx: {e}"))?;
    Ok(())
}

fn heading(text: &str, size: usize) -> Paragraph {
    Paragraph::new().add_run(Run::new().add_text(text).bold().size(size))
}

fn kv_paragraph(k: &str, v: &str) -> Paragraph {
    Paragraph::new()
        .add_run(Run::new().add_text(format!("{k}: ")).bold().size(22))
        .add_run(Run::new().add_text(v).size(22))
}

fn cell(text: &str, header: bool) -> TableCell {
    let run = if header {
        Run::new().add_text(text).bold().size(20)
    } else {
        Run::new().add_text(text).size(20)
    };
    TableCell::new().add_paragraph(Paragraph::new().add_run(run))
}

fn kv_table(rows: &[(String, String)]) -> Table {
    let mut trows = vec![TableRow::new(vec![cell("Campo", true), cell("Valor", true)])];
    for (k, v) in rows {
        trows.push(TableRow::new(vec![cell(k, false), cell(v, false)]));
    }
    Table::new(trows)
}

fn grid_table(headers: &[String], rows: &[Vec<String>]) -> Table {
    let mut trows = vec![TableRow::new(
        headers.iter().map(|h| cell(h, true)).collect(),
    )];
    for row in rows {
        trows.push(TableRow::new(row.iter().map(|c| cell(c, false)).collect()));
    }
    Table::new(trows)
}

fn write_pdf(path: &Path, spec: &Spec) -> Result<()> {
    let (doc, mut page, mut layer) =
        PdfDocument::new(&pdf_info_title(&spec.title), Mm(210.0), Mm(297.0), "Layer 1");
    let font = load_font(&doc)?;
    let bg = load_pdf_bg_jpeg();
    let mut y = PAGE_TOP;
    draw_page_metal(&doc, page, layer, &bg);

    let mut meta_lines = Vec::new();
    for (k, v) in &spec.meta {
        if !filled(v) {
            continue;
        }
        meta_lines.extend(ink_wrap(&format!("{k}: {v}"), body_cols(), body_pt(), 0.0, false, body_gap()));
    }
    draw_header_card(
        &doc,
        &mut page,
        &mut layer,
        &mut y,
        &font,
        &bg,
        &spec.title,
        &spec.subtitle,
        &meta_lines,
        spec.logo.as_ref().filter(|p| p.is_file()).map(|p| p.as_path()),
    );

    for sec in &spec.sections {
        let mut lines = Vec::new();
        for (k, v) in &sec.rows {
            if !filled(v) {
                continue;
            }
            lines.extend(ink_wrap(&format!("{k}: {v}"), body_cols(), body_pt(), 0.0, false, body_gap()));
        }
        for b in &sec.bullets {
            if !filled(b) {
                continue;
            }
            lines.extend(ink_wrap(&format!("- {b}"), body_cols(), body_pt(), 0.0, false, body_gap()));
        }
        if let Some((headers, rows)) = &sec.table {
            if !rows.is_empty() {
                lines.extend(ink_wrap(&headers.join("  ·  "), body_cols(), body_pt() - 1.0, 0.0, true, body_gap()));
                for row in rows {
                    lines.extend(ink_wrap(&row.join("  ·  "), body_cols(), body_pt() - 1.0, 0.0, false, body_gap()));
                }
            }
        }
        if lines.is_empty() {
            continue;
        }
        draw_card(
            &doc,
            &mut page,
            &mut layer,
            &mut y,
            &font,
            &bg,
            &sec.heading,
            None,
            &lines,
        );
    }
    if filled(&spec.footer) {
        let foot = ink_wrap(&spec.footer, body_cols(), body_pt() - 2.0, 0.0, true, body_gap());
        draw_card(
            &doc,
            &mut page,
            &mut layer,
            &mut y,
            &font,
            &bg,
            "",
            None,
            &foot,
        );
    }
    add_photo_pages(&doc, &mut page, &mut layer, &font, &bg, &spec.photos);

    let mut raw = Vec::new();
    {
        let mut w = BufWriter::new(Cursor::new(&mut raw));
        doc.save(&mut w)?;
        w.flush()?;
    }
    fs::write(path, sanitize_printpdf(&raw))?;
    Ok(())
}

/// JPEG bytes for a Media photo/receipt. PNG alpha lands on white. Adobe-safe.
pub fn image_file_to_jpeg(path: &Path) -> Result<Vec<u8>> {
    let dynimg = ::image::open(path).map_err(|e| anyhow::anyhow!("anexo: {e}"))?;
    let rgba = dynimg.to_rgba8();
    let (w, h) = ::image::GenericImageView::dimensions(&rgba);
    let mut rgb = ::image::RgbImage::from_pixel(w.max(1), h.max(1), ::image::Rgb([255, 255, 255]));
    for (x, y, p) in ::image::GenericImageView::pixels(&rgba) {
        let a = p.0[3] as f32 / 255.0;
        if a <= 0.01 {
            continue;
        }
        let dst = rgb.get_pixel_mut(x, y);
        for i in 0..3 {
            dst.0[i] = (p.0[i] as f32 * a + dst.0[i] as f32 * (1.0 - a)).round() as u8;
        }
    }
    let max_side = 1400u32;
    let rgb = if w.max(h) > max_side {
        let s = max_side as f32 / w.max(h) as f32;
        let nw = ((w as f32) * s).max(1.0) as u32;
        let nh = ((h as f32) * s).max(1.0) as u32;
        ::image::imageops::resize(&rgb, nw, nh, ::image::imageops::FilterType::Triangle)
    } else {
        rgb
    };
    let mut jpeg = Vec::new();
    ::image::DynamicImage::ImageRgb8(rgb)
        .write_to(&mut Cursor::new(&mut jpeg), ::image::ImageOutputFormat::Jpeg(82))
        .map_err(|e| anyhow::anyhow!("anexo jpeg: {e}"))?;
    Ok(jpeg)
}

fn load_photo_jpegs(photos: &[(String, PathBuf)]) -> Result<Vec<(String, Vec<u8>)>> {
    let mut out = Vec::new();
    for (cap, p) in photos {
        out.push((cap.clone(), image_file_to_jpeg(p)?));
    }
    Ok(out)
}

fn add_photo_pages(
    doc: &PdfDocumentReference,
    page: &mut PdfPageIndex,
    layer: &mut PdfLayerIndex,
    font: &IndirectFontRef,
    bg: &Option<Vec<u8>>,
    photos: &[(String, Vec<u8>)],
) {
    for (caption, jpeg) in photos {
        let (p, l) = doc.add_page(Mm(210.0), Mm(297.0), "Anexo");
        *page = p;
        *layer = l;
        draw_page_metal(doc, *page, *layer, bg);
        gold_card(doc, *page, *layer, CARD_X0, 16.0, CARD_X1, PAGE_TOP);
        paint_line(
            doc,
            *page,
            *layer,
            font,
            bg,
            CARD_X0 + CARD_PAD,
            PAGE_TOP - CARD_PAD - 6.0,
            caption,
            11.0,
            false,
        );
        let (px_w, px_h) = jpeg_dimensions(jpeg);
        let aspect = px_h / px_w.max(1.0);
        let max_w = 170.0_f32;
        let max_h = 230.0_f32;
        let mut w = max_w;
        let mut h = w * aspect;
        if h > max_h {
            h = max_h;
            w = h / aspect.max(0.01);
        }
        let x = CARD_X0 + (CARD_X1 - CARD_X0 - w) / 2.0;
        place_jpeg(doc, *page, *layer, jpeg, x, PAGE_TOP - CARD_PAD - 14.0, h);
    }
}

pub fn quote_as_comercial(q: &Quote, veiculo: &str) -> OrcamentoComercial {
    let data = if q.created.trim().is_empty() {
        today()
    } else {
        q.created.clone()
    };
    OrcamentoComercial {
        titulo: "Orçamento".into(),
        numero: q.numero.clone(),
        data,
        hora: String::new(),
        colaborador: q.colaborador.clone(),
        departamento: ops::tipo_label(&q.tipo).into(),
        cliente_n: String::new(),
        cliente: q.cliente.clone(),
        nif: String::new(),
        veiculo: veiculo.trim().to_string(),
        matricula: q.matricula.clone(),
        vin: q.vin.clone(),
        linhas: q
            .linhas
            .iter()
            .map(|l| {
                let desc = if l.is_mao() && l.desc.trim().is_empty() {
                    "Mão de obra".into()
                } else {
                    l.desc.clone()
                };
                OrcamentoLinha {
                    designacao: desc,
                    qty: l.qty,
                    unitario: l.cents as f64 / 100.0,
                    kind: if l.is_mao() {
                        "mao".into()
                    } else {
                        "peca".into()
                    },
                    disc_pct: l.disc_pct,
                }
            })
            .collect(),
        notas: if q.notas.trim().is_empty() {
            vec![]
        } else {
            vec![q.notas.clone()]
        },
        disclaimer: String::new(),
    }
}

pub fn write_quote(dir: &Path, q: &Quote) -> Result<Vec<PathBuf>> {
    write_quote_car(dir, q, "")
}

pub fn write_quote_car(dir: &Path, q: &Quote, viatura: &str) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(dir)?;
    let o = quote_as_comercial(q, viatura);
    let base = format!("Orcamento_{}", crate::slug::slug(&q.numero));
    let pdf = dir.join(format!("{base}.pdf"));
    write_orcamento_comercial_pdf(&pdf, &o)?;
    Ok(vec![pdf])
}

pub fn write_conta_car(dir: &Path, q: &Quote, viatura: &str) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(dir)?;
    let mut o = quote_as_comercial(q, viatura);
    o.titulo = "Conta".into();
    o.disclaimer = "Documento de oficina. Sem NIF. Não é fatura certificada AT.".into();
    let base = format!("Conta_{}", crate::slug::slug(&q.numero));
    let pdf = dir.join(format!("{base}.pdf"));
    write_orcamento_comercial_pdf(&pdf, &o)?;
    Ok(vec![pdf])
}

/// One line of a client-facing orçamento (unit price in euro). No IVA.
/// `kind`: `"peca"` (qty = units) or `"mao"` (qty = hours).
#[derive(Clone)]
pub struct OrcamentoLinha {
    pub designacao: String,
    pub qty: f64,
    pub unitario: f64,
    pub kind: String,
    pub disc_pct: f64,
}

impl OrcamentoLinha {
    pub fn peca(designacao: impl Into<String>, qty: f64, unitario: f64) -> Self {
        Self {
            designacao: designacao.into(),
            qty,
            unitario,
            kind: "peca".into(),
            disc_pct: 0.0,
        }
    }

    pub fn mao(designacao: impl Into<String>, horas: f64, eur_hora: f64) -> Self {
        Self {
            designacao: designacao.into(),
            qty: horas,
            unitario: eur_hora,
            kind: "mao".into(),
            disc_pct: 0.0,
        }
    }

    pub fn is_mao(&self) -> bool {
        self.kind == "mao" || self.kind == "mao_de_obra"
    }

    pub fn unit_cents(&self) -> i64 {
        (self.unitario * 100.0).round() as i64
    }

    pub fn valor_cents(&self) -> i64 {
        ops::net_cents(self.qty, self.unit_cents(), self.disc_pct)
    }

    pub fn discount_cents(&self) -> i64 {
        ops::gross_cents(self.qty, self.unit_cents()) - self.valor_cents()
    }
}

pub struct OrcamentoComercial {
    pub titulo: String,
    pub numero: String,
    pub data: String,
    pub hora: String,
    pub colaborador: String,
    pub departamento: String,
    pub cliente_n: String,
    pub cliente: String,
    pub nif: String,
    pub veiculo: String,
    pub matricula: String,
    pub vin: String,
    pub linhas: Vec<OrcamentoLinha>,
    pub notas: Vec<String>,
    pub disclaimer: String,
}

fn fmt_qty_pt(q: f64) -> String {
    if (q - q.round()).abs() < 0.001 {
        format!("{}", q.round() as i64)
    } else {
        format!("{q:.2}").replace('.', ",")
    }
}

fn fmt_disc_pct(p: f64) -> String {
    let d = ops::clamp_disc_pct(p);
    if d <= 0.0 {
        String::new()
    } else if (d - d.round()).abs() < 0.05 {
        format!("{} %", d.round() as i64)
    } else {
        format!("{d:.1} %").replace('.', ",")
    }
}

fn orc_ensure_space(
    doc: &PdfDocumentReference,
    bg: &Option<Vec<u8>>,
    page: &mut PdfPageIndex,
    layer: &mut PdfLayerIndex,
    y: &mut f32,
    h: f32,
) {
    if *y - h < PAGE_BOT {
        turn_page(doc, bg, page, layer, y);
    }
}

fn draw_orc_budget(
    doc: &PdfDocumentReference,
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    font: &IndirectFontRef,
    bg: &Option<Vec<u8>>,
    y_top: f32,
    o: &OrcamentoComercial,
) -> f32 {
    if o.linhas.is_empty() {
        return 0.0;
    }
    let x0 = CARD_X0 + CARD_PAD;
    let x1 = CARD_X1 - CARD_PAD;
    let off: i64 = o.linhas.iter().map(|l| l.discount_cents()).sum();
    let show_disc = off > 0;
    let valor_w = 28.0;
    let disc_w = if show_disc { 16.0 } else { 0.0 };
    let unit_w = 30.0;
    let qty_w = 18.0;
    let gutter = 4.0;
    let col_valor = x1;
    let col_disc = x1 - valor_w;
    let col_unit = if show_disc {
        col_disc - disc_w
    } else {
        x1 - valor_w
    };
    let col_qty = col_unit - unit_w;
    let desc_w = (col_qty - qty_w - gutter - x0).max(36.0);
    let pt = body_pt();
    let step = line_skip(pt, 4.2);
    let title_h = 6.4_f32;
    let cols = chars_for_width_mm(desc_w, pt);
    let rows: Vec<(Vec<String>, String, String, String, String)> = o
        .linhas
        .iter()
        .map(|l| {
            let qty = if l.is_mao() {
                format!("{} h", fmt_qty_pt(l.qty))
            } else {
                fmt_qty_pt(l.qty)
            };
            let unit = if l.is_mao() {
                format!("{} /h", ops::euro(l.unit_cents()).replace(" €", ""))
            } else {
                ops::euro(l.unit_cents())
            };
            (
                wrap(&l.designacao, cols)
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>(),
                qty,
                unit,
                fmt_disc_pct(l.disc_pct),
                ops::euro(l.valor_cents()),
            )
        })
        .collect();
    let rows_h: f32 = rows
        .iter()
        .map(|(d, _, _, _, _)| d.len().max(1) as f32 * step)
        .sum();
    let disc_row = if show_disc { step } else { 0.0 };
    let h = CARD_PAD + title_h + 3.0 + step + rows_h + disc_row + step + 2.0 + CARD_PAD;
    gold_card(doc, page, layer, CARD_X0, y_top - h, CARD_X1, y_top);
    let mut cy = y_top - CARD_PAD - 1.0;
    place_cinzel(doc, page, layer, bg, &o.titulo, x0, cy, title_h);
    cy -= title_h + 1.2;
    gold_rule(doc, page, layer, x0, x1, cy + 1.2);
    cy -= 5.4;
    let head_y = cy;
    paint_line(doc, page, layer, font, bg, x0, head_y, "Designação", pt, true);
    paint_right(doc, page, layer, font, bg, col_qty, head_y, "Qtd", pt, true);
    paint_right(doc, page, layer, font, bg, col_unit, head_y, "EUR", pt, true);
    if show_disc {
        paint_right(doc, page, layer, font, bg, col_disc, head_y, "Desc. %", pt, true);
    }
    paint_right(doc, page, layer, font, bg, col_valor, head_y, "Valor", pt, true);
    cy -= 2.2;
    gold_rule(doc, page, layer, x0, x1, cy);
    cy -= step - 0.6;
    for (desc, qty, unit, disc, valor) in &rows {
        let row_y = cy;
        let lines = if desc.is_empty() {
            vec!["-".to_string()]
        } else {
            desc.clone()
        };
        for (i, line) in lines.iter().enumerate() {
            paint_line(
                doc,
                page,
                layer,
                font,
                bg,
                x0,
                row_y - i as f32 * step,
                line,
                pt,
                false,
            );
        }
        paint_right(doc, page, layer, font, bg, col_qty, row_y, qty, pt, false);
        paint_right(doc, page, layer, font, bg, col_unit, row_y, unit, pt, false);
        if show_disc {
            paint_right(doc, page, layer, font, bg, col_disc, row_y, disc, pt, false);
        }
        paint_right(doc, page, layer, font, bg, col_valor, row_y, valor, pt, false);
        let used = lines.len().max(1) as f32 * step;
        gold_rule(doc, page, layer, x0, x1, cy - used + step - 1.2);
        cy -= used;
    }
    let tot = o.linhas.iter().map(|l| l.valor_cents()).sum::<i64>();
    if show_disc {
        paint_line(doc, page, layer, font, bg, x0, cy, "Desconto", pt, false);
        paint_right(
            doc,
            page,
            layer,
            font,
            bg,
            col_valor,
            cy,
            &format!("-{}", ops::euro(off)),
            pt,
            false,
        );
        cy -= step;
    }
    paint_line(doc, page, layer, font, bg, x0, cy, "Total", pt + 1.0, false);
    paint_right(
        doc,
        page,
        layer,
        font,
        bg,
        col_valor,
        cy,
        &ops::euro(tot),
        pt + 1.0,
        false,
    );
    h
}

fn draw_orc_notes(
    doc: &PdfDocumentReference,
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    font: &IndirectFontRef,
    bg: &Option<Vec<u8>>,
    y_top: f32,
    notas: &[String],
) -> f32 {
    let body: Vec<InkLine> = notas
        .iter()
        .filter(|n| filled(n))
        .flat_map(|n| {
            n.split('\n').flat_map(|para| {
                if para.trim().is_empty() {
                    vec![]
                } else {
                    ink_wrap(para, body_cols(), body_pt(), 0.0, false, body_gap())
                }
            })
        })
        .collect();
    if body.is_empty() {
        return 0.0;
    }
    let x0 = CARD_X0 + CARD_PAD;
    let title_h = 6.2_f32;
    let body_h: f32 = body.iter().map(ink_skip).sum();
    let h = CARD_PAD + title_h + 3.0 + body_h + CARD_PAD;
    gold_card(doc, page, layer, CARD_X0, y_top - h, CARD_X1, y_top);
    let mut cy = y_top - CARD_PAD - 1.0;
    place_cinzel(doc, page, layer, bg, "Notas", x0, cy, title_h);
    cy -= title_h + 1.0;
    gold_rule(doc, page, layer, x0, CARD_X1 - CARD_PAD, cy + 1.4);
    cy -= 5.2;
    for line in &body {
        paint_line(
            doc,
            page,
            layer,
            font,
            bg,
            x0 + line.indent,
            cy,
            &line.text,
            line.size,
            line.dim,
        );
        cy -= ink_skip(line);
    }
    h
}

fn draw_orc_top_pair(
    doc: &PdfDocumentReference,
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    font: &IndirectFontRef,
    bg: &Option<Vec<u8>>,
    y_top: f32,
    o: &OrcamentoComercial,
) -> f32 {
    let gap = 3.0;
    let mid = (CARD_X0 + CARD_X1) / 2.0;
    let left1 = mid - gap / 2.0;
    let right0 = mid + gap / 2.0;
    let body = 5.0_f32;
    let step = 6.0_f32;
    let logo_mm = 14.0_f32;

    let mut left_lines: Vec<String> = vec!["VANGUARDA AUTOMÓVEL".into(), "Orçamento".into()];
    let mut num = format!("N.º {}", o.numero);
    if !o.data.trim().is_empty() {
        num.push_str("  ·  ");
        num.push_str(o.data.trim());
    }
    left_lines.push(num);
    if !o.colaborador.trim().is_empty() {
        left_lines.push(format!("Por  {}", o.colaborador.trim()));
    }
    if !o.departamento.trim().is_empty() {
        left_lines.push(o.departamento.trim().to_string());
    }

    let mut right_lines: Vec<String> = Vec::new();
    if !o.cliente.trim().is_empty() {
        right_lines.push(o.cliente.trim().to_string());
    }
    if !o.veiculo.trim().is_empty() {
        right_lines.push(o.veiculo.trim().to_string());
    }
    if !o.matricula.trim().is_empty() {
        right_lines.push(format!("Matrícula  {}", o.matricula.trim()));
    }
    if !o.vin.trim().is_empty() {
        right_lines.push(format!("VIN  {}", o.vin.trim()));
    }

    let left_inner = (left1 - CARD_X0 - CARD_PAD * 2.0).max(40.0);
    let right_inner = (CARD_X1 - right0 - CARD_PAD * 2.0).max(40.0);
    let left_h: f32 = left_lines
        .iter()
        .map(|s| cinzel_wrap(s, left_inner - logo_mm - 4.0, body).len().max(1) as f32 * step)
        .sum();
    let right_h: f32 = right_lines
        .iter()
        .map(|s| cinzel_wrap(s, right_inner, body).len().max(1) as f32 * step)
        .sum::<f32>()
        + 8.0;
    let h = (CARD_PAD + logo_mm.max(left_h) + CARD_PAD)
        .max(CARD_PAD + 8.0 + right_h + CARD_PAD)
        .max(36.0);

    gold_card(doc, page, layer, CARD_X0, y_top - h, left1, y_top);
    gold_card(doc, page, layer, right0, y_top - h, CARD_X1, y_top);

    let mut text_x = CARD_X0 + CARD_PAD;
    if let Some(path) = media::find_logo().filter(|p| p.is_file()) {
        if let Ok(dynimg) = ::image::open(&path) {
            let rgba = trim_rgba(dynimg.to_rgba8());
            let (pw, ph) = ::image::GenericImageView::dimensions(&rgba);
            let w_mm = if ph > 0 {
                logo_mm * (pw as f32 / ph as f32)
            } else {
                logo_mm
            };
            if let Some(jpeg) = flatten_rgba_on_page(
                &rgba,
                bg,
                CARD_X0 + CARD_PAD,
                y_top - CARD_PAD,
                w_mm,
                logo_mm,
            ) {
                place_jpeg(
                    doc,
                    page,
                    layer,
                    &jpeg,
                    CARD_X0 + CARD_PAD,
                    y_top - CARD_PAD,
                    logo_mm,
                );
                text_x = CARD_X0 + CARD_PAD + w_mm + 3.0;
            }
        }
    }
    let wrap_w = (left1 - text_x - CARD_PAD).max(20.0);
    let mut ly = y_top - CARD_PAD - 1.0;
    place_cinzel(doc, page, layer, bg, "VANGUARDA AUTOMÓVEL", text_x, ly, 5.4);
    ly -= 7.4;
    place_cinzel(doc, page, layer, bg, &o.titulo, text_x, ly, 7.2);
    ly -= 8.4;
    let pt = body_pt();
    let gap = line_skip(pt, 3.6);
    let info: Vec<String> = left_lines.into_iter().skip(2).collect();
    for line in &info {
        let cols = chars_for_width_mm(wrap_w, pt);
        for w in wrap(line, cols) {
            if w.is_empty() {
                continue;
            }
            paint_line(doc, page, layer, font, bg, text_x, ly, &w, pt, false);
            ly -= gap;
        }
    }

    let rx = right0 + CARD_PAD;
    let mut ry = y_top - CARD_PAD - 1.0;
    place_cinzel(doc, page, layer, bg, "Cliente", rx, ry, 5.4);
    ry -= 7.2;
    let rcols = chars_for_width_mm(right_inner, pt);
    for line in &right_lines {
        for w in wrap(line, rcols) {
            if w.is_empty() {
                continue;
            }
            paint_line(doc, page, layer, font, bg, rx, ry, &w, pt, false);
            ry -= gap;
        }
    }
    h
}

/// Client-facing orçamento A4 (house metal, gold, logo flattened — no PNG /SMask).
/// No IVA, no NIF, no page footer.
pub fn write_orcamento_comercial_pdf(path: &Path, o: &OrcamentoComercial) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let (doc, mut page, mut layer) =
        PdfDocument::new("Orcamento", Mm(210.0), Mm(297.0), "Layer 1");
    let font = load_font(&doc)?;
    let bg = load_pdf_bg_jpeg();
    let mut y = PAGE_TOP;
    draw_page_metal(&doc, page, layer, &bg);

    let head_h = 42.0;
    orc_ensure_space(&doc, &bg, &mut page, &mut layer, &mut y, head_h);
    let pair_h = draw_orc_top_pair(&doc, page, layer, &font, &bg, y, o);
    y -= pair_h + CARD_GAP;

    if !o.linhas.is_empty() {
        let est = 32.0 + o.linhas.len() as f32 * 8.0;
        orc_ensure_space(&doc, &bg, &mut page, &mut layer, &mut y, est);
        let h = draw_orc_budget(&doc, page, layer, &font, &bg, y, o);
        y -= h + CARD_GAP;
    }

    let notes: Vec<String> = o
        .notas
        .iter()
        .filter(|n| filled(n))
        .cloned()
        .collect();
    if !notes.is_empty() {
        let est = 28.0 + notes.iter().map(|n| n.len() / 40 + 2).sum::<usize>() as f32 * 5.0;
        orc_ensure_space(&doc, &bg, &mut page, &mut layer, &mut y, est);
        let h = draw_orc_notes(&doc, page, layer, &font, &bg, y, &notes);
        y -= h + CARD_GAP;
    }
    if !o.disclaimer.trim().is_empty() {
        paint_line(
            &doc,
            page,
            layer,
            &font,
            &bg,
            CARD_X0 + CARD_PAD,
            PAGE_BOT + 5.0,
            o.disclaimer.trim(),
            body_pt() - 2.0,
            true,
        );
    }
    let _ = y;

    let mut raw = Vec::new();
    {
        let mut w = BufWriter::new(Cursor::new(&mut raw));
        doc.save(&mut w)?;
        w.flush()?;
    }
    fs::write(path, sanitize_printpdf(&raw))?;
    Ok(())
}

pub fn write_job(dir: &Path, j: &Job) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(dir)?;
    let mut rows = vec![
        ("N.º".into(), format!("{}", j.id)),
        ("Departamento".into(), ops::tipo_label(&j.tipo).into()),
        ("Estado".into(), j.estado.clone()),
        ("Cliente".into(), j.cliente.clone()),
        ("Matrícula".into(), j.matricula.clone()),
        ("VIN".into(), j.vin.clone()),
        ("Pack".into(), j.pack.clone()),
        ("Previsão".into(), ops::euro(j.previsao_cents)),
        ("Agendado".into(), j.agendado.clone()),
        ("Garantia (meses)".into(), if j.garantia_meses > 0 {
            j.garantia_meses.to_string()
        } else {
            String::new()
        }),
    ];
    rows.retain(|(_, v)| !v.trim().is_empty());
    let table = if j.linhas.is_empty() {
        None
    } else {
        let show_disc = j.linhas.iter().any(|l| l.discount_cents() > 0);
        Some((
            if show_disc {
                vec![
                    "Descrição".into(),
                    "Qtd".into(),
                    "Desc. %".into(),
                    "Valor".into(),
                ]
            } else {
                vec!["Descrição".into(), "Qtd".into(), "Valor".into()]
            },
            j.linhas
                .iter()
                .map(|l| {
                    if show_disc {
                        vec![
                            l.desc.clone(),
                            format!("{}", l.qty),
                            fmt_disc_pct(l.disc_pct),
                            ops::euro(l.total_cents()),
                        ]
                    } else {
                        vec![
                            l.desc.clone(),
                            format!("{}", l.qty),
                            ops::euro(l.total_cents()),
                        ]
                    }
                })
                .collect(),
        ))
    };
    let spec = Spec {
        title: "Ordem de serviço".into(),
        subtitle: j.label(),
        meta: rows,
        sections: vec![Section {
            heading: "Trabalho".into(),
            rows: vec![],
            bullets: if j.notas.trim().is_empty() {
                vec![]
            } else {
                vec![j.notas.clone()]
            },
            table,
        }],
        footer: format!("{}  ·  {}", today(), EMPRESA),
        logo: media::find_logo(),
        photos: vec![],
    };
    let base = format!("OS_{}", j.id);
    write_spec(dir, &base, &spec, &WriteOpts::default())
}

pub fn write_guia_pdf(path: &Path, title: &str, body: &str) -> Result<()> {
    let (doc, mut page, mut layer) =
        PdfDocument::new("Guia", Mm(210.0), Mm(297.0), "Layer 1");
    let font = load_font(&doc)?;
    let bg = load_pdf_bg_jpeg();
    let mut y = PAGE_TOP;
    draw_page_metal(&doc, page, layer, &bg);
    draw_header_card(
        &doc,
        &mut page,
        &mut layer,
        &mut y,
        &font,
        &bg,
        title,
        "Vanguarda Automovel",
        &[],
        media::find_logo().filter(|p| p.is_file()).as_deref(),
    );
    let mut heading = String::new();
    let mut buf: Vec<InkLine> = Vec::new();
    let cols = body_cols();
    let flush = |heading: &mut String,
                 buf: &mut Vec<InkLine>,
                 doc: &PdfDocumentReference,
                 page: &mut PdfPageIndex,
                 layer: &mut PdfLayerIndex,
                 y: &mut f32,
                 font: &IndirectFontRef,
                 bg: &Option<Vec<u8>>| {
        if heading.trim().is_empty() && buf.is_empty() {
            return;
        }
        draw_card(doc, page, layer, y, font, bg, heading, None, buf);
        heading.clear();
        buf.clear();
    };
    for raw in body.lines() {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("# ") && !t.starts_with("## ") {
            continue;
        }
        if let Some(rest) = t.strip_prefix("## ") {
            flush(
                &mut heading,
                &mut buf,
                &doc,
                &mut page,
                &mut layer,
                &mut y,
                &font,
                &bg,
            );
            heading = rest.to_string();
            continue;
        }
        let line = t.strip_prefix("- ").unwrap_or(t);
        buf.extend(ink_wrap(
            line,
            cols,
            body_pt(),
            0.0,
            t.starts_with("- "),
            body_gap(),
        ));
    }
    flush(
        &mut heading,
        &mut buf,
        &doc,
        &mut page,
        &mut layer,
        &mut y,
        &font,
        &bg,
    );
    let mut raw = Vec::new();
    {
        let mut w = BufWriter::new(Cursor::new(&mut raw));
        doc.save(&mut w)?;
        w.flush()?;
    }
    fs::write(path, sanitize_printpdf(&raw))?;
    Ok(())
}

/// Reprint a diagnostic PDF with extra photo pages. `dest` is a new file.
pub fn write_diag_pdf_with_photos(
    dest: &Path,
    scan: &Scan,
    cliente: &str,
    stars: &[SistemaNota],
    photos: &[(String, PathBuf)],
) -> Result<()> {
    let jpegs = load_photo_jpegs(photos)?;
    write_diag_pdf(dest, scan, cliente, stars, &jpegs)
}

fn write_spec_pdf_with_photos(dest: &Path, mut spec: Spec, photos: &[(String, PathBuf)]) -> Result<()> {
    spec.photos = load_photo_jpegs(photos)?;
    write_pdf(dest, &spec)
}

pub fn client_pdf_with_photos(
    dest: &Path,
    f: &ClientFicha,
    dir: &Path,
    photos: &[(String, PathBuf)],
) -> Result<()> {
    write_spec_pdf_with_photos(dest, spec_client(f, dir), photos)
}

pub fn carro_pdf_with_photos(
    dest: &Path,
    car: &Carro,
    dir: &Path,
    photos: &[(String, PathBuf)],
) -> Result<()> {
    write_spec_pdf_with_photos(dest, spec_carro(car, dir), photos)
}

pub fn staff_pdf_with_photos(
    dest: &Path,
    f: &StaffFicha,
    dir: &Path,
    photos: &[(String, PathBuf)],
) -> Result<()> {
    write_spec_pdf_with_photos(dest, spec_staff(f, dir), photos)
}

pub fn consumo_pdf_with_photos(
    dest: &Path,
    log: &ConsumoLog,
    dir: &Path,
    photos: &[(String, PathBuf)],
) -> Result<()> {
    write_spec_pdf_with_photos(dest, spec_consumo(log, dir), photos)
}

fn sanitize_printpdf(bytes: &[u8]) -> Vec<u8> {
    // printpdf always writes `/SMask null` and a Form-style `/BBox` on Image
    // XObjects. Adobe Reader reports that as error 135.
    let mut out = bytes.to_vec();
    for pat in [
        &b"/SMask null"[..],
        &b"/BBox[1 0 0 1 0 0]"[..],
        &b"/DecodeParams<</ColorTransform 0>>"[..],
    ] {
        out = replace_bytes(&out, pat, b"");
    }
    out
}

fn replace_bytes(hay: &[u8], needle: &[u8], repl: &[u8]) -> Vec<u8> {
    if needle.is_empty() {
        return hay.to_vec();
    }
    let mut out = Vec::with_capacity(hay.len());
    let mut i = 0;
    while i < hay.len() {
        if hay[i..].starts_with(needle) {
            out.extend_from_slice(repl);
            i += needle.len();
        } else {
            out.push(hay[i]);
            i += 1;
        }
    }
    out
}

fn logo_emu_size(bytes: &[u8], width_mm: f32) -> (u32, u32) {
    let w_emu = (width_mm / 25.4 * 914_400.0) as u32;
    let (px_w, px_h) = ::image::load_from_memory(bytes)
        .ok()
        .map(|i| ::image::GenericImageView::dimensions(&i))
        .unwrap_or((1, 1));
    let h_emu = ((w_emu as u64) * (px_h.max(1) as u64) / (px_w.max(1) as u64)) as u32;
    (w_emu, h_emu.max(1))
}

fn trim_rgba(img: ::image::RgbaImage) -> ::image::RgbaImage {
    let (w, h) = ::image::GenericImageView::dimensions(&img);
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    for (x, y, p) in ::image::GenericImageView::pixels(&img) {
        if p.0[3] > 12 {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    if min_x >= max_x || min_y >= max_y {
        return img;
    }
    let pad = 4u32;
    let x0 = min_x.saturating_sub(pad);
    let y0 = min_y.saturating_sub(pad);
    let x1 = (max_x + 1 + pad).min(w);
    let y1 = (max_y + 1 + pad).min(h);
    ::image::imageops::crop_imm(&img, x0, y0, x1 - x0, y1 - y0).to_image()
}

fn jpeg_dimensions(jpeg: &[u8]) -> (f32, f32) {
    ::image::load_from_memory(jpeg)
        .ok()
        .map(|i| {
            let (w, h) = ::image::GenericImageView::dimensions(&i);
            (w as f32, h as f32)
        })
        .unwrap_or((1.0, 1.0))
}

fn pdf_info_title(s: &str) -> String {
    pdf_safe(s)
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ã' | 'â' => 'a',
            'Á' | 'À' | 'Ã' | 'Â' => 'A',
            'é' | 'è' | 'ê' => 'e',
            'É' | 'È' | 'Ê' => 'E',
            'ó' | 'ò' | 'õ' | 'ô' => 'o',
            'Ó' | 'Ò' | 'Õ' | 'Ô' => 'O',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
            'ç' => 'c',
            'Ç' => 'C',
            'ñ' => 'n',
            'Ñ' => 'N',
            c if c.is_ascii() => c,
            _ => ' ',
        })
        .collect()
}

fn load_font(doc: &PdfDocumentReference) -> Result<IndirectFontRef> {
    // Helvetica (WinAnsi) keeps Portuguese accents and correct glyph widths.
    // Embedded TTF via printpdf 0.7 stretches letter-spacing on this machine.
    doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| anyhow::anyhow!("fonte PDF: {e}"))
}

fn pdf_safe(s: &str) -> String {
    s.replace('●', "*")
        .replace('—', "-")
        .replace('–', "-")
        .replace('‘', "'")
        .replace('’', "'")
        .replace('“', "\"")
        .replace('”', "\"")
}

fn wrap(s: &str, max: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for raw in s.split('\n') {
        if raw.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut cur = String::new();
        for word in raw.split_whitespace() {
            if cur.is_empty() {
                cur = word.to_string();
            } else if cur.chars().count() + 1 + word.chars().count() <= max {
                cur.push(' ');
                cur.push_str(word);
            } else {
                lines.push(std::mem::take(&mut cur));
                cur = word.to_string();
            }
        }
        if !cur.is_empty() {
            lines.push(cur);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag_parse::Dtc;
    use crate::model::ConsumoEntry;

    #[test]
    fn writes_and_edits_temp_client() {
        let root = std::env::temp_dir().join(format!("ficha-smoke-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let mut c = ClientFicha::blank("TesteGrok");
        c.telemovel = "+351 911 000 000".into();
        crate::media::ensure_client_media(&root).unwrap();
        fs::write(root.join("Media").join("Antes").join("risco.jpg"), b"x").unwrap();
        let files = write_client(&root, &c, WriteOpts::default()).unwrap();
        assert!(files.iter().any(|p| p.extension().and_then(|e| e.to_str()) == Some("pdf")));
        assert!(files.iter().any(|p| p.extension().and_then(|e| e.to_str()) == Some("docx")));
        let md = fs::read_to_string(root.join("Ficha_Cliente_TesteGrok.md")).unwrap();
        assert!(md.contains("Antes/risco.jpg"), "media must be listed in the ficha");
        assert!(!md.contains("[●]"), "empty fields must not print");
        assert!(md.contains("Vanguarda Automóvel"));
        assert!(root.join("Media").join("Veiculo").is_dir());

        c.email = "teste@vanguardaautomovel.com".into();
        write_client(&root, &c, WriteOpts::default()).unwrap();
        let mut loaded: ClientFicha =
            serde_json::from_str(&fs::read_to_string(root.join("ficha.json")).unwrap()).unwrap();
        let key = crate::vault::load_or_create_key(&crate::vault::data_root_guess(&root)).unwrap();
        crate::vault::reveal_client(&key, &mut loaded);
        assert_eq!(loaded.email, "teste@vanguardaautomovel.com");
        assert!(fs::read_to_string(root.join("ficha.json"))
            .unwrap()
            .contains("enc:v1:"));

        let mut log = ConsumoLog::blank("Rodrigo");
        log.entradas.push(ConsumoEntry {
            data: "2026-08-20".into(),
            tipo: "reparação".into(),
            descricao: "teste interno".into(),
            custo_interno: "0,00".into(),
            ..Default::default()
        });
        write_consumo(&root.join("consumo"), &log, WriteOpts::default()).unwrap();
        assert!(root.join("consumo").join("Consumo_Interno_Rodrigo.pdf").exists());

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn writes_vanguarda_diagnostico_not_autocom() {
        let root = std::env::temp_dir().join(format!("ficha-diag-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let scan = Scan {
            data: "20/08/2026".into(),
            vin: "WVGZZZ1TZ9W034244".into(),
            matricula: "60-HU-86".into(),
            veiculo: "Volkswagen - Touran [03-10] - 2009".into(),
            source_name: "2026_08_20_23_12_ERROS.pdf".into(),
            dtcs: vec![Dtc {
                sistema: "Sistema electrónico do motor".into(),
                codigo: "P161A".into(),
                descricao: "Vela de incandescência do cilindro 1, circuito aberto.".into(),
                estado: "Permanente".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let files =
            write_diagnostico(&root, "Diagnostico_60_HU_86_20-08-2026", &scan, "Inês").unwrap();
        assert!(files
            .iter()
            .any(|p| p.extension().and_then(|e| e.to_str()) == Some("pdf")));
        let pdf = root.join("Diagnostico_60_HU_86_20-08-2026.pdf");
        assert!(pdf.is_file());
        assert!(fs::metadata(&pdf).unwrap().len() > 400);
        let md = fs::read_to_string(root.join("Diagnostico_60_HU_86_20-08-2026.md")).unwrap();
        assert!(md.contains("Relatório de diagnóstico"));
        assert!(md.contains("WVGZZZ1TZ9W034244"));
        assert!(md.contains("P161A"));
        assert!(md.contains("Vanguarda Automóvel"));
        assert!(md.contains("Inês"));
        assert!(md.contains("### Motor"));
        assert!(!md.contains("Autocom"));
        assert!(!md.contains("Ficheiro origem"));
        assert!(!md.to_lowercase().contains("code4bin"));
        assert!(!md.to_lowercase().contains("autocom"));
        assert!(root.join("Diagnostico_60_HU_86_20-08-2026.json").is_file());
        let pdf_bytes = fs::read(&pdf).unwrap();
        let ascii = String::from_utf8_lossy(&pdf_bytes);
        assert!(!ascii.contains("Leitura das avarias"));
        assert!(!ascii.contains("A reparacao deve ser confirmada"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn pdf_with_transparent_logo_has_no_inline_smask() {
        let dir = std::env::temp_dir().join(format!("ficha-pdf-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let mut img = ::image::RgbaImage::new(16, 16);
        for p in img.pixels_mut() {
            *p = ::image::Rgba([0xC4, 0xA3, 0x5A, 128]);
        }
        let logo = dir.join("logo.png");
        img.save(&logo).unwrap();
        std::env::set_var("FICHA_LOGO", logo.display().to_string());
        let scan = Scan {
            vin: "WVGZZZ1TZ9W034244".into(),
            dtcs: vec![Dtc {
                sistema: "Travao (ABS/ESP)".into(),
                codigo: "P161A".into(),
                descricao: "teste".into(),
                estado: "Permanente".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        write_diagnostico(&dir, "t", &scan, "Inês").unwrap();
        let bytes = fs::read(dir.join("t.pdf")).unwrap();
        let ascii = String::from_utf8_lossy(&bytes);
        assert!(ascii.starts_with("%PDF"));
        assert!(ascii.contains("startxref"));
        assert!(
            !ascii.contains("/SMask"),
            "printpdf SMask (null or inline) is invalid PDF (Adobe error 135)"
        );
        let title_at = ascii.find("/Title(").expect("title");
        let title = &ascii[title_at..title_at + 40];
        assert!(title.is_ascii(), "Info Title must be ASCII for Adobe: {title}");
        std::env::remove_var("FICHA_LOGO");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn example_pdfs_are_metal_jpeg() {
        let dir = std::env::temp_dir().join(format!("v-ex-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let written = write_tab_examples(&dir).unwrap();
        assert_eq!(written.len(), 5, "{written:?}");
        for p in &written {
            let bytes = fs::read(p).unwrap();
            let ascii = String::from_utf8_lossy(&bytes);
            assert!(ascii.starts_with("%PDF"), "{}", p.display());
            assert!(!ascii.contains("/SMask"), "{}", p.display());
            assert!(
                ascii.contains("/DCTDecode") || ascii.contains("/DCT"),
                "expected jpeg on {}: {}",
                p.display(),
                ascii.len()
            );
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn recado_stays_off_viatura_pdf() {
        let dir = std::env::temp_dir().join(format!("v-recado-pdf-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let mut car = Carro::blank("Ana");
        car.matricula = "00-XX-00".into();
        car.vin = "WVGZZZ1TZ9W000000".into();
        car.marca = "Volkswagen".into();
        car.recado = "SEGREDO-INTERNO-XYZ".into();
        car.recado_quem = "Gil".into();
        write_carro(&dir, &car, WriteOpts::default()).unwrap();
        let md = fs::read_to_string(dir.join("Ficha_Viatura_00_XX_00.md")).unwrap();
        assert!(!md.contains("SEGREDO-INTERNO-XYZ"));
        assert!(!md.to_lowercase().contains("recado"));
        let pdf = fs::read(dir.join("Ficha_Viatura_00_XX_00.pdf")).unwrap();
        let ascii = String::from_utf8_lossy(&pdf);
        assert!(!ascii.contains("SEGREDO-INTERNO-XYZ"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rounded_card_path_has_corners() {
        let pts = rounded_card_points(14.0, 100.0, 196.0, 180.0, CARD_RADIUS);
        assert!(pts.len() >= 16);
        assert!(pts.iter().any(|(_, bez)| *bez));
        assert!(pts.iter().any(|(_, bez)| !*bez));
    }

    #[test]
    fn formal_pdf_is_adobe_safe() {
        set_house_style(DocStyle::formal());
        let dir = std::env::temp_dir().join(format!("v-formal-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let scan = Scan {
            vin: "WVGZZZ1TZ9W000000".into(),
            matricula: "00-XX-00".into(),
            titulo: "Relatório".into(),
            dtcs: vec![Dtc {
                categoria: "Motor".into(),
                codigo: "P161A".into(),
                descricao: "teste".into(),
                estado: "Presente".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        write_diagnostico(&dir, "t", &scan, "Ana").unwrap();
        let bytes = fs::read(dir.join("t.pdf")).unwrap();
        let ascii = String::from_utf8_lossy(&bytes);
        assert!(ascii.starts_with("%PDF"));
        assert!(!ascii.contains("/SMask"), "adobe 135");
        assert!(ascii.contains("/DCTDecode") || ascii.contains("/DCT"));
        set_house_style(DocStyle::default());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn estilizado_outline_pdf_is_adobe_safe() {
        set_house_style(DocStyle::default());
        let dir = std::env::temp_dir().join(format!("v-outline-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let scan = Scan {
            vin: "WVGZZZ1TZ9W000000".into(),
            matricula: "00-XX-00".into(),
            titulo: "Relatório".into(),
            dtcs: vec![Dtc {
                categoria: "Motor".into(),
                codigo: "P161A".into(),
                descricao: "teste de contorno".into(),
                estado: "Presente".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        write_diagnostico(&dir, "t", &scan, "Ana").unwrap();
        let bytes = fs::read(dir.join("t.pdf")).unwrap();
        let ascii = String::from_utf8_lossy(&bytes);
        assert!(ascii.starts_with("%PDF"));
        assert!(!ascii.contains("/SMask"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    #[ignore]
    fn write_fabric_previews_for_design() {
        let _ = crate::media::seed_design_kit();
        let fundos = crate::media::design_fundos_dir();
        let samples = [
            ("tecido-veludo.jpg", "veludo"),
            ("tecido-liso.jpg", "liso"),
            ("tecido-cetim.jpg", "cetim"),
        ];
        let scan = Scan {
            vin: "WVGZZZ1TZ9W000000".into(),
            matricula: "00-XX-00".into(),
            titulo: "Relatório".into(),
            veiculo: "Volkswagen Touran".into(),
            km: "214 000 km".into(),
            sumario: "Pré-visualização do texto dourado sobre o tecido.".into(),
            dtcs: vec![Dtc {
                categoria: "Motor".into(),
                codigo: "P161A".into(),
                descricao: "Circuito aberto.".into(),
                estado: "Presente".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let fundo_path = crate::media::fundo_bg_path();
        let backup = fs::read(&fundo_path).ok();
        for (file, tag) in samples {
            let src = fundos.join(file);
            if !src.is_file() {
                continue;
            }
            crate::media::save_fundo_bg(&src).unwrap();
            let mut st = DocStyle::default();
            st.fundo = "imagem".into();
            set_house_style(st);
            let pdf = fundos.join(format!("_prova-{tag}.pdf"));
            write_diag_pdf_with_photos(&pdf, &scan, "Ana", &[], &[]).unwrap();
            if let Ok(jpeg) = crate::diag_parse::render_pdf_first_page_jpeg(&pdf) {
                fs::write(fundos.join(format!("_prova-{tag}.jpg")), jpeg).unwrap();
            }
        }
        if let Some(b) = backup {
            let _ = fs::write(&fundo_path, b);
        } else if fundo_path.is_file() {
            let _ = fs::remove_file(&fundo_path);
        }
        set_house_style(DocStyle::default());
    }

    #[test]
    fn care_menu_pdf_has_iva_and_premium() {
        set_house_style(DocStyle::default());
        let dir = std::env::temp_dir().join(format!("v-guia-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let body = crate::guias::seed_text(crate::auth::Departamento::Care, false);
        write_guia_pdf(&dir.join("m.pdf"), "Care — Menu", body).unwrap();
        let bytes = fs::read(dir.join("m.pdf")).unwrap();
        let ascii = String::from_utf8_lossy(&bytes);
        assert!(ascii.starts_with("%PDF"));
        assert!(!ascii.contains("/SMask"));
        assert!(body.contains("IVA incluído"));
        assert!(body.contains("Premium"));
        fs::remove_dir_all(&dir).unwrap();
        set_house_style(DocStyle::default());
    }

    #[test]
    fn xona_identificacao_does_not_eat_sumario() {
        set_house_style(DocStyle::default());
        let dir = std::env::temp_dir().join(format!("v-xona-id-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let scan = Scan {
            data: "2026-08-21".into(),
            vin: "achas q eu sei dred".into(),
            matricula: "n vou espigar esses mambos".into(),
            veiculo: "Citroen AX ya daqueles antigos tas a ver".into(),
            km: "bem pouco wi ya".into(),
            sumario: "Ya, bwede mambos espigados nesse bote, motor nao eh d origem.".into(),
            mecanico: "Rodrigo Sousa".into(),
            titulo: "Relatorio".into(),
            ..Default::default()
        };
        let stars = vec![
            SistemaNota {
                categoria: "Motor".into(),
                estrelas: 5,
            },
            SistemaNota {
                categoria: "Travoes e estabilidade".into(),
                estrelas: 5,
            },
        ];
        write_diag_pdf_with_photos(&dir.join("t.pdf"), &scan, "Xona bwe da street", &stars, &[])
            .unwrap();
        let bytes = fs::read(dir.join("t.pdf")).unwrap();
        let ascii = String::from_utf8_lossy(&bytes);
        assert!(ascii.starts_with("%PDF"));
        assert!(!ascii.contains("/SMask"));
        let cols = body_cols();
        let id = ink_wrap(
            "Matricula: n vou espigar esses mambos",
            cols,
            10.0,
            0.0,
            false,
            5.0,
        );
        assert_eq!(id.len(), 1, "this value must fit one line");
        assert!(line_skip(10.0, 5.0) >= 5.0);
        set_house_style(DocStyle::default());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn flatten_png_corners_keep_white_page() {
        set_house_style(DocStyle::formal());
        let mut rgba = ::image::RgbaImage::from_pixel(16, 16, ::image::Rgba([0, 0, 0, 0]));
        rgba.put_pixel(8, 8, ::image::Rgba([0xD4, 0xB0, 0x6A, 255]));
        let jpeg = flatten_rgba_on_page(&rgba, &None, 20.0, 280.0, 20.0, 20.0).unwrap();
        assert!(jpeg.starts_with(&[0xFF, 0xD8]));
        let img = ::image::load_from_memory(&jpeg).unwrap().to_rgb8();
        let c = img.get_pixel(0, 0).0;
        assert!(c[0] > 240 && c[1] > 240 && c[2] > 240, "corner {c:?}");
        let m = img.get_pixel(8, 8).0;
        assert!(m[0] > 160 && m[1] > 120, "ink {m:?}");
        set_house_style(DocStyle::default());
    }

    #[test]
    fn flatten_text_strip_keeps_width() {
        let rgba =
            ::image::RgbaImage::from_pixel(1200, 48, ::image::Rgba([0xD4, 0xB0, 0x6A, 200]));
        let jpeg =
            flatten_rgba_on_page_max(&rgba, &None, 20.0, 280.0, 170.0, 5.2, 1800).unwrap();
        let img = ::image::load_from_memory(&jpeg).unwrap();
        assert!(
            img.width() >= 800,
            "text strip must not crush to 256, got {}",
            img.width()
        );
        let crushed =
            flatten_rgba_on_page_max(&rgba, &None, 20.0, 280.0, 170.0, 5.2, 256).unwrap();
        let small = ::image::load_from_memory(&crushed).unwrap();
        assert!(small.width() <= 256);
    }

    #[test]
    fn cinzel_wrap_keeps_paragraphs() {
        let lines = cinzel_wrap("Primeira linha do relatório.\nSegunda linha.", 120.0, 5.2);
        assert!(lines.len() >= 2, "{lines:?}");
        assert!(lines[0].contains("Primeira"), "{lines:?}");
        assert!(
            lines.iter().any(|l| l.contains("Segunda")),
            "{lines:?}"
        );
    }

    #[test]
    fn gold_art_punches_plate() {
        let mut img = ::image::RgbaImage::from_pixel(8, 8, ::image::Rgba([0x40, 0x44, 0x48, 255]));
        img.put_pixel(3, 3, ::image::Rgba([0xD4, 0xB0, 0x6A, 255]));
        let out = gold_art_transparent(img);
        assert_eq!(out.get_pixel(0, 0).0[3], 0);
        assert_eq!(out.get_pixel(3, 3).0, [0xD4, 0xB0, 0x6A, 255]);
    }

    #[test]
    fn photo_pages_keep_original_and_are_jpeg() {
        let dir = std::env::temp_dir().join(format!("v-anexo-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let scan = Scan {
            vin: "WVGZZZ1TZ9W000000".into(),
            matricula: "00-XX-00".into(),
            dtcs: vec![Dtc {
                categoria: "Motor".into(),
                codigo: "P161A".into(),
                descricao: "teste".into(),
                estado: "Presente".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        write_diagnostico(&dir, "t", &scan, "Ana").unwrap();
        let base = dir.join("t.pdf");
        let orig = fs::read(&base).unwrap();
        let mut img = ::image::RgbImage::new(32, 24);
        for p in img.pixels_mut() {
            *p = ::image::Rgb([0xC4, 0x40, 0x30]);
        }
        let foto = dir.join("risco.jpg");
        ::image::DynamicImage::ImageRgb8(img)
            .save(&foto)
            .unwrap();
        let dest = dir.join("t_com_anexos.pdf");
        write_diag_pdf_with_photos(
            &dest,
            &scan,
            "Ana",
            &[],
            &[("Antes / risco.jpg".into(), foto)],
        )
        .unwrap();
        let after = fs::read(&base).unwrap();
        assert_eq!(orig, after, "original report must not change");
        let extra = fs::read(&dest).unwrap();
        let ascii = String::from_utf8_lossy(&extra);
        assert!(ascii.starts_with("%PDF"));
        assert!(!ascii.contains("/SMask"), "adobe 135");
        assert!(extra.len() > orig.len());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn orcamento_linha_qtd_vezes_unitario() {
        let l = OrcamentoLinha::peca("Kit", 2.0, 28.974);
        assert_eq!(l.unit_cents(), 2897);
        assert_eq!(l.valor_cents(), 5794);
        let mao = OrcamentoLinha::mao("Reparação", 3.0, 40.0);
        assert_eq!(mao.valor_cents(), 12000);
        assert!(mao.is_mao());
        assert!(!l.is_mao());
        let mut cut = l.clone();
        cut.disc_pct = 10.0;
        assert_eq!(cut.unit_cents(), 2897);
        assert_eq!(cut.valor_cents(), 5215);
        assert_eq!(cut.discount_cents(), 579);
    }

    #[test]
    fn orcamento_comercial_pdf_sem_iva() {
        let dir = std::env::temp_dir().join(format!("ficha-orc-com-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("o.pdf");
        write_orcamento_comercial_pdf(
            &dest,
            &OrcamentoComercial {
                titulo: "Orçamento".into(),
                numero: "1".into(),
                data: "04/09/2026".into(),
                hora: "20:46".into(),
                colaborador: "Gil Salvador".into(),
                departamento: "Oficina".into(),
                cliente_n: "1".into(),
                cliente: "Joana".into(),
                nif: String::new(),
                veiculo: "Volkswagen Touran".into(),
                matricula: "60-HU-86".into(),
                vin: "WVGZZZ1TZ9W034244".into(),
                linhas: vec![OrcamentoLinha::mao("Reparação", 3.0, 40.0)],
                notas: vec![
                    "Relatório 04/09/2026\nPonteiras de direcção, apoios do triângulo, depósito de expansão e tubo de saída. Fora deste orçamento: fecho centralizado e sensores do habitáculo.".into(),
                ],
                disclaimer: String::new(),
            },
        )
        .unwrap();
        let raw = fs::read(&dest).unwrap();
        let ascii = String::from_utf8_lossy(&raw);
        assert!(ascii.starts_with("%PDF"));
        assert!(!ascii.contains("/SMask"));
        assert!(!ascii.contains("Valores sem IVA"));
        assert!(!ascii.contains("IVA incluído"));
        assert!(!ascii.contains("IVA a acrescer"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn write_quote_sem_iva_pecas_e_mao() {
        let root = std::env::temp_dir().join(format!("ficha-orc-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let mut q = Quote {
            numero: "ORC-2026-0001".into(),
            cliente: "Ana Teste".into(),
            tipo: "oficina".into(),
            colaborador: "Gil Salvador".into(),
            iva_incluido: false,
            linhas: vec![
                crate::ops::Line::peca("Filtro de óleo", 1.0, 1200),
                crate::ops::Line::mao("Substituição do filtro", 3.0, 4000),
            ],
            ..Default::default()
        };
        q.recompute();
        write_quote(&root, &q).unwrap();
        let pdf = root.join("Orcamento_ORC_2026_0001.pdf");
        assert!(pdf.is_file());
        let raw = fs::read(&pdf).unwrap();
        let ascii = String::from_utf8_lossy(&raw);
        assert!(ascii.starts_with("%PDF"));
        assert!(!ascii.contains("/SMask"));
        assert!(!ascii.contains("Valores sem IVA"));
        assert!(!ascii.contains("IVA incluído"));
        let o = quote_as_comercial(&q, "Touran");
        assert_eq!(o.colaborador, "Gil Salvador");
        assert_eq!(o.departamento, "Oficina");
        assert_eq!(o.linhas.len(), 2);
        assert!(o.linhas[1].is_mao());
        assert_eq!(o.linhas[1].qty, 3.0);
        assert_eq!(o.linhas[1].valor_cents(), 12000);
        assert_eq!(o.linhas[0].disc_pct, 0.0);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn peca_margin_is_house_only() {
        let mut l = crate::ops::Line::peca("Multi-V belt", 1.0, 0);
        l.custo_cents = 2000;
        l.margem_pct = 40.0;
        l.apply_margin();
        let mut q = Quote {
            numero: "ORC-2026-0010".into(),
            cliente: "Susana Sousa".into(),
            tipo: "oficina".into(),
            linhas: vec![l],
            ..Default::default()
        };
        q.recompute();
        assert_eq!(q.total_cents, 2800);
        assert_eq!(q.pecas_custo_cents(), 2000);
        assert_eq!(q.lucro_cents(), 800);
        let o = quote_as_comercial(&q, "Focus");
        assert_eq!(o.linhas[0].unit_cents(), 2800);
        assert_eq!(o.linhas[0].disc_pct, 0.0);
        let dir = std::env::temp_dir().join(format!("ficha-orc-mg-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        write_quote(&dir, &q).unwrap();
        let raw = fs::read(dir.join("Orcamento_ORC_2026_0010.pdf")).unwrap();
        let ascii = String::from_utf8_lossy(&raw);
        assert!(ascii.starts_with("%PDF"));
        assert!(!ascii.contains("Custo pecas") && !ascii.contains("Margem"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn quote_disc_pct_lands_on_comercial() {
        let mut q = Quote {
            numero: "ORC-2026-0009".into(),
            cliente: "Joana".into(),
            tipo: "oficina".into(),
            linhas: vec![{
                let mut l = crate::ops::Line::peca("Kit", 2.0, 2897);
                l.disc_pct = 10.0;
                l
            }],
            ..Default::default()
        };
        q.recompute();
        assert_eq!(q.total_cents, 5215);
        let o = quote_as_comercial(&q, "Touran");
        assert_eq!(o.linhas[0].disc_pct, 10.0);
        assert_eq!(o.linhas[0].valor_cents(), 5215);
        assert_eq!(o.linhas[0].discount_cents(), 579);
        let dir = std::env::temp_dir().join(format!("ficha-orc-disc-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        write_quote(&dir, &q).unwrap();
        let pdf = dir.join("Orcamento_ORC_2026_0009.pdf");
        assert!(pdf.is_file());
        let raw = fs::read(&pdf).unwrap();
        let ascii = String::from_utf8_lossy(&raw);
        assert!(ascii.starts_with("%PDF"));
        assert!(!ascii.contains("/SMask"));
        let _ = fs::remove_dir_all(&dir);
    }
}

pub fn write_tab_examples(dir: &Path) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(dir)?;
    let mut out = Vec::new();
    let mut cliente = ClientFicha::blank("Cliente Exemplo");
    cliente.telemovel = "+351 910 000 000".into();
    cliente.email = "oficina@vanguardaautomovel.com".into();
    cliente.veiculo_marca = "Volkswagen".into();
    cliente.veiculo_modelo = "Touran".into();
    cliente.veiculo_matricula = "00-XX-00".into();
    cliente.veiculo_vin = "WVGZZZ1TZ9W000000".into();
    cliente.estado = "Exemplo".into();
    let cdir = dir.join("Cliente");
    fs::create_dir_all(&cdir)?;
    out.extend(write_client(&cdir, &cliente, WriteOpts::default())?);

    let mut car = Carro::blank("Cliente Exemplo");
    car.matricula = "00-XX-00".into();
    car.vin = "WVGZZZ1TZ9W000000".into();
    car.marca = "Volkswagen".into();
    car.modelo = "Touran".into();
    car.ano = "2009".into();
    let vdir = dir.join("Viatura");
    fs::create_dir_all(&vdir)?;
    out.extend(write_carro(&vdir, &car, WriteOpts::default())?);

    let mut staff = StaffFicha::blank("Tecnico Exemplo");
    staff.funcao = "Mecanico".into();
    staff.telemovel = "+351 910 000 001".into();
    let sdir = dir.join("Staff");
    fs::create_dir_all(&sdir)?;
    out.extend(write_staff(&sdir, &staff, WriteOpts::default())?);

    let mut log = ConsumoLog::blank("Tecnico Exemplo");
    log.entradas.push(crate::model::ConsumoEntry {
        data: crate::model::today(),
        tipo: "reparação".into(),
        descricao: "Exemplo de linha interna".into(),
        custo_interno: "0,00".into(),
        quem_fez: "Tecnico Exemplo".into(),
        ..Default::default()
    });
    let kdir = dir.join("Consumo");
    fs::create_dir_all(&kdir)?;
    out.extend(write_consumo(&kdir, &log, WriteOpts::default())?);

    let scan = Scan {
        titulo: "Exemplo diagnostico".into(),
        data: "21/08/2026".into(),
        vin: "WVGZZZ1TZ9W000000".into(),
        matricula: "00-XX-00".into(),
        veiculo: "Volkswagen Touran".into(),
        km: "180000".into(),
        dtcs: vec![crate::diag_parse::Dtc {
            sistema: "Sistema electrónico do motor".into(),
            categoria: "Motor".into(),
            codigo: "P161A".into(),
            descricao: "Exemplo de código — não é uma viatura real.".into(),
            estado: "Presente".into(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let ddir = dir.join("Diagnostico");
    fs::create_dir_all(&ddir)?;
    out.extend(write_diagnostico(
        &ddir,
        "Exemplo_Diagnostico",
        &scan,
        "Cliente Exemplo",
    )?);

    let pdfs: Vec<PathBuf> = out
        .into_iter()
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("pdf"))
        .collect();
    Ok(pdfs)
}
