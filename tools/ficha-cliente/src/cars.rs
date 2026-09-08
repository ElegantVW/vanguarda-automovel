use crate::diag_parse;
use crate::model::{Carro, ClientFicha, DONO_INTERNO};
use crate::paths::{client_dir, interno_root, list_client_names};
use crate::scaffold;
use crate::slug::slug;
use crate::writers::{self, WriteOpts};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub fn owner_dir(root: &Path, dono: &str) -> PathBuf {
    if dono.trim().eq_ignore_ascii_case(DONO_INTERNO) {
        interno_root(root)
    } else {
        client_dir(root, dono.trim())
    }
}

pub fn car_key(car: &Carro) -> String {
    if !car.matricula.trim().is_empty() {
        slug(&car.matricula)
    } else if !car.vin.trim().is_empty() {
        slug(&car.vin)
    } else {
        "Viatura".into()
    }
}

pub fn car_dir(root: &Path, car: &Carro) -> PathBuf {
    owner_dir(root, &car.dono).join("Carros").join(car_key(car))
}

pub fn cars_for_owner(cars: &[Carro], dono: &str) -> Vec<Carro> {
    let d = dono.trim();
    cars.iter()
        .filter(|c| c.dono.trim().eq_ignore_ascii_case(d))
        .cloned()
        .collect()
}

pub fn list_cars(root: &Path) -> Vec<Carro> {
    let mut out = Vec::new();
    let mut owners = list_client_names(root);
    owners.push(DONO_INTERNO.to_string());
    for dono in owners {
        let cars_root = owner_dir(root, &dono).join("Carros");
        let Ok(rd) = fs::read_dir(&cars_root) else {
            continue;
        };
        for e in rd.flatten() {
            if !e.path().is_dir() {
                continue;
            }
            let json = e.path().join("carro.json");
            if let Ok(s) = fs::read_to_string(&json) {
                if let Ok(mut c) = serde_json::from_str::<Carro>(&s) {
                    if c.dono.trim().is_empty() {
                        c.dono = dono.clone();
                    }
                    out.push(c);
                }
            }
        }
    }
    out.sort_by(|a, b| {
        a.label()
            .to_lowercase()
            .cmp(&b.label().to_lowercase())
    });
    out
}

pub fn find_car(root: &Path, vin: &str, plate: &str) -> Option<Carro> {
    let vin = diag_parse::norm_id(vin);
    let plate = diag_parse::norm_id(plate);
    if vin.is_empty() && plate.is_empty() {
        return None;
    }
    for c in list_cars(root) {
        if !vin.is_empty() && diag_parse::ids_match(&c.vin, &vin) {
            return Some(c);
        }
        if !plate.is_empty() && diag_parse::ids_match(&c.matricula, &plate) {
            return Some(c);
        }
    }
    None
}

fn fill_empty(dst: &mut String, src: &str) {
    let src = src.trim();
    if crate::format::is_placeholder(src) {
        return;
    }
    if crate::format::is_placeholder(dst) || crate::format::looks_like_dtc_dump(dst) {
        *dst = src.to_string();
        return;
    }
    if dst.trim().is_empty() {
        *dst = src.to_string();
    }
}

fn merge_car(into: &mut Carro, from: &Carro) {
    fill_empty(&mut into.matricula, &from.matricula);
    fill_empty(&mut into.vin, &from.vin);
    fill_empty(&mut into.marca, &from.marca);
    fill_empty(&mut into.modelo, &from.modelo);
    fill_empty(&mut into.versao, &from.versao);
    fill_empty(&mut into.ano, &from.ano);
    fill_empty(&mut into.cor, &from.cor);
    fill_empty(&mut into.km, &from.km);
    fill_empty(&mut into.pneus, &from.pneus);
    fill_empty(&mut into.notas, &from.notas);
    if !from.estado_oficina.trim().is_empty() {
        into.estado_oficina = from.estado_oficina.clone();
    }
    fill_empty(&mut into.pecas_pendentes, &from.pecas_pendentes);
    if !from.recado_quem.trim().is_empty() {
        into.recado = from.recado.clone();
        into.recado_quem = from.recado_quem.clone();
        into.recado_quando = from.recado_quando.clone();
    }
    into.ensure_sistemas();
    for s in &from.sistemas {
        if s.estrelas == 0 {
            continue;
        }
        if let Some(row) = into.sistemas.iter_mut().find(|r| r.categoria == s.categoria) {
            row.estrelas = s.estrelas;
        }
    }
    if into.dono.trim().is_empty() {
        into.dono = from.dono.clone();
    }
}

pub fn upsert_car(root: &Path, incoming: &Carro) -> Result<PathBuf> {
    let mut car = incoming.clone();
    tidy_car(&mut car);
    if car.dono.trim().is_empty() {
        anyhow::bail!("o carro precisa de um dono (cliente ou Interno)");
    }
    if car.matricula.trim().is_empty() && car.vin.trim().is_empty() {
        anyhow::bail!("indica a matrícula ou o VIN");
    }
    if let Some(existing) = find_car(root, &car.vin, &car.matricula) {
        if existing.dono == car.dono {
            let mut merged = existing;
            merge_car(&mut merged, &car);
            car = merged;
        }
    }
    let dir = car_dir(root, &car);
    writers::write_carro(&dir, &car, WriteOpts::default())?;
    fill_client_from_car(root, &car);
    Ok(dir)
}

pub fn upsert_from_client(root: &Path, ficha: &ClientFicha) -> Result<Option<PathBuf>> {
    if ficha.veiculo_vin.trim().is_empty() && ficha.veiculo_matricula.trim().is_empty() {
        return Ok(None);
    }
    let incoming = Carro {
        dono: ficha.nome_completo.trim().to_string(),
        matricula: ficha.veiculo_matricula.clone(),
        vin: ficha.veiculo_vin.clone(),
        marca: ficha.veiculo_marca.clone(),
        modelo: ficha.veiculo_modelo.clone(),
        versao: ficha.veiculo_versao.clone(),
        ano: ficha.veiculo_ano.clone(),
        cor: ficha.veiculo_cor.clone(),
        km: ficha.veiculo_km.clone(),
        pneus: ficha.pneus.clone(),
        notas: ficha.observacoes_veiculo.clone(),
        ..Default::default()
    };
    Ok(Some(upsert_car(root, &incoming)?))
}

pub fn fill_client_from_car(root: &Path, car: &Carro) {
    if car.dono.trim().eq_ignore_ascii_case(DONO_INTERNO) {
        return;
    }
    let dir = client_dir(root, car.dono.trim());
    if scaffold::is_protected(&dir) {
        return;
    }
    let json = dir.join("ficha.json");
    let mut f = if json.exists() {
        fs::read_to_string(&json)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| ClientFicha::blank(&car.dono))
    } else {
        return;
    };
    fill_empty(&mut f.veiculo_matricula, &car.matricula);
    fill_empty(&mut f.veiculo_vin, &car.vin);
    fill_empty(&mut f.veiculo_marca, &car.marca);
    fill_empty(&mut f.veiculo_modelo, &car.modelo);
    fill_empty(&mut f.veiculo_versao, &car.versao);
    fill_empty(&mut f.veiculo_ano, &car.ano);
    fill_empty(&mut f.veiculo_cor, &car.cor);
    fill_empty(&mut f.veiculo_km, &car.km);
    fill_empty(&mut f.pneus, &car.pneus);
    if let Ok(body) = serde_json::to_string_pretty(&f) {
        let _ = fs::write(&json, body);
    }
}

pub fn apply_car_to_scan(car: &Carro, scan: &mut crate::diag_parse::Scan) {
    fill_empty(&mut scan.vin, &car.vin);
    fill_empty(&mut scan.matricula, &car.matricula);
    fill_empty(&mut scan.km, &car.km);
    if scan.veiculo.trim().is_empty() {
        let bits: Vec<&str> = [&car.marca, &car.modelo, &car.ano]
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if !bits.is_empty() {
            scan.veiculo = bits.join(" ");
        }
    }
}

pub fn upsert_from_scan(root: &Path, cliente: &str, scan: &crate::diag_parse::Scan) -> Result<Option<PathBuf>> {
    if scan.vin.trim().is_empty() && scan.matricula.trim().is_empty() {
        return Ok(None);
    }
    let mut car = find_car(root, &scan.vin, &scan.matricula).unwrap_or_else(|| Carro::blank(cliente));
    car.dono = cliente.to_string();
    fill_empty(&mut car.vin, &scan.vin);
    fill_empty(&mut car.matricula, &scan.matricula);
    fill_empty(&mut car.km, &scan.km);
    let bits = diag_parse::split_veiculo(&scan.veiculo);
    fill_empty(&mut car.marca, &bits.marca);
    fill_empty(&mut car.modelo, &bits.modelo);
    fill_empty(&mut car.versao, &bits.versao);
    fill_empty(&mut car.ano, &bits.ano);
    hint_stars_from_scan(&mut car, scan);
    Ok(Some(upsert_car(root, &car)?))
}

fn hint_stars_from_scan(car: &mut Carro, scan: &crate::diag_parse::Scan) {
    car.ensure_sistemas();
    let scanned = !scan.dtcs.is_empty();
    for row in &mut car.sistemas {
        if row.estrelas > 0 {
            continue;
        }
        let in_cat: Vec<_> = scan
            .dtcs
            .iter()
            .filter(|d| d.categoria == row.categoria)
            .collect();
        if in_cat.is_empty() {
            if scanned {
                row.estrelas = 5;
            }
            continue;
        }
        let present = in_cat.iter().any(|d| {
            let e = d.estado.to_lowercase();
            e.contains("presente") || e.contains("permanente")
        });
        row.estrelas = if present { 1 } else { 2 };
    }
}

#[derive(Debug, Clone)]
pub struct HubPulse {
    pub clients: usize,
    pub cars: usize,
    pub diags: usize,
    pub awaiting_parts: usize,
    pub in_repair: usize,
    pub drafts: Vec<String>,
    pub recent: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SavedDiag {
    pub label: String,
    pub path: PathBuf,
    pub client: String,
    pub is_pdf: bool,
}

fn owner_from_rel(root: &Path, p: &Path) -> String {
    let rel = p.strip_prefix(root).unwrap_or(p);
    let first = rel
        .components()
        .next()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .unwrap_or_default();
    if first.eq_ignore_ascii_case(DONO_INTERNO) {
        DONO_INTERNO.to_string()
    } else {
        first
    }
}

fn in_diagnosticos_dir(p: &Path) -> bool {
    p.parent()
        .and_then(|d| d.file_name())
        .map(|n| n.to_string_lossy().eq_ignore_ascii_case("Diagnosticos"))
        .unwrap_or(false)
}

fn is_scan_json_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n != "ficha.json" && n != "carro.json" && n != "staff.json" && n != "consumo.json"
}

fn diag_label(path: &Path, client: &str, stem: &str) -> String {
    let meta = fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<crate::diag_parse::Scan>(&s).ok());
    let Some(scan) = meta else {
        return format!("{stem}  —  {client}");
    };
    let title = scan.titulo.trim();
    if !title.is_empty() {
        return format!("{title}  —  {client}");
    }
    let key = if !scan.matricula.trim().is_empty() {
        scan.matricula.trim().to_string()
    } else if !scan.vin.trim().is_empty() {
        scan.vin.trim().to_string()
    } else {
        String::new()
    };
    let day = scan.data.trim();
    let mut parts = vec![stem.to_string()];
    if !day.is_empty() {
        parts.push(day.to_string());
    }
    if !key.is_empty() && !stem.contains(&key) {
        parts.push(key);
    }
    format!("{}  —  {client}", parts.join(" · "))
}

fn diag_rank(path: &Path, is_pdf: bool) -> (u8, u8) {
    let s = path.to_string_lossy().replace('\\', "/").to_ascii_lowercase();
    let media = s.contains("/media/diagnosticos/");
    (u8::from(is_pdf), u8::from(!media))
}

pub fn list_saved_diags(root: &Path) -> Vec<SavedDiag> {
    let mut raw: Vec<(SavedDiag, String, (u8, u8), Option<SystemTime>)> = Vec::new();
    for e in walkdir::WalkDir::new(root).into_iter().flatten() {
        let p = e.path();
        if !p.is_file() {
            continue;
        }
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let ext = p
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let stem = p
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let named = name.to_ascii_lowercase().starts_with("diagnostico_");
        if !in_diagnosticos_dir(p) && !named {
            continue;
        }
        let is_pdf = ext == "pdf";
        let is_json = ext == "json";
        if !is_pdf && !is_json {
            continue;
        }
        if is_json && !is_scan_json_name(&name) {
            continue;
        }
        let client = owner_from_rel(root, p);
        let label = if is_json {
            diag_label(p, &client, &stem)
        } else {
            format!("{stem}  —  {client}")
        };
        let mtime = fs::metadata(p).and_then(|m| m.modified()).ok();
        raw.push((
            SavedDiag {
                label,
                path: p.to_path_buf(),
                client: client.clone(),
                is_pdf,
            },
            format!("{}||{}", client.to_ascii_lowercase(), stem.to_ascii_lowercase()),
            diag_rank(p, is_pdf),
            mtime,
        ));
    }
    raw.sort_by(|a, b| a.2.cmp(&b.2).then_with(|| b.3.cmp(&a.3)));
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for (d, key, _, _) in raw {
        if !seen.insert(key) {
            continue;
        }
        out.push(d);
    }
    out.sort_by(|a, b| {
        let ta = fs::metadata(&a.path).and_then(|m| m.modified()).ok();
        let tb = fs::metadata(&b.path).and_then(|m| m.modified()).ok();
        tb.cmp(&ta)
            .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
    });
    out
}

pub fn hub_pulse(root: &Path) -> HubPulse {
    let clients = list_client_names(root);
    let cars = list_cars(root);
    let mut drafts = Vec::new();
    for n in &clients {
        let json = client_dir(root, n).join("ficha.json");
        if let Ok(s) = fs::read_to_string(&json) {
            if let Ok(f) = serde_json::from_str::<ClientFicha>(&s) {
                if f.estado.to_lowercase().contains("rascunho") {
                    drafts.push(n.clone());
                }
            }
        }
    }
    drafts.sort();
    let awaiting_parts = cars
        .iter()
        .filter(|c| c.estado_oficina.to_lowercase().contains("peças") || c.estado_oficina.to_lowercase().contains("pecas"))
        .count();
    let in_repair = cars
        .iter()
        .filter(|c| c.estado_oficina.to_lowercase().contains("reparação") || c.estado_oficina.to_lowercase().contains("reparacao"))
        .count();
    let saved = list_saved_diags(root);
    let diags = saved.len();
    let recent: Vec<String> = saved.into_iter().take(8).map(|d| d.label).collect();
    HubPulse {
        clients: clients.len(),
        cars: cars.len(),
        diags,
        awaiting_parts,
        in_repair,
        drafts,
        recent,
    }
}

pub fn tidy_car(car: &mut Carro) {
    car.matricula = crate::format::format_plate(&car.matricula);
    car.vin = crate::format::format_vin(&car.vin);
    car.km = crate::format::format_km(&car.km);
    if crate::format::looks_like_dtc_dump(&car.modelo)
        || crate::format::looks_like_dtc_dump(&car.marca)
    {
        let bits = diag_parse::split_veiculo(&format!("{} {}", car.marca, car.modelo));
        if !bits.marca.is_empty() {
            car.marca = bits.marca;
        }
        if !bits.modelo.is_empty() {
            car.modelo = bits.modelo;
        }
        if !bits.versao.is_empty() {
            car.versao = bits.versao;
        }
        if !bits.ano.is_empty() {
            car.ano = bits.ano;
        }
    }
    car.marca = crate::format::blank_placeholder(&car.marca);
    car.modelo = crate::format::blank_placeholder(&car.modelo);
    car.versao = crate::format::blank_placeholder(&car.versao);
    car.ano = crate::format::blank_placeholder(&car.ano);
    car.cor = crate::format::blank_placeholder(&car.cor);
    car.pneus = crate::format::blank_placeholder(&car.pneus);
    car.notas = crate::format::blank_placeholder(&car.notas);
    car.pecas_pendentes = crate::format::blank_placeholder(&car.pecas_pendentes);
    car.ensure_sistemas();
}

#[derive(Debug, Clone)]
pub struct Conflict {
    pub client: String,
    pub field: String,
    pub current: String,
    pub incoming: String,
    pub source: String,
    pub apply: bool,
}

fn push_conflict(
    out: &mut Vec<Conflict>,
    client: &str,
    field: &str,
    current: &str,
    incoming: &str,
    source: &str,
) {
    let a = current.trim();
    let b = incoming.trim();
    if b.is_empty() || a == b {
        return;
    }
    if a.is_empty() {
        out.push(Conflict {
            client: client.into(),
            field: field.into(),
            current: String::new(),
            incoming: b.into(),
            source: source.into(),
            apply: true,
        });
        return;
    }
    out.push(Conflict {
        client: client.into(),
        field: field.into(),
        current: a.into(),
        incoming: b.into(),
        source: source.into(),
        apply: false,
    });
}

pub fn scan_conflicts(root: &Path) -> Vec<Conflict> {
    let key = crate::vault::load_or_create_key(root).ok();
    let mut out = Vec::new();
    for car in list_cars(root) {
        if car.dono.eq_ignore_ascii_case(DONO_INTERNO) {
            continue;
        }
        let json = client_dir(root, &car.dono).join("ficha.json");
        let Ok(s) = fs::read_to_string(&json) else {
            continue;
        };
        let Ok(mut f) = serde_json::from_str::<ClientFicha>(&s) else {
            continue;
        };
        if let Some(k) = key.as_ref() {
            crate::vault::reveal_client(k, &mut f);
        }
        let pairs = [
            ("matrícula", f.veiculo_matricula.as_str(), car.matricula.as_str()),
            ("VIN", f.veiculo_vin.as_str(), car.vin.as_str()),
            ("marca", f.veiculo_marca.as_str(), car.marca.as_str()),
            ("modelo", f.veiculo_modelo.as_str(), car.modelo.as_str()),
            ("km", f.veiculo_km.as_str(), car.km.as_str()),
        ];
        for (field, cur, inc) in pairs {
            push_conflict(&mut out, &car.dono, field, cur, inc, "carro");
        }
    }
    out
}

pub fn apply_conflicts(root: &Path, conflicts: &[Conflict]) -> Result<usize> {
    let key = crate::vault::load_or_create_key(root)?;
    let mut n = 0;
    for c in conflicts.iter().filter(|c| c.apply) {
        let dir = client_dir(root, &c.client);
        if scaffold::is_protected(&dir) && !c.current.is_empty() {
            // still allow json field update when user confirmed; skip only if we were rewriting docs
        }
        let json = dir.join("ficha.json");
        let Ok(s) = fs::read_to_string(&json) else {
            continue;
        };
        let Ok(mut f) = serde_json::from_str::<ClientFicha>(&s) else {
            continue;
        };
        crate::vault::reveal_client(&key, &mut f);
        let slot = match c.field.as_str() {
            "matrícula" => &mut f.veiculo_matricula,
            "VIN" => &mut f.veiculo_vin,
            "marca" => &mut f.veiculo_marca,
            "modelo" => &mut f.veiculo_modelo,
            "km" => &mut f.veiculo_km,
            _ => continue,
        };
        if c.current.is_empty() || *slot != c.incoming {
            *slot = c.incoming.clone();
            crate::vault::protect_client(&key, &mut f);
            fs::write(&json, serde_json::to_string_pretty(&f)?)?;
            n += 1;
        }
        let mut car = find_car(root, &f.veiculo_vin, &f.veiculo_matricula)
            .unwrap_or_else(|| Carro::blank(&c.client));
        car.dono = c.client.clone();
        match c.field.as_str() {
            "matrícula" => car.matricula = c.incoming.clone(),
            "VIN" => car.vin = c.incoming.clone(),
            "marca" => car.marca = c.incoming.clone(),
            "modelo" => car.modelo = c.incoming.clone(),
            "km" => car.km = c.incoming.clone(),
            _ => {}
        }
        if !car.matricula.is_empty() || !car.vin.is_empty() {
            let _ = upsert_car(root, &car);
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ClientFicha;

    #[test]
    fn car_roundtrip_and_interno_not_staff() {
        let root = std::env::temp_dir().join(format!("v-cars-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Ana")).unwrap();
        fs::create_dir_all(interno_root(&root).join("Gil")).unwrap();
        fs::write(
            interno_root(&root).join("Gil").join("staff.json"),
            "{\"nome\":\"Gil\"}",
        )
        .unwrap();
        let mut c = Carro::blank("Ana");
        c.matricula = "60-HU-86".into();
        c.vin = "WVGZZZ1TZ9W034244".into();
        c.marca = "Volkswagen".into();
        upsert_car(&root, &c).unwrap();
        let listed = list_cars(&root);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].matricula, "60-HU-86");
        let found = find_car(&root, "WVGZZZ1TZ9W034244", "").unwrap();
        assert_eq!(found.dono, "Ana");
        let mut fleet = Carro::blank(DONO_INTERNO);
        fleet.matricula = "AA-00-AA".into();
        upsert_car(&root, &fleet).unwrap();
        assert!(owner_dir(&root, DONO_INTERNO).join("Carros").is_dir());
        let staff = crate::paths::list_staff_names(&root);
        assert!(staff.contains(&"Gil".into()));
        assert!(!staff.iter().any(|s| s == "Carros"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn client_and_car_fill_each_other() {
        let root = std::env::temp_dir().join(format!("v-sync-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let dir = root.join("InesTeste");
        fs::create_dir_all(&dir).unwrap();
        let mut f = ClientFicha::blank("InesTeste");
        f.veiculo_vin = "WVGZZZ1TZ9W034244".into();
        f.veiculo_matricula = "60-HU-86".into();
        writers::write_client(&dir, &f, WriteOpts::default()).unwrap();
        upsert_from_client(&root, &f).unwrap();
        let car = find_car(&root, "WVGZZZ1TZ9W034244", "").unwrap();
        assert_eq!(car.matricula, "60-HU-86");
        let mut richer = car.clone();
        richer.marca = "Volkswagen".into();
        richer.km = "180000".into();
        upsert_car(&root, &richer).unwrap();
        let loaded: ClientFicha =
            serde_json::from_str(&fs::read_to_string(dir.join("ficha.json")).unwrap()).unwrap();
        assert_eq!(loaded.veiculo_marca, "Volkswagen");
        assert_eq!(loaded.veiculo_km, "180000");
        let mut scan = crate::diag_parse::Scan {
            titulo: "Touran erros".into(),
            vin: "WVGZZZ1TZ9W034244".into(),
            matricula: "60-HU-86".into(),
            data: "20/08/2026".into(),
            ..Default::default()
        };
        scan.dtcs.push(crate::diag_parse::Dtc {
            codigo: "P161A".into(),
            categoria: "Motor".into(),
            ..Default::default()
        });
        let media = dir.join("Media").join("Diagnosticos");
        fs::create_dir_all(&media).unwrap();
        writers::write_diagnostico(&media, "Diagnostico_Touran_erros_20-08-2026", &scan, "InesTeste")
            .unwrap();
        let listed = list_saved_diags(&root);
        assert!(
            listed.iter().any(|d| d.label.contains("Touran")),
            "{:?}",
            listed.iter().map(|d| d.label.clone()).collect::<Vec<_>>()
        );
        let conflicts = scan_conflicts(&root);
        assert!(
            conflicts.iter().all(|c| c.current.is_empty() || c.apply),
            "matching values are not conflicts: {conflicts:?}"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn lists_named_json_and_pdf_only_reports() {
        let root = std::env::temp_dir().join(format!("v-diags-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let media = root.join("InesTeste").join("Media").join("Diagnosticos");
        fs::create_dir_all(&media).unwrap();
        let scan = crate::diag_parse::Scan {
            titulo: "Pós Limpeza".into(),
            data: "20/08/2026".into(),
            matricula: "60-HU-86".into(),
            ..Default::default()
        };
        fs::write(
            media.join("Pós Limpeza.json"),
            serde_json::to_string(&scan).unwrap(),
        )
        .unwrap();
        fs::write(media.join("Pós Limpeza.pdf"), b"%PDF-fake").unwrap();
        fs::write(media.join("so-pdf.pdf"), b"%PDF-only").unwrap();
        fs::write(root.join("InesTeste").join("ficha.json"), "{}").unwrap();
        let listed = list_saved_diags(&root);
        let labels: Vec<_> = listed.iter().map(|d| d.label.clone()).collect();
        assert!(
            listed.iter().any(|d| d.label.contains("Pós Limpeza") && !d.is_pdf),
            "{labels:?}"
        );
        assert!(
            listed.iter().any(|d| d.label.contains("so-pdf") && d.is_pdf),
            "{labels:?}"
        );
        assert_eq!(
            listed
                .iter()
                .filter(|d| d.label.contains("Pós Limpeza"))
                .count(),
            1,
            "json+pdf same stem must be one row: {labels:?}"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn scan_does_not_dump_dtcs_into_modelo() {
        let root = std::env::temp_dir().join(format!("v-split-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Joana")).unwrap();
        let mut dirty = Carro::blank("Joana");
        dirty.matricula = "60-HU-86".into();
        dirty.vin = "WVGZZZ1TZ9W034244".into();
        dirty.modelo = "Volkswagen - Touran [03-1 OI Motor P161A Presente - 2009 Vela de incandescência".into();
        upsert_car(&root, &dirty).unwrap();
        let car = find_car(&root, "WVGZZZ1TZ9W034244", "").unwrap();
        assert_eq!(car.marca, "Volkswagen", "{:?}", car.marca);
        assert_eq!(car.modelo, "Touran", "{}", car.modelo);
        assert_eq!(car.ano, "2009");
        assert!(!car.modelo.contains("P161A"));
        let mut scan = crate::diag_parse::Scan {
            vin: "WVGZZZ1TZ9W034244".into(),
            matricula: "60-HU-86".into(),
            veiculo: "Volkswagen - Touran [03-10] - 2009 Códigos de erro Sistema electrónico do motor DTC P161A".into(),
            dtcs: vec![crate::diag_parse::Dtc {
                categoria: "Motor".into(),
                codigo: "P161A".into(),
                estado: "Presente".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        scan.veiculo = crate::diag_parse::split_veiculo(&scan.veiculo).label();
        upsert_from_scan(&root, "Joana", &scan).unwrap();
        let car = find_car(&root, "WVGZZZ1TZ9W034244", "").unwrap();
        assert_eq!(car.modelo, "Touran");
        assert_eq!(
            car.sistemas
                .iter()
                .find(|s| s.categoria == "Motor")
                .unwrap()
                .estrelas,
            1
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn recado_survives_client_and_scan_upsert() {
        let root = std::env::temp_dir().join(format!("v-recado-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Ana")).unwrap();
        let mut c = Carro::blank("Ana");
        c.matricula = "60-HU-86".into();
        c.vin = "WVGZZZ1TZ9W034244".into();
        c.estado_oficina = "A aguardar orçamento".into();
        c.recado = "precisa orçamento para bomba".into();
        c.recado_quem = "Gil".into();
        c.recado_quando = "2026-08-21 10:00".into();
        upsert_car(&root, &c).unwrap();
        let mut f = ClientFicha::blank("Ana");
        f.veiculo_vin = c.vin.clone();
        f.veiculo_matricula = c.matricula.clone();
        writers::write_client(&root.join("Ana"), &f, WriteOpts::default()).unwrap();
        upsert_from_client(&root, &f).unwrap();
        let loaded = find_car(&root, &c.vin, "").unwrap();
        assert_eq!(loaded.recado, "precisa orçamento para bomba");
        assert_eq!(loaded.recado_quem, "Gil");
        assert_eq!(loaded.estado_oficina, "A aguardar orçamento");
        let scan = crate::diag_parse::Scan {
            vin: c.vin.clone(),
            matricula: c.matricula.clone(),
            ..Default::default()
        };
        upsert_from_scan(&root, "Ana", &scan).unwrap();
        let loaded = find_car(&root, &c.vin, "").unwrap();
        assert_eq!(loaded.recado, "precisa orçamento para bomba");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn board_advance_writes_estado() {
        use crate::model::next_estado;
        assert_eq!(next_estado(""), "A aguardar orçamento");
        assert_eq!(next_estado("A aguardar peças"), "Em reparação");
        assert_eq!(next_estado("Entregue"), "Entregue");
        let root = std::env::temp_dir().join(format!("v-adv-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Ana")).unwrap();
        let mut c = Carro::blank("Ana");
        c.matricula = "60-HU-86".into();
        c.vin = "WVGZZZ1TZ9W034244".into();
        c.estado_oficina = "A aguardar peças".into();
        upsert_car(&root, &c).unwrap();
        let mut loaded = find_car(&root, &c.vin, "").unwrap();
        loaded.estado_oficina = next_estado(&loaded.estado_oficina).to_string();
        upsert_car(&root, &loaded).unwrap();
        let loaded = find_car(&root, &c.vin, "").unwrap();
        assert_eq!(loaded.estado_oficina, "Em reparação");
        fs::remove_dir_all(&root).unwrap();
    }
}
