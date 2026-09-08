use crate::model::{ClientFicha, StaffFicha};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{Context, Result};
use rand::RngCore;
use std::fs;
use std::path::{Path, PathBuf};

const PREFIX: &str = "enc:v1:";
const KEY_NAME: &str = ".vanguarda-key";

pub fn data_root_guess(person_dir: &Path) -> PathBuf {
    let parent = person_dir.parent().unwrap_or(person_dir);
    if parent
        .file_name()
        .map(|n| n.to_string_lossy().eq_ignore_ascii_case("Interno"))
        .unwrap_or(false)
    {
        parent.parent().unwrap_or(parent).to_path_buf()
    } else {
        parent.to_path_buf()
    }
}

pub fn load_or_create_key(root: &Path) -> Result<[u8; 32]> {
    let path = root.join(KEY_NAME);
    if let Ok(buf) = fs::read(&path) {
        if buf.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&buf);
            return Ok(key);
        }
    }
    fs::create_dir_all(root)?;
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    fs::write(&path, key).with_context(|| format!("não gravei {}", path.display()))?;
    Ok(key)
}

fn cipher(key: &[u8; 32]) -> Aes256Gcm {
    Aes256Gcm::new(key.into())
}

pub fn seal(key: &[u8; 32], plain: &str) -> String {
    let t = plain.trim();
    if t.is_empty() || t.starts_with(PREFIX) {
        return plain.to_string();
    }
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let Ok(ct) = cipher(key).encrypt(nonce, t.as_bytes()) else {
        return plain.to_string();
    };
    let mut blob = Vec::with_capacity(12 + ct.len());
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&ct);
    format!("{PREFIX}{}", hex_encode(&blob))
}

pub fn open(key: &[u8; 32], stored: &str) -> String {
    let Some(hex) = stored.strip_prefix(PREFIX) else {
        return stored.to_string();
    };
    let Ok(blob) = hex_decode(hex) else {
        return stored.to_string();
    };
    if blob.len() < 13 {
        return stored.to_string();
    }
    let nonce = Nonce::from_slice(&blob[..12]);
    match cipher(key).decrypt(nonce, &blob[12..]) {
        Ok(pt) => String::from_utf8(pt).unwrap_or_else(|_| stored.to_string()),
        Err(_) => stored.to_string(),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_decode(s: &str) -> Result<Vec<u8>, ()> {
    if s.len() % 2 != 0 {
        return Err(());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
        .collect()
}

fn protect_str(key: &[u8; 32], s: &mut String) {
    *s = seal(key, s);
}

fn reveal_str(key: &[u8; 32], s: &mut String) {
    *s = open(key, s);
}

pub fn protect_client(key: &[u8; 32], f: &mut ClientFicha) {
    protect_str(key, &mut f.nif);
    protect_str(key, &mut f.telemovel);
    protect_str(key, &mut f.email);
    protect_str(key, &mut f.morada);
    protect_str(key, &mut f.codigo_postal);
    protect_str(key, &mut f.data_nascimento);
}

pub fn reveal_client(key: &[u8; 32], f: &mut ClientFicha) {
    reveal_str(key, &mut f.nif);
    reveal_str(key, &mut f.telemovel);
    reveal_str(key, &mut f.email);
    reveal_str(key, &mut f.morada);
    reveal_str(key, &mut f.codigo_postal);
    reveal_str(key, &mut f.data_nascimento);
}

pub fn protect_staff(key: &[u8; 32], f: &mut StaffFicha) {
    protect_str(key, &mut f.telemovel);
    protect_str(key, &mut f.email);
}

pub fn reveal_staff(key: &[u8; 32], f: &mut StaffFicha) {
    reveal_str(key, &mut f.telemovel);
    reveal_str(key, &mut f.email);
}

pub fn rewrite_json_encrypted<T, F>(path: &Path, key: &[u8; 32], protect: F) -> Result<()>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
    F: FnOnce(&[u8; 32], &mut T),
{
    let s = fs::read_to_string(path)?;
    let mut v: T = serde_json::from_str(&s)?;
    protect(key, &mut v);
    fs::write(path, serde_json::to_string_pretty(&v)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_open_roundtrip_not_plaintext() {
        let dir = std::env::temp_dir().join(format!("v-vault-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let key = load_or_create_key(&dir).unwrap();
        let enc = seal(&key, "192836129");
        assert!(enc.starts_with(PREFIX));
        assert!(!enc.contains("192836129"));
        assert_eq!(open(&key, &enc), "192836129");
        assert_eq!(open(&key, "texto antigo"), "texto antigo");
        let mut f = ClientFicha::blank("Ana");
        f.nif = "123".into();
        f.telemovel = "910000000".into();
        protect_client(&key, &mut f);
        assert!(f.nif.starts_with(PREFIX));
        reveal_client(&key, &mut f);
        assert_eq!(f.nif, "123");
        assert_eq!(f.telemovel, "910000000");
        let _ = fs::remove_dir_all(&dir);
    }
}
