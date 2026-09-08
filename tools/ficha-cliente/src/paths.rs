use std::path::{Path, PathBuf};

pub fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

/// Local Documents\Vanguarda — not OneDrive. Client PII must not sync.
pub fn default_house() -> PathBuf {
    let profile = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into());
    PathBuf::from(profile).join("Documents").join("Vanguarda")
}

pub fn vanguarda_home() -> PathBuf {
    if let Ok(p) = std::env::var("VANGUARDA_HOME") {
        let pb = PathBuf::from(p);
        if !pb.as_os_str().is_empty() {
            return pb;
        }
    }
    if let Some(dir) = exe_dir() {
        if let Some(h) = load_alha_key(&dir, "home") {
            return h;
        }
    }
    let house = default_house();
    if house.is_dir() {
        return house;
    }
    let local = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into());
    PathBuf::from(local).join("Vanguarda")
}

pub fn dados_clientes(home: &Path) -> PathBuf {
    home.join("Dados").join("Clientes")
}

fn load_alha_key(install_dir: &Path, key: &str) -> Option<PathBuf> {
    let cfg = install_dir.join("alha.toml");
    let text = std::fs::read_to_string(cfg).ok()?;
    let prefix = format!("{key}");
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(rest) = line.strip_prefix(&prefix) else {
            continue;
        };
        let v = rest
            .trim()
            .trim_start_matches('=')
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        if v.is_empty() {
            continue;
        }
        return Some(PathBuf::from(v));
    }
    None
}

pub fn load_alha_clientes(install_dir: &Path) -> Option<PathBuf> {
    load_alha_key(install_dir, "clientes")
}

pub fn write_alha_toml(install_dir: &Path, clientes: &Path) -> std::io::Result<()> {
    write_alha_toml_full(install_dir, Some(&vanguarda_home()), clientes)
}

pub fn write_alha_toml_full(
    install_dir: &Path,
    home: Option<&Path>,
    clientes: &Path,
) -> std::io::Result<()> {
    let mut body = String::new();
    if let Some(h) = home {
        body.push_str(&format!("home = \"{}\"\n", h.display()));
    }
    body.push_str(&format!("clientes = \"{}\"\n", clientes.display()));
    std::fs::write(install_dir.join("alha.toml"), body)
}

pub fn find_clientes_root() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("FICHA_CLIENTES_DIR") {
        let pb = PathBuf::from(p);
        if pb.is_dir() {
            return Some(pb);
        }
    }
    if let Some(dir) = exe_dir() {
        if let Some(p) = load_alha_clientes(&dir) {
            let _ = std::fs::create_dir_all(&p);
            return Some(p);
        }
    }
    let house_cli = dados_clientes(&vanguarda_home());
    if house_cli.is_dir() {
        return Some(house_cli);
    }
    let default_cli = dados_clientes(&default_house());
    if default_cli.is_dir() {
        return Some(default_cli);
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let old = PathBuf::from(local).join("Vanguarda").join("Clientes");
        if old.is_dir() {
            return Some(old);
        }
    }
    for letter in b'C'..=b'Z' {
        let p = PathBuf::from(format!(r"{}:\Vanguarda\Office\Clientes", letter as char));
        if p.is_dir() {
            return Some(p);
        }
    }
    None
}

const RUN_VALUE: &str = "Vanguarda";

/// HKCU Run — opens this exe at logon. alha.toml is read from the exe folder.
pub fn ensure_start_at_logon() {
    #[cfg(windows)]
    {
        let Ok(exe) = std::env::current_exe() else {
            return;
        };
        let Ok(hkcu) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Run",
            winreg::enums::KEY_SET_VALUE,
        ) else {
            return;
        };
        let cmd = format!("\"{}\"", exe.display());
        let _ = hkcu.set_value(RUN_VALUE, &cmd);
    }
}

pub fn clear_start_at_logon() {
    #[cfg(windows)]
    {
        let Ok(hkcu) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Run",
            winreg::enums::KEY_SET_VALUE,
        ) else {
            return;
        };
        let _ = hkcu.delete_value(RUN_VALUE);
    }
}

pub fn client_dir(root: &Path, nome: &str) -> PathBuf {
    root.join(nome)
}

pub fn interno_root(root: &Path) -> PathBuf {
    root.join("Interno")
}

pub fn staff_dir(root: &Path, nome: &str) -> PathBuf {
    interno_root(root).join(nome)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alha_toml_roundtrip() {
        let dir = std::env::temp_dir().join(format!("alha-cfg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let clientes = dir.join("Clientes");
        let home = dir.join("House");
        write_alha_toml_full(&dir, Some(&home), &clientes).unwrap();
        let loaded = load_alha_clientes(&dir).unwrap();
        assert_eq!(loaded, clientes);
        let loaded_home = load_alha_key(&dir, "home").unwrap();
        assert_eq!(loaded_home, home);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn staff_list_skips_house_dirs() {
        let root = std::env::temp_dir().join(format!("alha-staff-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let interno = root.join("Interno");
        for n in ["Guias", "Catálogos", "Carros", "_arquivo", "Gil Salvador"] {
            std::fs::create_dir_all(interno.join(n)).unwrap();
        }
        std::fs::write(
            interno.join("Gil Salvador").join("staff.json"),
            "{\"nome\":\"Gil Salvador\"}",
        )
        .unwrap();
        std::fs::write(interno.join("Guias").join("x.md"), "no").unwrap();
        let names = list_staff_names(&root);
        assert_eq!(names, vec!["Gil Salvador".to_string()]);
        let _ = std::fs::remove_dir_all(&root);
    }
}

pub fn list_client_names(root: &Path) -> Vec<String> {
    list_dirs(root, &["Interno"])
}

/// House dirs that live under Interno but are not people.
pub const STAFF_SKIP: &[&str] = &[
    "Carros",
    "Guias",
    "Catálogos",
    "Catalogos",
    "Operacao",
    "Operação",
    "_arquivo",
];

pub fn list_staff_names(root: &Path) -> Vec<String> {
    list_dirs(&interno_root(root), STAFF_SKIP)
        .into_iter()
        .filter(|n| {
            let dir = interno_root(root).join(n);
            dir.join("staff.json").is_file() || dir.join("consumo.json").is_file()
        })
        .collect()
}

fn list_dirs(dir: &Path, skip: &[&str]) -> Vec<String> {
    let mut names = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return names;
    };
    for entry in rd.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        if skip.iter().any(|s| s.eq_ignore_ascii_case(&name)) {
            continue;
        }
        names.push(name);
    }
    names.sort();
    names
}
