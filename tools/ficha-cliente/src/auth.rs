use crate::model::StaffFicha;
use crate::paths::{interno_root, staff_dir};
use crate::vault;
use crate::writers::{self, WriteOpts};
use anyhow::{bail, Result};
use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

const USERS_FILE: &str = "utilizadores.json";

fn default_activo() -> bool {
    true
}

fn default_estado() -> String {
    "activo".into()
}

pub const EST_PEDIDO: &str = "pedido";
pub const EST_ACTIVO: &str = "activo";
pub const EST_RECUSADO: &str = "recusado";

/// Company side: Front (Escritório, Design) or Back (Care, Oficina, Interiores reserved).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ala {
    Front,
    Back,
}

impl Ala {
    pub fn label(self) -> &'static str {
        match self {
            Self::Front => "FRONT",
            Self::Back => "BACK",
        }
    }

    pub fn from_mesa(mesa: Departamento) -> Self {
        match mesa {
            Departamento::Care | Departamento::Oficina | Departamento::Interiores => Self::Back,
            Departamento::Escritorio | Departamento::Design => Self::Front,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Departamento {
    Escritorio,
    Oficina,
    Design,
    Care,
    Interiores,
}

impl Departamento {
    pub fn label(self) -> &'static str {
        match self {
            Self::Escritorio => "ESCRITÓRIO",
            Self::Oficina => "OFICINA",
            Self::Design => "DESIGN",
            Self::Care => "CARE",
            Self::Interiores => "INTERIORES",
        }
    }

    pub fn ala(self) -> Ala {
        Ala::from_mesa(self)
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Escritorio => "escritorio",
            Self::Oficina => "oficina",
            Self::Design => "design",
            Self::Care => "care",
            Self::Interiores => "interiores",
        }
    }

    pub fn all() -> [Self; 5] {
        [
            Self::Escritorio,
            Self::Oficina,
            Self::Design,
            Self::Care,
            Self::Interiores,
        ]
    }

    pub fn from_slug(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "oficina" => Self::Oficina,
            "design" => Self::Design,
            "care" => Self::Care,
            "interiores" => Self::Interiores,
            _ => Self::Escritorio,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub nome: String,
    pub departamento: Departamento,
    pub salt: String,
    pub hash: String,
    /// Extra role: Sistema, path, users, backup. Not a desk.
    #[serde(default)]
    pub admin: bool,
    /// Off = cannot log in. History stays.
    #[serde(default = "default_activo")]
    pub activo: bool,
    /// Software sysadmin identity — not a company desk. Separate from Gil/Rodrigo work logins.
    #[serde(default)]
    pub software: bool,
    /// pedido | activo | recusado
    #[serde(default = "default_estado")]
    pub estado: String,
    #[serde(default)]
    pub mesa_pedida: String,
    #[serde(default)]
    pub motivo: String,
    #[serde(default)]
    pub pedido_por: String,
    #[serde(default)]
    pub decidido_por: String,
    #[serde(default)]
    pub decidido_em: String,
    /// Reserved for later fine-grained grants. Unused in pass 1.
    #[serde(default)]
    pub permissoes: Vec<String>,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.admin && self.activo
    }

    pub fn is_software(&self) -> bool {
        self.software && self.activo
    }

    pub fn ala(&self) -> Ala {
        self.departamento.ala()
    }

    pub fn estado_label(&self) -> &'static str {
        match self.estado.as_str() {
            EST_PEDIDO => "PEDIDO",
            EST_RECUSADO => "RECUSADO",
            _ => "ACTIVO",
        }
    }

    pub fn is_pedido(&self) -> bool {
        self.estado == EST_PEDIDO
    }

    pub fn is_recusado(&self) -> bool {
        self.estado == EST_RECUSADO
    }

    pub fn mesa_label(&self) -> String {
        if self.software {
            return "SOFTWARE".to_string();
        }
        let extra = if self.departamento == Departamento::Interiores {
            " · reservado"
        } else {
            ""
        };
        format!("{} / {}{extra}", self.ala().label(), self.departamento.label())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct UserFile {
    #[serde(default)]
    users: Vec<User>,
}

fn users_path(root: &Path) -> std::path::PathBuf {
    interno_root(root).join(USERS_FILE)
}

fn hash_pw_sha(salt: &str, password: &str) -> String {
    let mut h = Sha256::new();
    h.update(salt.as_bytes());
    h.update(b"|");
    h.update(password.as_bytes());
    format!("{:x}", h.finalize())
}

fn hash_pw_argon2(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("argon2: {e}"))?;
    Ok(hash.to_string())
}

fn password_ok(stored: &str, salt: &str, password: &str) -> bool {
    if stored.starts_with("$argon2") {
        let Ok(parsed) = PasswordHash::new(stored) else {
            return false;
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    } else {
        hash_pw_sha(salt, password) == stored
    }
}

fn new_salt() -> String {
    let mut b = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// If nobody has admin yet, promote former Design logins (they were the only
/// ones who could open Sistema). Does not look up anyone by name.
fn ensure_admin(users: &mut Vec<User>) -> bool {
    if users.iter().any(|u| u.admin) {
        return false;
    }
    let mut changed = false;
    for u in users.iter_mut() {
        if u.departamento == Departamento::Design {
            u.admin = true;
            changed = true;
        }
    }
    if !users.iter().any(|u| u.admin) {
        if let Some(u) = users.first_mut() {
            u.admin = true;
            changed = true;
        }
    }
    changed
}

fn load_users_json(root: &Path) -> Vec<User> {
    let path = users_path(root);
    let Ok(raw) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let text = if let Ok(key) = vault::load_or_create_key(root) {
        vault::open(&key, &raw)
    } else {
        raw
    };
    serde_json::from_str::<UserFile>(&text)
        .map(|f| f.users)
        .unwrap_or_default()
}

fn load_users_sql(root: &Path) -> Result<Vec<User>> {
    let conn = crate::db::open(root)?;
    let mut st = conn.prepare(
        "SELECT nome, mesa, software, admin, activo, salt, hash,
                IFNULL(estado,'activo'), IFNULL(mesa_pedida,''), IFNULL(motivo,''),
                IFNULL(pedido_por,''), IFNULL(decidido_por,''), IFNULL(decidido_em,'')
         FROM pessoa ORDER BY id",
    )?;
    let rows = st.query_map([], |r| {
        let hash: String = r.get(6)?;
        let mut estado: String = r.get(7)?;
        if estado.trim().is_empty() {
            estado = if hash.is_empty() {
                EST_PEDIDO.to_string()
            } else {
                EST_ACTIVO.to_string()
            };
        }
        Ok(User {
            nome: r.get::<_, String>(0)?,
            departamento: Departamento::from_slug(&r.get::<_, String>(1).unwrap_or_default()),
            software: r.get::<_, i64>(2)? != 0,
            admin: r.get::<_, i64>(3)? != 0,
            activo: r.get::<_, i64>(4)? != 0,
            salt: r.get::<_, String>(5).unwrap_or_default(),
            hash,
            estado,
            mesa_pedida: r.get(8)?,
            motivo: r.get(9)?,
            pedido_por: r.get(10)?,
            decidido_por: r.get(11)?,
            decidido_em: r.get(12)?,
            permissoes: Vec::new(),
        })
    })?;
    let mut users = Vec::new();
    for row in rows {
        users.push(row?);
    }
    Ok(users)
}

pub fn load_users(root: &Path) -> Vec<User> {
    let mut users = load_users_sql(root).unwrap_or_default();
    if users.is_empty() {
        users = load_users_json(root);
        if !users.is_empty() {
            let _ = save_users(root, &users);
        }
    }
    if ensure_admin(&mut users) {
        let _ = save_users(root, &users);
    }
    users
}

fn save_users(root: &Path, users: &[User]) -> Result<()> {
    let conn = crate::db::open(root)?;
    conn.execute("DELETE FROM pessoa", [])?;
    let now = crate::model::now_stamp();
    for u in users {
        let estado = if u.estado.trim().is_empty() {
            if u.hash.is_empty() && !u.software {
                EST_PEDIDO
            } else {
                EST_ACTIVO
            }
        } else {
            u.estado.as_str()
        };
        conn.execute(
            "INSERT INTO pessoa(nome, mesa, software, admin, activo, salt, hash, created,
                estado, mesa_pedida, motivo, pedido_por, decidido_por, decidido_em)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            rusqlite::params![
                u.nome,
                u.departamento.slug(),
                if u.software { 1 } else { 0 },
                if u.admin { 1 } else { 0 },
                if u.activo { 1 } else { 0 },
                u.salt,
                u.hash,
                now,
                estado,
                u.mesa_pedida,
                u.motivo,
                u.pedido_por,
                u.decidido_por,
                u.decidido_em,
            ],
        )?;
    }
    Ok(())
}

pub fn verify(root: &Path, nome: &str, password: &str) -> Option<User> {
    let nome = nome.trim();
    let mut users = load_users(root);
    let idx = users.iter().position(|u| {
        u.activo && u.nome == nome && u.estado != EST_PEDIDO && u.estado != EST_RECUSADO
    })?;
    if !password_ok(&users[idx].hash, &users[idx].salt, password) {
        return None;
    }
    if !users[idx].hash.starts_with("$argon2") {
        if let Ok(h) = hash_pw_argon2(password) {
            users[idx].hash = h;
            let _ = save_users(root, &users);
        }
    }
    Some(users[idx].clone())
}

pub fn add_user(
    root: &Path,
    nome: &str,
    departamento: Departamento,
    password: &str,
    admin: bool,
) -> Result<User> {
    add_user_ex(root, nome, departamento, password, admin, false)
}

/// Login for the Vanguarda *software* (sysadmin). Not a Front/Back desk and not a staff folder.
pub fn add_software_user(root: &Path, nome: &str, password: &str) -> Result<User> {
    add_user_ex(
        root,
        nome,
        Departamento::Escritorio,
        password,
        true,
        true,
    )
}

fn add_user_ex(
    root: &Path,
    nome: &str,
    departamento: Departamento,
    password: &str,
    admin: bool,
    software: bool,
) -> Result<User> {
    let nome = nome.trim();
    if nome.is_empty() {
        bail!("O nome é obrigatório.");
    }
    if password.len() < 8 {
        bail!("A palavra-passe precisa de pelo menos 8 caracteres.");
    }
    let mut users = load_users(root);
    if users.iter().any(|u| u.nome.eq_ignore_ascii_case(nome)) {
        bail!("Já existe um utilizador com esse nome.");
    }
    let salt = new_salt();
    let hash = hash_pw_argon2(password)?;
    let admin = admin || software;
    let user = User {
        nome: nome.to_string(),
        departamento,
        hash,
        salt,
        admin,
        activo: true,
        software,
        estado: EST_ACTIVO.to_string(),
        mesa_pedida: departamento.slug().to_string(),
        motivo: String::new(),
        pedido_por: String::new(),
        decidido_por: String::new(),
        decidido_em: String::new(),
        permissoes: Vec::new(),
    };
    users.push(user.clone());
    save_users(root, &users)?;
    if !software {
        let dir = staff_dir(root, nome);
        let _ = writers::write_staff(
            &dir,
            &StaffFicha::blank(nome),
            WriteOpts { skip_existing: true },
        );
    }
    Ok(user)
}

pub fn create_pedido(
    root: &Path,
    nome: &str,
    mesa: Departamento,
    pedido_por: &str,
) -> Result<User> {
    let nome = nome.trim();
    if nome.is_empty() {
        bail!("O nome é obrigatório.");
    }
    let mut users = load_users(root);
    if users.iter().any(|u| u.nome.eq_ignore_ascii_case(nome)) {
        bail!("Já existe uma pessoa com esse nome.");
    }
    let user = User {
        nome: nome.to_string(),
        departamento: mesa,
        hash: String::new(),
        salt: String::new(),
        admin: false,
        activo: false,
        software: false,
        estado: EST_PEDIDO.to_string(),
        mesa_pedida: mesa.slug().to_string(),
        motivo: String::new(),
        pedido_por: pedido_por.trim().to_string(),
        decidido_por: String::new(),
        decidido_em: String::new(),
        permissoes: Vec::new(),
    };
    users.push(user.clone());
    save_users(root, &users)?;
    let dir = staff_dir(root, nome);
    let _ = writers::write_staff(
        &dir,
        &StaffFicha::blank(nome),
        WriteOpts { skip_existing: true },
    );
    Ok(user)
}

pub fn aprovar(
    root: &Path,
    nome: &str,
    mesa: Departamento,
    password: &str,
    quem: &str,
) -> Result<()> {
    if password.len() < 8 {
        bail!("A palavra-passe precisa de pelo menos 8 caracteres.");
    }
    let mut users = load_users(root);
    let Some(u) = users.iter_mut().find(|u| u.nome == nome) else {
        bail!("Pedido não encontrado.");
    };
    if u.software {
        bail!("A conta do software não é um pedido.");
    }
    u.departamento = mesa;
    u.hash = hash_pw_argon2(password)?;
    u.salt = new_salt();
    u.activo = true;
    u.estado = EST_ACTIVO.to_string();
    u.motivo.clear();
    u.decidido_por = quem.trim().to_string();
    u.decidido_em = crate::model::now_stamp();
    save_users(root, &users)
}

pub fn recusar(root: &Path, nome: &str, motivo: &str, quem: &str) -> Result<()> {
    let motivo = motivo.trim();
    if motivo.is_empty() {
        bail!("O motivo da recusa é obrigatório.");
    }
    let mut users = load_users(root);
    let Some(u) = users.iter_mut().find(|u| u.nome == nome) else {
        bail!("Pedido não encontrado.");
    };
    if u.software {
        bail!("A conta do software não é um pedido.");
    }
    u.activo = false;
    u.hash.clear();
    u.estado = EST_RECUSADO.to_string();
    u.motivo = motivo.to_string();
    u.decidido_por = quem.trim().to_string();
    u.decidido_em = crate::model::now_stamp();
    save_users(root, &users)
}

pub fn reabrir(root: &Path, nome: &str) -> Result<()> {
    let mut users = load_users(root);
    let Some(u) = users.iter_mut().find(|u| u.nome == nome) else {
        bail!("Pessoa não encontrada.");
    };
    if u.software {
        bail!("A conta do software não é um pedido.");
    }
    u.activo = false;
    u.hash.clear();
    u.estado = EST_PEDIDO.to_string();
    save_users(root, &users)
}

pub fn reset_password(root: &Path, nome: &str, password: &str) -> Result<()> {
    if password.len() < 8 {
        bail!("A palavra-passe precisa de pelo menos 8 caracteres.");
    }
    let mut users = load_users(root);
    let Some(u) = users.iter_mut().find(|u| u.nome == nome) else {
        bail!("Utilizador não encontrado.");
    };
    u.salt = new_salt();
    u.hash = hash_pw_argon2(password)?;
    save_users(root, &users)
}

pub fn set_user(
    root: &Path,
    nome: &str,
    departamento: Departamento,
    admin: bool,
    activo: bool,
) -> Result<()> {
    let mut users = load_users(root);
    let admins = users.iter().filter(|u| u.admin && u.activo).count();
    let Some(u) = users.iter_mut().find(|u| u.nome == nome) else {
        bail!("Utilizador não encontrado.");
    };
    let was_admin = u.admin && u.activo;
    if was_admin && admins <= 1 && (!admin || !activo) {
        bail!("Não desactivo nem tiro o último administrador.");
    }
    if activo && (u.hash.is_empty() || u.estado == EST_PEDIDO || u.estado == EST_RECUSADO) {
        bail!("Aprova em Pessoas com palavra-passe.");
    }
    u.departamento = departamento;
    u.admin = admin || u.software;
    u.activo = activo;
    if activo {
        u.estado = EST_ACTIVO.to_string();
    }
    save_users(root, &users)
}

pub fn set_software(root: &Path, nome: &str, software: bool) -> Result<()> {
    let mut users = load_users(root);
    let Some(u) = users.iter_mut().find(|u| u.nome == nome) else {
        bail!("Utilizador não encontrado.");
    };
    u.software = software;
    if software {
        u.admin = true;
        u.activo = true;
    }
    save_users(root, &users)
}

pub fn rename_user(root: &Path, from: &str, to: &str) -> Result<()> {
    let from = from.trim();
    let to = to.trim();
    if from.is_empty() || to.is_empty() {
        bail!("Nomes inválidos.");
    }
    if from.eq_ignore_ascii_case(to) {
        return Ok(());
    }
    let mut users = load_users(root);
    if users.iter().any(|u| u.nome.eq_ignore_ascii_case(to) && !u.nome.eq_ignore_ascii_case(from))
    {
        bail!("Já existe um login «{to}».");
    }
    let Some(u) = users.iter_mut().find(|u| u.nome == from) else {
        return Ok(());
    };
    u.nome = to.to_string();
    save_users(root, &users)
}

pub fn delete_user(root: &Path, nome: &str) -> Result<()> {
    let mut users = load_users(root);
    let target_admin = users
        .iter()
        .any(|u| u.nome == nome && u.admin && u.activo);
    let admins_left = users
        .iter()
        .filter(|u| u.admin && u.activo && u.nome != nome)
        .count();
    if target_admin && admins_left == 0 {
        bail!("Não apago o último administrador.");
    }
    let n = users.len();
    users.retain(|u| u.nome != nome);
    if users.len() == n {
        bail!("Utilizador não encontrado.");
    }
    save_users(root, &users)
}

pub fn is_first_run(root: &Path) -> bool {
    load_users(root).is_empty()
}

pub fn activo(root: &Path) -> Vec<User> {
    load_users(root)
        .into_iter()
        .filter(|u| {
            u.activo && u.estado != EST_PEDIDO && u.estado != EST_RECUSADO && !u.hash.is_empty()
        })
        .collect()
}

/// Keep Gil Salvador (shop) and any software login. Drop the rest.
pub fn keep_gil_and_software(root: &Path) -> Result<()> {
    let keep = "Gil Salvador";
    let mut users = load_users(root);
    for u in &mut users {
        if !u.software && u.nome.eq_ignore_ascii_case("Gil") {
            u.nome = keep.to_string();
        }
    }
    users.retain(|u| u.software || u.nome.eq_ignore_ascii_case(keep));
    let mut seen_gil = false;
    users.retain(|u| {
        if u.software {
            return true;
        }
        if seen_gil {
            return false;
        }
        seen_gil = true;
        true
    });
    if !users.iter().any(|u| u.admin && u.activo) {
        if let Some(u) = users.iter_mut().find(|u| u.nome.eq_ignore_ascii_case(keep)) {
            u.admin = true;
            u.activo = true;
        } else if let Some(u) = users.first_mut() {
            u.admin = true;
            u.activo = true;
        }
    }
    save_users(root, &users)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_roundtrip_and_first_is_admin() {
        let root = std::env::temp_dir().join(format!("v-auth-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        assert!(is_first_run(&root));
        let u = add_user(&root, "Casa", Departamento::Escritorio, "secret12", true).unwrap();
        assert!(u.admin);
        assert!(u.activo);
        assert_eq!(u.departamento, Departamento::Escritorio);
        assert_eq!(Departamento::all().len(), 5);
        assert!(Departamento::all().contains(&Departamento::Interiores));
        assert!(verify(&root, "Casa", "secret12").is_some());
        assert!(verify(&root, "Casa", "nope").is_none());
        add_user(&root, "Rita", Departamento::Escritorio, "mesa1234", false).unwrap();
        add_user(&root, "Gil", Departamento::Oficina, "chao1234", false).unwrap();
        assert!(delete_user(&root, "Casa").is_err());
        set_user(&root, "Casa", Departamento::Oficina, false, true).unwrap_err();
        reset_password(&root, "Rita", "mesa5678").unwrap();
        assert!(verify(&root, "Rita", "mesa5678").is_some());
        add_user(&root, "Outro", Departamento::Design, "abcdefgh", true).unwrap();
        set_user(&root, "Rita", Departamento::Escritorio, false, false).unwrap();
        assert!(verify(&root, "Rita", "mesa5678").is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn old_design_becomes_admin_on_load() {
        let root = std::env::temp_dir().join(format!("v-auth-mig-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let salt = new_salt();
        let legacy = User {
            nome: "Desenho".into(),
            departamento: Departamento::Design,
            salt: salt.clone(),
            hash: hash_pw_sha(&salt, "abcdefgh"),
            admin: false,
            activo: true,
            software: false,
            estado: EST_ACTIVO.into(),
            mesa_pedida: String::new(),
            motivo: String::new(),
            pedido_por: String::new(),
            decidido_por: String::new(),
            decidido_em: String::new(),
            permissoes: Vec::new(),
        };
        save_users(&root, &[legacy]).unwrap();
        let loaded = load_users(&root);
        assert!(loaded[0].admin);
        assert!(verify(&root, "Desenho", "abcdefgh").unwrap().is_admin());
        let upgraded = load_users(&root);
        assert!(
            upgraded[0].hash.starts_with("$argon2"),
            "login antigo deve passar a argon2"
        );
        assert!(verify(&root, "Desenho", "abcdefgh").is_some());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn new_user_is_argon2() {
        let root = std::env::temp_dir().join(format!("v-auth-ar-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let u = add_user(&root, "Nova", Departamento::Escritorio, "abcdefgh", true).unwrap();
        assert!(u.hash.starts_with("$argon2"));
        assert!(verify(&root, "Nova", "abcdefgh").is_some());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn ala_from_mesa_and_software_login() {
        assert_eq!(Departamento::Oficina.ala(), Ala::Back);
        assert_eq!(Departamento::Care.ala(), Ala::Back);
        assert_eq!(Departamento::Interiores.ala(), Ala::Back);
        assert_eq!(Departamento::Escritorio.ala(), Ala::Front);
        assert_eq!(Departamento::Design.ala(), Ala::Front);
        let root = std::env::temp_dir().join(format!("v-auth-sw-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let sw = add_software_user(&root, "Vanguarda", "secret12").unwrap();
        assert!(sw.software);
        assert!(sw.admin);
        assert_eq!(sw.mesa_label(), "SOFTWARE");
        assert!(!staff_dir(&root, "Vanguarda").join("staff.json").is_file());
        let gil = add_user(&root, "Gil Salvador", Departamento::Escritorio, "secret12", false)
            .unwrap();
        assert!(!gil.software);
        assert_eq!(gil.mesa_label(), "FRONT / ESCRITÓRIO");
        assert!(staff_dir(&root, "Gil Salvador").join("staff.json").is_file());
        rename_user(&root, "Gil Salvador", "Gil Salvador").unwrap();
        set_user(&root, "Gil Salvador", Departamento::Care, false, true).unwrap();
        let g = verify(&root, "Gil Salvador", "secret12").unwrap();
        assert_eq!(g.departamento, Departamento::Care);
        assert_eq!(g.ala(), Ala::Back);
        let again = load_users(&root);
        assert_eq!(again.len(), 2);
        assert!(again.iter().any(|u| u.software));
        let ped = create_pedido(&root, "Ana", Departamento::Oficina, "Gil Salvador").unwrap();
        assert_eq!(ped.estado, EST_PEDIDO);
        assert!(verify(&root, "Ana", "secret12").is_none());
        recusar(&root, "Ana", "ainda sem contrato", "Gil Salvador").unwrap();
        let a = load_users(&root).into_iter().find(|u| u.nome == "Ana").unwrap();
        assert_eq!(a.estado, EST_RECUSADO);
        assert_eq!(a.motivo, "ainda sem contrato");
        reabrir(&root, "Ana").unwrap();
        aprovar(&root, "Ana", Departamento::Oficina, "secret12", "Gil Salvador").unwrap();
        assert!(verify(&root, "Ana", "secret12").is_some());
        let ped2 = create_pedido(&root, "Rui", Departamento::Care, "Gil Salvador").unwrap();
        assert!(set_user(&root, &ped2.nome, Departamento::Care, false, true).is_err());
        assert!(verify(&root, "Rui", "secret12").is_none());
        let _ = fs::remove_dir_all(&root);
    }
}
