use crate::auth;
use crate::model::{ConsumoEntry, ConsumoLog, StaffFicha};
use crate::paths::{interno_root, staff_dir};
use anyhow::{bail, Result};
use std::fs;
use std::path::Path;

/// Nickname folder → canonical person. Software-admin logins are never in this list.
pub const SUGGESTED_MERGES: &[(&str, &str)] = &[
    ("Gil", "Gil Salvador"),
    ("Rodrigo", "Rodrigo Sousa"),
];

pub fn suggested_for(root: &Path) -> Vec<(String, String)> {
    SUGGESTED_MERGES
        .iter()
        .filter(|(from, _)| staff_dir(root, from).is_dir())
        .map(|(a, b)| ((*a).to_string(), (*b).to_string()))
        .collect()
}

fn consumo_key(e: &ConsumoEntry) -> String {
    format!(
        "{}|{}|{}|{}",
        e.data.trim(),
        e.tipo.trim(),
        e.descricao.trim(),
        e.custo_interno.trim()
    )
}

fn load_consumo(dir: &Path) -> ConsumoLog {
    let p = dir.join("consumo.json");
    fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn load_staff(dir: &Path, fallback: &str) -> StaffFicha {
    let p = dir.join("staff.json");
    fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| StaffFicha::blank(fallback))
}

/// Merge `from` folder into canonical `to`. Login renamed if it still uses `from`.
/// Leftover files go to `Interno\_arquivo\<from>-YYYY-MM-DD\`. Never deletes.
pub fn merge_person(root: &Path, from: &str, to: &str) -> Result<String> {
    let from = from.trim();
    let to = to.trim();
    if from.is_empty() || to.is_empty() {
        bail!("Nomes inválidos.");
    }
    if from.eq_ignore_ascii_case(to) {
        bail!("Origem e destino são o mesmo nome.");
    }
    let users = auth::load_users(root);
    if users.iter().any(|u| u.software && u.nome.eq_ignore_ascii_case(from))
        || users.iter().any(|u| u.software && u.nome.eq_ignore_ascii_case(to))
    {
        bail!("A conta do software não se junta a uma pessoa da empresa.");
    }
    let src = staff_dir(root, from);
    if !src.is_dir() {
        bail!("Não há pasta «{from}».");
    }
    let dest = staff_dir(root, to);
    fs::create_dir_all(&dest)?;

    let mut cons = load_consumo(&dest);
    if cons.nome.trim().is_empty() {
        cons.nome = to.to_string();
    }
    let extra = load_consumo(&src);
    let mut seen: Vec<String> = cons.entradas.iter().map(consumo_key).collect();
    let mut added = 0usize;
    for e in extra.entradas {
        let k = consumo_key(&e);
        if seen.iter().any(|s| s == &k) {
            continue;
        }
        seen.push(k);
        cons.entradas.push(e);
        added += 1;
    }
    cons.nome = to.to_string();
    fs::write(dest.join("consumo.json"), serde_json::to_string_pretty(&cons)?)?;

    let dest_staff = dest.join("staff.json");
    let mut ficha = if dest_staff.is_file() {
        load_staff(&dest, to)
    } else {
        load_staff(&src, to)
    };
    if ficha.nome.trim().is_empty() || ficha.nome.eq_ignore_ascii_case(from) {
        ficha.nome = to.to_string();
    }
    fs::write(&dest_staff, serde_json::to_string_pretty(&ficha)?)?;

    copy_missing(&src, &dest)?;

    let day = chrono::Local::now().format("%Y-%m-%d").to_string();
    let arquivo = interno_root(root).join("_arquivo");
    fs::create_dir_all(&arquivo)?;
    let mut archive = arquivo.join(format!("{from}-{day}"));
    let mut n = 2u32;
    while archive.exists() {
        archive = arquivo.join(format!("{from}-{day}-{n}"));
        n += 1;
    }
    fs::rename(&src, &archive)?;
    let _ = auth::rename_user(root, from, to);

    Ok(format!(
        "«{from}» → «{to}»: {added} linhas de consumo novas. Pasta antiga em {}.",
        archive.display()
    ))
}

fn copy_missing(src: &Path, dest: &Path) -> Result<()> {
    let Ok(rd) = fs::read_dir(src) else {
        return Ok(());
    };
    for entry in rd.flatten() {
        let name = entry.file_name();
        if name == "consumo.json" || name == "staff.json" {
            continue;
        }
        let to = dest.join(&name);
        if to.exists() {
            continue;
        }
        let from = entry.path();
        if from.is_dir() {
            copy_dir_missing(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn copy_dir_missing(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)?.flatten() {
        let to = dest.join(entry.file_name());
        let from = entry.path();
        if from.is_dir() {
            copy_dir_missing(&from, &to)?;
        } else if !to.exists() {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

pub const KEEP_PERSON: &str = "Gil Salvador";

fn absorb_then_delete(root: &Path, from: &str, to: &str) -> Result<usize> {
    let src = staff_dir(root, from);
    let dest = staff_dir(root, to);
    if !src.is_dir() {
        return Ok(0);
    }
    fs::create_dir_all(&dest)?;
    let mut cons = load_consumo(&dest);
    let extra = load_consumo(&src);
    let mut seen: Vec<String> = cons.entradas.iter().map(consumo_key).collect();
    let mut added = 0usize;
    for e in extra.entradas {
        let k = consumo_key(&e);
        if seen.iter().any(|s| s == &k) {
            continue;
        }
        seen.push(k);
        cons.entradas.push(e);
        added += 1;
    }
    cons.nome = to.to_string();
    fs::write(dest.join("consumo.json"), serde_json::to_string_pretty(&cons)?)?;
    let dest_staff = dest.join("staff.json");
    let mut ficha = if dest_staff.is_file() {
        load_staff(&dest, to)
    } else {
        load_staff(&src, to)
    };
    ficha.nome = to.to_string();
    fs::write(&dest_staff, serde_json::to_string_pretty(&ficha)?)?;
    copy_missing(&src, &dest)?;
    fs::remove_dir_all(&src)?;
    let _ = auth::rename_user(root, from, to);
    Ok(added)
}

/// Merge Gil → Gil Salvador, then delete every other person folder and login.
/// Software login is kept. Client folders are not touched.
pub fn eliminate_except_gil(root: &Path) -> Result<String> {
    let keep = KEEP_PERSON;
    let mut bits = Vec::new();
    if staff_dir(root, "Gil").is_dir() {
        let n = absorb_then_delete(root, "Gil", keep)?;
        bits.push(format!("Gil absorvido ({n} linhas de consumo)"));
    }
    let software: Vec<String> = auth::load_users(root)
        .into_iter()
        .filter(|u| u.software)
        .map(|u| u.nome)
        .collect();
    let interno = interno_root(root);
    let mut gone = Vec::new();
    if let Ok(rd) = fs::read_dir(&interno) {
        for e in rd.flatten() {
            if !e.path().is_dir() {
                continue;
            }
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            if crate::paths::STAFF_SKIP
                .iter()
                .any(|s| s.eq_ignore_ascii_case(&name))
            {
                continue;
            }
            if name.eq_ignore_ascii_case(keep) {
                continue;
            }
            if software.iter().any(|s| s.eq_ignore_ascii_case(&name)) {
                continue;
            }
            let p = e.path();
            if p.join("staff.json").is_file() || p.join("consumo.json").is_file() || p.join("Media").is_dir()
            {
                fs::remove_dir_all(&p)?;
                gone.push(name);
            }
        }
    }
    auth::keep_gil_and_software(root)?;
    let _ = fs::remove_file(interno.join("utilizadores.json"));
    if !gone.is_empty() {
        bits.push(format!("apagadas: {}", gone.join(", ")));
    }
    if bits.is_empty() {
        bits.push("já só há Gil Salvador".into());
    }
    Ok(bits.join(". "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::STAFF_SKIP;

    #[test]
    fn merge_keeps_both_consumo_and_archives() {
        let root = std::env::temp_dir().join(format!("v-people-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let gil = staff_dir(&root, "Gil");
        let canon = staff_dir(&root, "Gil Salvador");
        fs::create_dir_all(&gil).unwrap();
        fs::create_dir_all(&canon).unwrap();
        let a = ConsumoLog {
            nome: "Gil".into(),
            entradas: vec![ConsumoEntry {
                data: "2026-08-21".into(),
                tipo: "outro".into(),
                descricao: "Café".into(),
                custo_interno: "0.10€".into(),
                ..Default::default()
            }],
        };
        let b = ConsumoLog {
            nome: "Gil Salvador".into(),
            entradas: vec![ConsumoEntry {
                data: "2026-08-21".into(),
                tipo: "Peças de viatura".into(),
                descricao: "água".into(),
                ..Default::default()
            }],
        };
        fs::write(gil.join("consumo.json"), serde_json::to_string(&a).unwrap()).unwrap();
        fs::write(
            gil.join("staff.json"),
            serde_json::to_string(&StaffFicha::blank("Gil")).unwrap(),
        )
        .unwrap();
        fs::write(canon.join("consumo.json"), serde_json::to_string(&b).unwrap()).unwrap();
        let msg = merge_person(&root, "Gil", "Gil Salvador").unwrap();
        assert!(msg.contains("Gil Salvador"));
        assert!(!gil.is_dir());
        let merged: ConsumoLog =
            serde_json::from_str(&fs::read_to_string(canon.join("consumo.json")).unwrap()).unwrap();
        assert_eq!(merged.entradas.len(), 2);
        assert_eq!(merged.nome, "Gil Salvador");
        let staff: StaffFicha =
            serde_json::from_str(&fs::read_to_string(canon.join("staff.json")).unwrap()).unwrap();
        assert_eq!(staff.nome, "Gil Salvador");
        assert!(interno_root(&root).join("_arquivo").is_dir());
        assert!(STAFF_SKIP.contains(&"_arquivo"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn software_login_is_not_merged() {
        let root = std::env::temp_dir().join(format!("v-people-sw-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        auth::add_software_user(&root, "Vanguarda", "secret12").unwrap();
        fs::create_dir_all(staff_dir(&root, "Gil")).unwrap();
        assert!(merge_person(&root, "Gil", "Vanguarda").is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn eliminate_drops_other_people_keeps_gil() {
        let root = std::env::temp_dir().join(format!("v-elim-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        auth::add_user(&root, "Gil Salvador", crate::auth::Departamento::Escritorio, "secret12", true)
            .unwrap();
        auth::add_user(&root, "Marisa", crate::auth::Departamento::Escritorio, "secret12", false)
            .unwrap();
        fs::create_dir_all(staff_dir(&root, "Rodrigo")).unwrap();
        fs::write(
            staff_dir(&root, "Rodrigo").join("staff.json"),
            "{\"nome\":\"Rodrigo\"}",
        )
        .unwrap();
        eliminate_except_gil(&root).unwrap();
        assert!(staff_dir(&root, "Gil Salvador").is_dir());
        assert!(!staff_dir(&root, "Marisa").is_dir());
        assert!(!staff_dir(&root, "Rodrigo").is_dir());
        let users = auth::load_users(&root);
        assert!(users.iter().all(|u| u.software || u.nome == "Gil Salvador"));
        assert!(users.iter().any(|u| u.nome == "Gil Salvador" && u.admin));
        let _ = fs::remove_dir_all(&root);
    }
}
