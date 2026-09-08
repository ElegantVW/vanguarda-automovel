use crate::elm::Link;
use anyhow::Result;

/// ECU falsa (Academia 06). Sem carro.
#[derive(Default)]
pub struct MockLink {
    sh: String,
}

impl Link for MockLink {
    fn cmd(&mut self, line: &str) -> Result<String> {
        let c = line.trim().to_ascii_uppercase();
        if c.starts_with("ATSH") {
            self.sh = c.trim_start_matches("ATSH").replace(' ', "");
            return Ok("OK".into());
        }
        let r = match c.as_str() {
            "ATZ" | "ATI" => "ELM327 v2.3\nOK",
            "STI" => "STN1170 v2.3.04\nOK",
            "ATE0" | "ATL0" | "ATH1" | "ATSP6" => "OK",
            "ATRV" => "12.4V",
            "STP 33" | "STP 53" => "OK",
            "0902" => "7E8 10 14 49 02 01 57 46 30\n7E8 21 5A 5A 5A 30 47 30 46\n7E8 22 47 42 36 33 32 30 38",
            "03" => "7E8 06 43 02 24 63 25 98",
            "07" => "7E8 04 47 01 08 33",
            "04" => "OK",
            "0202" => "NO DATA",
            "1902AF" => {
                if self.sh.contains("7E0") {
                    "7E8 06 59 02 24 63 00"
                } else {
                    "NO DATA"
                }
            }
            "010C" => "7E8 04 41 0C 0C 80",
            "010D" => "7E8 03 41 0D 00",
            "0105" => "7E8 03 41 05 5A",
            _ => "NO DATA",
        };
        Ok(r.into())
    }
}
