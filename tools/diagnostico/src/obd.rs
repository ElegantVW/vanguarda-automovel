use crate::dtc::describe_dtc;
use crate::elm::Link;
use crate::ford;
use crate::mock::MockLink;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DtcOut {
    pub sistema: String,
    pub categoria: String,
    pub zona: String,
    pub codigo: String,
    pub descricao: String,
    pub estado: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanOut {
    pub data: String,
    pub vin: String,
    pub matricula: String,
    pub veiculo: String,
    pub km: String,
    pub dtcs: Vec<DtcOut>,
    pub source_name: String,
    pub titulo: String,
    pub sumario: String,
    pub mecanico: String,
    pub estilo: String,
    pub source: String,
}

pub fn mock_scan() -> Result<ScanOut> {
    let mut link = MockLink::default();
    let mut scan = scan_obd(&mut link)?;
    scan.source = "live".into();
    scan.source_name = "mock".into();
    scan.veiculo = "Ford Focus (simulado)".into();
    Ok(scan)
}

pub fn scan_obd(link: &mut dyn Link) -> Result<ScanOut> {
    let ident = link.cmd("ATI").unwrap_or_default();
    let volts_raw = link.cmd("ATRV").unwrap_or_default();
    let volts = parse_volts(&volts_raw);
    if let Some(v) = volts {
        if v < 11.0 {
            bail!("tensão {v:.1} V (< 11 V). Ignição ligada? Pino 16?");
        }
    }
    let _ = link.cmd("ATSP6");
    let _ = link.cmd("STP 33");
    let vin_raw = link.cmd("0902").unwrap_or_default();
    let stored = link.cmd("03").unwrap_or_default();
    let pending = link.cmd("07").unwrap_or_default();
    let freeze = link.cmd("0202").unwrap_or_default();
    let mut scan = ScanOut {
        data: chrono::Local::now().format("%Y-%m-%d").to_string(),
        vin: parse_vin(&vin_raw),
        source: "live".into(),
        source_name: ident
            .lines()
            .next()
            .unwrap_or("vLinker")
            .trim()
            .to_string(),
        ..Default::default()
    };
    scan.dtcs = parse_mode_dtc(&stored, 0x43, "Presente");
    for d in parse_mode_dtc(&pending, 0x47, "Pendente") {
        if !scan.dtcs.iter().any(|x| x.codigo == d.codigo) {
            scan.dtcs.push(d);
        }
    }
    let _ = ford::probe(link, &mut scan.dtcs);
    let mut bits = Vec::new();
    if let Some(v) = volts {
        bits.push(format!("{v:.1} V"));
    }
    let rpm = link.cmd("010C").unwrap_or_default();
    if let Some(n) = parse_pid_0c(&rpm) {
        bits.push(format!("rpm {n}"));
    }
    if !freeze.to_ascii_uppercase().contains("NO DATA") {
        bits.push("freeze-frame presente".into());
    }
    scan.sumario = bits.join(" · ");
    if scan.titulo.is_empty() {
        let who = if !scan.vin.is_empty() {
            scan.vin.clone()
        } else {
            "Diagnóstico".into()
        };
        scan.titulo = format!("{} — {}", who, scan.data);
    }
    Ok(scan)
}

/// Mode 04. Só depois do PDF da casa estar em disco.
pub fn clear_dtcs(link: &mut dyn Link) -> Result<String> {
    let r = link.cmd("04")?;
    if r.to_ascii_uppercase().contains("NO DATA") {
        bail!("o adaptador não confirmou o apagar ({r})");
    }
    Ok(r)
}

pub fn parse_volts(s: &str) -> Option<f32> {
    let t: String = s
        .chars()
        .map(|c| if c == ',' { '.' } else { c })
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    t.parse().ok()
}

pub fn live_to_scan_json(scan: &ScanOut) -> Result<String> {
    Ok(serde_json::to_string_pretty(scan)?)
}

fn parse_vin(resp: &str) -> String {
    let bytes = hex_bytes(resp);
    // Mode 09 PID 02: 49 02 … then ASCII VIN (possibly ISO-TP chunked).
    let ascii: String = bytes
        .iter()
        .copied()
        .filter(|b| b.is_ascii_alphanumeric())
        .map(|b| b as char)
        .collect();
    if let Some(i) = ascii.find("WF0") {
        return ascii.chars().skip(i).take(17).collect();
    }
    if ascii.len() >= 17 {
        return ascii.chars().rev().take(17).collect::<String>().chars().rev().collect();
    }
    ascii
}

pub fn parse_mode_dtc(resp: &str, sid: u8, estado: &str) -> Vec<DtcOut> {
    let bytes = hex_bytes(resp);
    let mut out = Vec::new();
    if let Some(i) = bytes.iter().position(|b| *b == sid) {
        let n = *bytes.get(i + 1).unwrap_or(&0) as usize;
        let mut k = i + 2;
        for _ in 0..n.max(1) {
            if k + 1 >= bytes.len() {
                break;
            }
            if let Some(code) = decode_dtc(bytes[k], bytes[k + 1]) {
                let desc = describe_dtc(&code)
                    .unwrap_or("sem texto público")
                    .to_string();
                out.push(DtcOut {
                    sistema: "PCM".into(),
                    categoria: "Motor".into(),
                    zona: String::new(),
                    codigo: code,
                    descricao: desc,
                    estado: estado.into(),
                });
            }
            k += 2;
        }
    }
    out
}

pub fn decode_dtc(a: u8, b: u8) -> Option<String> {
    if a == 0 && b == 0 {
        return None;
    }
    let letters = ['P', 'C', 'B', 'U'];
    let l = letters[((a >> 6) & 0x03) as usize];
    let n = ((a as u16) & 0x3F) << 8 | b as u16;
    Some(format!("{l}{n:04X}"))
}

pub fn hex_bytes(resp: &str) -> Vec<u8> {
    resp.split_whitespace()
        .filter_map(|t| {
            if t.len() == 2 {
                u8::from_str_radix(t, 16).ok()
            } else {
                None
            }
        })
        .collect()
}

fn parse_pid_0c(resp: &str) -> Option<u32> {
    let bytes = hex_bytes(resp);
    let i = bytes.windows(2).position(|w| w == [0x41, 0x0C])?;
    let a = *bytes.get(i + 2)? as u32;
    let b = *bytes.get(i + 3)? as u32;
    Some((a * 256 + b) / 4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockLink;

    #[test]
    fn mock_vin_and_rpm() {
        let mut link = MockLink::default();
        let scan = scan_obd(&mut link).unwrap();
        assert_eq!(scan.vin, "WF0ZZZ0G0FGB63208", "{}", scan.vin);
        assert!(scan.dtcs.iter().any(|d| d.codigo == "P2463"), "{:?}", scan.dtcs);
        assert!(scan.sumario.contains("800"), "{}", scan.sumario);
        assert!(scan.sumario.contains("V"), "{}", scan.sumario);
    }

    #[test]
    fn low_voltage_aborts() {
        struct Low;
        impl crate::elm::Link for Low {
            fn cmd(&mut self, line: &str) -> anyhow::Result<String> {
                Ok(match line.trim().to_ascii_uppercase().as_str() {
                    "ATI" => "ELM327".into(),
                    "ATRV" => "10.1V".into(),
                    _ => "NO DATA".into(),
                })
            }
        }
        assert!(scan_obd(&mut Low).is_err());
    }

    #[test]
    fn volts_parse() {
        assert_eq!(parse_volts("12.4V"), Some(12.4));
    }

    #[test]
    fn pid_0c_800rpm() {
        assert_eq!(parse_pid_0c("7E8 04 41 0C 0C 80"), Some(800));
    }
}
