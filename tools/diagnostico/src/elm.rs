use anyhow::{bail, Context, Result};
use std::io::{Read, Write};
use std::time::Duration;

pub const HOUSE_FTDI: &str = "D3C662V2";

pub trait Link {
    fn cmd(&mut self, line: &str) -> Result<String>;
}

pub struct SerialLink {
    port: Box<dyn serialport::SerialPort>,
}

impl SerialLink {
    pub fn open(prefer: Option<&str>) -> Result<Self> {
        let name = match prefer {
            Some(p) if !p.is_empty() => p.to_string(),
            _ => find_house_port().ok_or_else(|| {
                anyhow::anyhow!("não encontrei o vLinker (FTDI {HOUSE_FTDI}). Indica --port COMx.")
            })?,
        };
        let port = serialport::new(&name, 115_200)
            .timeout(Duration::from_millis(2_000))
            .open()
            .with_context(|| format!("não abri {name}"))?;
        let mut s = Self { port };
        let _ = s.cmd("ATZ");
        let _ = s.cmd("ATE0");
        let _ = s.cmd("ATL0");
        let _ = s.cmd("ATH1");
        Ok(s)
    }
}

impl Link for SerialLink {
    fn cmd(&mut self, line: &str) -> Result<String> {
        let payload = format!("{}\r", line.trim());
        self.port.write_all(payload.as_bytes())?;
        self.port.flush()?;
        let mut buf = Vec::new();
        let mut tmp = [0u8; 256];
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > Duration::from_secs(8) {
                bail!("timeout à espera de '>' depois de {line}");
            }
            match self.port.read(&mut tmp) {
                Ok(n) if n > 0 => {
                    buf.extend_from_slice(&tmp[..n]);
                    if buf.contains(&b'>') {
                        break;
                    }
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    if buf.contains(&b'>') {
                        break;
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
        let s = String::from_utf8_lossy(&buf).replace('\r', "\n");
        Ok(s.replace('>', "").trim().to_string())
    }
}

fn find_house_port() -> Option<String> {
    let Ok(ports) = serialport::available_ports() else {
        return None;
    };
    for p in &ports {
        if let serialport::SerialPortType::UsbPort(info) = &p.port_type {
            let ser = info.serial_number.as_deref().unwrap_or("");
            if ser.eq_ignore_ascii_case(HOUSE_FTDI) {
                return Some(p.port_name.clone());
            }
        }
    }
    ports
        .iter()
        .find(|p| matches!(p.port_type, serialport::SerialPortType::UsbPort(_)))
        .map(|p| p.port_name.clone())
}
