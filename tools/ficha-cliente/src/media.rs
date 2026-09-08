use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub const CLIENT_SUBS: &[&str] = &["Veiculo", "Antes", "Depois", "Recibos", "Diagnosticos"];
pub const STAFF_SUBS: &[&str] = &["Reparacoes", "Consumo"];
pub const CAR_SUBS: &[&str] = &["Veiculo", "Diagnosticos"];

const CLIENT_README: &str = "\
Media deste cliente (pen Vanguarda)
===================================
Veiculo/        fotos da viatura
Antes/          avarias / estado à entrada (com consentimento)
Depois/         após o serviço
Recibos/        comprovativos (não substitui faturação certificada)
Diagnosticos/   relatórios Vanguarda importados de PDFs de oficina

Não meter aqui vídeos de armazém nem logótipos da empresa
(isso fica em Design\\Media).
";

const STAFF_README: &str = "\
Media interno (pen Vanguarda)
=============================
Reparacoes/  fotos de reparações in-company
Consumo/     peças / uso de viatura da casa

Logótipos e vídeos de marca ficam em Design\\Media.
";

pub fn ensure_client_media(person_dir: &Path) -> Result<PathBuf> {
    ensure_tree(person_dir, CLIENT_SUBS, CLIENT_README)
}

pub fn ensure_staff_media(person_dir: &Path) -> Result<PathBuf> {
    ensure_tree(person_dir, STAFF_SUBS, STAFF_README)
}

pub fn ensure_car_media(car_dir: &Path) -> Result<PathBuf> {
    ensure_tree(
        car_dir,
        CAR_SUBS,
        "Media desta viatura\nVeiculo/       fotos\nDiagnosticos/  relatórios Vanguarda Automóvel\n",
    )
}

fn ensure_tree(person_dir: &Path, subs: &[&str], readme: &str) -> Result<PathBuf> {
    let media = person_dir.join("Media");
    fs::create_dir_all(&media)?;
    for sub in subs {
        fs::create_dir_all(media.join(sub))?;
    }
    let readme_path = media.join("README.txt");
    if !readme_path.exists() {
        fs::write(&readme_path, readme)?;
    }
    Ok(media)
}

/// (subpasta, nome do ficheiro)
pub fn list_media(person_dir: &Path) -> Vec<(String, String)> {
    let media = person_dir.join("Media");
    let mut out = Vec::new();
    let Ok(subs) = fs::read_dir(&media) else {
        return out;
    };
    let mut sub_names: Vec<_> = subs
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    sub_names.sort();
    for sub in sub_names {
        let Ok(files) = fs::read_dir(media.join(&sub)) else {
            continue;
        };
        let mut names: Vec<_> = files
            .flatten()
            .filter(|e| e.path().is_file())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        for n in names {
            out.push((sub.clone(), n));
        }
    }
    out
}

fn is_printable_image(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.ends_with(".jpg") || n.ends_with(".jpeg") || n.ends_with(".png")
}

/// Photos/receipts that can ride on a print PDF. Skip reports and README.
pub fn printable_attachments(roots: &[(&str, PathBuf)]) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    for (prefix, dir) in roots {
        if !dir.is_dir() {
            continue;
        }
        for (sub, name) in list_media(dir) {
            if sub.eq_ignore_ascii_case("Diagnosticos") {
                continue;
            }
            if name.eq_ignore_ascii_case("README.txt") {
                continue;
            }
            if !is_printable_image(&name) {
                continue;
            }
            let label = if prefix.is_empty() {
                format!("{sub} / {name}")
            } else {
                format!("{prefix} / {sub} / {name}")
            };
            out.push((label, dir.join("Media").join(&sub).join(&name)));
        }
    }
    out
}

pub fn media_bullets(person_dir: &Path) -> Vec<String> {
    let listed = list_media(person_dir);
    if listed.is_empty() {
        return vec![
            "Media/ ainda sem ficheiros — usar «Anexar» no formulário.".into(),
        ];
    }
    listed
        .into_iter()
        .map(|(sub, name)| format!("{sub}/{name}"))
        .collect()
}

pub fn attach_into(person_dir: &Path, sub: &str, sources: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let dest_dir = person_dir.join("Media").join(sub);
    fs::create_dir_all(&dest_dir)?;
    let mut copied = Vec::new();
    for src in sources {
        let name = src
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "anexo.bin".into());
        let dest = unique_path(&dest_dir, &name);
        fs::copy(src, &dest)
            .with_context(|| format!("a copiar {} → {}", src.display(), dest.display()))?;
        copied.push(dest);
    }
    Ok(copied)
}

fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let dest = dir.join(name);
    if !dest.exists() {
        return dest;
    }
    let stem = Path::new(name)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "anexo".into());
    let ext = Path::new(name)
        .extension()
        .map(|s| format!(".{}", s.to_string_lossy()))
        .unwrap_or_default();
    for i in 2..1000 {
        let p = dir.join(format!("{stem}-{i}{ext}"));
        if !p.exists() {
            return p;
        }
    }
    dir.join(format!("{stem}-mais{ext}"))
}

pub fn find_logo() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("FICHA_LOGO") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    let mut candidates = Vec::new();
    let home = crate::paths::vanguarda_home();
    candidates.push(home.join("App").join("logo.png"));
    candidates.push(home.join("Design").join("Logotipo").join("logo.png"));
    candidates.push(home.join("Empresa").join("Identidade").join("logo.png"));
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let base = PathBuf::from(local).join("Vanguarda");
        candidates.push(base.join("logo.png"));
        candidates.push(base.join("Design").join("Logotipo").join("logo.png"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("logo.png"));
            candidates.push(dir.join("logo.jpg"));
        }
    }
    for letter in b'C'..=b'Z' {
        let root = format!(r"{}:\Vanguarda\Design\Logotipo", letter as char);
        candidates.push(PathBuf::from(&root).join("Main Logo.png"));
        candidates.push(PathBuf::from(&root).join(
            "grok-image-ab00c227-e698-4d32-9f5c-40c9d8083c1c.png",
        ));
        candidates.push(PathBuf::from(format!(
            r"{}:\Vanguarda\Office\Vanguarda\logo.png",
            letter as char
        )));
        candidates.push(PathBuf::from(format!(
            r"{}:\Vanguarda\Office\FichaCliente\logo.png",
            letter as char
        )));
        candidates.push(PathBuf::from(format!(
            r"{}:\Vanguarda\Design\Media\Images\Vanguarda_Automovel_Logo_Primary.jpg",
            letter as char
        )));
    }
    candidates.into_iter().find(|p| p.is_file())
}

fn asset_candidates(name: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(p) = std::env::var("FICHA_ASSET_DIR") {
        candidates.push(PathBuf::from(p).join(name));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(name));
            candidates.push(dir.join("assets").join("bg").join(name));
        }
    }
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("bg")
            .join(name),
    );
    for letter in b'C'..=b'Z' {
        candidates.push(PathBuf::from(format!(
            r"{}:\Vanguarda\Office\Vanguarda\{name}",
            letter as char
        )));
        candidates.push(PathBuf::from(format!(
            r"{}:\Vanguarda\Office\FichaCliente\{name}",
            letter as char
        )));
    }
    candidates
}

pub fn find_ui_bg() -> Option<PathBuf> {
    asset_candidates("ui-bg.jpg")
        .into_iter()
        .chain(asset_candidates("ui-bg.png"))
        .find(|p| p.is_file())
}

pub fn find_chip_bg() -> Option<PathBuf> {
    asset_candidates("chip-bg.jpg").into_iter().find(|p| p.is_file())
}

pub fn find_pdf_bg() -> Option<PathBuf> {
    asset_candidates("pdf-bg.jpg").into_iter().find(|p| p.is_file())
}

fn fundos_dir() -> PathBuf {
    let house = design_fundos_dir();
    if house.is_dir() {
        return house;
    }
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into());
    PathBuf::from(base).join("Vanguarda").join("fundos")
}

pub fn simples_bg_path() -> PathBuf {
    fundos_dir().join("simples.jpg")
}

pub fn fundo_bg_path() -> PathBuf {
    fundos_dir().join("fundo.jpg")
}

pub fn find_simples_bg() -> Option<PathBuf> {
    let p = simples_bg_path();
    p.is_file().then_some(p)
}

pub fn find_fundo_bg() -> Option<PathBuf> {
    let p = fundo_bg_path();
    if p.is_file() {
        return Some(p);
    }
    find_simples_bg()
}

pub fn save_simples_bg(src: &Path) -> Result<PathBuf> {
    save_fundo_jpeg(src, &simples_bg_path())
}

pub fn save_fundo_bg(src: &Path) -> Result<PathBuf> {
    save_fundo_jpeg(src, &fundo_bg_path())
}

fn save_fundo_jpeg(src: &Path, dest: &Path) -> Result<PathBuf> {
    let jpeg = crate::writers::image_file_to_jpeg(src)?;
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(dest, jpeg)?;
    Ok(dest.to_path_buf())
}

pub fn design_root() -> PathBuf {
    let house = crate::paths::vanguarda_home().join("Design");
    if house.is_dir() {
        return house;
    }
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into());
    PathBuf::from(base).join("Vanguarda").join("Design")
}

pub fn design_icones_dir() -> PathBuf {
    design_root().join("Icones")
}

pub fn design_fundos_dir() -> PathBuf {
    design_root().join("Fundos")
}

pub fn is_design_icon(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.contains("Design") && (s.contains("Icones") || s.contains("Ícones"))
}

const ICON_STEMS: &[&str] = &[
    "adas",
    "airbag",
    "cliente",
    "clima",
    "conforto",
    "direccao",
    "electrico",
    "estado",
    "imobilizador",
    "mecanico",
    "motor",
    "outros",
    "painel",
    "portas",
    "radio",
    "rede",
    "sumario",
    "travoes",
];

const DESIGN_ICONES_README: &str = "\
Icones do PDF — pasta da Design
================================
Substitui estes ficheiros pelos que tratares no Grok Imagine (sem fundo).
Mantém o mesmo nome: motor.jpg, travoes.jpg, airbag.jpg, …

PNG sem fundo: usa o mesmo nome (motor.png, travoes.png, …).
O PDF da casa mistura o PNG com o fundo da página (Adobe não aceita transparência).

Fundos de página: pasta irmã Fundos\\
";

/// Copy bundled icons into Design\\Icones if that folder is empty, so Design
/// can swap files without touching the crate.
pub fn seed_design_kit() -> Result<PathBuf> {
    let icones = design_icones_dir();
    fs::create_dir_all(&icones)?;
    let readme = icones.join("LEIA-ME.txt");
    if !readme.is_file() {
        fs::write(&readme, DESIGN_ICONES_README)?;
    }
    for stem in ICON_STEMS {
        let dest = icones.join(format!("{stem}.jpg"));
        if dest.is_file() {
            continue;
        }
        if let Some(src) = bundled_icon(stem) {
            let _ = fs::copy(src, dest);
        }
    }
    let fundos = design_fundos_dir();
    fs::create_dir_all(&fundos)?;
    let fundos_readme = fundos.join("LEIA-ME.txt");
    if !fundos_readme.is_file() {
        fs::write(
            &fundos_readme,
            "Fundos de página (JPEG). Escolhe um em Documentos → Imagem.\n\
             Os três tecidos pretos da secretária estão aqui para testes.\n",
        )?;
    }
    seed_desktop_fundos(&fundos);
    seed_named_pngs(&icones);
    Ok(icones)
}

const GROK_ICON_MAP: &[(&str, &str)] = &[
    ("grok-image-2da07bda-45ad-469b-8812-4d32f45f5e90.png", "motor.png"),
    ("grok-image-e52efdb2-2f7e-46dc-a37d-46f610732c6c.png", "travoes.png"),
    ("grok-image-49b656ba-8ba5-4530-b7ed-3608ee169e6e.png", "painel.png"),
    ("grok-image-e0b5cfb4-cc76-46ac-8ec2-0b11d0db1f8f.png", "airbag.png"),
    ("grok-image-fabe9291-dfa4-4c3e-af76-b5e62e711d47.png", "direccao.png"),
    ("grok-image-066f6621-16a3-47be-982a-dcf63fa814d3.png", "portas.png"),
    ("grok-image-63e9aad9-936b-4602-a10e-4d18d0c9d8d8.png", "electrico.png"),
    ("grok-image-b0724220-5eae-438a-b86c-b79bce4a3149.png", "conforto.png"),
    ("grok-image-e65272e7-9aed-408c-bf26-6be7c49eaeae.png", "rede.png"),
    ("grok-image-529592c8-e733-47da-9a98-5174c880d0e0.png", "adas.png"),
    ("grok-image-0b9d03d2-0e2f-4b3e-b331-9db28f1c5933.png", "imobilizador.png"),
    ("grok-image-4d4a1e5a-d89c-4752-99aa-67ca6dbc1144.png", "radio.png"),
    ("grok-image-2ae75b6b-0f9c-49e4-af2e-fbd28576e34d.png", "mecanico.png"),
    ("grok-image-feed8ebf-783d-48c7-bda1-dfcc3bb385e2.png", "sumario.png"),
    ("grok-image-ff9882ed-80d6-4621-8532-9da969df1651.png", "estado.png"),
    ("grok-image-527a4465-d1e2-421a-bdaf-c6e155286ab2.png", "outros.png"),
];

fn seed_named_pngs(icones: &Path) {
    for (src_name, dest_name) in GROK_ICON_MAP {
        let dest = icones.join(dest_name);
        if dest.is_file() {
            continue;
        }
        let src = icones.join(src_name);
        if src.is_file() {
            let _ = fs::copy(&src, &dest);
        }
    }
}

pub fn list_fundos() -> Vec<PathBuf> {
    let dir = design_fundos_dir();
    let Ok(rd) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('_') || name.eq_ignore_ascii_case("LEIA-ME.txt") {
                return false;
            }
            matches!(
                p.extension().and_then(|e| e.to_str()).map(|s| s.to_ascii_lowercase()),
                Some(ref e) if e == "jpg" || e == "jpeg" || e == "png"
            )
        })
        .collect();
    out.sort();
    out
}

fn seed_desktop_fundos(fundos: &Path) {
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    let desks = [
        PathBuf::from(&home).join("OneDrive").join("Ambiente de Trabalho"),
        PathBuf::from(&home).join("Ambiente de Trabalho"),
        PathBuf::from(&home).join("Desktop"),
    ];
    let pairs = [
        ("Possible pdf background.jpg", "tecido-veludo.jpg"),
        ("Possible 3 (1).jpg", "tecido-liso.jpg"),
        ("Possible 3 (2).jpg", "tecido-cetim.jpg"),
    ];
    for desk in desks {
        if !desk.is_dir() {
            continue;
        }
        for (src_name, dest_name) in pairs {
            let dest = fundos.join(dest_name);
            if dest.is_file() {
                continue;
            }
            let src = desk.join(src_name);
            if src.is_file() {
                if let Ok(jpeg) = crate::writers::image_file_to_jpeg(&src) {
                    let _ = fs::write(&dest, jpeg);
                }
            }
        }
    }
}

fn bundled_icon(stem: &str) -> Option<PathBuf> {
    icon_candidates(stem)
        .into_iter()
        .find(|p| p.is_file() && !is_design_icon(p))
}

fn icon_candidates(stem: &str) -> Vec<PathBuf> {
    let jpg = format!("{stem}.jpg");
    let png = format!("{stem}.png");
    let mut out = Vec::new();
    let design = design_icones_dir();
    out.push(design.join(&png));
    out.push(design.join(&jpg));
    if let Ok(p) = std::env::var("FICHA_ASSET_DIR") {
        let base = PathBuf::from(p);
        out.push(base.join("icons").join(&jpg));
        out.push(base.join(&jpg));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            out.push(dir.join("icons").join(&jpg));
            out.push(dir.join(&jpg));
            out.push(dir.join("assets").join("icons").join(&jpg));
        }
    }
    out.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("icons")
            .join(&jpg),
    );
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        out.push(PathBuf::from(local).join("Vanguarda").join("icons").join(&jpg));
    }
    out
}

/// Category plate for PDF section heads. Missing file → `None` (hairline only).
pub fn find_icon(stem: &str) -> Option<PathBuf> {
    icon_candidates(stem).into_iter().find(|p| p.is_file())
}

fn fold_key(s: &str) -> String {
    s.to_lowercase()
        .replace('á', "a")
        .replace('à', "a")
        .replace('ã', "a")
        .replace('â', "a")
        .replace('é', "e")
        .replace('ê', "e")
        .replace('í', "i")
        .replace('ó', "o")
        .replace('ô', "o")
        .replace('õ', "o")
        .replace('ú', "u")
        .replace('ç', "c")
}

/// Map a heading / Autocom category to an icon stem (`motor.jpg`, …).
pub fn icon_stem(heading: &str) -> Option<&'static str> {
    let n = fold_key(heading);
    if n.contains("airbag") || n.contains("seguranca") {
        Some("airbag")
    } else if n.contains("trav") {
        Some("travoes")
    } else if n.contains("painel") {
        Some("painel")
    } else if n.contains("clima") || n.contains("aquecimento") {
        Some("clima")
    } else if n.contains("direc") {
        Some("direccao")
    } else if n.contains("porta") || n.contains("fecho") {
        Some("portas")
    } else if n.contains("electric") || n.contains("eletrico") {
        Some("electrico")
    } else if n.contains("conforto") || n.contains("habitaculo") {
        Some("conforto")
    } else if n.contains("rede") {
        Some("rede")
    } else if n.contains("ajuda") || n.contains("adas") || n.contains("conducao") {
        Some("adas")
    } else if n.contains("imobil") {
        Some("imobilizador")
    } else if n.contains("radio") {
        Some("radio")
    } else if n.contains("motor") {
        Some("motor")
    } else if n.contains("mecanico") {
        Some("mecanico")
    } else if n.contains("sumario") || n.contains("nota") {
        Some("sumario")
    } else if n.contains("contacto")
        || n.contains("identific")
        || n.contains("cliente")
        || n.contains("rgpd")
        || n.contains("privacidade")
    {
        Some("cliente")
    } else if n.contains("estado")
        || n.contains("saude")
        || n.contains("consumo")
        || n.contains("conta corrente")
    {
        Some("estado")
    } else if n.contains("veiculo") || n.contains("viatura") {
        Some("estado")
    } else if n.contains("outro") || n.contains("media") || n.contains("documento") || n.contains("pasta") {
        Some("outros")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_and_list() {
        let root = std::env::temp_dir().join(format!("ficha-media-{}", std::process::id()));
        let dir = root.join("Joana");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&dir).unwrap();
        ensure_client_media(&dir).unwrap();
        assert!(dir.join("Media").join("Antes").is_dir());
        assert!(dir.join("Media").join("README.txt").is_file());
        fs::write(dir.join("Media").join("Antes").join("risco.jpg"), b"fake").unwrap();
        let listed = list_media(&dir);
        assert_eq!(listed, vec![("Antes".into(), "risco.jpg".into())]);
        let bullets = media_bullets(&dir);
        assert_eq!(bullets, vec!["Antes/risco.jpg"]);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn find_logo_env() {
        let p = std::env::temp_dir().join(format!("ficha-logo-{}.png", std::process::id()));
        fs::write(&p, b"not-a-real-png-but-a-file").unwrap();
        std::env::set_var("FICHA_LOGO", p.display().to_string());
        let found = find_logo().unwrap();
        assert_eq!(found, p);
        std::env::remove_var("FICHA_LOGO");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn attach_renames_collision() {
        let root = std::env::temp_dir().join(format!("ficha-attach-{}", std::process::id()));
        let dir = root.join("Gil");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&dir).unwrap();
        ensure_staff_media(&dir).unwrap();
        let src = root.join("foto.png");
        fs::write(&src, b"aaa").unwrap();
        attach_into(&dir, "Reparacoes", &[src.clone()]).unwrap();
        attach_into(&dir, "Reparacoes", &[src]).unwrap();
        assert!(dir.join("Media/Reparacoes/foto.png").is_file());
        assert!(dir.join("Media/Reparacoes/foto-2.png").is_file());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn icon_stems_cover_categories() {
        assert_eq!(icon_stem("Motor"), Some("motor"));
        assert_eq!(icon_stem("Travões e estabilidade"), Some("travoes"));
        assert_eq!(icon_stem("Segurança (airbag)"), Some("airbag"));
        assert_eq!(icon_stem("Ajuda à condução"), Some("adas"));
        assert_eq!(icon_stem("Sumário do mecânico"), Some("mecanico"));
        assert_eq!(icon_stem("Estado da viatura (5 em ordem, 1 grave)"), Some("estado"));
        assert_eq!(icon_stem("Identificação"), Some("cliente"));
        assert_eq!(icon_stem("Contactos"), Some("cliente"));
        assert_eq!(icon_stem("Conta corrente"), Some("estado"));
        assert_eq!(icon_stem("Outros sistemas"), Some("outros"));
        assert!(find_icon("motor").is_some(), "motor.jpg must ship in assets/icons");
    }

    #[test]
    fn printable_skips_reports_and_readme() {
        let root = std::env::temp_dir().join(format!("ficha-print-{}", std::process::id()));
        let dir = root.join("Ana");
        let _ = fs::remove_dir_all(&root);
        ensure_client_media(&dir).unwrap();
        fs::write(dir.join("Media/Antes/risco.jpg"), b"xxxx").unwrap();
        fs::write(dir.join("Media/Diagnosticos/Diagnostico_x.pdf"), b"%PDF").unwrap();
        fs::write(dir.join("Media/README.txt"), b"no").unwrap();
        let list = printable_attachments(&[("Cliente", dir.clone())]);
        assert_eq!(list.len(), 1, "{list:?}");
        assert!(list[0].0.contains("Antes"));
        fs::remove_dir_all(&root).unwrap();
    }
}
