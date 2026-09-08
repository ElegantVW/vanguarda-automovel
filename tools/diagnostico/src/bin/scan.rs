use anyhow::Result;
use vanguarda_diagnostico::{live_to_scan_json, scan_live, scan_mock};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mock = args.iter().any(|a| a == "--mock");
    let port = args
        .windows(2)
        .find(|w| w[0] == "--port")
        .map(|w| w[1].as_str());
    let scan = if mock {
        scan_mock()?
    } else {
        scan_live(port)?
    };
    println!("{}", live_to_scan_json(&scan)?);
    Ok(())
}
