//! Diagnóstico Vanguarda: adaptador ELM/STN, OBD-II público, mock para testes.
//!
//! Proibido: dumps Autocom, bases FORScan, security-access brute force.

mod dtc;
mod elm;
mod ford;
mod mock;
mod obd;

pub use dtc::describe_dtc;
pub use elm::{Link, SerialLink};
pub use mock::MockLink;
pub use obd::{clear_dtcs, live_to_scan_json, mock_scan, scan_obd, ScanOut};

use anyhow::Result;

/// Scan de bancada sem dongle (Academia 06).
pub fn scan_mock() -> Result<ScanOut> {
    mock_scan()
}

/// Scan ao vivo. `port` None = auto FTDI da casa.
pub fn scan_live(port: Option<&str>) -> Result<ScanOut> {
    let mut link = SerialLink::open(port)?;
    obd::scan_obd(&mut link)
}
