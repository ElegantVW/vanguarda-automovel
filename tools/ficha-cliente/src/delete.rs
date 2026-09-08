use crate::model::DONO_INTERNO;
use crate::paths::{client_dir, staff_dir};
use crate::scaffold;
use anyhow::{bail, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn delete_client(root: &Path, nome: &str) -> Result<PathBuf> {
    let nome = nome.trim();
    if nome.is_empty() {
        bail!("Escolhe o cliente a apagar.");
    }
    if nome.eq_ignore_ascii_case(DONO_INTERNO) {
        bail!("Interno não se apaga.");
    }
    if crate::scaffold::is_locked_client(nome) {
        bail!("DJ xona bwé é da Vanguarda — não se apaga.");
    }
    let dir = client_dir(root, nome);
    if !dir.is_dir() {
        bail!("Não encontrei a pasta de {nome}.");
    }
    if scaffold::is_protected(&dir) {
        bail!("Pasta protegida — não apago.");
    }
    fs::remove_dir_all(&dir)?;
    Ok(dir)
}

pub fn delete_staff(root: &Path, nome: &str) -> Result<PathBuf> {
    let nome = nome.trim();
    if nome.is_empty() {
        bail!("Escolhe o staff a apagar.");
    }
    let dir = staff_dir(root, nome);
    if !dir.is_dir() {
        bail!("Não encontrei a pasta de {nome}.");
    }
    if scaffold::is_protected(&dir) {
        bail!("Pasta protegida — não apago.");
    }
    fs::remove_dir_all(&dir)?;
    Ok(dir)
}

pub fn delete_car_dir(dir: &Path) -> Result<PathBuf> {
    if !dir.is_dir() {
        bail!("Não encontrei a pasta da viatura.");
    }
    if let Some(owner) = dir.parent().and_then(|p| p.parent()) {
        if scaffold::is_protected(owner) {
            bail!("Pasta protegida — não apago.");
        }
    }
    fs::remove_dir_all(dir)?;
    Ok(dir.to_path_buf())
}

/// Remove every sibling of `path` that shares the file stem, then the same
/// stem under other `Diagnosticos` folders of that client.
pub fn delete_diag(root: &Path, path: &Path, client: &str) -> Result<usize> {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if stem.is_empty() {
        bail!("Relatório sem nome.");
    }
    let mut n = 0usize;
    n += delete_stem_in(path.parent().unwrap_or(path), &stem)?;
    let client = client.trim();
    if !client.is_empty() {
        let owner = if client.eq_ignore_ascii_case(DONO_INTERNO) {
            crate::paths::interno_root(root)
        } else {
            client_dir(root, client)
        };
        for e in walkdir::WalkDir::new(&owner).into_iter().flatten() {
            let p = e.path();
            if !p.is_file() {
                continue;
            }
            let parent_ok = p
                .parent()
                .and_then(|d| d.file_name())
                .map(|n| n.to_string_lossy().eq_ignore_ascii_case("Diagnosticos"))
                .unwrap_or(false);
            if !parent_ok {
                continue;
            }
            let same = p
                .file_stem()
                .map(|s| s.to_string_lossy() == stem)
                .unwrap_or(false);
            if same && fs::remove_file(p).is_ok() {
                n += 1;
            }
        }
    }
    Ok(n)
}

fn delete_stem_in(dir: &Path, stem: &str) -> Result<usize> {
    let mut n = 0usize;
    let Ok(rd) = fs::read_dir(dir) else {
        return Ok(0);
    };
    for e in rd.flatten() {
        let p = e.path();
        let same = p
            .file_stem()
            .map(|s| s.to_string_lossy() == stem)
            .unwrap_or(false);
        if same && p.is_file() {
            fs::remove_file(&p)?;
            n += 1;
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ClientFicha;
    use crate::writers::{self, WriteOpts};

    #[test]
    fn delete_refuses_protected_and_removes_temp() {
        let root = std::env::temp_dir().join(format!("v-del-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let dir = root.join("TempCliente");
        fs::create_dir_all(&dir).unwrap();
        writers::write_client(&dir, &ClientFicha::blank("TempCliente"), WriteOpts::default())
            .unwrap();
        let gone = delete_client(&root, "TempCliente").unwrap();
        assert!(!gone.is_dir());

        let prot = root.join("InesProt");
        fs::create_dir_all(&prot).unwrap();
        fs::write(prot.join("NAO-REGRAVAR.txt"), b"nao").unwrap();
        let err = delete_client(&root, "InesProt").unwrap_err().to_string();
        assert!(err.contains("protegida"), "{err}");
        assert!(prot.is_dir());
        let _ = fs::remove_dir_all(&root);
    }
}
