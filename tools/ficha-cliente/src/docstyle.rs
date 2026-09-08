use crate::paths::interno_root;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const FILE: &str = "documentos.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocStyle {
    /// `formal` (white + black + logo) or `estilizado` (metal + gold cards).
    #[serde(default = "default_estilo")]
    pub estilo: String,
    /// Raster title size, roughly PDF points.
    #[serde(default = "default_titulo_pt")]
    pub titulo_pt: f32,
    /// Extra millimetres around the header block.
    #[serde(default = "default_espaco")]
    pub espaco_titulo: f32,
    /// `preto` or `ouro`. Formal forces black.
    #[serde(default = "default_tinta")]
    pub tinta: String,
    /// `auto` | `branco` | `imagem`.
    #[serde(default = "default_fundo")]
    pub fundo: String,
    /// Gold outline weight in millimetres.
    #[serde(default = "default_contorno")]
    pub contorno_mm: f32,
    /// Rounded-card corner radius in millimetres.
    #[serde(default = "default_raio")]
    pub raio_mm: f32,
    /// Logo height in millimetres.
    #[serde(default = "default_logo")]
    pub logo_mm: f32,
    /// Body Helvetica size (PDF points).
    #[serde(default = "default_corpo")]
    pub corpo_pt: f32,
    /// System icons on Estilizado section heads.
    #[serde(default = "default_icon")]
    pub icon: bool,
}

fn default_estilo() -> String {
    "estilizado".into()
}
fn default_titulo_pt() -> f32 {
    22.0
}
fn default_espaco() -> f32 {
    4.0
}
fn default_tinta() -> String {
    "ouro".into()
}
fn default_fundo() -> String {
    "auto".into()
}
fn default_contorno() -> f32 {
    0.6
}
fn default_raio() -> f32 {
    3.0
}
fn default_logo() -> f32 {
    14.0
}
fn default_corpo() -> f32 {
    10.0
}
fn default_icon() -> bool {
    true
}

impl Default for DocStyle {
    fn default() -> Self {
        Self {
            estilo: default_estilo(),
            titulo_pt: default_titulo_pt(),
            espaco_titulo: default_espaco(),
            tinta: default_tinta(),
            fundo: default_fundo(),
            contorno_mm: default_contorno(),
            raio_mm: default_raio(),
            logo_mm: default_logo(),
            corpo_pt: default_corpo(),
            icon: default_icon(),
        }
    }
}

impl DocStyle {
    pub fn formal() -> Self {
        Self {
            estilo: "formal".into(),
            titulo_pt: 24.0,
            espaco_titulo: 6.0,
            tinta: "preto".into(),
            fundo: "branco".into(),
            contorno_mm: 0.45,
            raio_mm: 0.0,
            logo_mm: 16.0,
            corpo_pt: 10.0,
            icon: false,
        }
    }

    pub fn path(root: &Path) -> PathBuf {
        interno_root(root).join(FILE)
    }

    pub fn load(root: &Path) -> Self {
        let path = Self::path(root);
        let Ok(raw) = fs::read_to_string(&path) else {
            return Self::default();
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub fn save(&self, root: &Path) -> Result<PathBuf> {
        let path = Self::path(root);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).with_context(|| format!("pasta {}", dir.display()))?;
        }
        let body = serde_json::to_string_pretty(self)?;
        fs::write(&path, body).with_context(|| format!("não gravei {}", path.display()))?;
        Ok(path)
    }

    pub fn is_formal(&self) -> bool {
        self.estilo.trim().eq_ignore_ascii_case("formal")
    }

    pub fn ink_black(&self) -> bool {
        self.is_formal() || self.tinta.trim().eq_ignore_ascii_case("preto")
    }

    pub fn white_paper(&self) -> bool {
        self.is_formal()
            || self.fundo.trim().eq_ignore_ascii_case("branco")
            || (self.fundo.trim().eq_ignore_ascii_case("auto") && self.is_formal())
    }

    pub fn use_image_bg(&self) -> bool {
        if self.fundo.trim().eq_ignore_ascii_case("branco") {
            return false;
        }
        if self.fundo.trim().eq_ignore_ascii_case("imagem") {
            return true;
        }
        !self.is_formal()
    }

    pub fn show_cards(&self) -> bool {
        !self.is_formal()
    }

    pub fn show_icons(&self) -> bool {
        !self.is_formal() && self.icon
    }

    pub fn label(&self) -> &'static str {
        if self.is_formal() {
            "Formal"
        } else {
            "Estilizado"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_interno() {
        let root = std::env::temp_dir().join(format!("v-docstyle-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let mut s = DocStyle::formal();
        s.titulo_pt = 26.0;
        s.espaco_titulo = 8.0;
        let p = s.save(&root).unwrap();
        assert!(p.ends_with(FILE));
        let loaded = DocStyle::load(&root);
        assert!(loaded.is_formal());
        assert!(loaded.ink_black());
        assert!(loaded.white_paper());
        assert!(!loaded.show_cards());
        assert_eq!(loaded.titulo_pt, 26.0);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn default_is_estilizado() {
        let s = DocStyle::default();
        assert!(!s.is_formal());
        assert!(!s.ink_black());
        assert!(s.show_cards());
        assert!(s.use_image_bg());
        assert!(s.icon);
        assert!((s.contorno_mm - 0.6).abs() < 0.01);
    }

    #[test]
    fn old_json_gets_new_defaults() {
        let raw = r#"{"estilo":"estilizado","titulo_pt":22.0,"espaco_titulo":4.0,"tinta":"ouro","fundo":"auto"}"#;
        let s: DocStyle = serde_json::from_str(raw).unwrap();
        assert!(s.icon);
        assert!((s.logo_mm - 14.0).abs() < 0.01);
        assert!((s.corpo_pt - 10.0).abs() < 0.01);
        assert!((s.raio_mm - 3.0).abs() < 0.01);
    }
}
