#![windows_subsystem = "windows"]

use ficha_cliente::app::FichaApp;
use ficha_cliente::{paths, scaffold};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--exemplos") {
        attach_console();
        let dir = paths::exe_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .join("Exemplos");
        let written = ficha_cliente::writers::write_tab_examples(&dir)?;
        println!("Exemplos em {}:", dir.display());
        for p in written {
            println!("  {}", p.display());
        }
        return Ok(());
    }
    if args.iter().any(|a| a == "--limpar-pessoas") {
        attach_console();
        let root = paths::find_clientes_root().ok_or_else(|| {
            anyhow::anyhow!("não encontrei a pasta de clientes")
        })?;
        let msg = ficha_cliente::people::eliminate_except_gil(&root)?;
        println!("{msg}");
        return Ok(());
    }
    if args.iter().any(|a| a == "--scaffold" || a == "--resave") {
        attach_console();
        let root = paths::find_clientes_root().ok_or_else(|| {
            anyhow::anyhow!(
                "não encontrei a pasta de clientes (alha.toml ou X:\\Vanguarda\\Office\\Clientes)"
            )
        })?;
        let written = if args.iter().any(|a| a == "--resave") {
            scaffold::resave(&root)?
        } else {
            scaffold::run(&root)?
        };
        if written.is_empty() {
            println!("Nada em falta em {}", root.display());
        } else {
            println!("Criados {} ficheiros:", written.len());
            for p in written {
                println!("  {}", p.display());
            }
        }
        return Ok(());
    }

    ficha_cliente::paths::ensure_start_at_logon();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_min_inner_size([900.0, 600.0])
            .with_maximized(true)
            .with_title("Vanguarda"),
        ..Default::default()
    };
    eframe::run_native(
        "Vanguarda",
        options,
        Box::new(|cc| {
            ficha_cliente::fonts::apply_egui_fonts(&cc.egui_ctx);
            Ok(Box::new(FichaApp::new()))
        }),
    )
    .map_err(|e| anyhow::anyhow!("GUI: {e}"))?;
    Ok(())
}

fn attach_console() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Console::{
            AllocConsole, AttachConsole, ATTACH_PARENT_PROCESS,
        };
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            let _ = AllocConsole();
        }
    }
}
