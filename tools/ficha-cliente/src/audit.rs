use crate::model::now_stamp;
use crate::paths::interno_root;
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

#[derive(Serialize)]
struct Line<'a> {
    ts: String,
    quem: &'a str,
    acao: &'a str,
    alvo: &'a str,
}

pub fn append(root: &Path, quem: &str, acao: &str, alvo: &str) {
    let dir = interno_root(root);
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("audit.jsonl");
    let line = Line {
        ts: now_stamp(),
        quem,
        acao,
        alvo,
    };
    let Ok(mut json) = serde_json::to_string(&line) else {
        return;
    };
    json.push('\n');
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = f.write_all(json.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_jsonl() {
        let root = std::env::temp_dir().join(format!("v-audit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        append(&root, "Casa", "backup", "teste.zip");
        let body = std::fs::read_to_string(interno_root(&root).join("audit.jsonl")).unwrap();
        assert!(body.contains("\"acao\":\"backup\""));
        let _ = std::fs::remove_dir_all(&root);
    }
}
