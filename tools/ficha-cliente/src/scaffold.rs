use crate::media;
use crate::model::{ClientFicha, ConsumoLog, StaffFicha};
use crate::paths::{list_client_names, list_staff_names, staff_dir};
use crate::writers::{self, WriteOpts};
use anyhow::{bail, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn run(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.is_dir() {
        bail!("pasta de clientes não encontrada: {}", root.display());
    }
    let opts = WriteOpts {
        skip_existing: true,
    };
    let mut written = Vec::new();

    for nome in list_client_names(root) {
        let dir = root.join(&nome);
        if let Ok(p) = media::ensure_client_media(&dir) {
            written.push(p.join("README.txt"));
        }
        let extra = writers::fill_missing_from_markdown(&dir)?;
        written.extend(extra);
        let has_ficha = has_any_ficha(&dir, "Ficha_Cliente_");
        if !has_ficha {
            let files = writers::write_client(&dir, &ClientFicha::blank(&nome), opts)?;
            written.extend(files);
        } else {
            written.extend(ensure_json(
                &dir.join("ficha.json"),
                &ClientFicha::blank(&nome),
            )?);
        }
    }

    for nome in list_staff_names(root) {
        let dir = staff_dir(root, &nome);
        if let Ok(p) = media::ensure_staff_media(&dir) {
            written.push(p.join("README.txt"));
        }
        if !has_any_ficha(&dir, "Ficha_Staff_") {
            let files = writers::write_staff(&dir, &StaffFicha::blank(&nome), opts)?;
            written.extend(files);
        } else {
            written.extend(ensure_json(
                &dir.join("staff.json"),
                &StaffFicha::blank(&nome),
            )?);
        }
        if !has_any_ficha(&dir, "Consumo_Interno_") {
            let files = writers::write_consumo(&dir, &ConsumoLog::blank(&nome), opts)?;
            written.extend(files);
        } else {
            written.extend(ensure_json(
                &dir.join("consumo.json"),
                &ConsumoLog::blank(&nome),
            )?);
        }
    }

    Ok(written)
}

pub fn resave(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.is_dir() {
        bail!("pasta de clientes não encontrada: {}", root.display());
    }
    let opts = WriteOpts {
        skip_existing: false,
    };
    let mut written = Vec::new();
    for nome in list_client_names(root) {
        let dir = root.join(&nome);
        let json = dir.join("ficha.json");
        if json.exists() {
            if is_protected(&dir) {
                continue;
            }
            let f: ClientFicha = serde_json::from_str(&fs::read_to_string(&json)?)?;
            written.extend(writers::write_client(&dir, &f, opts)?);
        }
    }
    for nome in list_staff_names(root) {
        let dir = staff_dir(root, &nome);
        let staff_json = dir.join("staff.json");
        if staff_json.exists() {
            let f: StaffFicha = serde_json::from_str(&fs::read_to_string(&staff_json)?)?;
            written.extend(writers::write_staff(&dir, &f, opts)?);
        }
        let cons_json = dir.join("consumo.json");
        if cons_json.exists() {
            let f: ConsumoLog = serde_json::from_str(&fs::read_to_string(&cons_json)?)?;
            written.extend(writers::write_consumo(&dir, &f, opts)?);
        }
    }
    Ok(written)
}

pub fn is_locked_client(nome: &str) -> bool {
    let n = nome.to_lowercase();
    let n = n.replace('é', "e").replace('ê', "e");
    (n.contains("xona") && (n.contains("bw") || n.contains("street") || n.contains("dj")))
        || n.contains("dj xona")
}

pub fn is_protected(dir: &Path) -> bool {
    if dir
        .file_name()
        .map(|n| is_locked_client(&n.to_string_lossy()))
        .unwrap_or(false)
    {
        return true;
    }
    if dir.join("NAO-REGRAVAR.txt").exists() {
        return true;
    }
    let Ok(rd) = fs::read_dir(dir) else {
        return false;
    };
    rd.flatten().any(|e| {
        e.path()
            .extension()
            .and_then(|x| x.to_str())
            .map(|x| x.eq_ignore_ascii_case("html"))
            .unwrap_or(false)
    })
}

fn ensure_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<Vec<PathBuf>> {
    if path.exists() {
        return Ok(Vec::new());
    }
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(vec![path.to_path_buf()])
}

fn has_any_ficha(dir: &Path, prefix: &str) -> bool {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return false;
    };
    rd.flatten().any(|e| {
        e.file_name()
            .to_string_lossy()
            .starts_with(prefix)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_creates_media_trees() {
        let root = std::env::temp_dir().join(format!("ficha-scaf-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Ana")).unwrap();
        std::fs::create_dir_all(root.join("Interno").join("Gil")).unwrap();
        std::fs::write(
            root.join("Interno").join("Gil").join("staff.json"),
            "{\"nome\":\"Gil\"}",
        )
        .unwrap();
        run(&root).unwrap();
        assert!(root.join("Ana").join("Media").join("Antes").is_dir());
        assert!(root.join("Ana").join("Ficha_Cliente_Ana.md").is_file());
        assert!(root
            .join("Interno")
            .join("Gil")
            .join("Media")
            .join("Reparacoes")
            .is_dir());
        let ines = root.join("Inês");
        std::fs::create_dir_all(&ines).unwrap();
        std::fs::write(ines.join("Ficha_Cliente_Ines.md"), "# keep\n").unwrap();
        run(&root).unwrap();
        let md = std::fs::read_to_string(ines.join("Ficha_Cliente_Ines.md")).unwrap();
        assert_eq!(md, "# keep\n");
        assert!(ines.join("ficha.json").is_file());
        assert!(ines.join("Media").join("Depois").is_dir());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn xona_is_locked_meme() {
        assert!(is_locked_client("Xona bwé da street"));
        assert!(is_locked_client("DJ xona bwe"));
        assert!(!is_locked_client("Joana"));
        let root = std::env::temp_dir().join(format!("ficha-xona-{}", std::process::id()));
        let dir = root.join("Xona bwe da street");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(is_protected(&dir));
        let _ = std::fs::remove_dir_all(&root);
    }
}
