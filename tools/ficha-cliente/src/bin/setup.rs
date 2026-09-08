#![windows_subsystem = "windows"]

use anyhow::Result;
use eframe::egui::{self, Color32, RichText};
use ficha_cliente::paths;
use ficha_cliente::scaffold;
use std::path::{Path, PathBuf};
use winreg::enums::*;
use winreg::RegKey;

const GOLD: Color32 = Color32::from_rgb(0xC4, 0xA3, 0x5A);
const INK: Color32 = Color32::from_rgb(0xF4, 0xEF, 0xE6);
const APP_NAME: &str = "Vanguarda";
const EXE_NAME: &str = "Vanguarda.exe";
const ICO_NAME: &str = "Vanguarda.ico";
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Vanguarda";
const UNINSTALL_KEY_OLD: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Uninstall\VanguardaALHA";

fn main() -> Result<()> {
    let uninstall = std::env::args().any(|a| a == "--uninstall");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 480.0])
            .with_min_inner_size([520.0, 400.0])
            .with_title(if uninstall {
                "Desinstalar Vanguarda"
            } else {
                "Vanguarda — Instalação"
            }),
        ..Default::default()
    };
    eframe::run_native(
        "Vanguarda Setup",
        options,
        Box::new(move |_cc| Ok(Box::new(SetupApp::new(uninstall)))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Welcome,
    Checks,
    Location,
    Options,
    Install,
    Done,
    UninstallAsk,
    UninstallDone,
}

struct SetupApp {
    page: Page,
    dest: String,
    desktop_icon: bool,
    status: String,
    error: bool,
    win_ok: bool,
    gl_ok: bool,
    write_ok: bool,
    delete_data: bool,
}

impl SetupApp {
    fn new(uninstall: bool) -> Self {
        let dest = default_install_dir().display().to_string();
        let mut app = Self {
            page: if uninstall {
                Page::UninstallAsk
            } else {
                Page::Welcome
            },
            dest,
            desktop_icon: true,
            status: String::new(),
            error: false,
            win_ok: false,
            gl_ok: false,
            write_ok: false,
            delete_data: false,
        };
        app.refresh_checks();
        app
    }

    fn refresh_checks(&mut self) {
        self.win_ok = Path::new(r"C:\Windows\System32\ucrtbase.dll").exists();
        self.gl_ok = Path::new(r"C:\Windows\System32\opengl32.dll").exists();
        let dest = PathBuf::from(self.dest.trim());
        self.write_ok = std::fs::create_dir_all(&dest).is_ok();
    }

    fn payload_dir() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

impl eframe::App for SetupApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut vis = egui::Visuals::dark();
        vis.window_fill = Color32::from_rgb(20, 16, 14);
        vis.override_text_color = Some(INK);
        ctx.set_visuals(vis);

        egui::TopBottomPanel::top("t").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.label(RichText::new("Vanguarda").color(GOLD).size(22.0).strong());
            ui.add_space(6.0);
        });
        egui::TopBottomPanel::bottom("b").show(ctx, |ui| {
            ui.horizontal(|ui| {
                match self.page {
                    Page::Welcome => {
                        if ui.button("Seguinte").clicked() {
                            self.refresh_checks();
                            self.page = Page::Checks;
                        }
                    }
                    Page::Checks => {
                        if ui.button("Anterior").clicked() {
                            self.page = Page::Welcome;
                        }
                        let ok = self.win_ok && self.write_ok;
                        if ui.add_enabled(ok, egui::Button::new("Seguinte")).clicked() {
                            self.page = Page::Location;
                        }
                    }
                    Page::Location => {
                        if ui.button("Anterior").clicked() {
                            self.page = Page::Checks;
                        }
                        if ui.button("Seguinte").clicked() {
                            self.page = Page::Options;
                        }
                    }
                    Page::Options => {
                        if ui.button("Anterior").clicked() {
                            self.page = Page::Location;
                        }
                        if ui
                            .add(egui::Button::new(RichText::new("Instalar").color(Color32::from_rgb(0x14, 0x10, 0x0E)).strong()).fill(GOLD))
                            .clicked()
                        {
                            if self.desktop_icon {
                                self.status =
                                    "Vai criar um ícone no Ambiente de Trabalho.".into();
                            }
                            match do_install(Path::new(self.dest.trim()), self.desktop_icon) {
                                Ok(msg) => {
                                    self.status = msg;
                                    self.error = false;
                                    self.page = Page::Done;
                                }
                                Err(e) => {
                                    self.status = format!("{e:#}");
                                    self.error = true;
                                    self.page = Page::Install;
                                }
                            }
                        }
                    }
                    Page::Install => {
                        if ui.button("Voltar").clicked() {
                            self.page = Page::Options;
                        }
                    }
                    Page::Done => {
                        if ui.button("Abrir pasta").clicked() {
                            let _ = std::process::Command::new("explorer")
                                .arg(self.dest.trim())
                                .spawn();
                        }
                        if ui.button("Iniciar").clicked() {
                            let exe = PathBuf::from(self.dest.trim())
                                .join(EXE_NAME);
                            let _ = std::process::Command::new(exe)
                                .current_dir(self.dest.trim())
                                .spawn();
                        }
                        if ui.button("Fechar").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                    Page::UninstallAsk => {
                        if ui.button("Cancelar").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("Desinstalar").clicked() {
                            match do_uninstall(self.delete_data) {
                                Ok(msg) => {
                                    self.status = msg;
                                    self.error = false;
                                    self.page = Page::UninstallDone;
                                }
                                Err(e) => {
                                    self.status = format!("{e:#}");
                                    self.error = true;
                                }
                            }
                        }
                    }
                    Page::UninstallDone => {
                        if ui.button("Fechar").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);
            match self.page {
                Page::Welcome => {
                    ui.label("Assistente de instalação para Windows 10 e 11.");
                    ui.add_space(8.0);
                    ui.label("Não é preciso instalar Rust, Visual Studio nem Python.");
                    ui.label("O programa é um .exe autónomo. Os dados ficam nesta máquina,");
                    ui.label("não na pen USB.");
                }
                Page::Checks => {
                    ui.label(RichText::new("Verificações").strong().color(GOLD));
                    check_line(ui, self.win_ok, "Windows 10/11 (UCRT)");
                    check_line(ui, self.gl_ok, "OpenGL (opengl32.dll)");
                    check_line(ui, self.write_ok, "Pasta de destino gravável");
                    if !self.win_ok {
                        ui.colored_label(Color32::from_rgb(0xB3, 0x1B, 0x2C), "Windows demasiado antigo.");
                    }
                }
                Page::Location => {
                    ui.label("Pasta de instalação:");
                    ui.add(egui::TextEdit::singleline(&mut self.dest).desired_width(ui.available_width()));
                    if ui.button("Procurar…").clicked() {
                        if let Some(d) = rfd::FileDialog::new().pick_folder() {
                            self.dest = d.display().to_string();
                            self.refresh_checks();
                        }
                    }
                    ui.add_space(8.0);
                    ui.label("Lá dentro: o programa, logótipo, fundo, e a pasta Clientes.");
                }
                Page::Options => {
                    ui.checkbox(&mut self.desktop_icon, "Criar ícone no Ambiente de Trabalho");
                    ui.add_space(8.0);
                    ui.label("Pode desinstalar depois em Definições → Aplicações,");
                    ui.label("ou pelo atalho «Desinstalar Vanguarda».");
                }
                Page::Install | Page::Done => {
                    let c = if self.error {
                        Color32::from_rgb(0xB3, 0x1B, 0x2C)
                    } else {
                        GOLD
                    };
                    ui.label(RichText::new(&self.status).color(c));
                }
                Page::UninstallAsk => {
                    ui.label("Remover Vanguarda deste computador?");
                    ui.add_space(8.0);
                    ui.checkbox(
                        &mut self.delete_data,
                        "Apagar também a pasta Clientes (fichas locais)",
                    );
                    ui.label("A pen USB não é alterada.");
                    if self.error {
                        ui.colored_label(Color32::from_rgb(0xB3, 0x1B, 0x2C), &self.status);
                    }
                }
                Page::UninstallDone => {
                    ui.label(RichText::new(&self.status).color(GOLD));
                }
            }
        });
    }
}

fn check_line(ui: &mut egui::Ui, ok: bool, label: &str) {
    let mark = if ok { "OK" } else { "FALHA" };
    let color = if ok { GOLD } else { Color32::from_rgb(0xB3, 0x1B, 0x2C) };
    ui.horizontal(|ui| {
        ui.colored_label(color, mark);
        ui.label(label);
    });
}

fn default_install_dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into());
    PathBuf::from(base).join("Vanguarda")
}

fn find_app_payload(payload: &Path) -> Option<PathBuf> {
    for name in [EXE_NAME, "Vanguarda-0.2-ALHA.exe", "FichaCliente.exe"] {
        let p = payload.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn do_install(dest: &Path, desktop_icon: bool) -> Result<String> {
    std::fs::create_dir_all(dest)?;
    let payload = SetupApp::payload_dir();
    let app_src = find_app_payload(&payload).ok_or_else(|| {
        anyhow::anyhow!("não encontrei Vanguarda.exe ao lado do instalador")
    })?;
    std::fs::copy(&app_src, dest.join(EXE_NAME))?;
    for name in [
        "logo.png",
        "logo.jpg",
        "ui-bg.jpg",
        "chip-bg.jpg",
        "pdf-bg.jpg",
        ICO_NAME,
        "Vanguarda-0.2-ALHA.ico",
    ] {
        let src = payload.join(name);
        if src.is_file() {
            let dest_name = if name.ends_with(".ico") { ICO_NAME } else { name };
            std::fs::copy(&src, dest.join(dest_name))?;
        }
    }
    let icons_src = payload.join("icons");
    if icons_src.is_dir() {
        let dest_icons = dest.join("icons");
        std::fs::create_dir_all(&dest_icons)?;
        if let Ok(rd) = std::fs::read_dir(&icons_src) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_file() {
                    let _ = std::fs::copy(&p, dest_icons.join(e.file_name()));
                }
            }
        }
    }
    let this = std::env::current_exe()?;
    std::fs::copy(&this, dest.join("Uninstall.exe"))?;

    let house = paths::default_house();
    let clientes = paths::dados_clientes(&house);
    std::fs::create_dir_all(clientes.join("Interno"))?;
    paths::write_alha_toml_full(dest, Some(&house), &clientes)?;
    let _ = scaffold::run(&clientes);

    let ico = dest.join(ICO_NAME);
    let exe = dest.join(EXE_NAME);
    let ico_opt = ico.is_file().then_some(ico.as_path());
    for stale in old_desktop_lnks() {
        let _ = std::fs::remove_file(stale);
    }
    if desktop_icon {
        create_shortcut(&desktop_lnk(), &exe, dest, ico_opt)?;
    }
    let programs = start_menu_dir()?;
    std::fs::create_dir_all(&programs)?;
    let _ = std::fs::remove_file(programs.join("Vanguarda 0.2 ALHA.lnk"));
    let _ = std::fs::remove_file(programs.join("Desinstalar Vanguarda 0.2 ALHA.lnk"));
    create_shortcut(
        &programs.join("Vanguarda.lnk"),
        &exe,
        dest,
        ico_opt,
    )?;
    create_shortcut(
        &programs.join("Desinstalar Vanguarda.lnk"),
        &dest.join("Uninstall.exe"),
        dest,
        None,
    )?;

    write_uninstall_reg(dest, &exe, &ico)?;
    Ok(format!("Instalado em {}", dest.display()))
}

fn do_uninstall(delete_data: bool) -> Result<String> {
    let dest = install_location_from_reg().unwrap_or_else(default_install_dir);
    let _ = std::fs::remove_file(desktop_lnk());
    for stale in old_desktop_lnks() {
        let _ = std::fs::remove_file(stale);
    }
    if let Ok(programs) = start_menu_dir() {
        let _ = std::fs::remove_file(programs.join("Vanguarda.lnk"));
        let _ = std::fs::remove_file(programs.join("Desinstalar Vanguarda.lnk"));
        let _ = std::fs::remove_file(programs.join("Vanguarda 0.2 ALHA.lnk"));
        let _ = std::fs::remove_file(programs.join("Desinstalar Vanguarda 0.2 ALHA.lnk"));
        let _ = std::fs::remove_dir(&programs);
    }
    if let Ok(old) = start_menu_dir_old() {
        let _ = std::fs::remove_file(old.join("Vanguarda 0.2 ALHA.lnk"));
        let _ = std::fs::remove_file(old.join("Desinstalar Vanguarda 0.2 ALHA.lnk"));
        let _ = std::fs::remove_dir(&old);
    }
    if dest.is_dir() {
        if !delete_data {
            for name in [
                EXE_NAME,
                "Vanguarda-0.2-ALHA.exe",
                "Uninstall.exe",
                "logo.png",
                "logo.jpg",
                "ui-bg.jpg",
                "chip-bg.jpg",
                "pdf-bg.jpg",
                ICO_NAME,
                "Vanguarda-0.2-ALHA.ico",
                "alha.toml",
            ] {
                let _ = std::fs::remove_file(dest.join(name));
            }
        } else {
            let _ = std::fs::remove_dir_all(&dest);
        }
    }
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let _ = hkcu.delete_subkey_all(UNINSTALL_KEY);
    let _ = hkcu.delete_subkey_all(UNINSTALL_KEY_OLD);
    paths::clear_start_at_logon();
    Ok("Vanguarda foi removido.".into())
}

fn desktop_dir() -> PathBuf {
    let desk = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into());
    let od = PathBuf::from(&desk)
        .join("OneDrive")
        .join("Ambiente de Trabalho");
    if od.is_dir() {
        od
    } else {
        PathBuf::from(desk).join("Desktop")
    }
}

fn desktop_lnk() -> PathBuf {
    desktop_dir().join("Vanguarda.lnk")
}

fn old_desktop_lnks() -> Vec<PathBuf> {
    let d = desktop_dir();
    vec![
        d.join("Vanguarda 0.2 ALHA.lnk"),
        d.join("Vanguarda Automóvel.lnk"),
        d.join("Vanguarda Automóvel - Atalho.lnk"),
    ]
}

fn start_menu_dir() -> Result<PathBuf> {
    let appdata = std::env::var("APPDATA")?;
    Ok(PathBuf::from(appdata)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join(APP_NAME))
}

fn start_menu_dir_old() -> Result<PathBuf> {
    let appdata = std::env::var("APPDATA")?;
    Ok(PathBuf::from(appdata)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Vanguarda 0.2 ALHA"))
}

fn create_shortcut(lnk: &Path, target: &Path, workdir: &Path, ico: Option<&Path>) -> Result<()> {
    if let Some(parent) = lnk.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let ico_line = ico
        .map(|p| format!("$s.IconLocation = '{} ,0'", p.display()))
        .unwrap_or_default();
    let script = format!(
        "$s = (New-Object -ComObject WScript.Shell).CreateShortcut('{}'); $s.TargetPath = '{}'; $s.WorkingDirectory = '{}'; {ico_line}; $s.Save()",
        lnk.display(),
        target.display(),
        workdir.display(),
    );
    let st = std::process::Command::new("powershell")
        .args(["-NoProfile", "-STA", "-Command", &script])
        .status()?;
    if !st.success() {
        anyhow::bail!("não criei o atalho {}", lnk.display());
    }
    Ok(())
}

fn write_uninstall_reg(dest: &Path, exe: &Path, ico: &Path) -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(UNINSTALL_KEY)?;
    key.set_value("DisplayName", &APP_NAME)?;
    key.set_value("Publisher", &"Vanguarda Automóvel")?;
    key.set_value("InstallLocation", &dest.display().to_string())?;
    key.set_value(
        "UninstallString",
        &format!("\"{}\"", dest.join("Uninstall.exe").display()),
    )?;
    key.set_value("DisplayIcon", &if ico.is_file() {
        ico.display().to_string()
    } else {
        exe.display().to_string()
    })?;
    key.set_value("NoModify", &1u32)?;
    key.set_value("NoRepair", &1u32)?;
    Ok(())
}

fn install_location_from_reg() -> Option<PathBuf> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for key_path in [UNINSTALL_KEY, UNINSTALL_KEY_OLD] {
        if let Ok(key) = hkcu.open_subkey(key_path) {
            if let Ok(s) = key.get_value::<String, _>("InstallLocation") {
                return Some(PathBuf::from(s));
            }
        }
    }
    None
}
