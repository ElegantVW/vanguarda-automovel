use anyhow::{Context, Result};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

pub fn zip_clientes(root: &Path, dest_zip: &Path) -> Result<u64> {
    if !root.is_dir() {
        anyhow::bail!("pasta de clientes não existe: {}", root.display());
    }
    if let Some(parent) = dest_zip.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(dest_zip)
        .with_context(|| format!("não criei {}", dest_zip.display()))?;
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut buf = Vec::new();
    let mut files = 0u64;
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let rel = path.strip_prefix(root).unwrap_or(path);
        let name = rel.to_string_lossy().replace('\\', "/");
        if name.is_empty() {
            continue;
        }
        if entry.file_type().is_dir() {
            zip.add_directory(format!("{name}/"), opts)?;
            continue;
        }
        buf.clear();
        File::open(path)?.read_to_end(&mut buf)?;
        zip.start_file(&name, opts)?;
        zip.write_all(&buf)?;
        files += 1;
    }
    zip.finish()?;
    Ok(files)
}

pub fn default_backup_name() -> String {
    format!(
        "Vanguarda-backup-{}.zip",
        chrono::Local::now().format("%Y-%m-%d")
    )
}

pub fn stamp_path(root: &Path) -> std::path::PathBuf {
    crate::paths::interno_root(root).join("ultimo-backup.txt")
}

pub fn mark_done(root: &Path) {
    let p = stamp_path(root);
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(p, crate::model::today());
}

/// Days since last backup stamp. None = never.
pub fn days_since(root: &Path) -> Option<i64> {
    let raw = std::fs::read_to_string(stamp_path(root)).ok()?;
    let day = raw.trim();
    let last = chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()?;
    let today = chrono::Local::now().date_naive();
    Some((today - last).num_days())
}

pub fn default_dir() -> std::path::PathBuf {
    crate::paths::vanguarda_home().join("Backups")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zips_a_tree() {
        let root = std::env::temp_dir().join(format!("alha-zip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Ana")).unwrap();
        std::fs::write(root.join("Ana").join("ficha.json"), "{}").unwrap();
        let zip_path = std::env::temp_dir().join(format!("alha-zip-{}.zip", std::process::id()));
        let n = zip_clientes(&root, &zip_path).unwrap();
        assert!(n >= 1);
        assert!(zip_path.is_file());
        let _ = std::fs::remove_file(&zip_path);
        let _ = std::fs::remove_dir_all(&root);
    }
}
