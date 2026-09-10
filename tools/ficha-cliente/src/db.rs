use crate::ops::{Job, Line, Quote, Sku};
use crate::paths::interno_root;
use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

pub fn db_path(clientes: &Path) -> PathBuf {
    interno_root(clientes).join("ops.sqlite")
}

pub fn open(clientes: &Path) -> Result<Connection> {
    let path = db_path(clientes);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let conn = Connection::open(&path).with_context(|| format!("{}", path.display()))?;
    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        CREATE TABLE IF NOT EXISTS quotes (
            id INTEGER PRIMARY KEY,
            numero TEXT NOT NULL UNIQUE,
            estado TEXT NOT NULL,
            cliente TEXT NOT NULL,
            matricula TEXT NOT NULL DEFAULT '',
            vin TEXT NOT NULL DEFAULT '',
            tipo TEXT NOT NULL,
            linhas TEXT NOT NULL,
            total_cents INTEGER NOT NULL DEFAULT 0,
            iva_incluido INTEGER NOT NULL DEFAULT 1,
            notas TEXT NOT NULL DEFAULT '',
            job_id INTEGER,
            created TEXT NOT NULL,
            valid_until TEXT NOT NULL DEFAULT '',
            colaborador TEXT NOT NULL DEFAULT ''
        );
        CREATE TABLE IF NOT EXISTS jobs (
            id INTEGER PRIMARY KEY,
            tipo TEXT NOT NULL,
            estado TEXT NOT NULL,
            cliente TEXT NOT NULL,
            matricula TEXT NOT NULL DEFAULT '',
            vin TEXT NOT NULL DEFAULT '',
            pack TEXT NOT NULL DEFAULT '',
            linhas TEXT NOT NULL DEFAULT '[]',
            previsao_cents INTEGER NOT NULL DEFAULT 0,
            custo_cents INTEGER NOT NULL DEFAULT 0,
            agendado TEXT NOT NULL DEFAULT '',
            iniciado TEXT NOT NULL DEFAULT '',
            entregue TEXT NOT NULL DEFAULT '',
            garantia_meses INTEGER NOT NULL DEFAULT 0,
            notas TEXT NOT NULL DEFAULT '',
            quote_id INTEGER,
            created TEXT NOT NULL,
            updated TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sku (
            id INTEGER PRIMARY KEY,
            codigo TEXT NOT NULL UNIQUE,
            nome TEXT NOT NULL,
            tipo TEXT NOT NULL,
            qty REAL NOT NULL DEFAULT 0,
            unidade TEXT NOT NULL DEFAULT 'un',
            pedir_cents INTEGER NOT NULL DEFAULT 0,
            chao_cents INTEGER NOT NULL DEFAULT 0,
            estado TEXT NOT NULL DEFAULT 'em_stock',
            notas TEXT NOT NULL DEFAULT ''
        );
        CREATE TABLE IF NOT EXISTS contas (
            id INTEGER PRIMARY KEY,
            numero TEXT NOT NULL UNIQUE,
            estado TEXT NOT NULL,
            cliente TEXT NOT NULL,
            matricula TEXT NOT NULL DEFAULT '',
            vin TEXT NOT NULL DEFAULT '',
            tipo TEXT NOT NULL,
            linhas TEXT NOT NULL,
            total_cents INTEGER NOT NULL DEFAULT 0,
            notas TEXT NOT NULL DEFAULT '',
            quote_id INTEGER,
            job_id INTEGER,
            created TEXT NOT NULL,
            colaborador TEXT NOT NULL DEFAULT '',
            usd_eur REAL NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS seq (
            kind TEXT PRIMARY KEY,
            n INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS pessoa (
            id INTEGER PRIMARY KEY,
            nome TEXT NOT NULL UNIQUE,
            mesa TEXT NOT NULL DEFAULT 'escritorio',
            software INTEGER NOT NULL DEFAULT 0,
            admin INTEGER NOT NULL DEFAULT 0,
            activo INTEGER NOT NULL DEFAULT 1,
            salt TEXT NOT NULL DEFAULT '',
            hash TEXT NOT NULL DEFAULT '',
            created TEXT NOT NULL DEFAULT ''
        );
        ",
    )?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<()> {
    let mut st = conn.prepare("PRAGMA table_info(jobs)")?;
    let cols: Vec<String> = st
        .query_map([], |r| r.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .collect();
    drop(st);
    if !cols.iter().any(|c| c == "pago") {
        conn.execute(
            "ALTER TABLE jobs ADD COLUMN pago INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    let mut st = conn.prepare("PRAGMA table_info(quotes)")?;
    let qcols: Vec<String> = st
        .query_map([], |r| r.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .collect();
    drop(st);
    if !qcols.iter().any(|c| c == "colaborador") {
        conn.execute(
            "ALTER TABLE quotes ADD COLUMN colaborador TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    if !qcols.iter().any(|c| c == "usd_eur") {
        conn.execute(
            "ALTER TABLE quotes ADD COLUMN usd_eur REAL NOT NULL DEFAULT 0",
            [],
        )?;
    }
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS contas (
            id INTEGER PRIMARY KEY,
            numero TEXT NOT NULL UNIQUE,
            estado TEXT NOT NULL,
            cliente TEXT NOT NULL,
            matricula TEXT NOT NULL DEFAULT '',
            vin TEXT NOT NULL DEFAULT '',
            tipo TEXT NOT NULL,
            linhas TEXT NOT NULL,
            total_cents INTEGER NOT NULL DEFAULT 0,
            notas TEXT NOT NULL DEFAULT '',
            quote_id INTEGER,
            job_id INTEGER,
            created TEXT NOT NULL,
            colaborador TEXT NOT NULL DEFAULT '',
            usd_eur REAL NOT NULL DEFAULT 0
        );",
    )?;
    migrate_pessoa(conn)?;
    Ok(())
}

fn migrate_pessoa(conn: &Connection) -> Result<()> {
    let mut st = conn.prepare("PRAGMA table_info(pessoa)")?;
    let cols: Vec<String> = st
        .query_map([], |r| r.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .collect();
    drop(st);
    if cols.is_empty() {
        return Ok(());
    }
    let extras = [
        ("estado", "TEXT NOT NULL DEFAULT 'activo'"),
        ("mesa_pedida", "TEXT NOT NULL DEFAULT ''"),
        ("motivo", "TEXT NOT NULL DEFAULT ''"),
        ("pedido_por", "TEXT NOT NULL DEFAULT ''"),
        ("decidido_por", "TEXT NOT NULL DEFAULT ''"),
        ("decidido_em", "TEXT NOT NULL DEFAULT ''"),
    ];
    for (name, def) in extras {
        if !cols.iter().any(|c| c == name) {
            conn.execute(&format!("ALTER TABLE pessoa ADD COLUMN {name} {def}"), [])?;
        }
    }
    conn.execute(
        "UPDATE pessoa SET estado = 'activo' WHERE IFNULL(hash,'') != '' AND IFNULL(estado,'') = ''",
        [],
    )?;
    Ok(())
}

fn now() -> String {
    crate::model::now_stamp()
}

fn today() -> String {
    crate::model::today()
}

fn lines_json(v: &[Line]) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "[]".into())
}

fn lines_parse(s: &str) -> Vec<Line> {
    serde_json::from_str(s).unwrap_or_default()
}

pub fn next_quote_numero(conn: &Connection) -> Result<String> {
    let year = chrono::Local::now().format("%Y").to_string();
    let kind = format!("orc-{year}");
    let n: i64 = conn
        .query_row(
            "SELECT n FROM seq WHERE kind = ?1",
            params![kind],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or(0)
        + 1;
    conn.execute(
        "INSERT INTO seq(kind, n) VALUES (?1, ?2)
         ON CONFLICT(kind) DO UPDATE SET n = excluded.n",
        params![kind, n],
    )?;
    Ok(format!("ORC-{year}-{n:04}"))
}

pub fn next_conta_numero(conn: &Connection) -> Result<String> {
    let year = chrono::Local::now().format("%Y").to_string();
    let kind = format!("cta-{year}");
    let n: i64 = conn
        .query_row(
            "SELECT n FROM seq WHERE kind = ?1",
            params![kind],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or(0)
        + 1;
    conn.execute(
        "INSERT INTO seq(kind, n) VALUES (?1, ?2)
         ON CONFLICT(kind) DO UPDATE SET n = excluded.n",
        params![kind, n],
    )?;
    Ok(format!("CTA-{year}-{n:04}"))
}

pub fn save_conta(conn: &Connection, q: &mut Quote) -> Result<i64> {
    q.recompute();
    if q.created.trim().is_empty() {
        q.created = today();
    }
    if q.numero.trim().is_empty() {
        q.numero = next_conta_numero(conn)?;
    }
    if q.estado.trim().is_empty() {
        q.estado = "emitida".into();
    }
    if q.id == 0 {
        conn.execute(
            "INSERT INTO contas(numero, estado, cliente, matricula, vin, tipo, linhas, total_cents, notas, quote_id, job_id, created, colaborador, usd_eur)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                q.numero,
                q.estado,
                q.cliente,
                q.matricula,
                q.vin,
                q.tipo,
                lines_json(&q.linhas),
                q.total_cents,
                q.notas,
                q.parent_quote_id,
                q.job_id,
                q.created,
                q.colaborador,
                q.usd_eur
            ],
        )?;
        q.id = conn.last_insert_rowid();
    } else {
        conn.execute(
            "UPDATE contas SET numero=?1, estado=?2, cliente=?3, matricula=?4, vin=?5, tipo=?6, linhas=?7, total_cents=?8, notas=?9, quote_id=?10, job_id=?11, colaborador=?12, usd_eur=?13 WHERE id=?14",
            params![
                q.numero,
                q.estado,
                q.cliente,
                q.matricula,
                q.vin,
                q.tipo,
                lines_json(&q.linhas),
                q.total_cents,
                q.notas,
                q.parent_quote_id,
                q.job_id,
                q.colaborador,
                q.usd_eur,
                q.id
            ],
        )?;
    }
    Ok(q.id)
}

fn conta_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Quote> {
    let linhas: String = r.get(7)?;
    Ok(Quote {
        id: r.get(0)?,
        numero: r.get(1)?,
        estado: r.get(2)?,
        cliente: r.get(3)?,
        matricula: r.get(4)?,
        vin: r.get(5)?,
        tipo: r.get(6)?,
        linhas: lines_parse(&linhas),
        total_cents: r.get(8)?,
        iva_incluido: false,
        notas: r.get(9)?,
        parent_quote_id: r.get(10)?,
        job_id: r.get(11)?,
        created: r.get(12)?,
        valid_until: String::new(),
        colaborador: r.get(13)?,
        usd_eur: r.get::<_, f64>(14).unwrap_or(0.0),
    })
}

pub fn list_contas(conn: &Connection) -> Result<Vec<Quote>> {
    let mut st = conn.prepare(
        "SELECT id, numero, estado, cliente, matricula, vin, tipo, linhas, total_cents, notas, quote_id, job_id, created, colaborador, usd_eur FROM contas ORDER BY id DESC",
    )?;
    let rows = st.query_map([], conta_from_row)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn save_quote(conn: &Connection, q: &mut Quote) -> Result<i64> {
    q.recompute();
    if q.created.trim().is_empty() {
        q.created = today();
    }
    if q.numero.trim().is_empty() {
        q.numero = next_quote_numero(conn)?;
    }
    if q.id == 0 {
        conn.execute(
            "INSERT INTO quotes(numero, estado, cliente, matricula, vin, tipo, linhas, total_cents, iva_incluido, notas, job_id, created, valid_until, colaborador, usd_eur)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![
                q.numero,
                q.estado,
                q.cliente,
                q.matricula,
                q.vin,
                q.tipo,
                lines_json(&q.linhas),
                q.total_cents,
                if q.iva_incluido { 1 } else { 0 },
                q.notas,
                q.job_id,
                q.created,
                q.valid_until,
                q.colaborador,
                q.usd_eur
            ],
        )?;
        q.id = conn.last_insert_rowid();
    } else {
        conn.execute(
            "UPDATE quotes SET numero=?1, estado=?2, cliente=?3, matricula=?4, vin=?5, tipo=?6, linhas=?7, total_cents=?8, iva_incluido=?9, notas=?10, job_id=?11, valid_until=?12, colaborador=?13, usd_eur=?14 WHERE id=?15",
            params![
                q.numero,
                q.estado,
                q.cliente,
                q.matricula,
                q.vin,
                q.tipo,
                lines_json(&q.linhas),
                q.total_cents,
                if q.iva_incluido { 1 } else { 0 },
                q.notas,
                q.job_id,
                q.valid_until,
                q.colaborador,
                q.usd_eur,
                q.id
            ],
        )?;
    }
    Ok(q.id)
}

pub fn save_job(conn: &Connection, j: &mut Job) -> Result<i64> {
    let stamp = now();
    if j.created.trim().is_empty() {
        j.created = stamp.clone();
    }
    j.updated = stamp;
    if j.id == 0 {
        conn.execute(
            "INSERT INTO jobs(tipo, estado, cliente, matricula, vin, pack, linhas, previsao_cents, custo_cents, agendado, iniciado, entregue, garantia_meses, notas, quote_id, created, updated, pago)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
            params![
                j.tipo,
                j.estado,
                j.cliente,
                j.matricula,
                j.vin,
                j.pack,
                lines_json(&j.linhas),
                j.previsao_cents,
                j.custo_cents,
                j.agendado,
                j.iniciado,
                j.entregue,
                j.garantia_meses,
                j.notas,
                j.quote_id,
                j.created,
                j.updated,
                if j.pago { 1 } else { 0 }
            ],
        )?;
        j.id = conn.last_insert_rowid();
    } else {
        conn.execute(
            "UPDATE jobs SET tipo=?1, estado=?2, cliente=?3, matricula=?4, vin=?5, pack=?6, linhas=?7, previsao_cents=?8, custo_cents=?9, agendado=?10, iniciado=?11, entregue=?12, garantia_meses=?13, notas=?14, quote_id=?15, updated=?16, pago=?17 WHERE id=?18",
            params![
                j.tipo,
                j.estado,
                j.cliente,
                j.matricula,
                j.vin,
                j.pack,
                lines_json(&j.linhas),
                j.previsao_cents,
                j.custo_cents,
                j.agendado,
                j.iniciado,
                j.entregue,
                j.garantia_meses,
                j.notas,
                j.quote_id,
                j.updated,
                if j.pago { 1 } else { 0 },
                j.id
            ],
        )?;
    }
    Ok(j.id)
}

fn quote_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Quote> {
    let linhas: String = r.get(7)?;
    Ok(Quote {
        id: r.get(0)?,
        numero: r.get(1)?,
        estado: r.get(2)?,
        cliente: r.get(3)?,
        matricula: r.get(4)?,
        vin: r.get(5)?,
        tipo: r.get(6)?,
        linhas: lines_parse(&linhas),
        total_cents: r.get(8)?,
        iva_incluido: r.get::<_, i64>(9)? != 0,
        notas: r.get(10)?,
        job_id: r.get(11)?,
        created: r.get(12)?,
        valid_until: r.get(13)?,
        colaborador: r.get::<_, String>(14).unwrap_or_default(),
        usd_eur: r.get::<_, f64>(15).unwrap_or(0.0),
        parent_quote_id: None,
    })
}

fn job_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Job> {
    let linhas: String = r.get(7)?;
    Ok(Job {
        id: r.get(0)?,
        tipo: r.get(1)?,
        estado: r.get(2)?,
        cliente: r.get(3)?,
        matricula: r.get(4)?,
        vin: r.get(5)?,
        pack: r.get(6)?,
        linhas: lines_parse(&linhas),
        previsao_cents: r.get(8)?,
        custo_cents: r.get(9)?,
        agendado: r.get(10)?,
        iniciado: r.get(11)?,
        entregue: r.get(12)?,
        garantia_meses: r.get(13)?,
        notas: r.get(14)?,
        quote_id: r.get(15)?,
        created: r.get(16)?,
        updated: r.get(17)?,
        pago: r.get::<_, i64>(18).unwrap_or(0) != 0,
    })
}

pub fn list_quotes(conn: &Connection) -> Result<Vec<Quote>> {
    let mut st = conn.prepare(
        "SELECT id, numero, estado, cliente, matricula, vin, tipo, linhas, total_cents, iva_incluido, notas, job_id, created, valid_until, colaborador, usd_eur FROM quotes ORDER BY id DESC",
    )?;
    let rows = st.query_map([], quote_from_row)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn list_jobs(conn: &Connection) -> Result<Vec<Job>> {
    let mut st = conn.prepare(
        "SELECT id, tipo, estado, cliente, matricula, vin, pack, linhas, previsao_cents, custo_cents, agendado, iniciado, entregue, garantia_meses, notas, quote_id, created, updated, pago FROM jobs ORDER BY id DESC",
    )?;
    let rows = st.query_map([], job_from_row)?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn list_sku(conn: &Connection) -> Result<Vec<Sku>> {
    let mut st = conn.prepare(
        "SELECT id, codigo, nome, tipo, qty, unidade, pedir_cents, chao_cents, estado, notas FROM sku ORDER BY tipo, codigo",
    )?;
    let rows = st.query_map([], |r| {
        Ok(Sku {
            id: r.get(0)?,
            codigo: r.get(1)?,
            nome: r.get(2)?,
            tipo: r.get(3)?,
            qty: r.get(4)?,
            unidade: r.get(5)?,
            pedir_cents: r.get(6)?,
            chao_cents: r.get(7)?,
            estado: r.get(8)?,
            notas: r.get(9)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn save_sku(conn: &Connection, s: &mut Sku) -> Result<i64> {
    if s.id == 0 {
        conn.execute(
            "INSERT INTO sku(codigo, nome, tipo, qty, unidade, pedir_cents, chao_cents, estado, notas)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)
             ON CONFLICT(codigo) DO UPDATE SET nome=excluded.nome, tipo=excluded.tipo, qty=excluded.qty, unidade=excluded.unidade, pedir_cents=excluded.pedir_cents, chao_cents=excluded.chao_cents, estado=excluded.estado, notas=excluded.notas",
            params![
                s.codigo, s.nome, s.tipo, s.qty, s.unidade, s.pedir_cents, s.chao_cents, s.estado, s.notas
            ],
        )?;
        s.id = conn.query_row("SELECT id FROM sku WHERE codigo=?1", params![s.codigo], |r| r.get(0))?;
    } else {
        conn.execute(
            "UPDATE sku SET codigo=?1, nome=?2, tipo=?3, qty=?4, unidade=?5, pedir_cents=?6, chao_cents=?7, estado=?8, notas=?9 WHERE id=?10",
            params![s.codigo, s.nome, s.tipo, s.qty, s.unidade, s.pedir_cents, s.chao_cents, s.estado, s.notas, s.id],
        )?;
    }
    Ok(s.id)
}

pub fn seed_if_empty(conn: &Connection, pecas: &[Sku], stardust: &[Sku]) -> Result<usize> {
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM sku", [], |r| r.get(0))?;
    if n > 0 {
        return Ok(0);
    }
    let mut c = 0;
    for s in stardust.iter().chain(pecas.iter()) {
        let mut s = s.clone();
        save_sku(conn, &mut s)?;
        c += 1;
    }
    Ok(c)
}

pub fn upsert_sku_list(conn: &Connection, items: &[Sku]) -> Result<usize> {
    let mut c = 0;
    for s in items {
        let mut s = s.clone();
        save_sku(conn, &mut s)?;
        c += 1;
    }
    Ok(c)
}

pub fn jobs_on_day(jobs: &[Job], day: &str) -> Vec<Job> {
    jobs.iter()
        .filter(|j| j.agendado.trim() == day)
        .cloned()
        .collect()
}

pub fn week_days() -> Vec<String> {
    use chrono::{Datelike, Duration, Local};
    let today = Local::now().date_naive();
    let wd = today.weekday();
    let offset = wd.num_days_from_monday() as i64;
    let mon = today - Duration::days(offset);
    (0..7)
        .map(|i| (mon + Duration::days(i)).format("%Y-%m-%d").to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::{parse_precos_pecas, stardust_seed, Job, Line, Quote};

    #[test]
    fn quote_job_and_stock_roundtrip() {
        let dir = std::env::temp_dir().join(format!("v-ops-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let conn = open(&dir).unwrap();
        let mut q = Quote {
            cliente: "Ana Teste".into(),
            tipo: "pintura".into(),
            colaborador: "Gil Salvador".into(),
            linhas: vec![Line {
                desc: "Bronze".into(),
                qty: 1.0,
                cents: 29000,
                ..Default::default()
            }],
            ..Default::default()
        };
        save_quote(&conn, &mut q).unwrap();
        assert!(q.numero.starts_with("ORC-"));
        assert!(q.id > 0);
        let mut j = Job::from_quote(&q);
        save_job(&conn, &mut j).unwrap();
        q.estado = "aceite".into();
        q.job_id = Some(j.id);
        save_quote(&conn, &mut q).unwrap();
        let qs = list_quotes(&conn).unwrap();
        assert_eq!(qs[0].total_cents, 29000);
        assert_eq!(qs[0].job_id, Some(j.id));
        assert_eq!(qs[0].colaborador, "Gil Salvador");
        let pecas = parse_precos_pecas("  12  EGR Ford  60 €  45 €\n");
        let n = seed_if_empty(&conn, &pecas, &stardust_seed()).unwrap();
        assert!(n > 10);
        assert_eq!(seed_if_empty(&conn, &pecas, &stardust_seed()).unwrap(), 0);
        let skus = list_sku(&conn).unwrap();
        assert!(skus.iter().any(|s| s.codigo == "WPU001"));
        assert!(skus.iter().any(|s| s.codigo == "FOCUS-12"));
        let mut c = q.clone();
        c.id = 0;
        c.numero.clear();
        c.estado = "emitida".into();
        c.parent_quote_id = Some(q.id);
        save_conta(&conn, &mut c).unwrap();
        assert!(c.numero.starts_with("CTA-"));
        let cs = list_contas(&conn).unwrap();
        assert_eq!(cs[0].parent_quote_id, Some(q.id));
        assert_eq!(cs[0].total_cents, 29000);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
