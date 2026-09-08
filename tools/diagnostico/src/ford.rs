use crate::dtc::describe_dtc;
use crate::elm::Link;
use crate::obd::{decode_dtc, hex_bytes, DtcOut};
use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize)]
struct Map {
    modules: Vec<Mod>,
}

#[derive(Deserialize)]
struct Mod {
    id: String,
    name: String,
    bus: String,
    req: String,
}

const MAP_JSON: &str = include_str!("../data/ford-modules.vanguarda.json");

/// Probe house-owned module map. UDS 0x19 0x02. Skip OBD functional.
pub fn probe(link: &mut dyn Link, into: &mut Vec<DtcOut>) -> Result<usize> {
    let map: Map = serde_json::from_str(MAP_JSON)?;
    let mut n = 0;
    for m in map.modules {
        if m.id.eq_ignore_ascii_case("OBD") {
            continue;
        }
        let proto = if m.bus == "ms" { "STP 53" } else { "STP 33" };
        let _ = link.cmd(proto);
        let _ = link.cmd(&format!("ATSH {}", m.req));
        let raw = link.cmd("1902AF").unwrap_or_default();
        if raw.to_ascii_uppercase().contains("NO DATA")
            || raw.contains("7F 19")
            || raw.contains("7F19")
        {
            continue;
        }
        let before = into.len();
        parse_uds_dtcs(&raw, &m.name, into);
        n += into.len().saturating_sub(before);
    }
    let _ = link.cmd("STP 33");
    Ok(n)
}

fn parse_uds_dtcs(resp: &str, sistema: &str, into: &mut Vec<DtcOut>) {
    let bytes = hex_bytes(resp);
    let Some(i) = bytes.windows(2).position(|w| w == [0x59, 0x02]) else {
        return;
    };
    let mut k = i + 2;
    while k + 2 < bytes.len() {
        let a = bytes[k];
        let b = bytes[k + 1];
        let _status = bytes.get(k + 2).copied().unwrap_or(0);
        k += 3;
        if let Some(code) = decode_dtc(a, b) {
            if into.iter().any(|d| d.codigo == code && d.sistema == sistema) {
                continue;
            }
            let desc = describe_dtc(&code)
                .unwrap_or("sem texto público")
                .to_string();
            let categoria = if sistema.to_lowercase().contains("trav") {
                "Travões e estabilidade"
            } else if sistema.to_lowercase().contains("airbag") {
                "Segurança (airbag)"
            } else if sistema.to_lowercase().contains("painel") {
                "Painel de instrumentos"
            } else if sistema.to_lowercase().contains("carroç") || sistema.to_lowercase().contains("bcm") {
                "Sistema eléctrico"
            } else {
                "Motor"
            };
            into.push(DtcOut {
                sistema: sistema.into(),
                categoria: categoria.into(),
                zona: String::new(),
                codigo: code,
                descricao: desc,
                estado: "Presente".into(),
            });
        }
    }
}
