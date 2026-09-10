use crate::auth::{self, Ala, Departamento, User};
use crate::backup;
use crate::cars;
use crate::delete;
use crate::docstyle::DocStyle;
use crate::guias;
use crate::vault;
use crate::diag_parse::{self, Dtc, Scan};
use crate::fonts;
use crate::format;
use crate::media;
use crate::model::{
    care_check_label, care_checks_for, next_estado, next_estado_care, next_estado_interiores,
    now_stamp, pintura_check_label, today, Carro, ClientFicha, ConsumoEntry, ConsumoLog,
    StaffFicha, CARE_EXTRAS, CARE_PACOTES, CONSUMO_TIPOS, DONO_INTERNO, ESTADOS_CARE,
    ESTADOS_INTERIORES, ESTADOS_OFICINA, PINTURA_CHECKS, PINTURA_EXTRAS, PINTURA_PACOTES,
};
use crate::paths::{
    client_dir, find_clientes_root, list_client_names, list_staff_names, staff_dir,
};
use crate::scaffold;
use crate::people;
use crate::writers::{self, WriteOpts};
use crate::audit;
use crate::db;
use crate::ops::{self, Job, Line, Quote, Sku, CONTA_ESTADOS, JOB_ESTADOS, JOB_TIPOS, QUOTE_ESTADOS};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Vec2};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

mod ops_ui;

const GOLD: Color32 = Color32::from_rgb(0xD4, 0xB0, 0x6A);
const BRASS: Color32 = Color32::from_rgb(0xB8, 0xA0, 0x78);
const RED: Color32 = Color32::from_rgb(0xB3, 0x1B, 0x2C);
const WHITE: Color32 = Color32::from_rgb(0xE8, 0xE6, 0xE3);
const DARK: Color32 = Color32::from_rgb(0x16, 0x17, 0x18);
const PANEL: Color32 = Color32::from_rgb(0x1A, 0x1B, 0x1E);
const LINE: Color32 = Color32::from_rgb(0x3A, 0x3A, 0x3C);

#[derive(Clone, Copy, PartialEq, Eq)]
enum DeskGate {
    Menu,
    Abrir,
    Form,
    Apagar,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TrabKind {
    Quote,
    Job,
    Conta,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Inicio,
    Cliente,
    Carro,
    Staff,
    Diagnostico,
    Care,
    Interiores,
    Guias,
    Documentos,
    Sistema,
    Trabalho,
    Agenda,
    Stock,
}

pub struct FichaApp {
    root: String,
    mode: Mode,
    status: String,
    status_ok: bool,
    client_names: Vec<String>,
    staff_names: Vec<String>,
    selected_client: String,
    selected_staff: String,
    new_name: String,
    client: ClientFicha,
    staff: StaffFicha,
    consumo: ConsumoLog,
    nova_entrada: ConsumoEntry,
    scan: Scan,
    diag_client: String,
    cars: Vec<Carro>,
    selected_car: String,
    car: Carro,
    pulse: cars::HubPulse,
    show_consolidate: bool,
    conflicts: Vec<cars::Conflict>,
    consolidate_pick: bool,
    saved_diags: Vec<cars::SavedDiag>,
    selected_diag: String,
    bg: Option<egui::TextureHandle>,
    chip_tex: Option<egui::TextureHandle>,
    logo_tex: Option<egui::TextureHandle>,
    tex_tried: bool,
    t0: f64,
    show_greeting: bool,
    filter_client: String,
    filter_car: String,
    filter_staff: String,
    filter_diag: String,
    pending_delete: Option<PendingDelete>,
    session: Option<User>,
    login_name: String,
    login_pass: String,
    login_pass2: String,
    login_err: String,
    show_load_car: bool,
    load_car_pick: String,
    sys_new_name: String,
    sys_new_pass: String,
    login_dept: Departamento,
    pessoa_pass: String,
    pessoa_motivo: String,
    pessoa_filtro: String,
    merge_pending: Option<(String, String)>,
    merge_preview: String,
    show_consumo_wiz: bool,
    tab_t0: f64,
    login_t0: f64,
    busy: Option<BusyKind>,
    job_rx: Option<mpsc::Receiver<JobOut>>,
    last_diag_pdf: Option<PathBuf>,
    last_quote_pdf: Option<PathBuf>,
    filter_report: String,
    show_plate_conflict: bool,
    pending_scan: Option<Scan>,
    show_preview: bool,
    preview_tex: Option<egui::TextureHandle>,
    show_anexo: bool,
    anexo_items: Vec<(String, PathBuf, bool)>,
    anexo_base: PathBuf,
    anexo_export: bool,
    doc_style: DocStyle,
    fundo_thumbs: Vec<(PathBuf, String, Option<egui::TextureHandle>)>,
    status_at: f64,
    guia_desk: Departamento,
    guia_interno: bool,
    guia_body: String,
    dirty_fp: String,
    show_dirty: bool,
    pending_mode: Option<Mode>,
    quotes: Vec<Quote>,
    contas: Vec<Quote>,
    jobs: Vec<Job>,
    skus: Vec<Sku>,
    quote: Quote,
    conta: Quote,
    job: Job,
    sku: Sku,
    line_desc: String,
    line_eur: String,
    peca_qty: String,
    line_disc: String,
    peca_custo: String,
    peca_margem: String,
    usd_edit: String,
    mo_eur: String,
    mo_qty: String,
    mo_disc: String,
    ops_line_err: String,
    filter_ops: String,
    sku_qty: String,
    diag_pdf_saved: bool,
    pending_advance: Option<Carro>,
    desk_gate: DeskGate,
    trab_kind: TrabKind,
    filter_jobs: String,
    boot_max: u8,
    login_need_focus: bool,
}

#[derive(Clone, Copy)]
enum BusyKind {
    LerAutocom,
    LerAdaptador,
    GravarRelatorio,
    Actualizar,
    ApagarCodigos,
}

impl BusyKind {
    fn label(self) -> &'static str {
        match self {
            Self::LerAutocom => "A ler o ficheiro…",
            Self::LerAdaptador => "A ler vLinker…",
            Self::GravarRelatorio => "A gravar relatório…",
            Self::Actualizar => "A actualizar…",
            Self::ApagarCodigos => "A apagar códigos…",
        }
    }
}

enum JobOut {
    Scan(Result<Scan, String>),
    Cleared(Result<String, String>),
    Saved {
        pdf: Result<PathBuf, String>,
        car: Option<Carro>,
    },
    Lists {
        client_names: Vec<String>,
        staff_names: Vec<String>,
        cars: Vec<Carro>,
        saved_diags: Vec<cars::SavedDiag>,
        pulse: cars::HubPulse,
    },
}

enum PendingDelete {
    Client(String),
    Car { label: String, path: PathBuf },
    Diag { label: String, path: PathBuf, client: String },
}

impl FichaApp {
    pub fn new() -> Self {
        let root = find_clientes_root()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let doc_style = DocStyle::load(&PathBuf::from(&root));
        let mut app = Self {
            root,
            mode: Mode::Inicio,
            status: String::new(),
            status_ok: true,
            client_names: Vec::new(),
            staff_names: Vec::new(),
            selected_client: String::new(),
            selected_staff: String::new(),
            new_name: String::new(),
            client: ClientFicha::default(),
            staff: StaffFicha::default(),
            consumo: ConsumoLog::default(),
            nova_entrada: ConsumoEntry {
                data: today(),
                tipo: CONSUMO_TIPOS[0].to_string(),
                ..Default::default()
            },
            scan: Scan::default(),
            diag_client: String::new(),
            cars: Vec::new(),
            selected_car: String::new(),
            car: Carro::default(),
            pulse: cars::HubPulse {
                clients: 0,
                cars: 0,
                diags: 0,
                awaiting_parts: 0,
                in_repair: 0,
                drafts: Vec::new(),
                recent: Vec::new(),
            },
            show_consolidate: false,
            conflicts: Vec::new(),
            consolidate_pick: false,
            saved_diags: Vec::new(),
            selected_diag: String::new(),
            bg: None,
            chip_tex: None,
            logo_tex: None,
            tex_tried: false,
            t0: 0.0,
            show_greeting: true,
            filter_client: String::new(),
            filter_car: String::new(),
            filter_staff: String::new(),
            filter_diag: String::new(),
            pending_delete: None,
            session: None,
            login_name: "Gil Salvador".into(),
            login_pass: String::new(),
            login_pass2: String::new(),
            login_err: String::new(),
            show_load_car: false,
            load_car_pick: String::new(),
            sys_new_name: String::new(),
            sys_new_pass: String::new(),
            login_dept: Departamento::Escritorio,
            pessoa_pass: String::new(),
            pessoa_motivo: String::new(),
            pessoa_filtro: "pedido".into(),
            merge_pending: None,
            merge_preview: String::new(),
            show_consumo_wiz: false,
            tab_t0: 0.0,
            login_t0: 0.0,
            busy: None,
            job_rx: None,
            last_diag_pdf: None,
            last_quote_pdf: None,
            filter_report: String::new(),
            show_plate_conflict: false,
            pending_scan: None,
            show_preview: false,
            preview_tex: None,
            show_anexo: false,
            anexo_items: Vec::new(),
            anexo_base: PathBuf::new(),
            anexo_export: false,
            doc_style,
            fundo_thumbs: Vec::new(),
            status_at: 0.0,
            guia_desk: Departamento::Care,
            guia_interno: false,
            guia_body: String::new(),
            dirty_fp: String::new(),
            show_dirty: false,
            pending_mode: None,
            quotes: Vec::new(),
            contas: Vec::new(),
            jobs: Vec::new(),
            skus: Vec::new(),
            quote: Quote::default(),
            conta: Quote::default(),
            job: Job::default(),
            sku: Sku::default(),
            line_desc: String::new(),
            line_eur: String::new(),
            peca_qty: "1".into(),
            line_disc: String::new(),
            peca_custo: String::new(),
            peca_margem: "40".into(),
            usd_edit: String::new(),
            mo_eur: String::new(),
            mo_qty: "1".into(),
            mo_disc: String::new(),
            ops_line_err: String::new(),
            filter_ops: String::new(),
            sku_qty: String::new(),
            diag_pdf_saved: false,
            pending_advance: None,
            desk_gate: DeskGate::Menu,
            trab_kind: TrabKind::Quote,
            filter_jobs: String::new(),
            boot_max: 45,
            login_need_focus: true,
        };
        writers::set_house_style(app.doc_style.clone());
        let _ = media::seed_design_kit();
        app.refresh_lists();
        if app.root.is_empty() {
            app.status = "Não encontrei a pasta Clientes. Instala o programa ou liga a pen Vanguarda (Office\\Clientes).".into();
            app.status_ok = false;
        } else if app
            .root
            .to_lowercase()
            .replace('/', "\\")
            .contains("office\\clientes")
        {
            app.status = format!("Pasta: {}", app.root);
            app.status_ok = true;
        } else {
            app.status = format!("Pasta local: {}", app.root);
            app.status_ok = true;
        }
        app
    }

    fn root_path(&self) -> PathBuf {
        PathBuf::from(self.root.trim())
    }

    fn refresh_lists(&mut self) {
        let root = self.root_path();
        self.client_names = list_client_names(&root);
        self.staff_names = filter_people_names(&root, list_staff_names(&root));
        self.cars = cars::list_cars(&root);
        self.saved_diags = cars::list_saved_diags(&root);
        self.pulse = cars::hub_pulse(&root);
        self.doc_style = DocStyle::load(&root);
        writers::set_house_style(self.doc_style.clone());
        let _ = guias::seed(&root);
        if self.guia_body.is_empty() {
            self.load_guia_body();
        }
        for n in &self.client_names {
            if scaffold::is_locked_client(n) {
                let dir = client_dir(&root, n);
                let mark = dir.join("NAO-REGRAVAR.txt");
                if !mark.exists() {
                    let _ = fs::create_dir_all(&dir);
                    let _ = fs::write(
                        &mark,
                        "DJ xona bwe — meme da Vanguarda. Nao apagar, nao editar, nao tocar.\n",
                    );
                }
            }
        }
        self.reload_ops();
    }

    fn reload_ops(&mut self) {
        let root = self.root_path();
        if root.as_os_str().is_empty() {
            return;
        }
        match db::open(&root) {
            Ok(conn) => {
                let _ = db::seed_if_empty(&conn, &[], &ops::stardust_seed());
                self.quotes = db::list_quotes(&conn).unwrap_or_default();
                self.contas = db::list_contas(&conn).unwrap_or_default();
                self.jobs = db::list_jobs(&conn).unwrap_or_default();
                self.skus = db::list_sku(&conn).unwrap_or_default();
            }
            Err(_) => {}
        }
    }

    fn ops_dir(&self) -> PathBuf {
        crate::paths::interno_root(&self.root_path()).join("Operacao")
    }

    fn dept(&self) -> Option<Departamento> {
        self.session.as_ref().map(|u| u.departamento)
    }

    fn is_admin(&self) -> bool {
        self.session.as_ref().map(|u| u.is_admin()).unwrap_or(false)
    }

    fn is_software(&self) -> bool {
        self.session.as_ref().map(|u| u.is_software()).unwrap_or(false)
    }

    fn ala(&self) -> Option<Ala> {
        self.session.as_ref().map(|u| u.ala())
    }

    /// Shop tabs follow the *mesa*. Admin/software overlay adds Sistema (and
    /// Pessoas edit). A software-only login is not a Front/Back desk.
    fn can(&self, m: Mode) -> bool {
        if self.session.is_none() {
            return false;
        }
        if m == Mode::Sistema {
            return self.is_admin();
        }
        if self.is_software() {
            return matches!(
                m,
                Mode::Inicio | Mode::Staff | Mode::Guias | Mode::Documentos | Mode::Sistema
            );
        }
        match self.dept() {
            Some(Departamento::Design) => matches!(
                m,
                Mode::Inicio
                    | Mode::Cliente
                    | Mode::Carro
                    | Mode::Staff
                    | Mode::Diagnostico
                    | Mode::Guias
                    | Mode::Documentos
                    | Mode::Trabalho
                    | Mode::Agenda
                    | Mode::Stock
            ),
            Some(Departamento::Escritorio) => matches!(
                m,
                Mode::Inicio
                    | Mode::Cliente
                    | Mode::Carro
                    | Mode::Staff
                    | Mode::Diagnostico
                    | Mode::Guias
                    | Mode::Trabalho
                    | Mode::Agenda
                    | Mode::Stock
            ),
            Some(Departamento::Oficina) => matches!(
                m,
                Mode::Inicio
                    | Mode::Cliente
                    | Mode::Carro
                    | Mode::Diagnostico
                    | Mode::Guias
                    | Mode::Trabalho
                    | Mode::Agenda
                    | Mode::Stock
            ),
            Some(Departamento::Care) => matches!(
                m,
                Mode::Inicio
                    | Mode::Cliente
                    | Mode::Carro
                    | Mode::Diagnostico
                    | Mode::Care
                    | Mode::Guias
                    | Mode::Trabalho
                    | Mode::Agenda
            ),
            Some(Departamento::Interiores) => matches!(
                m,
                Mode::Inicio
                    | Mode::Cliente
                    | Mode::Carro
                    | Mode::Diagnostico
                    | Mode::Interiores
                    | Mode::Guias
                    | Mode::Trabalho
                    | Mode::Agenda
            ),
            None => false,
        }
    }

    fn can_add_client(&self) -> bool {
        self.is_software()
            || matches!(self.dept(), Some(Departamento::Escritorio | Departamento::Design))
    }

    fn can_backup(&self) -> bool {
        self.is_admin()
    }

    fn can_edit_pessoa(&self) -> bool {
        self.is_software()
            || matches!(self.dept(), Some(Departamento::Escritorio | Departamento::Design))
    }

    fn can_edit_relatorio(&self) -> bool {
        self.can(Mode::Diagnostico) && !self.is_software()
    }

    fn can_edit_saude(&self) -> bool {
        self.can_edit_relatorio()
    }

    fn can_edit_data(&self) -> bool {
        self.is_admin()
    }

    fn can_edit_documentos(&self) -> bool {
        self.is_software() || matches!(self.dept(), Some(Departamento::Design))
    }

    fn can_edit_care(&self) -> bool {
        matches!(self.dept(), Some(Departamento::Care))
    }

    fn can_edit_interiores(&self) -> bool {
        matches!(self.dept(), Some(Departamento::Interiores))
    }

    fn can_edit_ops(&self) -> bool {
        self.is_software()
            || matches!(
                self.dept(),
                Some(Departamento::Escritorio | Departamento::Design)
            )
    }

    fn can_pedir_pessoa(&self) -> bool {
        self.is_admin()
            || matches!(self.dept(), Some(Departamento::Escritorio | Departamento::Design))
    }

    fn can_decidir_pessoa(&self) -> bool {
        self.is_admin()
    }

    fn pessoa_registo(&self) -> Option<auth::User> {
        let n = self.selected_staff.trim();
        if n.is_empty() {
            return None;
        }
        auth::load_users(&self.root_path())
            .into_iter()
            .find(|u| u.nome == n && !u.software)
    }

    fn can_edit_staff_ficha(&self) -> bool {
        if self.is_admin() {
            return true;
        }
        if !self.can_pedir_pessoa() {
            return false;
        }
        match self.pessoa_registo() {
            None => true,
            Some(u) => u.is_pedido() || u.is_recusado(),
        }
    }

    fn can_edit_guia(&self, interno: bool) -> bool {
        if self.is_software() {
            return true;
        }
        if interno {
            matches!(self.dept(), Some(Departamento::Design))
        } else {
            match self.dept() {
                Some(Departamento::Design | Departamento::Escritorio) => {
                    matches!(
                        self.guia_desk,
                        Departamento::Design | Departamento::Escritorio
                    )
                }
                Some(d) => d == self.guia_desk,
                None => false,
            }
        }
    }

    fn load_guia_body(&mut self) {
        self.guia_body = guias::load(&self.root_path(), self.guia_desk, self.guia_interno);
    }

    fn set_own_mesa(&mut self, mesa: Departamento) {
        if self.is_software() {
            return;
        }
        let Some(u) = self.session.clone() else {
            return;
        };
        match auth::set_user(
            &self.root_path(),
            &u.nome,
            mesa,
            u.admin,
            u.activo,
        ) {
            Ok(()) => {
                if let Some(s) = self.session.as_mut() {
                    s.departamento = mesa;
                }
                self.guia_desk = mesa;
                self.load_guia_body();
                if !self.can(self.mode) {
                    self.mode = Mode::Inicio;
                }
                self.set_status(true, format!("Mesa do dia: {}", mesa.label()));
            }
            Err(e) => self.set_status(false, format!("{e:#}")),
        }
    }

    fn bind_pdf_style(&self) {
        writers::set_house_style(self.doc_style.clone());
    }

    fn client_locked(&self) -> bool {
        scaffold::is_locked_client(&self.client.nome_completo)
            || scaffold::is_locked_client(&self.selected_client)
            || scaffold::is_protected(&client_dir(&self.root_path(), &self.selected_client))
    }

    fn set_status(&mut self, ok: bool, msg: impl Into<String>) {
        self.status_ok = ok;
        self.status = msg.into();
        self.status_at = 0.0;
    }

    fn fingerprint(&self) -> String {
        match self.mode {
            Mode::Cliente => serde_json::to_string(&self.client).unwrap_or_default(),
            Mode::Carro | Mode::Care | Mode::Interiores => {
                serde_json::to_string(&self.car).unwrap_or_default()
            }
            Mode::Diagnostico => format!(
                "{}|{}",
                serde_json::to_string(&self.scan).unwrap_or_default(),
                serde_json::to_string(&self.car).unwrap_or_default()
            ),
            Mode::Guias => self.guia_body.clone(),
            Mode::Sistema => serde_json::to_string(&self.staff).unwrap_or_default(),
            Mode::Trabalho => format!(
                "{}|{}|{}",
                serde_json::to_string(&self.quote).unwrap_or_default(),
                serde_json::to_string(&self.conta).unwrap_or_default(),
                serde_json::to_string(&self.job).unwrap_or_default()
            ),
            Mode::Stock => serde_json::to_string(&self.sku).unwrap_or_default(),
            _ => String::new(),
        }
    }

    fn mark_clean(&mut self) {
        self.dirty_fp = self.fingerprint();
    }

    fn is_dirty(&self) -> bool {
        match self.mode {
            Mode::Inicio | Mode::Documentos | Mode::Agenda => false,
            _ => !self.dirty_fp.is_empty() && self.fingerprint() != self.dirty_fp,
        }
    }

    fn save_current_tab(&mut self) {
        match self.mode {
            Mode::Cliente => self.save_client(),
            Mode::Carro | Mode::Care => self.save_carro(),
            Mode::Diagnostico => self.save_vanguarda_diag(),
            Mode::Guias => {
                if self.can_edit_guia(self.guia_interno) {
                    match guias::save(
                        &self.root_path(),
                        self.guia_desk,
                        self.guia_interno,
                        &self.guia_body,
                    ) {
                        Ok(p) => {
                            self.set_status(true, format!("Guia: {}", p.display()));
                            self.mark_clean();
                        }
                        Err(e) => self.set_status(false, format!("{e:#}")),
                    }
                }
            }
            Mode::Sistema => self.save_staff(),
            Mode::Interiores => self.save_carro(),
            Mode::Staff => self.save_staff(),
            Mode::Trabalho => {
                let _ = self.save_quote();
                let _ = self.save_conta();
                let _ = self.save_job();
            }
            Mode::Stock => {
                let _ = self.save_sku_row();
            }
            _ => {}
        }
    }

    fn apply_pending_mode(&mut self) {
        if let Some(m) = self.pending_mode.take() {
            self.mode = m;
            self.refresh_lists();
            self.tab_t0 = 0.0;
            self.desk_gate = DeskGate::Menu;
            self.mark_clean();
        }
        self.show_dirty = false;
    }

    fn ui_dirty_dialog(&mut self, ctx: &egui::Context) {
        let mut open = self.show_dirty;
        egui::Window::new("Por gravar")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Há alterações neste separador.");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if gold_button(ui, "Gravar", [110.0, 30.0]) {
                        self.save_current_tab();
                        self.apply_pending_mode();
                    }
                    if steel_button(ui, "Descartar", [110.0, 30.0]) {
                        self.apply_pending_mode();
                    }
                    if steel_button(ui, "Ficar", [110.0, 30.0]) {
                        self.pending_mode = None;
                        self.show_dirty = false;
                    }
                });
            });
        if !open {
            self.pending_mode = None;
            self.show_dirty = false;
        }
    }

    fn start_job(&mut self, kind: BusyKind, rx: mpsc::Receiver<JobOut>) {
        self.busy = Some(kind);
        self.job_rx = Some(rx);
    }

    fn poll_jobs(&mut self) {
        let Some(rx) = &self.job_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(msg) => {
                self.job_rx = None;
                self.busy = None;
                match msg {
                    JobOut::Scan(Ok(scan)) => {
                        self.diag_pdf_saved = false;
                        self.apply_parsed_scan(scan);
                    }
                    JobOut::Scan(Err(e)) => self.set_status(false, format!("Não li o diagnóstico: {e}")),
                    JobOut::Cleared(Ok(msg)) => {
                        self.set_status(true, format!("Códigos apagados no adaptador ({msg})."));
                    }
                    JobOut::Cleared(Err(e)) => self.set_status(false, format!("Não apaguei: {e}")),
                    JobOut::Saved { pdf, car } => match pdf {
                        Ok(p) => {
                            if let Some(c) = car {
                                self.car = c;
                                self.selected_car = self.car.label();
                            }
                            self.last_diag_pdf = Some(p.clone());
                            self.diag_pdf_saved = true;
                            self.refresh_lists();
                            self.mark_clean();
                            self.set_status(
                                true,
                                format!("Guardado {} ({})", p.display(), self.diag_client),
                            );
                        }
                        Err(e) => self.set_status(false, format!("Não gravei: {e}")),
                    },
                    JobOut::Lists {
                        client_names,
                        staff_names,
                        cars,
                        saved_diags,
                        pulse,
                    } => {
                        self.client_names = client_names;
                        self.staff_names = staff_names;
                        self.cars = cars;
                        self.saved_diags = saved_diags;
                        self.pulse = pulse;
                        for n in &self.client_names {
                            if scaffold::is_locked_client(n) {
                                let dir = client_dir(&self.root_path(), n);
                                let mark = dir.join("NAO-REGRAVAR.txt");
                                if !mark.exists() {
                                    let _ = fs::create_dir_all(&dir);
                                    let _ = fs::write(
                                        &mark,
                                        "DJ xona bwe — meme da Vanguarda. Nao apagar, nao editar, nao tocar.\n",
                                    );
                                }
                            }
                        }
                        self.set_status(true, "Listas actualizadas.");
                    }
                }
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.job_rx = None;
                self.busy = None;
                self.set_status(false, "A tarefa parou a meio.");
            }
        }
    }

    fn apply_parsed_scan(&mut self, mut scan: Scan) {
        scan.dtcs = diag_parse::enrich_dtcs(scan.dtcs);
        if self.car_can_persist(&self.car) {
            let a = diag_parse::norm_id(&self.car.matricula);
            let b = diag_parse::norm_id(&scan.matricula);
            if !a.is_empty() && !b.is_empty() && a != b {
                self.pending_scan = Some(scan);
                self.show_plate_conflict = true;
                return;
            }
            self.commit_scan_keep_car(scan);
        } else {
            self.commit_scan_find_car(scan);
        }
        if self.mode == Mode::Diagnostico {
            self.desk_gate = DeskGate::Form;
        }
    }

    fn commit_scan_keep_car(&mut self, mut scan: Scan) {
        let keep_plate = self.car.matricula.clone();
        let keep_vin = self.car.vin.clone();
        cars::apply_car_to_scan(&self.car, &mut scan);
        if !keep_plate.trim().is_empty() {
            scan.matricula = keep_plate;
        }
        if !keep_vin.trim().is_empty() {
            scan.vin = keep_vin;
        }
        self.scan = scan;
        self.diag_client = self.car.dono.clone();
        self.set_status(
            true,
            format!(
                "Lido {} ({} códigos) na viatura {}.",
                self.scan.source_name,
                self.scan.dtcs.len(),
                self.car.label()
            ),
        );
    }

    fn commit_scan_find_car(&mut self, scan: Scan) {
        self.scan = scan;
        if let Some(car) = cars::find_car(&self.root_path(), &self.scan.vin, &self.scan.matricula)
        {
            cars::apply_car_to_scan(&car, &mut self.scan);
            self.diag_client = car.dono.clone();
            self.car = car.clone();
            self.selected_car = car.label();
            self.set_status(
                true,
                format!(
                    "Lido {} ({} códigos). Viatura: {}.",
                    self.scan.source_name,
                    self.scan.dtcs.len(),
                    car.label()
                ),
            );
        } else if let Some(name) = self.suggest_diag_client() {
            self.diag_client = name;
            self.set_status(
                true,
                format!(
                    "Lido {} ({} códigos). Cliente sugerido: {}.",
                    self.scan.source_name,
                    self.scan.dtcs.len(),
                    self.diag_client
                ),
            );
        } else {
            self.set_status(
                true,
                format!(
                    "Lido {} ({} códigos). Escolhe a viatura ou o cliente.",
                    self.scan.source_name,
                    self.scan.dtcs.len()
                ),
            );
        }
    }

    fn fill_scan_from_car(&mut self) {
        if !self.car_can_persist(&self.car) {
            return;
        }
        cars::apply_car_to_scan(&self.car, &mut self.scan);
        if self.diag_client.trim().is_empty() {
            self.diag_client = self.car.dono.clone();
        }
        if self.scan.data.trim().is_empty() {
            self.scan.data = today();
        }
        if self.scan.titulo.trim().is_empty() {
            self.scan.titulo = diag_parse::default_titulo(&self.scan);
        }
    }

    fn open_report(&mut self, car: Carro) {
        self.car = car;
        self.car.ensure_sistemas();
        cars::tidy_car(&mut self.car);
        self.selected_car = self.car.label();
        self.fill_scan_from_car();
        self.mode = Mode::Diagnostico;
        self.desk_gate = DeskGate::Form;
        self.mark_clean();
    }

    fn load_report_car(&mut self, car: Carro) {
        self.open_report(car);
        self.desk_gate = DeskGate::Form;
        self.set_status(true, format!("Relatório: {}", self.car.label()));
    }

    fn begin_orcamento_from_diag(&mut self) {
        if !self.can_edit_ops() {
            self.set_status(false, "Este departamento não edita orçamentos.");
            return;
        }
        if !self.car_can_persist(&self.car) {
            self.set_status(false, "Escolhe a viatura primeiro.");
            return;
        }
        self.reset_quote_draft();
        self.quote.cliente = if !self.diag_client.trim().is_empty() {
            self.diag_client.clone()
        } else {
            self.car.dono.clone()
        };
        self.quote.matricula = self.car.matricula.clone();
        self.quote.vin = self.car.vin.clone();
        self.quote.tipo = "diagnostico".into();
        self.quote.notas = crate::diag_parse::notes_from_scan(&self.scan);
        self.mode = Mode::Trabalho;
        self.trab_kind = TrabKind::Quote;
        self.desk_gate = DeskGate::Form;
        self.set_status(true, "Orçamento desta visita — confirma peças e mão de obra.");
    }

    fn begin_preview(&mut self, ctx: &egui::Context) {
        self.bind_pdf_style();
        self.fill_scan_from_car();
        if self.diag_client.trim().is_empty() && !self.car.dono.trim().is_empty() {
            self.diag_client = self.car.dono.clone();
        }
        if self.diag_client.trim().is_empty()
            && self.scan.vin.is_empty()
            && self.scan.dtcs.is_empty()
        {
            self.set_status(false, "Escolhe uma viatura ou importa um Autocom.");
            return;
        }
        let tmp = std::env::temp_dir().join("vanguarda-preview.pdf");
        match writers::write_diag_pdf_with_photos(
            &tmp,
            &self.scan,
            &self.diag_client,
            &self.car.sistemas,
            &[],
        ) {
            Ok(()) => match diag_parse::render_pdf_first_page_jpeg(&tmp) {
                Ok(jpeg) => match load_texture_bytes(ctx, &jpeg, "pdf-preview") {
                    Some(tex) => {
                        self.preview_tex = Some(tex);
                        self.show_preview = true;
                        self.set_status(true, "Pré-visualização da 1.ª página.");
                    }
                    None => self.set_status(false, "Não desenhei a pré-visualização."),
                },
                Err(e) => self.set_status(false, format!("Preview: {e:#}")),
            },
            Err(e) => self.set_status(false, format!("Não gerei o PDF: {e:#}")),
        }
    }

    fn begin_read_autocom(&mut self, path: PathBuf) {
        if self.busy.is_some() || !self.can_edit_relatorio() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.start_job(BusyKind::LerAutocom, rx);
        std::thread::spawn(move || {
            let r = diag_parse::parse_diag_file(&path).map_err(|e| format!("{e:#}"));
            let _ = tx.send(JobOut::Scan(r));
        });
    }

    fn begin_read_vlinker(&mut self, mock: bool) {
        if self.busy.is_some() || !self.can_edit_relatorio() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.start_job(BusyKind::LerAdaptador, rx);
        std::thread::spawn(move || {
            let r = (|| {
                let out = if mock {
                    vanguarda_diagnostico::scan_mock()
                } else {
                    vanguarda_diagnostico::scan_live(None)
                }
                .map_err(|e| format!("{e:#}"))?;
                let v = serde_json::to_value(&out).map_err(|e| format!("{e:#}"))?;
                serde_json::from_value::<Scan>(v).map_err(|e| format!("{e:#}"))
            })();
            let _ = tx.send(JobOut::Scan(r));
        });
    }

    fn begin_clear_dtcs(&mut self) {
        if self.busy.is_some() || !self.can_edit_relatorio() {
            return;
        }
        if !self.diag_pdf_saved {
            self.set_status(false, "Grava o PDF da casa antes de apagar códigos.");
            return;
        }
        if !self.diag_source_live() {
            self.set_status(false, "Apagar no adaptador só para leitura vLinker/live.");
            return;
        }
        let mock = self.scan.source_name.eq_ignore_ascii_case("mock")
            || self.scan.source_name.to_lowercase().contains("simulado");
        let (tx, rx) = mpsc::channel();
        self.start_job(BusyKind::ApagarCodigos, rx);
        std::thread::spawn(move || {
            let r = (|| {
                if mock {
                    let mut link = vanguarda_diagnostico::MockLink::default();
                    vanguarda_diagnostico::clear_dtcs(&mut link)
                } else {
                    let mut link = vanguarda_diagnostico::SerialLink::open(None)?;
                    vanguarda_diagnostico::clear_dtcs(&mut link)
                }
            })()
            .map_err(|e| format!("{e:#}"));
            let _ = tx.send(JobOut::Cleared(r));
        });
    }

    fn begin_refresh_async(&mut self) {
        if self.busy.is_some() {
            return;
        }
        let root = self.root_path();
        let (tx, rx) = mpsc::channel();
        self.start_job(BusyKind::Actualizar, rx);
        std::thread::spawn(move || {
            let client_names = list_client_names(&root);
            let staff_names = filter_people_names(&root, list_staff_names(&root));
            let cars = cars::list_cars(&root);
            let saved_diags = cars::list_saved_diags(&root);
            let pulse = cars::hub_pulse(&root);
            let _ = tx.send(JobOut::Lists {
                client_names,
                staff_names,
                cars,
                saved_diags,
                pulse,
            });
        });
    }

    fn export_diag_pdf(&mut self) {
        let Some(p) = self.last_diag_pdf.clone() else {
            self.set_status(false, "Grava o PDF da casa primeiro.");
            return;
        };
        if !p.is_file() {
            self.set_status(false, "Não encontro o PDF. Grava outra vez.");
            return;
        }
        self.finish_pdf_out(&p, true);
    }

    fn open_last_diag_pdf(&mut self) {
        let Some(p) = self.last_diag_pdf.clone() else {
            self.set_status(false, "Ainda não há um PDF deste relatório. Grava primeiro.");
            return;
        };
        if !p.is_file() {
            self.set_status(false, "Não encontro o PDF. Grava outra vez.");
            return;
        }
        self.begin_pdf_out(p, false);
    }

    fn anexo_roots(&self) -> Vec<(String, PathBuf)> {
        let mut roots = Vec::new();
        match self.mode {
            Mode::Diagnostico => {
                if self.car_can_persist(&self.car) {
                    roots.push(("Viatura".into(), cars::car_dir(&self.root_path(), &self.car)));
                }
                let n = self.diag_client.trim();
                if !n.is_empty() {
                    roots.push(("Cliente".into(), client_dir(&self.root_path(), n)));
                }
            }
            Mode::Carro => {
                if self.car_can_persist(&self.car) {
                    roots.push((String::new(), cars::car_dir(&self.root_path(), &self.car)));
                }
            }
            Mode::Cliente | Mode::Staff => {
                if let Some(d) = self.current_person_dir() {
                    roots.push((String::new(), d));
                }
            }
            Mode::Inicio | Mode::Documentos | Mode::Sistema | Mode::Guias | Mode::Care | Mode::Interiores | Mode::Trabalho | Mode::Agenda | Mode::Stock => {}
        }
        roots
    }

    fn begin_pdf_out(&mut self, base: PathBuf, export: bool) {
        let roots: Vec<(String, PathBuf)> = self.anexo_roots();
        let refs: Vec<(&str, PathBuf)> = roots
            .iter()
            .map(|(p, d)| (p.as_str(), d.clone()))
            .collect();
        let mut items: Vec<(String, PathBuf, bool)> = media::printable_attachments(&refs)
            .into_iter()
            .map(|(l, p)| (l, p, false))
            .collect();
        items.truncate(8);
        if items.is_empty() {
            self.finish_pdf_out(&base, export);
            return;
        }
        self.anexo_base = base;
        self.anexo_export = export;
        self.anexo_items = items;
        self.show_anexo = true;
    }

    fn finish_pdf_out(&mut self, pdf: &Path, export: bool) {
        if export {
            let name = pdf
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Vanguarda.pdf".into());
            let dest = rfd::FileDialog::new()
                .set_file_name(&name)
                .add_filter("PDF", &["pdf"])
                .save_file();
            let Some(dest) = dest else {
                return;
            };
            match fs::copy(pdf, &dest) {
                Ok(_) => self.set_status(true, format!("Exportado: {}", dest.display())),
                Err(e) => self.set_status(false, format!("Exportar falhou: {e}")),
            }
        } else {
            match std::process::Command::new("cmd")
                .args(["/C", "start", "", &pdf.display().to_string()])
                .spawn()
            {
                Ok(_) => self.set_status(true, format!("A abrir {}", pdf.display())),
                Err(e) => self.set_status(false, format!("Não abri o PDF: {e}")),
            }
        }
    }

    fn apply_anexos_and_out(&mut self) {
        let chosen: Vec<(String, PathBuf)> = self
            .anexo_items
            .iter()
            .filter(|(_, _, on)| *on)
            .map(|(l, p, _)| (l.clone(), p.clone()))
            .collect();
        let base = self.anexo_base.clone();
        let export = self.anexo_export;
        self.show_anexo = false;
        if chosen.is_empty() {
            self.finish_pdf_out(&base, export);
            return;
        }
        let stem = base
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Vanguarda".into());
        let dest = if stem.ends_with("_com_anexos") {
            base.clone()
        } else {
            base.with_file_name(format!("{stem}_com_anexos.pdf"))
        };
        self.bind_pdf_style();
        let printed = match self.mode {
            Mode::Diagnostico => writers::write_diag_pdf_with_photos(
                &dest,
                &self.scan,
                &self.diag_client,
                &self.car.sistemas,
                &chosen,
            ),
            Mode::Cliente => {
                let dir = self.current_person_dir().unwrap_or_else(|| self.root_path());
                writers::client_pdf_with_photos(&dest, &self.client, &dir, &chosen)
            }
            Mode::Carro => {
                let dir = cars::car_dir(&self.root_path(), &self.car);
                writers::carro_pdf_with_photos(&dest, &self.car, &dir, &chosen)
            }
            Mode::Staff => {
                let dir = self.current_person_dir().unwrap_or_else(|| self.root_path());
                writers::staff_pdf_with_photos(&dest, &self.staff, &dir, &chosen)
            }
            Mode::Inicio | Mode::Documentos | Mode::Sistema | Mode::Guias | Mode::Care | Mode::Interiores | Mode::Trabalho | Mode::Agenda | Mode::Stock => {
                self.set_status(false, "Neste separador não há PDF com anexos.");
                return;
            }
        };
        match printed {
            Ok(()) => {
                self.last_diag_pdf = Some(dest.clone());
                self.finish_pdf_out(&dest, export);
            }
            Err(e) => {
                self.set_status(false, format!("Não juntei anexos: {e:#}"));
                self.finish_pdf_out(&base, export);
            }
        }
    }

    fn ui_plate_conflict(&mut self, ctx: &egui::Context) {
        let mut open = self.show_plate_conflict;
        let car_p = self.car.matricula.clone();
        let scan_p = self
            .pending_scan
            .as_ref()
            .map(|s| s.matricula.clone())
            .unwrap_or_default();
        egui::Window::new("Matrícula diferente")
            .open(&mut open)
            .collapsible(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("O Autocom não coincide com a viatura escolhida.")
                        .color(GOLD),
                );
                ui.label(format!("Viatura: {car_p}"));
                ui.label(format!("Autocom: {scan_p}"));
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    if gold_button(ui, "Manter a viatura", [180.0, 28.0]) {
                        if let Some(scan) = self.pending_scan.take() {
                            self.commit_scan_keep_car(scan);
                        }
                        self.show_plate_conflict = false;
                    }
                    if steel_button(ui, "Usar o Autocom", [180.0, 28.0]) {
                        if let Some(scan) = self.pending_scan.take() {
                            self.commit_scan_find_car(scan);
                        }
                        self.show_plate_conflict = false;
                    }
                });
            });
        self.show_plate_conflict = open && self.show_plate_conflict;
    }

    fn ui_preview(&mut self, ctx: &egui::Context) {
        let mut open = self.show_preview;
        egui::Window::new("Pré-visualizar PDF")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(480.0)
            .default_height(640.0)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(format!(
                        "{} — 1.ª página",
                        self.doc_style.label()
                    ))
                    .color(GOLD)
                    .family(fonts::heading_family())
                    .size(16.0),
                );
                ui.add_space(6.0);
                if let Some(tex) = &self.preview_tex {
                    let w = ui.available_width().min(440.0);
                    let size = tex.size_vec2();
                    let h = if size.x > 1.0 {
                        w * (size.y / size.x)
                    } else {
                        560.0
                    };
                    ui.add(egui::Image::new(tex).fit_to_exact_size(Vec2::new(w, h)));
                } else {
                    ui.label("Sem imagem.");
                }
                ui.add_space(8.0);
                if gold_button(ui, "Fechar", [100.0, 28.0]) {
                    self.show_preview = false;
                }
            });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            open = false;
        }
        self.show_preview = open && self.show_preview;
    }

    fn ui_anexo_picker(&mut self, ctx: &egui::Context) {
        let mut open = self.show_anexo;
        let export = self.anexo_export;
        egui::Window::new("Anexar ao PDF?")
            .open(&mut open)
            .collapsible(false)
            .default_width(460.0)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(
                        "Queres meter alguma foto ou recibo neste PDF? O relatório original fica como está.",
                    )
                    .color(GOLD),
                );
                ui.add_space(8.0);
                for (label, _, on) in &mut self.anexo_items {
                    ui.checkbox(on, label.as_str());
                }
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    if steel_button(ui, "Continuar sem anexos", [200.0, 28.0]) {
                        let base = self.anexo_base.clone();
                        self.show_anexo = false;
                        self.finish_pdf_out(&base, export);
                    }
                    let go = if export {
                        "Incluir e exportar"
                    } else {
                        "Incluir e abrir"
                    };
                    if gold_button(ui, go, [200.0, 28.0]) {
                        self.apply_anexos_and_out();
                    }
                });
            });
        self.show_anexo = open && self.show_anexo;
    }

    fn take_diag_drops(&mut self, ctx: &egui::Context) {
        if self.mode != Mode::Diagnostico || self.busy.is_some() || !self.can_edit_relatorio() {
            return;
        }
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for f in dropped {
            let Some(path) = f.path else {
                continue;
            };
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if ext == "pdf" || ext == "txt" || ext == "log" {
                self.begin_read_autocom(path);
                break;
            }
        }
    }

    fn paint_busy(&self, ctx: &egui::Context, kind: BusyKind) {
        let now = ctx.input(|i| i.time);
        ctx.request_repaint();
        egui::Area::new(egui::Id::new("v-busy"))
            .fixed_pos(Pos2::ZERO)
            .order(egui::Order::Foreground)
            .interactable(true)
            .show(ctx, |ui| {
                let rect = ctx.screen_rect();
                ui.allocate_rect(rect, Sense::click_and_drag());
                let painter = ui.painter();
                painter.rect_filled(
                    rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(10, 8, 6, 210),
                );
                let c = rect.center();
                let tau = std::f64::consts::TAU;
                for i in 0..3 {
                    let a = now * tau / 1.5 + i as f64 * tau / 3.0;
                    let p = c
                        + Vec2::new(
                            (a.cos() as f32) * 22.0,
                            (a.sin() as f32) * 22.0 - 12.0,
                        );
                    painter.circle_filled(p, 4.0, GOLD);
                }
                let pulse = (0.55 + 0.45 * (now * 2.4).sin()) as f32;
                painter.circle_stroke(
                    c + Vec2::new(0.0, -12.0),
                    26.0 + 6.0 * pulse,
                    egui::Stroke::new(
                        1.4_f32,
                        Color32::from_rgba_unmultiplied(0xD4, 0xB0, 0x6A, (140.0 + 80.0 * pulse) as u8),
                    ),
                );
                painter.text(
                    c + Vec2::new(0.0, 28.0),
                    egui::Align2::CENTER_CENTER,
                    kind.label(),
                    egui::FontId::proportional(16.0),
                    GOLD,
                );
            });
    }

    fn load_client(&mut self, nome: &str) {
        self.selected_client = nome.to_string();
        let path = client_dir(&self.root_path(), nome).join("ficha.json");
        self.client = if let Ok(s) = fs::read_to_string(&path) {
            serde_json::from_str(&s).unwrap_or_else(|_| ClientFicha::blank(nome))
        } else {
            ClientFicha::blank(nome)
        };
        if let Ok(key) = vault::load_or_create_key(&self.root_path()) {
            vault::reveal_client(&key, &mut self.client);
        }
        tidy_client_placeholders(&mut self.client);
        self.apply_saldo_from_jobs();
        self.mark_clean();
    }

    fn apply_saldo_from_jobs(&mut self) {
        let cents = ops::saldo_cents(&self.jobs, &self.client.nome_completo);
        self.client.saldo_aberto = ops::euro(cents);
    }

    fn who(&self) -> String {
        self.session
            .as_ref()
            .map(|u| u.nome.clone())
            .unwrap_or_else(|| "—".into())
    }

    fn warn_stale_backup(&mut self) {
        match backup::days_since(&self.root_path()) {
            None => {
                self.set_status(false, "Ainda não há backup zip desta casa. Admin: Backup zip.");
            }
            Some(d) if d >= 7 => {
                self.set_status(
                    false,
                    format!("Último backup há {d} dias. Admin: Backup zip."),
                );
            }
            Some(_) => {}
        }
    }

    fn load_staff(&mut self, nome: &str) {
        self.selected_staff = nome.to_string();
        let dir = staff_dir(&self.root_path(), nome);
        let staff_path = dir.join("staff.json");
        self.staff = if let Ok(s) = fs::read_to_string(&staff_path) {
            serde_json::from_str(&s).unwrap_or_else(|_| StaffFicha::blank(nome))
        } else {
            StaffFicha::blank(nome)
        };
        if let Ok(key) = vault::load_or_create_key(&self.root_path()) {
            vault::reveal_staff(&key, &mut self.staff);
        }
        let cons_path = dir.join("consumo.json");
        self.consumo = if let Ok(s) = fs::read_to_string(&cons_path) {
            serde_json::from_str(&s).unwrap_or_else(|_| ConsumoLog::blank(nome))
        } else {
            ConsumoLog::blank(nome)
        };
        self.mark_clean();
    }

    fn save_client(&mut self) {
        self.bind_pdf_style();
        tidy_client_placeholders(&mut self.client);
        self.client.nif = format::format_nif(&self.client.nif);
        if !format::nif_valido(&self.client.nif) {
            self.set_status(false, "NIF inválido (9 dígitos, dígito de controlo).");
            return;
        }
        self.client.telemovel = format::format_phone(&self.client.telemovel);
        self.client.data_nascimento = format::format_date(&self.client.data_nascimento);
        self.client.veiculo_matricula = format::format_plate(&self.client.veiculo_matricula);
        self.client.veiculo_vin = format::format_vin(&self.client.veiculo_vin);
        self.client.veiculo_km = format::format_km(&self.client.veiculo_km);
        if format::looks_like_dtc_dump(&self.client.veiculo_modelo) {
            let bits = diag_parse::split_veiculo(&self.client.veiculo_modelo);
            if !bits.marca.is_empty() {
                self.client.veiculo_marca = bits.marca;
            }
            self.client.veiculo_modelo = bits.modelo;
            if !bits.versao.is_empty() {
                self.client.veiculo_versao = bits.versao;
            }
            if !bits.ano.is_empty() {
                self.client.veiculo_ano = bits.ano;
            }
        }
        let nome = self.client.nome_completo.trim().to_string();
        if nome.is_empty() {
            self.set_status(false, "O nome completo é obrigatório.");
            return;
        }
        if scaffold::is_locked_client(&nome) {
            self.set_status(false, "DJ xona bwé é da Vanguarda — não se edita.");
            return;
        }
        if !self.can_add_client() && self.selected_client.is_empty() {
            self.set_status(false, "A oficina não cria clientes novos — só atribui viaturas.");
            return;
        }
        let dir = client_dir(&self.root_path(), &nome);
        if scaffold::is_protected(&dir) {
            self.set_status(
                false,
                "Pasta protegida (NAO-REGRAVAR.txt ou ficha .html). Não regravo os documentos.",
            );
            return;
        }
        if !self.can_edit_pessoa() {
            let json = dir.join("ficha.json");
            if let Ok(s) = fs::read_to_string(&json) {
                if let Ok(mut f) = serde_json::from_str::<ClientFicha>(&s) {
                    if let Ok(key) = vault::load_or_create_key(&self.root_path()) {
                        vault::reveal_client(&key, &mut f);
                    }
                    f.veiculo_marca = self.client.veiculo_marca.clone();
                    f.veiculo_modelo = self.client.veiculo_modelo.clone();
                    f.veiculo_versao = self.client.veiculo_versao.clone();
                    f.veiculo_matricula = self.client.veiculo_matricula.clone();
                    f.veiculo_ano = self.client.veiculo_ano.clone();
                    f.veiculo_cor = self.client.veiculo_cor.clone();
                    f.veiculo_vin = self.client.veiculo_vin.clone();
                    f.veiculo_km = self.client.veiculo_km.clone();
                    f.pneus = self.client.pneus.clone();
                    f.observacoes_veiculo = self.client.observacoes_veiculo.clone();
                    f.data_ultima_visita = self.client.data_ultima_visita.clone();
                    self.client = f;
                }
            }
        }
        self.apply_saldo_from_jobs();
        match writers::write_client(&dir, &self.client, WriteOpts::default()) {
            Ok(files) => {
                let _ = cars::upsert_from_client(&self.root_path(), &self.client);
                audit::append(&self.root_path(), &self.who(), "guardar-cliente", &nome);
                self.refresh_lists();
                self.selected_client = nome;
                self.set_status(
                    true,
                    format!(
                        "Guardado {} ficheiros em {}",
                        files.len(),
                        dir.display()
                    ),
                );
                self.mark_clean();
            }
            Err(e) => self.set_status(false, format!("Erro a guardar: {e:#}")),
        }
    }

    fn save_staff(&mut self) {
        if !self.can_edit_staff_ficha() {
            self.set_status(false, "Só a conta do software edita fichas de pessoas.");
            return;
        }
        self.bind_pdf_style();
        let nome = self.staff.nome.trim().to_string();
        if nome.is_empty() {
            self.set_status(false, "O nome da pessoa é obrigatório.");
            return;
        }
        let dir = staff_dir(&self.root_path(), &nome);
        match writers::write_staff(&dir, &self.staff, WriteOpts::default()) {
            Ok(files) => {
                self.refresh_lists();
                self.selected_staff = nome;
                self.set_status(
                    true,
                    format!(
                        "Ficha staff: {} ficheiros em {}",
                        files.len(),
                        dir.display()
                    ),
                );
                self.mark_clean();
            }
            Err(e) => self.set_status(false, format!("Erro a guardar: {e:#}")),
        }
    }

    fn save_consumo(&mut self) {
        let nome = self
            .session
            .as_ref()
            .map(|u| u.nome.clone())
            .filter(|n| !n.is_empty())
            .or_else(|| {
                if !self.consumo.nome.trim().is_empty() {
                    Some(self.consumo.nome.trim().to_string())
                } else if !self.selected_staff.trim().is_empty() {
                    Some(self.selected_staff.trim().to_string())
                } else {
                    None
                }
            });
        let Some(nome) = nome else {
            self.set_status(false, "Não há sessão — entra outra vez.");
            return;
        };
        if self.is_software() {
            self.set_status(
                false,
                "A conta do software não tem consumo. Entra com o login da pessoa (Gil, Rodrigo).",
            );
            return;
        }
        if self.nova_entrada.descricao.trim().is_empty()
            && self.nova_entrada.tipo.trim().is_empty()
        {
            self.set_status(false, "Preenche a descrição do consumo / reparação.");
            return;
        }
        self.consumo.nome = nome.clone();
        let mut entry = self.nova_entrada.clone();
        if entry.data.trim().is_empty() {
            entry.data = today();
        }
        self.consumo.entradas.push(entry);
        self.bind_pdf_style();
        let dir = staff_dir(&self.root_path(), &nome);
        match writers::write_consumo(&dir, &self.consumo, WriteOpts::default()) {
            Ok(files) => {
                self.nova_entrada = ConsumoEntry {
                    data: today(),
                    tipo: CONSUMO_TIPOS[0].to_string(),
                    ..Default::default()
                };
                self.set_status(
                    true,
                    format!(
                        "Consumo actualizado ({} linhas, {} ficheiros) em {}",
                        self.consumo.entradas.len(),
                        files.len(),
                        dir.display()
                    ),
                );
            }
            Err(e) => {
                self.consumo.entradas.pop();
                self.set_status(false, format!("Erro a guardar: {e:#}"));
            }
        }
    }

    fn backup_zip(&mut self) {
        let root = self.root_path();
        if !root.is_dir() {
            self.set_status(false, "Pasta de clientes inválida.");
            return;
        }
        let name = backup::default_backup_name();
        let dir = backup::default_dir();
        let _ = fs::create_dir_all(&dir);
        let dest = rfd::FileDialog::new()
            .set_directory(&dir)
            .set_file_name(&name)
            .add_filter("Zip", &["zip"])
            .save_file();
        let Some(dest) = dest else {
            return;
        };
        match backup::zip_clientes(&root, &dest) {
            Ok(n) => {
                backup::mark_done(&root);
                audit::append(&root, &self.who(), "backup", &dest.display().to_string());
                self.set_status(
                    true,
                    format!("Cópia: {n} ficheiros → {}", dest.display()),
                );
            }
            Err(e) => self.set_status(false, format!("Backup falhou: {e:#}")),
        }
    }

    fn current_person_dir(&self) -> Option<PathBuf> {
        match self.mode {
            Mode::Inicio => None,
            Mode::Carro => {
                if self.car.dono.trim().is_empty() {
                    None
                } else if self.car.matricula.trim().is_empty() && self.car.vin.trim().is_empty() {
                    None
                } else {
                    Some(cars::car_dir(&self.root_path(), &self.car))
                }
            }
            Mode::Cliente => {
                let n = if !self.client.nome_completo.trim().is_empty() {
                    self.client.nome_completo.trim().to_string()
                } else {
                    self.selected_client.clone()
                };
                if n.is_empty() {
                    None
                } else {
                    Some(client_dir(&self.root_path(), &n))
                }
            }
            Mode::Diagnostico => {
                if self.car_can_persist(&self.car) {
                    Some(cars::car_dir(&self.root_path(), &self.car))
                } else {
                    let n = self.diag_client.trim();
                    if n.is_empty() {
                        None
                    } else {
                        Some(client_dir(&self.root_path(), n))
                    }
                }
            }
            Mode::Staff => {
                let n = if !self.staff.nome.trim().is_empty() {
                    self.staff.nome.trim().to_string()
                } else if !self.consumo.nome.trim().is_empty() {
                    self.consumo.nome.trim().to_string()
                } else {
                    self.selected_staff.clone()
                };
                if n.is_empty() {
                    None
                } else {
                    Some(staff_dir(&self.root_path(), &n))
                }
            }
            Mode::Documentos | Mode::Sistema | Mode::Guias | Mode::Care | Mode::Interiores | Mode::Trabalho | Mode::Agenda | Mode::Stock => None,
        }
    }

    fn attach(&mut self, sub: &str) {
        let allowed = match self.mode {
            Mode::Cliente => self.can_edit_pessoa(),
            Mode::Diagnostico => self.can_edit_relatorio(),
            _ => true,
        };
        if !allowed {
            self.set_status(false, "Este departamento só lê media.");
            return;
        }
        let Some(dir) = self.current_person_dir() else {
            self.set_status(false, "Escolhe ou cria a pessoa primeiro.");
            return;
        };
        match self.mode {
            Mode::Cliente => {
                let _ = media::ensure_client_media(&dir);
            }
            Mode::Diagnostico => {
                if self.car_can_persist(&self.car) {
                    let _ = media::ensure_car_media(&dir);
                } else {
                    let _ = media::ensure_client_media(&dir);
                }
            }
            Mode::Carro => {
                let _ = media::ensure_car_media(&dir);
            }
            Mode::Staff => {
                let _ = media::ensure_staff_media(&dir);
            }
            Mode::Inicio | Mode::Documentos | Mode::Sistema | Mode::Guias | Mode::Care | Mode::Interiores | Mode::Trabalho | Mode::Agenda | Mode::Stock => {}
        }
        let files = rfd::FileDialog::new()
            .add_filter(
                "Fotos e documentos",
                &["jpg", "jpeg", "png", "webp", "heic", "bmp", "pdf"],
            )
            .pick_files();
        let Some(files) = files else {
            return;
        };
        match media::attach_into(&dir, sub, &files) {
            Ok(copied) => self.set_status(
                true,
                format!(
                    "Copiados {} ficheiros para Media\\{sub} — Guardar actualiza a ficha.",
                    copied.len()
                ),
            ),
            Err(e) => self.set_status(false, format!("Erro a anexar: {e:#}")),
        }
    }

    fn open_folder(&mut self) {
        let Some(dir) = self.current_person_dir() else {
            self.set_status(false, "Escolhe ou cria a pessoa primeiro.");
            return;
        };
        let _ = fs::create_dir_all(&dir);
        match std::process::Command::new("explorer").arg(&dir).spawn() {
            Ok(_) => self.set_status(true, format!("Pasta: {}", dir.display())),
            Err(e) => self.set_status(false, format!("Não abri o Explorer: {e}")),
        }
    }

    fn ui_media_bar(&mut self, ui: &mut egui::Ui, photo_sub: &str, receipt_sub: Option<&str>) {
        let can_attach = match self.mode {
            Mode::Cliente => self.can_edit_pessoa(),
            Mode::Diagnostico => self.can_edit_relatorio(),
            _ => true,
        };
        ui.horizontal(|ui| {
            if can_attach {
                if steel_button(ui, "Anexar fotos", [120.0, 26.0]) {
                    self.attach(photo_sub);
                }
                if let Some(sub) = receipt_sub {
                    if steel_button(ui, "Anexar recibo", [120.0, 26.0]) {
                        self.attach(sub);
                    }
                }
            }
            if steel_button(ui, "Abrir pasta", [110.0, 26.0]) {
                self.open_folder();
            }
        });
        if let Some(dir) = self.current_person_dir() {
            let listed = media::list_media(&dir);
            if listed.is_empty() {
                ui.label("Media/: (vazio)");
            } else {
                ui.label("Media nesta pasta:");
                for (sub, name) in listed {
                    ui.label(format!("  {sub}/{name}"));
                }
            }
        }
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        let mut v = egui::Visuals::dark();
        v.window_fill = DARK;
        v.panel_fill = Color32::TRANSPARENT;
        v.extreme_bg_color = PANEL;
        v.faint_bg_color = Color32::from_rgb(0x1E, 0x1F, 0x22);
        v.override_text_color = Some(WHITE);
        v.window_stroke = egui::Stroke::new(1.0_f32, LINE);
        v.selection.bg_fill = BRASS;
        v.selection.stroke = egui::Stroke::new(1.0_f32, DARK);
        v.widgets.inactive.bg_fill = PANEL;
        v.widgets.inactive.weak_bg_fill = PANEL;
        v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, WHITE);
        v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, LINE);
        v.widgets.hovered.bg_fill = Color32::from_rgb(0x24, 0x25, 0x28);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x24, 0x25, 0x28);
        v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, WHITE);
        v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, BRASS);
        v.widgets.active.bg_fill = BRASS;
        v.widgets.active.weak_bg_fill = BRASS;
        v.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, DARK);
        v.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, BRASS);
        v.widgets.open.bg_fill = PANEL;
        v.widgets.open.weak_bg_fill = PANEL;
        v.widgets.open.fg_stroke = egui::Stroke::new(1.0_f32, WHITE);
        v.widgets.inactive.corner_radius = 4.0.into();
        v.widgets.hovered.corner_radius = 4.0.into();
        v.widgets.active.corner_radius = 4.0.into();
        ctx.set_visuals(v);
    }

    fn ensure_textures(&mut self, ctx: &egui::Context) {
        if self.tex_tried {
            return;
        }
        self.tex_tried = true;
        // Desk wallpaper stays flat charcoal. Logo only. chip-bg kept for login plate.
        if let Some(p) = media::find_chip_bg() {
            self.chip_tex = load_texture(ctx, &p, "chip-bg");
        }
        if let Some(p) = media::find_logo() {
            self.logo_tex = load_texture(ctx, &p, "logo");
        }
    }

    fn ensure_fundo_thumbs(&mut self, ctx: &egui::Context) {
        let files = media::list_fundos();
        let same = self.fundo_thumbs.len() == files.len()
            && self
                .fundo_thumbs
                .iter()
                .zip(files.iter())
                .all(|((p, _, _), q)| p == q);
        if same && !self.fundo_thumbs.is_empty() {
            return;
        }
        self.fundo_thumbs = files
            .into_iter()
            .map(|p| {
                let name = p
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let tex = load_texture(ctx, &p, &format!("fundo-{}", p.display()));
                (p, name, tex)
            })
            .collect();
    }

    fn fit_to_screen(&mut self, ctx: &egui::Context) {
        if self.boot_max == 0 {
            return;
        }
        let vp = ctx.input(|i| i.viewport().clone());
        if vp.maximized == Some(true) {
            self.boot_max = 0;
            return;
        }
        if let (Some(mon), Some(inner)) = (vp.monitor_size, vp.inner_rect) {
            if inner.width() >= mon.x * 0.90 && inner.height() >= mon.y * 0.82 {
                self.boot_max = 0;
                return;
            }
        }
        if let Some(mon) = vp.monitor_size {
            if mon.x > 400.0 && mon.y > 300.0 {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(mon));
            }
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
        self.boot_max = self.boot_max.saturating_sub(1);
        ctx.request_repaint();
    }

    fn paint_window_bg(&self, ctx: &egui::Context) {
        let layer = egui::LayerId::background();
        let rect = ctx.screen_rect();
        let painter = ctx.layer_painter(layer);
        painter.rect_filled(rect, 0.0, DARK);
        let _ = &self.bg;
    }

    fn paint_greeting(&self, ctx: &egui::Context, elapsed: f64) {
        self.paint_window_bg(ctx);
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let fade = ((elapsed / 0.28) as f32).clamp(0.0, 1.0);
                let scale = 0.88 + 0.12 * fade;
                let title_fade = (((elapsed - 0.18) / 0.28) as f32).clamp(0.0, 1.0);
                ui.allocate_space(ui.available_size());
                let center = ui.max_rect().center();
                if let Some(logo) = &self.logo_tex {
                    let size = Vec2::splat(160.0 * scale);
                    let rect = Rect::from_center_size(center - Vec2::new(0.0, 36.0), size);
                    let a = (fade * 255.0) as u8;
                    ui.painter().image(
                        logo.id(),
                        rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::from_white_alpha(a),
                    );
                }
                let a = (title_fade * 255.0) as u8;
                ui.painter().text(
                    center + Vec2::new(0.0, 70.0),
                    egui::Align2::CENTER_CENTER,
                    "Vanguarda",
                    egui::FontId::proportional(26.0),
                    Color32::from_rgba_unmultiplied(0xC4, 0xA3, 0x5A, a),
                );
                ui.interact(ui.max_rect(), ui.id().with("greet"), Sense::click());
            });
    }

    fn paint_login(&mut self, ctx: &egui::Context) {
        self.paint_window_bg(ctx);
        let first = auth::is_first_run(&self.root_path());
        let now = ctx.input(|i| i.time);
        let fade = if self.login_t0 <= 0.0 {
            1.0
        } else {
            ((now - self.login_t0) / 0.18).clamp(0.0, 1.0) as f32
        };
        if fade < 1.0 {
            ctx.request_repaint();
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |_| {});
        egui::Area::new(egui::Id::new("v-login"))
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                ui.multiply_opacity(fade);
                const COL: f32 = 360.0;
                egui::Frame::new()
                    .fill(PANEL)
                    .stroke(egui::Stroke::new(1.0_f32, LINE))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(28))
                    .show(ui, |ui| {
                        ui.set_width(COL);
                        ui.spacing_mut().item_spacing.y = 8.0;
                        ui.vertical_centered(|ui| {
                            ui.set_width(COL);
                            if let Some(logo) = &self.logo_tex {
                                ui.add(
                                    egui::Image::new(logo).fit_to_exact_size(Vec2::splat(56.0)),
                                );
                                ui.add_space(6.0);
                            }
                            ui.label(
                                RichText::new("VANGUARDA")
                                    .color(BRASS)
                                    .size(24.0)
                                    .family(fonts::heading_family())
                                    .strong(),
                            );
                        });
                        ui.add_space(6.0);
                        let y = ui.cursor().top();
                        ui.painter().hline(
                            ui.max_rect().x_range(),
                            y,
                            egui::Stroke::new(1.0_f32, LINE),
                        );
                        ui.add_space(14.0);
                        if first {
                            ui.label(
                                RichText::new("Primeira pessoa")
                                    .color(WHITE)
                                    .size(13.0),
                            );
                            ui.label(
                                RichText::new("A conta do software cria-se depois em Sistema.")
                                    .color(BRASS)
                                    .size(12.0),
                            );
                            ui.add_space(4.0);
                            ui.label(RichText::new("Nome").color(WHITE).size(12.0));
                            if self.login_name.trim().is_empty() {
                                self.login_name = "Gil Salvador".into();
                            }
                            ui.add(
                                egui::TextEdit::singleline(&mut self.login_name)
                                    .desired_width(COL),
                            );
                            ui.label(RichText::new("Mesa").color(WHITE).size(12.0));
                            let mut d = self.login_dept;
                            egui::ComboBox::from_id_salt("first-dept")
                                .selected_text(d.label())
                                .width(COL)
                                .show_ui(ui, |ui| {
                                    for x in Departamento::all() {
                                        ui.selectable_value(&mut d, x, x.label());
                                    }
                                });
                            self.login_dept = d;
                            ui.label(RichText::new("Palavra-passe").color(WHITE).size(12.0));
                            let pass = ui.add(
                                egui::TextEdit::singleline(&mut self.login_pass)
                                    .password(true)
                                    .desired_width(COL)
                                    .hint_text("Palavra-passe"),
                            );
                            if self.login_need_focus {
                                pass.request_focus();
                                self.login_need_focus = false;
                            }
                            ui.label(RichText::new("Confirmar").color(WHITE).size(12.0));
                            let pass2 = ui.add(
                                egui::TextEdit::singleline(&mut self.login_pass2)
                                    .password(true)
                                    .desired_width(COL)
                                    .hint_text("Confirmar"),
                            );
                            let enter = (pass.has_focus()
                                || pass.lost_focus()
                                || pass2.has_focus()
                                || pass2.lost_focus())
                                && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            ui.add_space(8.0);
                            if gold_button(ui, "Criar", [COL, 36.0]) || enter {
                                if self.login_pass != self.login_pass2 {
                                    self.login_err = "As palavras-passe não coincidem.".into();
                                    self.login_need_focus = true;
                                } else {
                                    let nome = if self.login_name.trim().is_empty() {
                                        "Gil Salvador"
                                    } else {
                                        self.login_name.trim()
                                    };
                                    match auth::add_user(
                                        &self.root_path(),
                                        nome,
                                        self.login_dept,
                                        &self.login_pass,
                                        true,
                                    ) {
                                        Ok(u) => {
                                            self.guia_desk = u.departamento;
                                            self.session = Some(u);
                                            self.load_guia_body();
                                            self.login_pass.clear();
                                            self.login_pass2.clear();
                                            self.login_err.clear();
                                            self.mark_clean();
                                        }
                                        Err(e) => {
                                            self.login_err = format!("{e:#}");
                                            self.login_need_focus = true;
                                        }
                                    }
                                }
                            }
                        } else {
                            let users = auth::activo(&self.root_path());
                            let shop: Vec<_> =
                                users.iter().filter(|u| !u.software).cloned().collect();
                            let sw: Vec<_> =
                                users.iter().filter(|u| u.software).cloned().collect();
                            if shop.len() == 1 && self.login_name.trim().is_empty() {
                                self.login_name = shop[0].nome.clone();
                            }
                            if shop.len() == 1 && sw.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.set_width(COL);
                                    ui.label(
                                        RichText::new(&shop[0].nome)
                                            .color(WHITE)
                                            .size(16.0)
                                            .strong(),
                                    );
                                    ui.label(
                                        RichText::new(shop[0].mesa_label())
                                            .color(BRASS)
                                            .size(12.0),
                                    );
                                });
                                self.login_name = shop[0].nome.clone();
                            } else if shop.len() == 1 && sw.len() == 1 {
                                ui.label(RichText::new("Conta").color(WHITE).size(12.0));
                                let a = shop[0].nome.clone();
                                let b = sw[0].nome.clone();
                                let pill = ((COL - 8.0) / 2.0).floor();
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 8.0;
                                    if self.login_name == a {
                                        let _ = gold_button(ui, &a, [pill, 30.0]);
                                    } else if steel_button(ui, &a, [pill, 30.0]) {
                                        self.login_name = a.clone();
                                    }
                                    if self.login_name == b {
                                        let _ = gold_button(ui, "Software", [pill, 30.0]);
                                    } else if steel_button(ui, "Software", [pill, 30.0]) {
                                        self.login_name = b;
                                    }
                                });
                            } else {
                                ui.label(RichText::new("Utilizador").color(WHITE).size(12.0));
                                for u in &users {
                                    let lab = if u.software {
                                        format!("{}  ·  software", u.nome)
                                    } else {
                                        u.nome.clone()
                                    };
                                    if ui
                                        .selectable_label(self.login_name == u.nome, lab)
                                        .clicked()
                                    {
                                        self.login_name = u.nome.clone();
                                    }
                                }
                            }
                            ui.add_space(4.0);
                            ui.label(RichText::new("Palavra-passe").color(WHITE).size(12.0));
                            let pass = ui.add(
                                egui::TextEdit::singleline(&mut self.login_pass)
                                    .password(true)
                                    .desired_width(COL)
                                    .hint_text("Palavra-passe"),
                            );
                            if self.login_need_focus {
                                pass.request_focus();
                                self.login_need_focus = false;
                            }
                            let enter = (pass.has_focus() || pass.lost_focus())
                                && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            ui.add_space(10.0);
                            if gold_button(ui, "Entrar", [COL, 36.0]) || enter {
                                match auth::verify(
                                    &self.root_path(),
                                    &self.login_name,
                                    &self.login_pass,
                                ) {
                                    Some(u) => {
                                        let name = u.nome.clone();
                                        self.guia_desk = u.departamento;
                                        self.session = Some(u);
                                        self.load_guia_body();
                                        self.login_pass.clear();
                                        self.login_err.clear();
                                        self.mode = Mode::Inicio;
                                        self.tab_t0 = now;
                                        if !self.is_software() {
                                            self.load_staff(&name);
                                        }
                                        self.mark_clean();
                                        audit::append(
                                            &self.root_path(),
                                            &name,
                                            "login",
                                            "",
                                        );
                                        self.warn_stale_backup();
                                    }
                                    None => {
                                        self.login_err =
                                            "Nome ou palavra-passe incorrectos.".into();
                                        self.login_need_focus = true;
                                    }
                                }
                            }
                        }
                        if !self.login_err.is_empty() {
                            ui.add_space(8.0);
                            ui.label(RichText::new(&self.login_err).color(RED).size(12.0));
                        }
                    });
            });
    }
}

fn persist_vanguarda_diag(
    root: &Path,
    cliente: &str,
    mut scan: Scan,
    mut incoming: Carro,
    mecanico: &str,
) -> Result<(PathBuf, Option<Carro>), String> {
    if !mecanico.is_empty() {
        scan.mecanico = mecanico.to_string();
    }
    let stem = writers::diagnostic_basename(&scan);
    let client_dir = {
        if cliente.eq_ignore_ascii_case(DONO_INTERNO) {
            let dir = crate::paths::interno_root(root);
            let _ = fs::create_dir_all(&dir);
            let _ = media::ensure_client_media(&dir);
            dir
        } else {
            let dir = client_dir(root, cliente);
            let _ = fs::create_dir_all(&dir);
            let _ = media::ensure_client_media(&dir);
            dir
        }
    };
    let media_dir = client_dir.join("Media").join("Diagnosticos");
    fs::create_dir_all(&media_dir).map_err(|e| format!("{e}"))?;
    cars::upsert_from_scan(root, cliente, &scan).map_err(|e| format!("{e:#}"))?;
    let mut out_car = None;
    let can_car = !incoming.dono.trim().is_empty()
        && (!incoming.matricula.trim().is_empty() || !incoming.vin.trim().is_empty());
    if can_car {
        let prev_recado = cars::find_car(root, &scan.vin, &scan.matricula).map(|c| c.recado);
        let recado_changed = match prev_recado {
            Some(old) => old != incoming.recado,
            None => !incoming.recado.trim().is_empty(),
        };
        let mut car =
            cars::find_car(root, &scan.vin, &scan.matricula).unwrap_or_else(|| incoming.clone());
        if !incoming.estado_oficina.trim().is_empty() {
            car.estado_oficina = incoming.estado_oficina.clone();
        }
        car.recado = incoming.recado.clone();
        car.ensure_sistemas();
        incoming.ensure_sistemas();
        for s in &incoming.sistemas {
            if s.estrelas == 0 {
                continue;
            }
            if let Some(row) = car
                .sistemas
                .iter_mut()
                .find(|r| r.categoria == s.categoria)
            {
                row.estrelas = s.estrelas;
            }
        }
        if recado_changed && !mecanico.is_empty() {
            car.recado_quem = mecanico.to_string();
            car.recado_quando = now_stamp();
        }
        let _ = cars::upsert_car(root, &car);
        out_car = Some(car);
    }
    let stars = out_car
        .as_ref()
        .map(|c| c.sistemas.clone())
        .unwrap_or_else(|| incoming.sistemas.clone());
    let written = writers::write_diagnostico_stars(&media_dir, &stem, &scan, cliente, &stars)
        .map_err(|e| format!("{e:#}"))?;
    if let Some(car) = cars::find_car(root, &scan.vin, &scan.matricula) {
        let car_media = cars::car_dir(root, &car)
            .join("Media")
            .join("Diagnosticos");
        let _ = fs::create_dir_all(&car_media);
        let _ = writers::write_diagnostico_stars(&car_media, &stem, &scan, cliente, &stars);
    }
    let pdf = written
        .into_iter()
        .find(|p| p.extension().and_then(|e| e.to_str()) == Some("pdf"))
        .unwrap_or_else(|| media_dir.join(format!("{stem}.pdf")));
    Ok((pdf, out_car))
}

fn load_texture(ctx: &egui::Context, path: &Path, name: &str) -> Option<egui::TextureHandle> {
    let img = image::open(path).ok()?.into_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, &img);
    Some(ctx.load_texture(name, color, egui::TextureOptions::LINEAR))
}

fn load_texture_bytes(ctx: &egui::Context, jpeg: &[u8], name: &str) -> Option<egui::TextureHandle> {
    let img = image::load_from_memory(jpeg).ok()?.into_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, &img);
    Some(ctx.load_texture(name, color, egui::TextureOptions::LINEAR))
}

impl eframe::App for FichaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_theme(ctx);
        self.ensure_textures(ctx);
        self.fit_to_screen(ctx);
        let now = ctx.input(|i| i.time);
        if self.t0 == 0.0 {
            self.t0 = now;
        }
        if self.show_greeting {
            let skip = ctx.input(|i| {
                i.pointer.any_click()
                    || i.key_pressed(egui::Key::Escape)
                    || i.key_pressed(egui::Key::Space)
                    || i.key_pressed(egui::Key::Enter)
            });
            if skip || now - self.t0 > 1.35 {
                self.show_greeting = false;
                self.login_t0 = now;
                self.login_need_focus = true;
            } else {
                self.paint_greeting(ctx, now - self.t0);
                ctx.request_repaint();
                return;
            }
        }

        if self.session.is_none() {
            self.paint_login(ctx);
            return;
        }

        self.poll_jobs();
        self.take_diag_drops(ctx);
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
            self.save_current_tab();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F5)) {
            self.begin_refresh_async();
        }
        if self.busy.is_some() {
            ctx.request_repaint();
        }

        if !self.can(self.mode) {
            self.mode = Mode::Inicio;
        }

        ctx.data_mut(|d| d.insert_temp(egui::Id::new("v-tab-t0"), self.tab_t0));
        self.paint_window_bg(ctx);

        let plate = self.chip_tex.as_ref().map(|t| t.id());
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE.inner_margin(egui::Margin {
                    left: 8,
                    right: 8,
                    top: 8,
                    bottom: 8,
                }),
            )
            .show(ctx, |ui| {
                metal_chip(ui, plate, |ui| {
                    ui.horizontal(|ui| {
                        if let Some(logo) = &self.logo_tex {
                            ui.add(egui::Image::new(logo).fit_to_exact_size(Vec2::splat(28.0)));
                        }
                        ui.label(
                            RichText::new("VANGUARDA")
                                .color(GOLD)
                                .size(22.0)
                                .family(fonts::heading_family())
                                .strong(),
                        );
                        if let Some(u) = &self.session {
                            ui.label(
                                RichText::new(format!("·  {}", u.nome))
                                    .color(WHITE)
                                    .size(11.0),
                            );
                            ui.label(
                                RichText::new(format!("·  {}", u.mesa_label()))
                                    .color(BRASS)
                                    .size(11.0),
                            );
                            if u.admin {
                                ui.label(
                                    RichText::new("·  ADMIN")
                                        .color(BRASS)
                                        .size(11.0)
                                        .strong(),
                                );
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if steel_button(ui, "Actualizar", [100.0, 24.0]) {
                                self.begin_refresh_async();
                            }
                            if steel_button(ui, "Sair", [72.0, 24.0]) {
                                self.session = None;
                                self.login_pass.clear();
                                self.login_need_focus = true;
                            }
                            if !self.is_software() {
                                if let Some(u) = self.session.clone() {
                                    let mut d = u.departamento;
                                    egui::ComboBox::from_id_salt("duty-mesa")
                                        .selected_text(d.label())
                                        .width(130.0)
                                        .show_ui(ui, |ui| {
                                            for x in Departamento::all() {
                                                ui.selectable_value(&mut d, x, x.label());
                                            }
                                        });
                                    if d != u.departamento {
                                        self.set_own_mesa(d);
                                    }
                                }
                            }
                        });
                    });
                    ui.add_space(8.0);
                    let show_cli = self.can(Mode::Cliente);
                    let show_car = self.can(Mode::Carro);
                    let show_staff = self.can(Mode::Staff);
                    let show_diag = self.can(Mode::Diagnostico);
                    let show_care = self.can(Mode::Care);
                    let show_int = self.can(Mode::Interiores);
                    let show_trab = self.can(Mode::Trabalho);
                    let show_ag = self.can(Mode::Agenda);
                    let show_stock = self.can(Mode::Stock);
                    let show_guias = self.can(Mode::Guias);
                    let show_docs = self.can(Mode::Documentos);
                    let show_sys = self.can(Mode::Sistema);
                    let prev_mode = self.mode;
                    let tab_hit = ui
                        .horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            ui.spacing_mut().button_padding = Vec2::new(8.0, 4.0);
                            let mut hit = false;
                            hit |= shop_tab(ui, &mut self.mode, Mode::Inicio, "Início");
                            if show_cli || show_car {
                                ui.label(RichText::new("|").color(LINE).size(14.0));
                            }
                            if show_cli {
                                hit |= shop_tab(ui, &mut self.mode, Mode::Cliente, "Cliente");
                            }
                            if show_car {
                                hit |= shop_tab(ui, &mut self.mode, Mode::Carro, "Carro");
                            }
                            if show_diag || show_care || show_int {
                                ui.label(RichText::new("|").color(LINE).size(14.0));
                            }
                            if show_diag {
                                hit |= shop_tab(ui, &mut self.mode, Mode::Diagnostico, "Relatório");
                            }
                            if show_care {
                                hit |= shop_tab(ui, &mut self.mode, Mode::Care, "Care");
                            }
                            if show_int {
                                hit |= shop_tab(ui, &mut self.mode, Mode::Interiores, "Interiores");
                            }
                            if show_trab || show_ag || show_stock {
                                ui.label(RichText::new("|").color(LINE).size(14.0));
                            }
                            if show_trab {
                                hit |= shop_tab(ui, &mut self.mode, Mode::Trabalho, "Trabalho");
                            }
                            if show_ag {
                                hit |= shop_tab(ui, &mut self.mode, Mode::Agenda, "Agenda");
                            }
                            if show_stock {
                                hit |= shop_tab(ui, &mut self.mode, Mode::Stock, "Stock");
                            }
                            if show_staff {
                                ui.label(RichText::new("|").color(LINE).size(14.0));
                                hit |= shop_tab(ui, &mut self.mode, Mode::Staff, "Pessoas");
                            }
                            if show_guias {
                                ui.label(RichText::new("|").color(LINE).size(14.0));
                                hit |= shop_tab(ui, &mut self.mode, Mode::Guias, "Guias");
                            }
                            if show_docs {
                                hit |= shop_tab(ui, &mut self.mode, Mode::Documentos, "Documentos");
                            }
                            if show_sys {
                                ui.label(RichText::new("|").color(LINE).size(14.0));
                                hit |= shop_tab(ui, &mut self.mode, Mode::Sistema, "Sistema");
                            }
                            hit
                        })
                        .inner;
                    if tab_hit {
                        if self.is_dirty() && self.mode != prev_mode {
                            self.pending_mode = Some(self.mode);
                            self.mode = prev_mode;
                            self.show_dirty = true;
                        } else {
                            self.refresh_lists();
                            self.tab_t0 = ui.ctx().input(|i| i.time);
                            self.mark_clean();
                            if self.mode != prev_mode {
                                self.desk_gate = DeskGate::Menu;
                            }
                        }
                    }
                });
                ui.add_space(6.0);
                if self.mode == Mode::Sistema && (self.can_backup() || self.can_edit_data()) {
                    metal_chip(ui, plate, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("DATA").color(GOLD).size(11.0).strong());
                            let w = (ui.available_width() - 8.0).max(80.0);
                            if self.can_edit_data() {
                                if ui
                                    .add(
                                        egui::TextEdit::singleline(&mut self.root)
                                            .desired_width(w.min(420.0))
                                            .text_color(WHITE),
                                    )
                                    .changed()
                                {
                                    self.refresh_lists();
                                }
                            } else {
                                ui.label(RichText::new(&self.root).color(WHITE).size(13.0));
                            }
                        });
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            if steel_button(ui, "Actualizar", [100.0, 26.0]) {
                                self.begin_refresh_async();
                            }
                            if steel_button(ui, "Exportar", [90.0, 26.0]) {
                                self.export_tab();
                            }
                            if self.can_backup() {
                                if steel_button(ui, "Consolidar", [100.0, 26.0]) {
                                    self.begin_consolidate();
                                }
                                if steel_button(ui, "Backup zip", [100.0, 26.0]) {
                                    self.backup_zip();
                                }
                            }
                        });
                    });
                }
                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .id_salt("desk-body")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let now = ui.ctx().input(|i| i.time);
                        let fade = if self.tab_t0 <= 0.0 {
                            1.0
                        } else {
                            ((now - self.tab_t0) / 0.14).clamp(0.0, 1.0) as f32
                        };
                        if fade < 1.0 {
                            ui.ctx().request_repaint();
                        }
                        ui.multiply_opacity(fade);
                        ui.horizontal(|ui| {
                            ui.add_space(10.0 * (1.0 - fade));
                            ui.vertical(|ui| {
                                ui.set_width(ui.available_width());
                        match self.mode {
                            Mode::Inicio => self.ui_inicio(ui, plate),
                            Mode::Diagnostico => {
                                metal_chip_fill(ui, plate, |ui| self.ui_diagnostico(ui));
                            }
                            Mode::Cliente => {
                                metal_chip_fill(ui, plate, |ui| self.ui_cliente(ui));
                            }
                            Mode::Carro => {
                                metal_chip_fill(ui, plate, |ui| self.ui_carro(ui));
                            }
                            Mode::Staff => {
                                metal_chip(ui, plate, |ui| self.ui_staff(ui));
                            }
                            Mode::Care => {
                                metal_chip(ui, plate, |ui| self.ui_care(ui));
                            }
                            Mode::Interiores => {
                                metal_chip(ui, plate, |ui| self.ui_interiores(ui));
                            }
                            Mode::Trabalho => {
                                metal_chip_fill(ui, plate, |ui| self.ui_trabalho(ui));
                            }
                            Mode::Agenda => {
                                metal_chip(ui, plate, |ui| self.ui_agenda(ui));
                            }
                            Mode::Stock => {
                                metal_chip(ui, plate, |ui| self.ui_stock(ui));
                            }
                            Mode::Guias => {
                                metal_chip(ui, plate, |ui| self.ui_guias(ui));
                            }
                            Mode::Documentos => {
                                metal_chip(ui, plate, |ui| self.ui_documentos(ui));
                            }
                            Mode::Sistema => {
                                metal_chip(ui, plate, |ui| self.ui_sistema(ui));
                            }
                        }
                            });
                        });
                    });
                ui.add_space(6.0);
                metal_chip(ui, plate, |ui| {
                    let now = ui.ctx().input(|i| i.time);
                    if self.status_at <= 0.0 {
                        self.status_at = now;
                    }
                    let flash = ((0.28 - (now - self.status_at)).max(0.0) / 0.28) as f32;
                    if flash > 0.0 {
                        ui.ctx().request_repaint();
                    }
                    let color = if self.status_ok { GOLD } else { RED };
                    ui.label(
                        RichText::new(&self.status)
                            .color(color)
                            .size(13.0 + 2.0 * flash),
                    );
                });
            });
        if self.show_consolidate {
            self.ui_consolidate(ctx);
        }
        if self.pending_delete.is_some() {
            self.ui_delete_confirm(ctx);
        }
        if self.pending_advance.is_some() {
            self.ui_advance_confirm(ctx);
        }
        if self.show_load_car {
            self.ui_load_car(ctx);
        }
        if self.show_consumo_wiz && !self.is_software() {
            self.ui_consumo_wiz(ctx);
        }
        if self.merge_pending.is_some() {
            self.ui_merge_confirm(ctx);
        }
        if self.show_anexo {
            self.ui_anexo_picker(ctx);
        }
        if self.show_plate_conflict {
            self.ui_plate_conflict(ctx);
        }
        if self.show_preview {
            self.ui_preview(ctx);
        }
        if self.show_dirty {
            self.ui_dirty_dialog(ctx);
        }
        if let Some(k) = self.busy {
            self.paint_busy(ctx, k);
        }
    }
}

impl FichaApp {
    fn ui_consumo_graph(&mut self, ui: &mut egui::Ui) {
        let me = self.session.as_ref().map(|u| u.nome.clone()).unwrap_or_default();
        let mine = load_consumo_log(&self.root_path(), &me);
        let grow = {
            let now = ui.ctx().input(|i| i.time);
            ((now - self.tab_t0).max(0.0) / 0.16).clamp(0.0, 1.0) as f32
        };
        section_title(ui, "O teu consumo", false);
        draw_consumo_bars(ui, &mine, grow);
        if matches!(self.dept(), Some(Departamento::Escritorio | Departamento::Design))
            || self.is_admin()
        {
            ui.add_space(8.0);
            section_title(ui, "Toda a casa", false);
            let mut house = ConsumoLog::default();
            for n in &self.staff_names {
                house.entradas.extend(load_consumo_log(&self.root_path(), n).entradas);
            }
            draw_consumo_bars(ui, &house, grow);
        }
    }

    fn ui_consumo_wiz(&mut self, ctx: &egui::Context) {
        let mut open = self.show_consumo_wiz;
        egui::Window::new("Registar consumo")
            .open(&mut open)
            .collapsible(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.label(RichText::new("O que a casa pagou").color(GOLD));
                ui.add_space(6.0);
                ui.label("Categoria:");
                let mut tipo = self.nova_entrada.tipo.clone();
                egui::ComboBox::from_id_salt("wiz-tipo")
                    .selected_text(tipo.clone())
                    .width(360.0)
                    .show_ui(ui, |ui| {
                        for t in CONSUMO_TIPOS {
                            ui.selectable_value(&mut tipo, (*t).to_string(), *t);
                        }
                    });
                self.nova_entrada.tipo = tipo;
                ui.add_space(6.0);
                ui.label("O quê, em concreto:");
                ui.add(
                    egui::TextEdit::multiline(&mut self.nova_entrada.descricao)
                        .desired_width(360.0)
                        .desired_rows(3),
                );
                ui.add_space(4.0);
                ui.label("Viatura / referência (podes deixar vazio):");
                ui.add(
                    egui::TextEdit::singleline(&mut self.nova_entrada.veiculo)
                        .desired_width(280.0)
                        .hint_text("matrícula ou —"),
                );
                ui.label("Custo interno (€):");
                ui.add(
                    egui::TextEdit::singleline(&mut self.nova_entrada.custo_interno)
                        .desired_width(120.0)
                        .hint_text("0,00"),
                );
                ui.add_space(10.0);
                if gold_button(ui, "Registar", [120.0, 28.0]) {
                    self.save_consumo();
                    self.show_consumo_wiz = false;
                }
            });
        self.show_consumo_wiz = open && self.show_consumo_wiz;
    }

    fn ui_load_car(&mut self, ctx: &egui::Context) {
        let mut open = self.show_load_car;
        let cliente = if !self.client.nome_completo.trim().is_empty() {
            self.client.nome_completo.trim().to_string()
        } else {
            self.selected_client.clone()
        };
        egui::Window::new("Carregar viatura")
            .open(&mut open)
            .collapsible(false)
            .default_width(460.0)
            .show(ctx, |ui| {
                ui.label(format!("Cliente: {cliente}"));
                ui.add_space(8.0);
                ui.label("Carro já no sistema:");
                let mut pick = self.load_car_pick.clone();
                egui::ComboBox::from_id_salt("load-car-combo")
                    .selected_text(if pick.is_empty() {
                        "(escolher)".to_string()
                    } else {
                        pick.clone()
                    })
                    .width(320.0)
                    .show_ui(ui, |ui| {
                        for c in &self.cars {
                            ui.selectable_value(&mut pick, c.label(), c.label());
                        }
                    });
                self.load_car_pick = pick.clone();
                if gold_button(ui, "Atribuir este carro", [220.0, 28.0]) {
                    if let Some(c) = self.cars.iter().find(|c| c.label() == pick).cloned() {
                        let mut c = c;
                        c.dono = cliente.clone();
                        match cars::upsert_car(&self.root_path(), &c) {
                            Ok(_) => {
                                self.client.veiculo_marca = c.marca.clone();
                                self.client.veiculo_modelo = c.modelo.clone();
                                self.client.veiculo_versao = c.versao.clone();
                                self.client.veiculo_matricula = c.matricula.clone();
                                self.client.veiculo_ano = c.ano.clone();
                                self.client.veiculo_vin = c.vin.clone();
                                self.client.veiculo_km = c.km.clone();
                                self.car = c;
                                self.refresh_lists();
                                self.set_status(true, "Viatura atribuída a este cliente.");
                                self.show_load_car = false;
                            }
                            Err(e) => self.set_status(false, format!("{e:#}")),
                        }
                    }
                }
                if self.can_edit_relatorio() {
                    ui.add_space(10.0);
                    ui.label("Ou um PDF Autocom:");
                    if gold_button(ui, "Abrir PDF Autocom…", [220.0, 28.0]) {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("PDF", &["pdf"])
                            .pick_file()
                        {
                            match diag_parse::parse_autocom_pdf(&path) {
                                Ok(scan) => {
                                    self.scan = scan;
                                    match cars::upsert_from_scan(
                                        &self.root_path(),
                                        &cliente,
                                        &self.scan,
                                    ) {
                                        Ok(_) => {
                                            let bits = diag_parse::split_veiculo(&self.scan.veiculo);
                                            self.client.veiculo_marca = bits.marca;
                                            self.client.veiculo_modelo = bits.modelo;
                                            self.client.veiculo_versao = bits.versao;
                                            self.client.veiculo_ano = bits.ano;
                                            self.client.veiculo_vin = self.scan.vin.clone();
                                            self.client.veiculo_matricula = self.scan.matricula.clone();
                                            self.client.veiculo_km = self.scan.km.clone();
                                            self.refresh_lists();
                                            self.set_status(
                                                true,
                                                format!(
                                                    "Autocom lido ({} códigos) e associado a {cliente}.",
                                                    self.scan.dtcs.len()
                                                ),
                                            );
                                            self.show_load_car = false;
                                        }
                                        Err(e) => self.set_status(false, format!("{e:#}")),
                                    }
                                }
                                Err(e) => self.set_status(false, format!("Não li o PDF: {e:#}")),
                            }
                        }
                    }
                }
                ui.add_space(10.0);
                if self.can_backup() && gold_button(ui, "Consolidar este cliente", [240.0, 28.0]) {
                    self.begin_consolidate();
                    self.show_load_car = false;
                }
            });
        self.show_load_car = open && self.show_load_car;
    }

    fn ui_care(&mut self, ui: &mut egui::Ui) {
        let edit = self.can_edit_care();
        section_title(ui, "Care", !edit);
        ui.label(
            RichText::new("Pacotes de detalhe · IVA incluído. SUV e heavy soil à parte.")
                .color(WHITE)
                .size(13.0),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Filtro:");
            ui.add(
                egui::TextEdit::singleline(&mut self.filter_report)
                    .desired_width(200.0)
                    .hint_text("matrícula, dono"),
            );
        });
        let q = self.filter_report.to_lowercase();
        let cars: Vec<Carro> = self
            .cars
            .iter()
            .filter(|c| {
                q.is_empty()
                    || c.label().to_lowercase().contains(&q)
                    || c.vin.to_lowercase().contains(&q)
            })
            .cloned()
            .collect();
        let mut pick: Option<Carro> = None;
        egui::ScrollArea::vertical()
            .id_salt("care-cars")
            .max_height(100.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                for c in &cars {
                    let on = self.selected_car == c.label();
                    let line = format!("{}{}", if on { "●  " } else { "·  " }, c.label());
                    if board_hit(ui, &line) {
                        pick = Some(c.clone());
                    }
                }
            });
        if let Some(c) = pick {
            self.open_car(c);
            self.mode = Mode::Care;
        }
        if !self.car_can_persist(&self.car) {
            ui.label(RichText::new("Escolhe uma viatura.").color(WHITE));
            return;
        }
        ui.add_space(8.0);
        ui.label(RichText::new(self.car.label()).color(WHITE).strong());
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Pacote").color(GOLD));
            for (id, nome, preco) in CARE_PACOTES {
                let on = self.car.care_pacote == *id;
                if ui.selectable_label(on, format!("{nome}  {preco}")).clicked() && edit {
                    self.car.care_pacote = (*id).to_string();
                }
            }
        });
        ui.add_space(6.0);
        ui.label(RichText::new("À la carte").color(GOLD));
        for (id, nome, preco) in CARE_EXTRAS {
            let mut on = self.car.care_extras.iter().any(|x| x == id);
            ui.add_enabled_ui(edit, |ui| {
                if ui
                    .checkbox(&mut on, format!("{nome}  ·  {preco}"))
                    .changed()
                {
                    if on {
                        if !self.car.care_extras.iter().any(|x| x == id) {
                            self.car.care_extras.push((*id).to_string());
                        }
                    } else {
                        self.car.care_extras.retain(|x| x != id);
                    }
                }
            });
        }
        ui.add_space(4.0);
        ui.add_enabled_ui(edit, |ui| {
            ui.checkbox(
                &mut self.car.care_suv,
                "SUV / carrinha / 7 lugares  ·  suplemento 5€–30€",
            );
            ui.checkbox(
                &mut self.car.care_heavy,
                "Heavy soil  ·  avisar o Escritório antes de começar",
            );
        });
        if self.car.care_heavy {
            ui.label(
                RichText::new("Não comeces sem o responsável fechar a taxa.")
                    .color(RED)
                    .size(12.0),
            );
        }
        ui.add_space(8.0);
        section_title(ui, "Checklist", !edit);
        let pac = self.car.care_pacote.clone();
        let steps = care_checks_for(&pac);
        if steps.is_empty() {
            ui.label("Escolhe um pacote para ver o fluxo.");
        } else {
            for id in steps {
                let mut on = self.car.care_checks.iter().any(|x| x == id);
                ui.add_enabled_ui(edit, |ui| {
                    if ui.checkbox(&mut on, care_check_label(id)).changed() {
                        if on {
                            self.car.care_checks.push(id.to_string());
                        } else {
                            self.car.care_checks.retain(|x| x != id);
                        }
                    }
                });
            }
        }
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if edit && gold_button(ui, "Guardar Care", [160.0, 32.0]) {
                if self.car.estado_oficina.trim().is_empty()
                    || !ESTADOS_CARE.contains(&self.car.estado_oficina.as_str())
                {
                    self.car.estado_oficina = "A aguardar Care".into();
                }
                self.save_carro();
            }
            if gold_button(ui, "Menu cliente (PDF)", [180.0, 32.0]) {
                self.print_care_menu();
            }
        });
    }

    fn print_care_menu(&mut self) {
        self.bind_pdf_style();
        let _ = guias::seed(&self.root_path());
        let body = guias::load(&self.root_path(), Departamento::Care, false);
        let dir = guias::guias_root(&self.root_path()).join("care");
        let _ = fs::create_dir_all(&dir);
        let dest = dir.join("Menu_Care.pdf");
        match writers::write_guia_pdf(&dest, "Care — Menu de servico", &body) {
            Ok(()) => {
                self.last_diag_pdf = Some(dest.clone());
                self.finish_pdf_out(&dest, false);
            }
            Err(e) => self.set_status(false, format!("{e:#}")),
        }
    }

    fn ui_interiores(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("Back · serviço ainda não à venda")
                .color(BRASS)
                .size(13.0)
                .strong(),
        );
        ui.label(
            RichText::new("Ensaio da mesa. Não orçamentar Gold ao cliente.")
                .color(WHITE)
                .size(12.0),
        );
        ui.add_space(6.0);
        let edit = self.can_edit_interiores();
        section_title(ui, "Interiores", !edit);
        ui.label(
            RichText::new("Ensaio interno. Packs não saem ao cliente até o Escritório abrir a mesa.")
                .color(WHITE)
                .size(13.0),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Filtro:");
            ui.add(
                egui::TextEdit::singleline(&mut self.filter_report)
                    .desired_width(200.0)
                    .hint_text("matrícula, dono"),
            );
        });
        let q = self.filter_report.to_lowercase();
        let cars: Vec<Carro> = self
            .cars
            .iter()
            .filter(|c| {
                q.is_empty()
                    || c.label().to_lowercase().contains(&q)
                    || c.vin.to_lowercase().contains(&q)
            })
            .cloned()
            .collect();
        let mut pick: Option<Carro> = None;
        egui::ScrollArea::vertical()
            .id_salt("int-cars")
            .max_height(100.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                for c in &cars {
                    let on = self.selected_car == c.label();
                    let line = format!("{}{}", if on { "●  " } else { "·  " }, c.label());
                    if board_hit(ui, &line) {
                        pick = Some(c.clone());
                    }
                }
            });
        if let Some(c) = pick {
            self.open_car(c);
            self.mode = Mode::Interiores;
        }
        if !self.car_can_persist(&self.car) {
            ui.label(RichText::new("Escolhe uma viatura.").color(WHITE));
            return;
        }
        ui.add_space(8.0);
        ui.label(RichText::new(self.car.label()).color(WHITE).strong());
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Pack").color(GOLD));
            for (id, nome, preco) in PINTURA_PACOTES {
                let on = self.car.interiores_pacote == *id;
                if ui.selectable_label(on, format!("{nome}  {preco}")).clicked() && edit {
                    self.car.interiores_pacote = (*id).to_string();
                }
            }
        });
        ui.add_space(6.0);
        ui.label(RichText::new("Extras").color(GOLD));
        for (id, nome, preco) in PINTURA_EXTRAS {
            let mut on = self.car.interiores_extras.iter().any(|x| x == id);
            ui.add_enabled_ui(edit, |ui| {
                if ui
                    .checkbox(&mut on, format!("{nome}  ·  {preco}"))
                    .changed()
                {
                    if on {
                        if !self.car.interiores_extras.iter().any(|x| x == id) {
                            self.car.interiores_extras.push((*id).to_string());
                        }
                    } else {
                        self.car.interiores_extras.retain(|x| x != id);
                    }
                }
            });
        }
        ui.add_space(8.0);
        section_title(ui, "Fluxo Stardust", !edit);
        for id in PINTURA_CHECKS {
            let mut on = self.car.interiores_checks.iter().any(|x| x == id);
            ui.add_enabled_ui(edit, |ui| {
                if ui.checkbox(&mut on, pintura_check_label(id)).changed() {
                    if on {
                        self.car.interiores_checks.push((*id).to_string());
                    } else {
                        self.car.interiores_checks.retain(|x| x != id);
                    }
                }
            });
        }
        ui.add_space(8.0);
        if edit && gold_button(ui, "Guardar Interiores", [200.0, 32.0]) {
            if self.car.estado_oficina.trim().is_empty()
                || !ESTADOS_INTERIORES.contains(&self.car.estado_oficina.as_str())
            {
                self.car.estado_oficina = "A aguardar Interiores".into();
            }
            self.save_carro();
        }
    }

    fn ui_guias(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "Guias da casa", false);
        ui.label(
            RichText::new("Menu do cliente e SOP interno. Cada departamento tem o seu.")
                .color(WHITE)
                .size(13.0),
        );
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            let me = self.dept();
            for d in Departamento::all() {
                let mark = if Some(d) == me {
                    format!("● {}", d.label())
                } else {
                    d.label().to_string()
                };
                if ui
                    .selectable_label(self.guia_desk == d, mark)
                    .clicked()
                {
                    self.guia_desk = d;
                    self.load_guia_body();
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(!self.guia_interno, "Menu cliente")
                .clicked()
            {
                self.guia_interno = false;
                self.load_guia_body();
            }
            if ui
                .selectable_label(self.guia_interno, "Guia interno")
                .clicked()
            {
                self.guia_interno = true;
                self.load_guia_body();
            }
        });
        ui.add_space(6.0);
        let edit = self.can_edit_guia(self.guia_interno);
        let w = ui.available_width();
        ui.add_enabled(
            edit,
            egui::TextEdit::multiline(&mut self.guia_body)
                .desired_width(w)
                .desired_rows(18),
        );
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if edit && gold_button(ui, "Guardar guia", [150.0, 30.0]) {
                match guias::save(
                    &self.root_path(),
                    self.guia_desk,
                    self.guia_interno,
                    &self.guia_body,
                ) {
                    Ok(p) => self.set_status(true, format!("Guia: {}", p.display())),
                    Err(e) => self.set_status(false, format!("{e:#}")),
                }
            }
            if gold_button(ui, "Exportar PDF", [140.0, 30.0]) {
                self.bind_pdf_style();
                let dir = guias::guia_path(
                    &self.root_path(),
                    self.guia_desk,
                    self.guia_interno,
                )
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| guias::guias_root(&self.root_path()));
                let _ = fs::create_dir_all(&dir);
                let dest = dir.join(if self.guia_interno {
                    "Guia_interno.pdf"
                } else {
                    "Menu_cliente.pdf"
                });
                match writers::write_guia_pdf(
                    &dest,
                    &format!(
                        "{} — {}",
                        self.guia_desk.label(),
                        if self.guia_interno {
                            "Guia interno"
                        } else {
                            "Menu cliente"
                        }
                    ),
                    &self.guia_body,
                ) {
                    Ok(()) => {
                        self.last_diag_pdf = Some(dest.clone());
                        self.finish_pdf_out(&dest, false);
                    }
                    Err(e) => self.set_status(false, format!("{e:#}")),
                }
            }
            if steel_button(ui, "Abrir pasta", [120.0, 30.0]) {
                let p = guias::guias_root(&self.root_path());
                let _ = fs::create_dir_all(&p);
                let _ = std::process::Command::new("explorer").arg(p).spawn();
            }
        });
    }

    fn ui_documentos(&mut self, ui: &mut egui::Ui) {
        if !self.can_edit_documentos() {
            ui.label(RichText::new("Só Design (ou a conta do software) edita o aspeto dos documentos.").color(WHITE));
            return;
        }
        section_title(ui, "Documentos", false);
        ui.label(
            RichText::new("Como saem os PDFs da casa — relatórios, fichas, consumo. Os outros departamentos só pré-visualizam.")
                .color(WHITE)
                .size(13.0),
        );
        ui.add_space(8.0);
        section_title(ui, "Estilo", false);
        ui.horizontal_wrapped(|ui| {
            let formal = self.doc_style.is_formal();
            if ui.selectable_label(formal, "Formal").clicked() {
                let keep_img = self.doc_style.fundo.eq_ignore_ascii_case("imagem");
                self.doc_style = DocStyle::formal();
                if keep_img {
                    self.doc_style.fundo = "imagem".into();
                }
            }
            if ui.selectable_label(!formal, "Estilizado").clicked() {
                let keep_img = self.doc_style.fundo.eq_ignore_ascii_case("imagem");
                self.doc_style = DocStyle::default();
                if keep_img {
                    self.doc_style.fundo = "imagem".into();
                }
            }
        });
        ui.label(
            RichText::new(if self.doc_style.is_formal() {
                "Logo clássico, papel branco, tinta preta. Sem cartões nem ícones."
            } else {
                "Metal, ouro, contorno sem fundo preto, ícones."
            })
            .color(GOLD)
            .size(12.0),
        );
        ui.add_space(8.0);
        section_title(ui, "Fundo", false);
        ui.horizontal_wrapped(|ui| {
            let mut fundo = self.doc_style.fundo.clone();
            if ui.selectable_label(fundo == "auto", "Automático").clicked() {
                fundo = "auto".into();
            }
            if ui.selectable_label(fundo == "branco", "Branco").clicked() {
                fundo = "branco".into();
            }
            if ui.selectable_label(fundo == "imagem", "Imagem").clicked() {
                fundo = "imagem".into();
            }
            self.doc_style.fundo = fundo;
            if self.doc_style.fundo.eq_ignore_ascii_case("imagem")
                && steel_button(ui, "Escolher fundo…", [150.0, 26.0])
            {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("Imagem", &["jpg", "jpeg", "png"])
                    .pick_file()
                {
                    match media::save_fundo_bg(&p) {
                        Ok(d) => self.set_status(true, format!("Fundo: {}", d.display())),
                        Err(e) => self.set_status(false, format!("{e:#}")),
                    }
                }
            }
            if media::find_fundo_bg().is_some() {
                ui.label(RichText::new("imagem definida").color(WHITE).size(12.0));
            }
        });
        ui.add_space(6.0);
        self.ensure_fundo_thumbs(ui.ctx());
        ui.horizontal_wrapped(|ui| {
            let mut pick: Option<PathBuf> = None;
            for (path, name, tex) in &self.fundo_thumbs {
                ui.vertical(|ui| {
                    let on = media::find_fundo_bg().as_ref() == Some(path)
                        || (self.doc_style.fundo.eq_ignore_ascii_case("imagem")
                            && media::fundo_bg_path() == *path);
                    let stroke = if on { 2.0_f32 } else { 1.0 };
                    if let Some(tex) = tex {
                        let img = ui.add(
                            egui::Image::new(tex)
                                .fit_to_exact_size(Vec2::new(72.0, 96.0))
                                .sense(egui::Sense::click()),
                        );
                        if img.clicked() {
                            pick = Some(path.clone());
                        }
                    }
                    ui.label(RichText::new(name).color(GOLD).size(11.0));
                    let _ = stroke;
                });
            }
            if let Some(p) = pick {
                match media::save_fundo_bg(&p) {
                    Ok(_) => {
                        self.doc_style.fundo = "imagem".into();
                        self.set_status(true, format!("Fundo: {}", p.display()));
                    }
                    Err(e) => self.set_status(false, format!("{e:#}")),
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            if steel_button(ui, "Abrir pasta Ícones", [170.0, 26.0]) {
                let _ = std::process::Command::new("explorer")
                    .arg(media::design_icones_dir())
                    .spawn();
            }
            if steel_button(ui, "Abrir pasta Fundos", [170.0, 26.0]) {
                let _ = std::process::Command::new("explorer")
                    .arg(media::design_fundos_dir())
                    .spawn();
            }
        });
        ui.add_space(8.0);
        section_title(ui, "Título", false);
        ui.add(
            egui::Slider::new(&mut self.doc_style.titulo_pt, 14.0..=36.0)
                .text("Tamanho")
                .suffix(" pt"),
        );
        ui.add(
            egui::Slider::new(&mut self.doc_style.espaco_titulo, 0.0..=12.0)
                .text("Espaço à volta")
                .suffix(" mm"),
        );
        ui.add_space(6.0);
        ui.add_enabled_ui(!self.doc_style.is_formal(), |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Tinta").color(GOLD));
                let mut tinta = if self.doc_style.ink_black() {
                    "preto"
                } else {
                    "ouro"
                };
                if ui.selectable_label(tinta == "ouro", "Ouro").clicked() {
                    tinta = "ouro";
                }
                if ui.selectable_label(tinta == "preto", "Preto").clicked() {
                    tinta = "preto";
                }
                self.doc_style.tinta = tinta.to_string();
            });
        });
        if self.doc_style.is_formal() {
            ui.label(
                RichText::new("Formal usa sempre tinta preta.")
                    .color(GOLD)
                    .size(12.0),
            );
        }
        ui.add_space(8.0);
        section_title(ui, "Cartões e corpo", false);
        ui.label(
            RichText::new("Os cartões são só o contorno dourado — o fundo da página vê-se através.")
                .color(WHITE)
                .size(12.0),
        );
        ui.add(
            egui::Slider::new(&mut self.doc_style.contorno_mm, 0.3..=1.8)
                .text("Contorno")
                .suffix(" mm"),
        );
        ui.add(
            egui::Slider::new(&mut self.doc_style.raio_mm, 0.0..=8.0)
                .text("Cantos")
                .suffix(" mm"),
        );
        ui.add(
            egui::Slider::new(&mut self.doc_style.logo_mm, 8.0..=24.0)
                .text("Logo")
                .suffix(" mm"),
        );
        ui.add(
            egui::Slider::new(&mut self.doc_style.corpo_pt, 8.0..=13.0)
                .text("Texto")
                .suffix(" pt"),
        );
        ui.add_enabled_ui(!self.doc_style.is_formal(), |ui| {
            ui.checkbox(&mut self.doc_style.icon, "Ícones nos títulos de secção");
        });
        ui.label(
            RichText::new(format!(
                "Ícones:  {}",
                media::design_icones_dir().display()
            ))
            .color(GOLD)
            .size(11.0),
        );
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            if gold_button(ui, "Pré-visualizar", [160.0, 32.0]) {
                let ctx = ui.ctx().clone();
                self.begin_style_preview(&ctx);
            }
            if gold_button(ui, "Guardar estilo", [160.0, 32.0]) {
                match self.doc_style.save(&self.root_path()) {
                    Ok(p) => {
                        writers::set_house_style(self.doc_style.clone());
                        self.set_status(true, format!("Estilo da casa: {}", p.display()));
                    }
                    Err(e) => self.set_status(false, format!("{e:#}")),
                }
            }
        });
    }

    fn begin_style_preview(&mut self, ctx: &egui::Context) {
        self.bind_pdf_style();
        if !self.scan.vin.is_empty()
            || !self.scan.matricula.is_empty()
            || !self.scan.dtcs.is_empty()
            || self.car_can_persist(&self.car)
        {
            self.begin_preview(ctx);
            return;
        }
        let mut sample = Scan {
            titulo: "Relatório".into(),
            data: today(),
            veiculo: "Exemplo de estilo".into(),
            ..Default::default()
        };
        sample.sumario = "Pré-visualização do aspeto. Não é um relatório real.".into();
        let tmp = std::env::temp_dir().join("vanguarda-preview.pdf");
        match writers::write_diag_pdf_with_photos(&tmp, &sample, "Vanguarda Automóvel", &[], &[]) {
            Ok(()) => match diag_parse::render_pdf_first_page_jpeg(&tmp) {
                Ok(jpeg) => match load_texture_bytes(ctx, &jpeg, "pdf-preview") {
                    Some(tex) => {
                        self.preview_tex = Some(tex);
                        self.show_preview = true;
                        self.set_status(true, "Pré-visualização da 1.ª página.");
                    }
                    None => self.set_status(false, "Não desenhei a pré-visualização."),
                },
                Err(e) => self.set_status(false, format!("Preview: {e:#}")),
            },
            Err(e) => self.set_status(false, format!("Não gerei o PDF: {e:#}")),
        }
    }

    fn ui_sistema(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "Sistema", false);
        ui.label(
            RichText::new(
                "Só a conta do software. Pessoas da empresa (Gil, Rodrigo) têm mesa Front/Back. Esta conta não é uma mesa.",
            )
            .color(WHITE)
            .size(12.0),
        );
        ui.add_space(8.0);
        let users = auth::load_users(&self.root_path());
        if !users.iter().any(|u| u.software) {
            ui.label(
                RichText::new(
                    "Ainda não há conta do software. Cria uma abaixo (nome Vanguarda) e tira o ADMIN do login de trabalho do Gil.",
                )
                .color(BRASS)
                .size(13.0),
            );
            ui.add_space(6.0);
        }
        let mut apply: Option<(String, Departamento, bool, bool)> = None;
        for u in &users {
            ui.group(|ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&u.nome).strong().color(WHITE));
                    ui.label(
                        RichText::new(u.mesa_label())
                            .color(BRASS)
                            .size(12.0),
                    );
                    if u.admin {
                        ui.label(RichText::new("ADMIN").color(BRASS).size(11.0));
                    }
                    if !u.activo {
                        ui.label(RichText::new("inactivo").color(RED).size(11.0));
                    }
                });
                if u.is_pedido() || u.is_recusado() {
                    ui.label(
                        RichText::new(format!(
                            "{} — abre Pessoas para aprovar (precisa de palavra-passe).",
                            u.estado_label()
                        ))
                        .color(BRASS)
                        .size(12.0),
                    );
                    if steel_button(ui, "Abrir em Pessoas", [180.0, 26.0]) {
                        self.selected_staff = u.nome.clone();
                        self.pessoa_filtro = if u.is_recusado() {
                            "recusado".into()
                        } else {
                            "pedido".into()
                        };
                        self.load_staff(&u.nome);
                        self.mode = Mode::Staff;
                    }
                } else if !u.software {
                    let mut d = u.departamento;
                    let mut admin = u.admin;
                    let mut activo = u.activo;
                    egui::ComboBox::from_id_salt(format!("sys-mesa-{}", u.nome))
                        .selected_text(d.label())
                        .show_ui(ui, |ui| {
                            for x in Departamento::all() {
                                ui.selectable_value(&mut d, x, x.label());
                            }
                        });
                    ui.checkbox(&mut admin, "Admin (overlay — preferir conta do software)");
                    ui.checkbox(&mut activo, "Activo");
                    if steel_button(ui, "Guardar mesa", [130.0, 24.0]) {
                        apply = Some((u.nome.clone(), d, admin, activo));
                    }
                }
                if u.nome == self.selected_staff || self.selected_staff.is_empty() {
                    // pick for reset/delete
                }
                if ui
                    .selectable_label(self.selected_staff == u.nome, "escolher")
                    .clicked()
                {
                    self.selected_staff = u.nome.clone();
                    if !u.software {
                        self.load_staff(&u.nome);
                    }
                }
            });
        }
        if let Some((nome, d, admin, activo)) = apply {
            match auth::set_user(&self.root_path(), &nome, d, admin, activo) {
                Ok(()) => {
                    if let Some(s) = self.session.as_mut() {
                        if s.nome == nome {
                            s.departamento = d;
                            s.admin = admin || s.software;
                            s.activo = activo;
                        }
                    }
                    self.set_status(true, format!("Actualizado {nome}."));
                }
                Err(e) => self.set_status(false, format!("{e:#}")),
            }
        }
        ui.separator();
        ui.label("Pessoas novas pedem-se em Pessoas. Aqui só o programa.");
        ui.add(
            egui::TextEdit::singleline(&mut self.sys_new_pass)
                .password(true)
                .hint_text("nova palavra-passe")
                .desired_width(220.0),
        );
        ui.horizontal_wrapped(|ui| {
            if steel_button(ui, "Repor passe", [140.0, 26.0]) {
                let nome = self.selected_staff.clone();
                match auth::reset_password(&self.root_path(), &nome, &self.sys_new_pass) {
                    Ok(()) => self.set_status(true, format!("Palavra-passe de {nome} reposta.")),
                    Err(e) => self.set_status(false, format!("{e:#}")),
                }
            }
            if red_button(ui, "Desactivar login", [160.0, 26.0]) {
                let nome = self.selected_staff.clone();
                if let Some(u) = users.iter().find(|x| x.nome == nome) {
                    match auth::set_user(
                        &self.root_path(),
                        &nome,
                        u.departamento,
                        u.admin,
                        false,
                    ) {
                        Ok(()) => self.set_status(true, format!("{nome} sem acesso.")),
                        Err(e) => self.set_status(false, format!("{e:#}")),
                    }
                }
            }
        });
        if !users.iter().any(|u| u.software) {
            ui.separator();
            section_title(ui, "Conta do software", false);
            ui.label("Não é uma mesa. Nome por omissão Vanguarda.");
            ui.add(
                egui::TextEdit::singleline(&mut self.sys_new_name)
                    .hint_text("Vanguarda")
                    .desired_width(220.0),
            );
            if gold_button(ui, "Criar conta do software", [240.0, 28.0]) {
                let nome = if self.sys_new_name.trim().is_empty() {
                    "Vanguarda"
                } else {
                    self.sys_new_name.trim()
                };
                match auth::add_software_user(&self.root_path(), nome, &self.sys_new_pass) {
                    Ok(_) => {
                        self.sys_new_name.clear();
                        self.sys_new_pass.clear();
                        self.set_status(true, "Conta do software criada.");
                    }
                    Err(e) => self.set_status(false, format!("{e:#}")),
                }
            }
        }
        if steel_button(ui, "Abrir Pessoas", [140.0, 26.0]) {
            self.mode = Mode::Staff;
        }
    }

    fn ui_merge_confirm(&mut self, ctx: &egui::Context) {
        let Some((from, to)) = self.merge_pending.clone() else {
            return;
        };
        let mut open = true;
        let mut go = false;
        let mut cancel = false;
        egui::Window::new(format!("Unir «{from}» → «{to}»?"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(&self.merge_preview);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if gold_button(ui, "Unir", [120.0, 30.0]) {
                        go = true;
                    }
                    if steel_button(ui, "Cancelar", [120.0, 30.0]) {
                        cancel = true;
                    }
                });
            });
        if !open || cancel {
            self.merge_pending = None;
            return;
        }
        if !go {
            return;
        }
        self.merge_pending = None;
        match people::merge_person(&self.root_path(), &from, &to) {
            Ok(msg) => {
                self.refresh_lists();
                if self.selected_staff == from {
                    self.load_staff(&to);
                }
                if let Some(s) = self.session.as_mut() {
                    if s.nome == from {
                        s.nome = to.clone();
                    }
                }
                audit::append(&self.root_path(), &self.who(), "unir-pastas", &format!("{from}->{to}"));
                self.set_status(true, msg);
            }
            Err(e) => self.set_status(false, format!("{e:#}")),
        }
    }

    fn ui_delete_confirm(&mut self, ctx: &egui::Context) {
        let Some(pending) = self.pending_delete.as_ref() else {
            return;
        };
        let title = match pending {
            PendingDelete::Client(n) => format!("Apagar cliente «{n}»?"),
            PendingDelete::Car { label, .. } => format!("Apagar viatura «{label}»?"),
            PendingDelete::Diag { label, .. } => format!("Apagar relatório «{label}»?"),
        };
        let mut open = true;
        let mut go = false;
        let mut cancel = false;
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Isto remove a pasta (ou os ficheiros do relatório) e não volta atrás.");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if red_button(ui, "Apagar", [120.0, 30.0]) {
                        go = true;
                    }
                    if steel_button(ui, "Cancelar", [120.0, 30.0]) {
                        cancel = true;
                    }
                });
            });
        if !open || cancel {
            self.pending_delete = None;
            return;
        }
        if !go {
            return;
        }
        let pending = self.pending_delete.take().unwrap();
        if matches!(&pending, PendingDelete::Diag { .. }) && !self.can_edit_relatorio() {
            self.set_status(false, "Este departamento só lê relatórios.");
            return;
        }
        let root = self.root_path();
        let result = match pending {
            PendingDelete::Client(n) => delete::delete_client(&root, &n).map(|p| {
                self.client = ClientFicha::default();
                self.selected_client.clear();
                format!("Apagado {}", p.display())
            }),
            PendingDelete::Car { label, path } => delete::delete_car_dir(&path).map(|_| {
                self.car = Carro::default();
                self.selected_car.clear();
                format!("Apagada viatura {label}")
            }),
            PendingDelete::Diag { label, path, client } => {
                delete::delete_diag(&root, &path, &client).map(|n| {
                    self.selected_diag.clear();
                    self.scan = Scan::default();
                    format!("Apagados {n} ficheiros de «{label}»")
                })
            }
        };
        match result {
            Ok(msg) => {
                self.refresh_lists();
                self.set_status(true, msg);
            }
            Err(e) => self.set_status(false, format!("{e:#}")),
        }
    }

    fn ui_inicio(&mut self, ui: &mut egui::Ui, plate: Option<egui::TextureId>) {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(500));
        if self.is_software() {
            self.ui_inicio_software(ui, plate);
            return;
        }
        let front = matches!(self.ala(), Some(Ala::Front));
        let now = chrono::Local::now();
        metal_chip(ui, plate, |ui| {
            ui.horizontal_wrapped(|ui| {
                let bay = if front { "FRONT" } else { "BACK" };
                ui.label(RichText::new(bay).color(BRASS).size(12.0).strong());
                ui.label(RichText::new("·").color(LINE));
                ui.label(
                    RichText::new(now.format("%H:%M").to_string())
                        .size(16.0)
                        .color(WHITE)
                        .strong(),
                );
                ui.label(
                    RichText::new(now.format("%d/%m/%Y").to_string())
                        .size(13.0)
                        .color(BRASS),
                );
                ui.label(RichText::new("·").color(LINE));
                ui.label(
                    RichText::new(format!(
                        "{} clientes  ·  {} carros  ·  {} diags",
                        self.pulse.clients, self.pulse.cars, self.pulse.diags
                    ))
                    .size(13.0)
                    .color(WHITE),
                );
            });
            if !front {
                ui.label(
                    RichText::new(format!(
                        "{} a aguardar peças    ·    {} em reparação",
                        self.pulse.awaiting_parts, self.pulse.in_repair
                    ))
                    .size(13.0)
                    .color(WHITE),
                );
            }
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if !front && self.can(Mode::Interiores) {
                    ui.label(
                        RichText::new("Interiores · ainda não à venda")
                            .color(BRASS)
                            .size(12.0),
                    );
                }
                if !self.is_software() && steel_button(ui, "Registar consumo", [160.0, 30.0]) {
                    self.nova_entrada = ConsumoEntry {
                        data: today(),
                        tipo: CONSUMO_TIPOS[0].to_string(),
                        quem_fez: self
                            .session
                            .as_ref()
                            .map(|u| u.nome.clone())
                            .unwrap_or_default(),
                        ..Default::default()
                    };
                    self.show_consumo_wiz = true;
                }
            });
        });
        ui.add_space(6.0);
        self.ui_bay_board(ui, plate);
        ui.add_space(6.0);
        let mut open_quote: Option<Quote> = None;
        let mut open_job: Option<Job> = None;
        if front && self.can(Mode::Trabalho) {
            metal_chip(ui, plate, |ui| {
                section_title(ui, "Orçamentos", false);
                let rascunho: Vec<&Quote> = self
                    .quotes
                    .iter()
                    .filter(|q| q.estado.eq_ignore_ascii_case("rascunho"))
                    .collect();
                let enviado: Vec<&Quote> = self
                    .quotes
                    .iter()
                    .filter(|q| q.estado.eq_ignore_ascii_case("enviado"))
                    .collect();
                let aceite: Vec<&Quote> = self
                    .quotes
                    .iter()
                    .filter(|q| q.estado.eq_ignore_ascii_case("aceite"))
                    .collect();
                ui.label(
                    RichText::new(format!(
                        "Rascunho {}  ·  enviado {}  ·  aceite {}",
                        rascunho.len(),
                        enviado.len(),
                        aceite.len()
                    ))
                    .color(WHITE)
                    .size(13.0),
                );
                for q in aceite.iter().chain(enviado.iter()).chain(rascunho.iter()).take(8) {
                    let line = format!("·  {}", q.label());
                    if board_hit(ui, &line) {
                        open_quote = Some((*q).clone());
                    }
                }
            });
            ui.add_space(6.0);
            metal_chip(ui, plate, |ui| {
                section_title(ui, "Ordens sem data", false);
                let undated: Vec<&Job> = self
                    .jobs
                    .iter()
                    .filter(|j| {
                        j.agendado.trim().is_empty()
                            && j.estado != "cancelado"
                            && j.estado != "entregue"
                    })
                    .collect();
                if undated.is_empty() {
                    ui.label(RichText::new("Nenhuma.").color(WHITE));
                } else {
                    for j in undated.iter().take(8) {
                        let line = format!("·  {}", j.label());
                        if board_hit(ui, &line) {
                            open_job = Some((*j).clone());
                        }
                    }
                }
            });
            ui.add_space(6.0);
        }
        if self.can(Mode::Agenda) {
            let days = db::week_days();
            let names = ["Seg", "Ter", "Qua", "Qui", "Sex", "Sáb", "Dom"];
            let today_s = today();
            let any = days.iter().any(|d| !db::jobs_on_day(&self.jobs, d).is_empty());
            if any {
                metal_chip(ui, plate, |ui| {
                    section_title(ui, "Esta semana", false);
                    ui.horizontal_wrapped(|ui| {
                        for (i, day) in days.iter().enumerate() {
                            let n = db::jobs_on_day(&self.jobs, day).len();
                            if n == 0 {
                                continue;
                            }
                            let lab = format!("{} {}", names[i], n);
                            let col = if *day == today_s { BRASS } else { WHITE };
                            ui.label(RichText::new(lab).color(col).size(12.0));
                        }
                    });
                });
                ui.add_space(6.0);
            }
        }
        let low: Vec<&Sku> = self.skus.iter().filter(|s| s.low()).collect();
        if front && !low.is_empty() && self.can(Mode::Stock) {
            metal_chip(ui, plate, |ui| {
                section_title(ui, "Stock baixo", false);
                for s in low.iter().take(6) {
                    ui.label(format!("·  {}  ({})", s.codigo, s.nome));
                }
            });
            ui.add_space(6.0);
        }
        let mut open_client: Option<String> = None;
        let mut open_diag: Option<String> = None;
        if front {
            metal_chip(ui, plate, |ui| {
                section_title(ui, "Fichas em rascunho", false);
                if self.pulse.drafts.is_empty() {
                    ui.label(RichText::new("Nenhuma.").color(WHITE));
                } else {
                    for n in &self.pulse.drafts {
                        if board_hit(ui, &format!("·  {n}")) {
                            open_client = Some(n.clone());
                        }
                    }
                }
            });
            ui.add_space(6.0);
        }
        if !front {
            metal_chip(ui, plate, |ui| {
                section_title(ui, "Últimos diagnósticos", false);
                if self.pulse.recent.is_empty() {
                    ui.label(RichText::new("Ainda não há relatórios.").color(WHITE));
                } else {
                    for line in &self.pulse.recent {
                        if board_hit(ui, &format!("·  {line}")) {
                            open_diag = Some(line.clone());
                        }
                    }
                }
            });
            ui.add_space(6.0);
        }
        if let Some(n) = open_client {
            if self.can(Mode::Cliente) {
                self.load_client(&n);
                self.mode = Mode::Cliente;
                self.desk_gate = DeskGate::Form;
            }
        }
        if let Some(q) = open_quote {
            self.quote = q;
            self.bind_quote_pdf_if_on_disk();
            self.mode = Mode::Trabalho;
            self.trab_kind = TrabKind::Quote;
            self.desk_gate = DeskGate::Form;
        }
        if let Some(j) = open_job {
            self.job = j;
            self.mode = Mode::Trabalho;
            self.trab_kind = TrabKind::Job;
            self.desk_gate = DeskGate::Form;
        }
        if let Some(label) = open_diag {
            if self.can(Mode::Diagnostico) {
                if let Some(d) = self
                    .saved_diags
                    .iter()
                    .find(|d| d.label == label)
                    .cloned()
                {
                    self.selected_diag = d.path.display().to_string();
                    self.load_saved_diag(&d.path, &d.client);
                    self.mode = Mode::Diagnostico;
                    self.desk_gate = DeskGate::Form;
                }
            }
        }
        if !self.is_software() {
            metal_chip(ui, plate, |ui| {
                self.ui_consumo_graph(ui);
            });
        }
    }

    fn ui_inicio_software(&mut self, ui: &mut egui::Ui, plate: Option<egui::TextureId>) {
        metal_chip(ui, plate, |ui| {
            section_title(ui, "Software", false);
            ui.label(
                RichText::new(
                    "Esta conta é o sysadmin do programa. Não é uma mesa Front/Back. O trabalho da empresa (Gil, Rodrigo) usa o login da pessoa.",
                )
                .color(WHITE)
                .size(13.0),
            );
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if gold_button(ui, "Sistema", [120.0, 30.0]) {
                    self.mode = Mode::Sistema;
                }
                if self.can(Mode::Staff) && steel_button(ui, "Pessoas", [120.0, 30.0]) {
                    self.mode = Mode::Staff;
                }
                if self.can_backup() && steel_button(ui, "Backup zip", [120.0, 30.0]) {
                    self.backup_zip();
                }
            });
        });
        ui.add_space(6.0);
        metal_chip(ui, plate, |ui| {
            section_title(ui, "Logins", false);
            for u in auth::load_users(&self.root_path()) {
                let off = if u.activo { "" } else { "  ·  inactivo" };
                ui.label(format!(
                    "·  {}  ·  {}{off}",
                    u.nome,
                    u.mesa_label()
                ));
            }
        });
        ui.add_space(6.0);
        let pairs = people::suggested_for(&self.root_path());
        if !pairs.is_empty() {
            metal_chip(ui, plate, |ui| {
                section_title(ui, "Pastas a unir", false);
                ui.label("Apelidos e nome completo ainda estão separados. Unir em Sistema.");
                for (a, b) in &pairs {
                    ui.label(format!("·  {a}  →  {b}"));
                }
            });
        }
    }

    fn ui_carro(&mut self, ui: &mut egui::Ui) {
        let door = ui_doors(ui, &mut self.desk_gate);
        if door == Some(DeskGate::Form) {
            let d = self.car.dono.clone();
            self.car = Carro::blank(&d);
            self.selected_car.clear();
            self.mark_clean();
        }
        if self.desk_gate == DeskGate::Menu {
            ui.label(
                RichText::new("Abrir uma viatura, criar uma nova, ou apagar. Procurar fica em Abrir.")
                    .color(WHITE)
                    .size(13.0),
            );
            return;
        }
        if self.desk_gate == DeskGate::Abrir || self.desk_gate == DeskGate::Apagar {
        ui.horizontal_wrapped(|ui| {
            ui.label("Filtro:");
            ui.add(
                egui::TextEdit::singleline(&mut self.filter_car)
                    .desired_width(140.0)
                    .hint_text("procurar"),
            );
            ui.label("Existente:");
            let mut pick = self.selected_car.clone();
            let q = self.filter_car.to_lowercase();
            egui::ComboBox::from_id_salt("carros")
                .selected_text(if pick.is_empty() {
                    "(escolher)".to_string()
                } else {
                    pick.clone()
                })
                .width(280.0)
                .show_ui(ui, |ui| {
                    for c in &self.cars {
                        let lab = c.label();
                        if q.is_empty() || lab.to_lowercase().contains(&q) {
                            ui.selectable_value(&mut pick, lab.clone(), lab);
                        }
                    }
                });
            if pick != self.selected_car {
                self.selected_car = pick.clone();
                if let Some(c) = self.cars.iter().find(|c| c.label() == pick) {
                    self.car = c.clone();
                    self.car.ensure_sistemas();
                    cars::tidy_car(&mut self.car);
                    if self.desk_gate == DeskGate::Apagar {
                        let path = cars::car_dir(&self.root_path(), &self.car);
                        self.pending_delete = Some(PendingDelete::Car {
                            label: self.car.label(),
                            path,
                        });
                    } else {
                        self.desk_gate = DeskGate::Form;
                    }
                }
            }
            ui.label("Dono:");
            let mut dono = self.car.dono.clone();
            egui::ComboBox::from_id_salt("car-dono")
                .selected_text(if dono.is_empty() {
                    "(obrigatório)".to_string()
                } else {
                    dono.clone()
                })
                .show_ui(ui, |ui| {
                    for n in &self.client_names {
                        ui.selectable_value(&mut dono, n.clone(), n);
                    }
                    ui.selectable_value(&mut dono, DONO_INTERNO.to_string(), DONO_INTERNO);
                });
            self.car.dono = dono;
        });
        } // abrir/apagar
        if self.desk_gate == DeskGate::Abrir || self.desk_gate == DeskGate::Apagar {
            return;
        }
        ui.separator();
        let est_before = self.car.estado_oficina.clone();
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Estado oficina").color(GOLD));
            let mut est = self.car.estado_oficina.clone();
            egui::ComboBox::from_id_salt("car-estado")
                .selected_text(if est.is_empty() {
                    "(escolher)".to_string()
                } else {
                    est.clone()
                })
                .show_ui(ui, |ui| {
                    for e in ESTADOS_OFICINA {
                        ui.selectable_value(&mut est, (*e).to_string(), *e);
                    }
                });
            self.car.estado_oficina = est;
        });
        if est_before != self.car.estado_oficina && self.car_can_persist(&self.car) {
            self.save_carro();
        }
        ui.add_space(6.0);
        section_title(ui, "Recado interno", false);
        ui.label(
            RichText::new("não sai no PDF do cliente")
                .color(GOLD)
                .size(12.0),
        );
        let rec = stretch_multiline(ui, &mut self.car.recado);
        if rec.lost_focus() && self.car_can_persist(&self.car) {
            let prev = self
                .cars
                .iter()
                .find(|c| c.label() == self.selected_car)
                .map(|c| c.recado.as_str())
                .unwrap_or("");
            if prev != self.car.recado {
                self.save_carro();
            }
        }
        if !self.car.recado_quem.trim().is_empty() {
            ui.label(
                RichText::new(format!(
                    "Último: {}  ·  {}",
                    self.car.recado_quem, self.car.recado_quando
                ))
                .color(GOLD)
                .size(12.0),
            );
        }
        grid_fields(ui, "carro", true, &mut [
            ("Matrícula", &mut self.car.matricula),
            ("VIN", &mut self.car.vin),
            ("Marca", &mut self.car.marca),
            ("Modelo", &mut self.car.modelo),
            ("Versão", &mut self.car.versao),
            ("Ano", &mut self.car.ano),
            ("Cor", &mut self.car.cor),
            ("Km", &mut self.car.km),
            ("Pneus", &mut self.car.pneus),
            ("Peças pendentes", &mut self.car.pecas_pendentes),
        ]);
        ui.add_space(8.0);
        section_title(ui, "Saúde da viatura", !self.can_edit_saude());
        ui.label(
            RichText::new("5 em ordem  ·  1 grave")
                .color(GOLD)
                .size(12.0),
        );
        self.car.ensure_sistemas();
        let saude = self.can_edit_saude();
        let n = self.car.sistemas.len();
        for i in 0..n {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [220.0, 20.0],
                    egui::Label::new(
                        RichText::new(&self.car.sistemas[i].categoria).color(WHITE),
                    ),
                );
                let cur = self.car.sistemas[i].estrelas;
                for star in (1u8..=5).rev() {
                    let on = cur >= star;
                    if star_hit(ui, i, star, on, saude) {
                        self.car.sistemas[i].estrelas = if cur == star { 0 } else { star };
                    }
                }
            });
        }
        section_title(ui, "Notas", false);
        stretch_multiline(ui, &mut self.car.notas);
        ui.add_space(8.0);
        self.ui_media_bar(ui, "Veiculo", Some("Diagnosticos"));
        ui.add_space(8.0);
        if gold_button(ui, "Guardar viatura", [220.0, 34.0]) {
            self.save_carro();
        }
    }

    fn open_car(&mut self, car: Carro) {
        self.car = car;
        self.car.ensure_sistemas();
        cars::tidy_car(&mut self.car);
        self.selected_car = self.car.label();
        self.mode = Mode::Carro;
        self.mark_clean();
    }

    fn recado_for_desk(&self, car: &Carro) -> bool {
        if car.recado.trim().is_empty() {
            return false;
        }
        let Some(me) = self.dept() else {
            return false;
        };
        if matches!(me, Departamento::Design | Departamento::Escritorio) {
            return !car.recado.trim().is_empty()
                && self
                    .session
                    .as_ref()
                    .map(|u| !car.recado_quem.eq_ignore_ascii_case(&u.nome))
                    .unwrap_or(true);
        }
        let users = auth::load_users(&self.root_path());
        match users
            .iter()
            .find(|u| u.nome.eq_ignore_ascii_case(car.recado_quem.trim()))
            .map(|u| u.departamento)
        {
            Some(d) => d != me,
            None => {
                let mine = self
                    .session
                    .as_ref()
                    .map(|u| u.nome.as_str())
                    .unwrap_or("");
                !car.recado_quem.eq_ignore_ascii_case(mine)
            }
        }
    }

    fn ui_bay_board(&mut self, ui: &mut egui::Ui, plate: Option<egui::TextureId>) {
        let fila: &[&str] = match self.dept() {
            Some(Departamento::Escritorio | Departamento::Design) => &[
                "A aguardar orçamento",
                "A aguardar cliente",
                "Pronto",
                "A aguardar Care",
                "Care pronto",
            ],
            Some(Departamento::Oficina) => &["A aguardar peças", "Em reparação"],
            Some(Departamento::Care) => ESTADOS_CARE,
            Some(Departamento::Interiores) => ESTADOS_INTERIORES,
            None => &[],
        };
        let cars = self.cars.clone();
        let queued: Vec<Carro> = cars
            .iter()
            .filter(|c| fila.iter().any(|e| c.estado_oficina == *e))
            .cloned()
            .collect();
        let notes: Vec<Carro> = cars
            .iter()
            .filter(|c| self.recado_for_desk(c))
            .cloned()
            .collect();
        let mut open: Option<Carro> = None;
        let mut advance: Option<Carro> = None;
        metal_chip(ui, plate, |ui| {
            section_title(ui, "Fila da oficina", false);
            ui.label(
                RichText::new("→ avança o estado")
                    .color(GOLD)
                    .size(12.0),
            );
            if queued.is_empty() {
                ui.label(RichText::new("Nada à espera neste departamento.").color(WHITE));
            } else {
                for c in &queued {
                    ui.horizontal(|ui| {
                        ui.push_id(c.label(), |ui| {
                            if gold_button(ui, "→", [36.0, 24.0]) {
                                advance = Some(c.clone());
                            }
                        });
                        let line = format!("·  {}  ·  {}", c.label(), c.estado_oficina);
                        if board_hit(ui, &line) {
                            open = Some(c.clone());
                        }
                    });
                }
            }
        });
        ui.add_space(6.0);
        metal_chip(ui, plate, |ui| {
            section_title(ui, "Recados dos outros", false);
            if notes.is_empty() {
                ui.label(RichText::new("Nenhum recado para este departamento.").color(WHITE));
            } else {
                for c in &notes {
                    let msg = c.recado.replace('\n', " ");
                    let msg = if msg.chars().count() > 72 {
                        format!(
                            "{}…",
                            msg.chars().take(72).collect::<String>()
                        )
                    } else {
                        msg
                    };
                    let line = format!("·  {}  —  {} ({})", c.label(), msg, c.recado_quem);
                    if board_hit(ui, &line) {
                        open = Some(c.clone());
                    }
                }
            }
        });
        if let Some(c) = open {
            let care_st = ESTADOS_CARE.contains(&c.estado_oficina.as_str());
            let int_st = ESTADOS_INTERIORES.contains(&c.estado_oficina.as_str());
            if care_st && self.can(Mode::Care) {
                self.open_car(c);
                self.mode = Mode::Care;
            } else if int_st && self.can(Mode::Interiores) {
                self.open_car(c);
                self.mode = Mode::Interiores;
            } else if self.can(Mode::Diagnostico) {
                self.open_report(c);
            } else {
                self.open_car(c);
            }
        }
        if let Some(c) = advance {
            self.pending_advance = Some(c);
        }
    }

    fn apply_advance(&mut self) {
        let Some(mut c) = self.pending_advance.take() else {
            return;
        };
        let from = c.estado_oficina.clone();
        c.estado_oficina = if matches!(self.dept(), Some(Departamento::Care)) {
            next_estado_care(&c.estado_oficina).to_string()
        } else if matches!(self.dept(), Some(Departamento::Interiores)) {
            next_estado_interiores(&c.estado_oficina).to_string()
        } else {
            next_estado(&c.estado_oficina).to_string()
        };
        let to = c.estado_oficina.clone();
        self.persist_car(c, false, &format!("Estado: {from} → {to}"));
    }

    fn ui_advance_confirm(&mut self, ctx: &egui::Context) {
        let Some(c) = self.pending_advance.as_ref() else {
            return;
        };
        let nxt = if matches!(self.dept(), Some(Departamento::Care)) {
            next_estado_care(&c.estado_oficina)
        } else if matches!(self.dept(), Some(Departamento::Interiores)) {
            next_estado_interiores(&c.estado_oficina)
        } else {
            next_estado(&c.estado_oficina)
        };
        let title = format!("{}: {} → {}?", c.label(), c.estado_oficina, nxt);
        let mut open = true;
        let mut go = false;
        let mut cancel = false;
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Avançar o estado desta viatura na baía.");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if gold_button(ui, "Avançar", [120.0, 30.0]) {
                        go = true;
                    }
                    if steel_button(ui, "Cancelar", [120.0, 30.0]) {
                        cancel = true;
                    }
                });
            });
        if go {
            self.apply_advance();
        }
        if cancel || !open {
            self.pending_advance = None;
        }
    }

    fn car_can_persist(&self, car: &Carro) -> bool {
        !car.dono.trim().is_empty()
            && (!car.matricula.trim().is_empty() || !car.vin.trim().is_empty())
    }

    fn persist_car(&mut self, mut car: Carro, recado_changed: bool, ok_msg: &str) {
        if !self.car_can_persist(&car) {
            return;
        }
        self.bind_pdf_style();
        if recado_changed {
            if let Some(u) = &self.session {
                car.recado_quem = u.nome.clone();
                car.recado_quando = now_stamp();
            }
        }
        match cars::upsert_car(&self.root_path(), &car) {
            Ok(_) => {
                let lab = car.label();
                self.refresh_lists();
                if self.selected_car == lab || self.car.label() == lab {
                    if let Some(c) = self.cars.iter().find(|c| c.label() == lab) {
                        self.car = c.clone();
                        self.selected_car = lab;
                    }
                }
                self.set_status(true, ok_msg);
                self.mark_clean();
            }
            Err(e) => self.set_status(false, format!("{e:#}")),
        }
    }

    fn save_carro(&mut self) {
        self.bind_pdf_style();
        if !self.can_edit_saude() {
            if let Some(old) = self.cars.iter().find(|c| c.label() == self.selected_car) {
                self.car.sistemas = old.sistemas.clone();
            }
        }
        let prev = self
            .cars
            .iter()
            .find(|c| c.label() == self.selected_car)
            .map(|c| c.recado.clone());
        let recado_changed = match prev {
            Some(old) => old != self.car.recado,
            None => !self.car.recado.trim().is_empty(),
        };
        if recado_changed {
            if let Some(u) = &self.session {
                self.car.recado_quem = u.nome.clone();
                self.car.recado_quando = now_stamp();
            }
        }
        match cars::upsert_car(&self.root_path(), &self.car) {
            Ok(dir) => {
                self.refresh_lists();
                self.selected_car = self.car.label();
                if let Some(c) = self.cars.iter().find(|c| c.label() == self.selected_car) {
                    self.car = c.clone();
                }
                self.set_status(true, format!("Viatura guardada em {}", dir.display()));
                self.mark_clean();
            }
            Err(e) => self.set_status(false, format!("{e:#}")),
        }
    }

    fn ui_cliente(&mut self, ui: &mut egui::Ui) {
        let door = ui_doors(ui, &mut self.desk_gate);
        if door == Some(DeskGate::Form) {
            self.client = ClientFicha::default();
            self.selected_client.clear();
            self.new_name.clear();
            self.mark_clean();
        }
        if self.desk_gate == DeskGate::Menu {
            ui.label(
                RichText::new("Abrir uma ficha, criar uma nova, ou apagar. Procurar fica em Abrir.")
                    .color(WHITE)
                    .size(13.0),
            );
            return;
        }
        if self.desk_gate == DeskGate::Abrir || self.desk_gate == DeskGate::Apagar {
        ui.horizontal_wrapped(|ui| {
            ui.label("Filtro:");
            ui.add(
                egui::TextEdit::singleline(&mut self.filter_client)
                    .desired_width(140.0)
                    .hint_text("procurar"),
            );
        });
        let q = self.filter_client.to_lowercase();
        let mut pick = self.selected_client.clone();
        egui::ScrollArea::vertical()
            .id_salt("clientes-list")
            .max_height(120.0)
            .show(ui, |ui| {
                for n in &self.client_names {
                    if q.is_empty() || n.to_lowercase().contains(&q) {
                        if ui.selectable_label(pick == *n, n).clicked() {
                            pick = n.clone();
                        }
                    }
                }
            });
        if pick != self.selected_client && !pick.is_empty() {
            if self.desk_gate == DeskGate::Apagar {
                self.pending_delete = Some(PendingDelete::Client(pick.clone()));
            } else {
                self.load_client(&pick);
                self.desk_gate = DeskGate::Form;
            }
        }
        if self.desk_gate != DeskGate::Form {
            return;
        }
        } // abrir/apagar list
        ui.add_space(4.0);
        if self.desk_gate == DeskGate::Form && self.selected_client.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label("Nome:");
                ui.add(egui::TextEdit::singleline(&mut self.new_name).desired_width(200.0));
                if gold_button(ui, "Começar ficha", [150.0, 28.0]) {
                    let n = self.new_name.trim().to_string();
                    if n.is_empty() {
                        self.set_status(false, "Escreve o nome do novo cliente.");
                    } else if scaffold::is_locked_client(&n) {
                        self.set_status(false, "Esse nome está reservado.");
                    } else {
                        self.client = ClientFicha::blank(&n);
                        self.set_status(true, format!("Formulário novo: {n}"));
                    }
                }
            });
        }
        if self.client_locked() {
            ui.label(
                RichText::new("DJ xona bwé — meme da casa. Só leitura, para sempre.")
                    .color(GOLD)
                    .strong(),
            );
        }
        ui.horizontal_wrapped(|ui| {
            if gold_button(ui, "Carregar viatura…", [180.0, 28.0]) {
                if self.selected_client.is_empty() && self.client.nome_completo.trim().is_empty() {
                    self.set_status(false, "Escolhe o cliente primeiro.");
                } else if self.client_locked() {
                    self.set_status(false, "DJ xona bwé não se toca.");
                } else {
                    self.show_load_car = true;
                }
            }
        });
        ui.separator();
        if self.client_locked() {
            ui.label(
                RichText::new("Meme da casa — só se vê.")
                    .color(GOLD)
                    .size(13.0),
            );
            ro_pair(ui, "Nome", &self.client.nome_completo);
            ro_pair(ui, "Preferido", &self.client.nome_preferido);
            ro_pair(ui, "Telemóvel", &self.client.telemovel);
            ro_pair(ui, "Marca", &self.client.veiculo_marca);
            ro_pair(ui, "Modelo", &self.client.veiculo_modelo);
            ro_pair(ui, "Matrícula", &self.client.veiculo_matricula);
            ro_pair(ui, "VIN", &self.client.veiculo_vin);
            ro_pair(ui, "Ano", &self.client.veiculo_ano);
            ro_pair(ui, "Notas", &self.client.notas);
            return;
        }
        let pessoa = self.can_edit_pessoa();
        let wide = ui.available_width() >= 700.0;
        if wide {
            ui.columns(2, |cols| {
                section_title(&mut cols[0], "Identificação e contactos", !pessoa);
                grid_fields(&mut cols[0], "cliente-id", pessoa, &mut [
                    ("Nome completo", &mut self.client.nome_completo),
                    ("Nome preferido", &mut self.client.nome_preferido),
                    ("NIF", &mut self.client.nif),
                    ("Data nascimento", &mut self.client.data_nascimento),
                    ("Idioma", &mut self.client.idioma),
                    ("Telemóvel", &mut self.client.telemovel),
                    ("Email", &mut self.client.email),
                    ("Morada", &mut self.client.morada),
                    ("Código postal", &mut self.client.codigo_postal),
                    ("Localidade", &mut self.client.localidade),
                    ("Consent. contacto", &mut self.client.consentimento_contacto),
                    ("Consent. marketing", &mut self.client.consentimento_marketing),
                ]);
                self.ui_client_cars(&mut cols[1]);
                section_title(&mut cols[1], "Conta", !pessoa);
                self.apply_saldo_from_jobs();
                ro_pair(&mut cols[1], "Saldo aberto (entregue, não pago)", &self.client.saldo_aberto);
                grid_fields(&mut cols[1], "cliente-conta", pessoa, &mut [
                    ("Pagamento", &mut self.client.forma_pagamento),
                    ("Fatura com NIF", &mut self.client.fatura_com_nif),
                    ("Crédito", &mut self.client.credito),
                    ("Estado da ficha", &mut self.client.estado),
                ]);
            });
        } else {
            section_title(ui, "Identificação e contactos", !pessoa);
            grid_fields(ui, "cliente-id", pessoa, &mut [
                ("Nome completo", &mut self.client.nome_completo),
                ("Nome preferido", &mut self.client.nome_preferido),
                ("NIF", &mut self.client.nif),
                ("Data nascimento", &mut self.client.data_nascimento),
                ("Idioma", &mut self.client.idioma),
                ("Telemóvel", &mut self.client.telemovel),
                ("Email", &mut self.client.email),
                ("Morada", &mut self.client.morada),
                ("Código postal", &mut self.client.codigo_postal),
                ("Localidade", &mut self.client.localidade),
                ("Consent. contacto", &mut self.client.consentimento_contacto),
                ("Consent. marketing", &mut self.client.consentimento_marketing),
            ]);
            self.ui_client_cars(ui);
            section_title(ui, "Conta", !pessoa);
            self.apply_saldo_from_jobs();
            ro_pair(ui, "Saldo aberto (entregue, não pago)", &self.client.saldo_aberto);
            grid_fields(ui, "cliente-conta", pessoa, &mut [
                ("Pagamento", &mut self.client.forma_pagamento),
                ("Fatura com NIF", &mut self.client.fatura_com_nif),
                ("Crédito", &mut self.client.credito),
                ("Estado da ficha", &mut self.client.estado),
            ]);
        }
        ui.add_space(8.0);
        section_title(ui, "Preferências", !pessoa);
        stretch_multiline_enabled(ui, &mut self.client.preferencias, pessoa);
        section_title(ui, "Notas", !pessoa);
        stretch_multiline_enabled(ui, &mut self.client.notas, pessoa);
        ui.add_space(8.0);
        self.ui_media_bar(ui, "Antes", Some("Recibos"));
        ui.add_space(8.0);
        if gold_button(ui, "Guardar cliente", [220.0, 34.0]) {
            self.save_client();
        }
    }

    fn ui_client_cars(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "Viaturas", false);
        ui.label(
            RichText::new("A ficha da viatura vive em Carro. Aqui só se liga e abre.")
                .color(WHITE)
                .size(12.0),
        );
        let nome = if !self.client.nome_completo.trim().is_empty() {
            self.client.nome_completo.trim().to_string()
        } else {
            self.selected_client.clone()
        };
        let linked = cars::cars_for_owner(&self.cars, &nome);
        let mut open: Option<Carro> = None;
        if linked.is_empty() {
            ui.label(
                RichText::new("Nenhuma viatura ligada. «Carregar viatura…» ou criar em Carro.")
                    .color(WHITE)
                    .size(12.0),
            );
            if !self.client.veiculo_matricula.trim().is_empty()
                || !self.client.veiculo_vin.trim().is_empty()
            {
                ro_pair(ui, "Matrícula (ficha antiga)", &self.client.veiculo_matricula);
                ro_pair(ui, "VIN (ficha antiga)", &self.client.veiculo_vin);
            }
        } else {
            for c in &linked {
                let line = format!("·  {}  ·  {}", c.label(), c.estado_oficina);
                if board_hit(ui, &line) {
                    open = Some(c.clone());
                }
            }
        }
        if let Some(c) = open {
            self.open_car(c);
        }
    }

    fn ui_staff(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "Pessoas", false);
        ui.label(
            RichText::new(
                "O escritório pede. O administrador do software aprova ou recusa (e diz porquê), ou corrige a ficha e aprova.",
            )
            .color(WHITE)
            .size(12.0),
        );
        ui.add_space(6.0);
        let pane_h = (ui.available_height() - 12.0).max(380.0);
        ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(300.0, pane_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
        ui.horizontal_wrapped(|ui| {
            for (id, lab) in [
                ("pedido", "Pedidos"),
                ("activo", "Activos"),
                ("recusado", "Recusados"),
                ("todos", "Todos"),
            ] {
                let on = self.pessoa_filtro == id;
                if ui.selectable_label(on, lab).clicked() {
                    self.pessoa_filtro = id.to_string();
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Filtro:");
            ui.add(
                egui::TextEdit::singleline(&mut self.filter_staff)
                    .desired_width(180.0)
                    .hint_text("nome"),
            );
        });
        let users = auth::load_users(&self.root_path());
        let q = self.filter_staff.to_lowercase();
        let fil = self.pessoa_filtro.as_str();
        let mut rows: Vec<(String, String)> = users
            .iter()
            .filter(|u| !u.software)
            .filter(|u| match fil {
                "pedido" => u.is_pedido(),
                "recusado" => u.is_recusado(),
                "activo" => !u.is_pedido() && !u.is_recusado(),
                _ => true,
            })
            .filter(|u| q.is_empty() || u.nome.to_lowercase().contains(&q))
            .map(|u| (u.nome.clone(), u.estado_label().to_string()))
            .collect();
        for n in &self.staff_names {
            if users.iter().any(|u| u.nome == *n || u.software && u.nome == *n) {
                continue;
            }
            if q.is_empty() || n.to_lowercase().contains(&q) {
                rows.push((n.clone(), "PASTA".into()));
            }
        }
        let mut pick = self.selected_staff.clone();
        egui::ScrollArea::vertical()
            .id_salt("pessoas-list")
            .max_height(pane_h - 120.0)
            .show(ui, |ui| {
                for (n, est) in &rows {
                    let lab = format!("{n}  ·  {est}");
                    if ui.selectable_label(pick == *n, lab).clicked() {
                        pick = n.clone();
                    }
                }
            });
        if pick != self.selected_staff && !pick.is_empty() {
            self.load_staff(&pick);
            self.pessoa_motivo.clear();
            if let Some(u) = users.iter().find(|u| u.nome == pick) {
                self.pessoa_motivo = u.motivo.clone();
                self.login_dept = Departamento::from_slug(&u.mesa_pedida);
                if u.mesa_pedida.is_empty() {
                    self.login_dept = u.departamento;
                }
            }
        }
        ui.add_space(6.0);
        if self.can_pedir_pessoa() && gold_button(ui, "Novo pedido", [160.0, 28.0]) {
            self.staff = StaffFicha::default();
            self.consumo = ConsumoLog::default();
            self.selected_staff.clear();
            self.pessoa_motivo.clear();
            self.login_dept = Departamento::Escritorio;
            self.set_status(true, "Preenche a ficha e envia o pedido.");
        }
        });
        ui.separator();
        ui.vertical(|ui| {
        ui.set_min_width((ui.available_width() - 8.0).max(360.0));
        let edit = self.can_edit_staff_ficha();
        let reg = self.pessoa_registo();
        if let Some(u) = &reg {
            ui.label(
                RichText::new(format!("{}  ·  {}", u.estado_label(), u.mesa_label()))
                    .color(BRASS)
                    .size(13.0)
                    .strong(),
            );
            if !u.pedido_por.trim().is_empty() {
                ui.label(format!("Pedido por {}", u.pedido_por));
            }
            if u.is_recusado() && !u.motivo.trim().is_empty() {
                ui.label(
                    RichText::new(format!("Recusado: {}", u.motivo))
                        .color(RED)
                        .size(13.0),
                );
            }
        }
        if self.can_decidir_pessoa() {
            if let Some(u) = self.pessoa_registo() {
                if u.is_pedido() || u.is_recusado() {
                    section_title(ui, "Decisão", false);
                    ui.label(
                        RichText::new("Palavra-passe inicial (obrigatória para aprovar)")
                            .color(WHITE)
                            .size(12.0),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.pessoa_pass)
                            .password(true)
                            .desired_width(280.0)
                            .hint_text("mínimo 8"),
                    );
                    ui.label(RichText::new("Motivo se recusares").color(WHITE).size(12.0));
                    ui.add(
                        egui::TextEdit::multiline(&mut self.pessoa_motivo)
                            .desired_width(ui.available_width())
                            .desired_rows(2),
                    );
                    ui.horizontal_wrapped(|ui| {
                        if gold_button(ui, "Aprovar", [140.0, 32.0]) {
                            self.save_staff();
                            match auth::aprovar(
                                &self.root_path(),
                                &u.nome,
                                self.login_dept,
                                &self.pessoa_pass,
                                &self.who(),
                            ) {
                                Ok(()) => {
                                    self.pessoa_pass.clear();
                                    self.pessoa_filtro = "activo".into();
                                    self.refresh_lists();
                                    self.set_status(true, format!("{} pode entrar.", u.nome));
                                }
                                Err(e) => self.set_status(false, format!("{e:#}")),
                            }
                        }
                        if red_button(ui, "Recusar", [120.0, 30.0]) {
                            match auth::recusar(
                                &self.root_path(),
                                &u.nome,
                                &self.pessoa_motivo,
                                &self.who(),
                            ) {
                                Ok(()) => {
                                    self.pessoa_filtro = "recusado".into();
                                    self.refresh_lists();
                                    self.set_status(true, format!("{} recusado.", u.nome));
                                }
                                Err(e) => self.set_status(false, format!("{e:#}")),
                            }
                        }
                        if u.is_recusado() && steel_button(ui, "Reabrir pedido", [160.0, 28.0]) {
                            match auth::reabrir(&self.root_path(), &u.nome) {
                                Ok(()) => {
                                    self.pessoa_filtro = "pedido".into();
                                    self.refresh_lists();
                                    self.set_status(true, "Pedido reaberto.");
                                }
                                Err(e) => self.set_status(false, format!("{e:#}")),
                            }
                        }
                    });
                    ui.separator();
                }
            }
        }
        grid_fields(ui, "staff", edit, &mut [
            ("Nome", &mut self.staff.nome),
            ("Função / papel", &mut self.staff.funcao),
            ("Telemóvel", &mut self.staff.telemovel),
            ("Email", &mut self.staff.email),
            ("Data de início", &mut self.staff.data_inicio),
        ]);
        ui.label(RichText::new("Mesa").color(WHITE).size(12.0));
        ui.add_enabled_ui(edit || self.can_decidir_pessoa(), |ui| {
            let mut d = self.login_dept;
            egui::ComboBox::from_id_salt("pessoa-mesa")
                .selected_text(d.label())
                .show_ui(ui, |ui| {
                    for x in Departamento::all() {
                        ui.selectable_value(&mut d, x, x.label());
                    }
                });
            self.login_dept = d;
        });
        section_title(ui, "Notas", false);
        stretch_multiline(ui, &mut self.staff.notas);
        ui.add_space(8.0);
        self.ui_media_bar(ui, "Reparacoes", Some("Consumo"));
        ui.add_space(8.0);
        if edit && gold_button(ui, "Guardar ficha", [180.0, 32.0]) {
            self.save_staff();
        }
        if self.can_pedir_pessoa() && self.pessoa_registo().is_none() && !self.staff.nome.trim().is_empty()
        {
            ui.add_space(4.0);
            if gold_button(ui, "Enviar pedido", [180.0, 32.0]) {
                let nome = self.staff.nome.trim().to_string();
                match auth::create_pedido(
                    &self.root_path(),
                    &nome,
                    self.login_dept,
                    &self.who(),
                ) {
                    Ok(_) => {
                        self.save_staff();
                        self.selected_staff = nome.clone();
                        self.pessoa_filtro = "pedido".into();
                        self.refresh_lists();
                        self.set_status(true, format!("Pedido enviado: {nome}. À espera de aprovação."));
                    }
                    Err(e) => self.set_status(false, format!("{e:#}")),
                }
            }
        }
        if let Some(u) = self.pessoa_registo() {
            if !u.is_pedido() && !u.is_recusado() {
                ui.add_space(8.0);
                section_title(ui, "Consumo desta pessoa", false);
                if self.consumo.entradas.is_empty() {
                    ui.label(RichText::new("Ainda sem linhas.").color(WHITE).size(12.0));
                } else {
                    for (i, e) in self.consumo.entradas.iter().enumerate() {
                        ui.label(format!(
                            "{}.  {}  ·  {}  ·  {}  ·  {}",
                            i + 1,
                            e.data,
                            e.tipo,
                            e.descricao,
                            e.custo_interno
                        ));
                    }
                }
            }
        }
        });
        });
    }

    fn ui_diag_picker(&mut self, ui: &mut egui::Ui) {
        let apagar = self.desk_gate == DeskGate::Apagar;
        section_title(
            ui,
            if apagar {
                "Apagar relatório"
            } else {
                "Relatórios guardados"
            },
            false,
        );
        ui.label(
            RichText::new(if apagar {
                "Clica o relatório a apagar."
            } else {
                "Clica para abrir a visita."
            })
            .color(WHITE)
            .size(13.0),
        );
        ui.horizontal(|ui| {
            ui.label("Filtro:");
            ui.add(
                egui::TextEdit::singleline(&mut self.filter_diag)
                    .desired_width(220.0)
                    .hint_text("nome, cliente, matrícula"),
            );
        });
        let q = self.filter_diag.to_lowercase();
        let diags: Vec<_> = self
            .saved_diags
            .iter()
            .filter(|d| q.is_empty() || d.label.to_lowercase().contains(&q))
            .cloned()
            .collect();
        if diags.is_empty() {
            ui.add_space(4.0);
            ui.label(
                RichText::new("Nenhum relatório nesta pasta DATA.")
                    .color(WHITE)
                    .strong(),
            );
            ui.label(
                RichText::new(
                    "Procuro JSON e PDF em pastas Diagnosticos (cliente e carro). Importa um Autocom e usa Gravar, ou verifica o caminho DATA.",
                )
                .color(WHITE)
                .size(12.0),
            );
            if !self.root.trim().is_empty() {
                ui.label(
                    RichText::new(format!("Pasta: {}", self.root.trim()))
                        .color(GOLD)
                        .size(11.0),
                );
            }
            return;
        }
        ui.label(
            RichText::new(if diags.len() == 1 {
                "1 relatório".into()
            } else {
                format!("{} relatórios", diags.len())
            })
            .color(WHITE)
            .size(12.0),
        );
        ui.add_space(4.0);
        let mut chosen: Option<(PathBuf, String)> = None;
        let mut kill: Option<(String, PathBuf, String)> = None;
        let list_h = (ui.available_height() - 8.0).max(180.0);
        egui::ScrollArea::vertical()
            .id_salt("saved-diag-list")
            .max_height(list_h)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                for d in &diags {
                    let key = d.path.display().to_string();
                    let on = self.selected_diag == key;
                    let hid = ui.id().with(("drow", &key));
                    let hov = ui
                        .ctx()
                        .data(|d| d.get_temp::<bool>(hid))
                        .unwrap_or(false);
                    let fill = if on {
                        GOLD
                    } else {
                        Color32::from_rgb(0x14, 0x11, 0x0E)
                    };
                    let color = if on { DARK } else { WHITE };
                    let kind = if d.is_pdf { "PDF" } else { "ficha" };
                    let resp = ui.add_sized(
                        [ui.available_width(), 30.0],
                        egui::Button::new(
                            RichText::new(format!("{}   ({kind})", d.label))
                                .color(color)
                                .size(13.0)
                                .strong(),
                        )
                        .fill(fill)
                        .stroke(egui::Stroke::new(
                            if hov || on { 1.6_f32 } else { 1.0_f32 },
                            GOLD,
                        ))
                        .corner_radius(4.0),
                    );
                    ui.ctx().data_mut(|d| d.insert_temp(hid, resp.hovered()));
                    if resp.hovered() {
                        ui.ctx().request_repaint();
                    }
                    if resp.clicked() {
                        self.selected_diag = key;
                        if apagar {
                            kill = Some((d.label.clone(), d.path.clone(), d.client.clone()));
                        } else {
                            chosen = Some((d.path.clone(), d.client.clone()));
                        }
                    }
                }
            });
        if let Some((label, path, client)) = kill {
            self.pending_delete = Some(PendingDelete::Diag {
                label,
                path,
                client,
            });
        }
        if let Some((path, client)) = chosen {
            self.load_saved_diag(&path, &client);
        }
    }

    fn diag_source_live(&self) -> bool {
        self.scan.source == "live"
            || self.scan.source_name.to_lowercase().contains("vlinker")
            || self.scan.source_name.to_lowercase().contains("elm")
            || self.scan.source_name.eq_ignore_ascii_case("mock")
    }

    fn ui_diag_footer(&mut self, ui: &mut egui::Ui) {
        let edit = self.can_edit_relatorio();
        ui.horizontal_wrapped(|ui| {
            if edit && gold_button(ui, "Gravar", [120.0, 34.0]) {
                self.save_vanguarda_diag();
            }
            if steel_button(ui, "Pré-visualizar", [140.0, 30.0]) {
                let ctx = ui.ctx().clone();
                self.begin_preview(&ctx);
            }
            if steel_button(ui, "Abrir PDF", [120.0, 30.0]) {
                self.open_last_diag_pdf();
            }
            if gold_button(ui, "Exportar PDF", [150.0, 34.0]) {
                self.export_diag_pdf();
            }
            if edit
                && self.diag_pdf_saved
                && self.can_edit_ops()
                && gold_button(ui, "Fazer orçamento", [180.0, 34.0])
            {
                self.begin_orcamento_from_diag();
            }
            if edit
                && self.diag_pdf_saved
                && self.diag_source_live()
                && red_button(ui, "Apagar DTCs no adaptador", [220.0, 30.0])
            {
                self.begin_clear_dtcs();
            }
        });
        if edit && !self.diag_pdf_saved {
            ui.label(
                RichText::new("Grava para exportar e para fazer orçamento.")
                    .color(BRASS)
                    .size(11.0),
            );
        }
    }

    fn ui_diag_car_list(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Filtro:");
            let w = (ui.available_width() - 8.0).max(160.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.filter_report)
                    .desired_width(w)
                    .hint_text("matrícula, dono, VIN"),
            );
        });
        let q = self.filter_report.to_lowercase();
        let cars: Vec<Carro> = self
            .cars
            .iter()
            .filter(|c| {
                if q.is_empty() {
                    return true;
                }
                c.label().to_lowercase().contains(&q)
                    || c.vin.to_lowercase().contains(&q)
                    || c.marca.to_lowercase().contains(&q)
            })
            .cloned()
            .collect();
        let mut pick_car: Option<Carro> = None;
        let list_h = (ui.available_height() - 8.0).max(280.0);
        ui.set_min_height(list_h);
        egui::ScrollArea::vertical()
            .id_salt("report-car-list")
            .max_height(list_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                if cars.is_empty() {
                    ui.label(RichText::new("Nenhuma viatura — cria-a em Carro.").color(WHITE));
                }
                let row_w = ui.available_width();
                for c in &cars {
                    let on = self.selected_car == c.label();
                    let est = c.estado_oficina.trim();
                    let line = if est.is_empty() {
                        format!("{}{}", if on { "●  " } else { "·  " }, c.label())
                    } else {
                        format!("{}{}  ·  {est}", if on { "●  " } else { "·  " }, c.label())
                    };
                    if board_hit_wide(ui, &line, row_w) {
                        pick_car = Some(c.clone());
                    }
                }
            });
        if let Some(c) = pick_car {
            self.load_report_car(c);
        }
    }

    fn ui_diag_codes(&mut self, ui: &mut egui::Ui, edit: bool) {
        let n = self.scan.dtcs.len();
        section_title(
            ui,
            &if n == 0 {
                "Códigos".into()
            } else {
                format!("Códigos  ·  {n}")
            },
            !edit,
        );
        if self.scan.dtcs.is_empty() {
            ui.label(
                RichText::new("Nenhum código — importa o Autocom ou acrescenta à mão.")
                    .color(WHITE)
                    .size(12.0),
            );
        } else {
            let buckets = diag_parse::group_dtc_indices(&self.scan.dtcs);
            let mut remove = None;
            let avail = ui.available_width();
            for (cat, idxs) in &buckets {
                ui.label(
                    RichText::new(format!("{cat}  ·  {}", idxs.len()))
                        .color(GOLD)
                        .size(13.0)
                        .strong(),
                );
                for &i in idxs {
                    let sistema = self.scan.dtcs[i].sistema.trim().to_string();
                    let bucket = diag_parse::dtc_bucket(&self.scan.dtcs[i]);
                    ui.horizontal(|ui| {
                        ui.add_enabled(
                            edit,
                            egui::TextEdit::singleline(&mut self.scan.dtcs[i].codigo)
                                .desired_width(72.0)
                                .hint_text("DTC"),
                        );
                        ui.add_enabled(
                            edit,
                            egui::TextEdit::singleline(&mut self.scan.dtcs[i].descricao)
                                .desired_width((avail - 220.0).max(140.0))
                                .hint_text("o que está errado"),
                        );
                        ui.add_enabled(
                            edit,
                            egui::TextEdit::singleline(&mut self.scan.dtcs[i].estado)
                                .desired_width(90.0)
                                .hint_text("estado"),
                        );
                        if edit && ui.small_button("×").clicked() {
                            remove = Some(i);
                        }
                    });
                    if !sistema.is_empty() && sistema != bucket {
                        ui.label(
                            RichText::new(sistema)
                                .color(BRASS)
                                .size(11.0),
                        );
                    }
                }
                ui.add_space(4.0);
            }
            if let Some(i) = remove {
                self.scan.dtcs.remove(i);
            }
        }
        if edit {
            let w = ui.available_width().max(180.0);
            if steel_button(ui, "Acrescentar código", [w, 26.0]) {
                self.scan.dtcs.push(Dtc::default());
            }
        }
    }

    fn ui_diag_star_row(&mut self, ui: &mut egui::Ui, i: usize, saude: bool, col_w: f32) {
        ui.allocate_ui_with_layout(
            Vec2::new(col_w, 24.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_max_width(col_w);
                let name_w = (col_w - 130.0).clamp(80.0, 240.0);
                ui.add_sized(
                    [name_w, 18.0],
                    egui::Label::new(
                        RichText::new(&self.car.sistemas[i].categoria)
                            .color(WHITE)
                            .size(12.0),
                    )
                    .truncate(),
                );
                let cur = self.car.sistemas[i].estrelas;
                for star in (1u8..=5).rev() {
                    let on = cur >= star;
                    if star_hit(ui, i, star, on, saude) {
                        self.car.sistemas[i].estrelas = if cur == star { 0 } else { star };
                    }
                }
            },
        );
    }

    fn ui_diag_stars(&mut self, ui: &mut egui::Ui, saude: bool) {
        section_title(ui, "Saúde (1–5)", !saude);
        self.car.ensure_sistemas();
        let n_sys = self.car.sistemas.len();
        let avail = ui.available_width();
        let block = 96.0 + 130.0 + 8.0;
        let two = avail >= block * 2.0 + 12.0;
        if two {
            let col_w = ((avail - 12.0) / 2.0).floor().max(block);
            let mut i = 0;
            while i < n_sys {
                ui.horizontal(|ui| {
                    ui.set_max_width(avail);
                    self.ui_diag_star_row(ui, i, saude, col_w);
                    if i + 1 < n_sys {
                        ui.add_space(12.0);
                        self.ui_diag_star_row(ui, i + 1, saude, col_w);
                    }
                });
                i += 2;
            }
        } else {
            for i in 0..n_sys {
                self.ui_diag_star_row(ui, i, saude, avail);
            }
        }
    }

    fn ui_diag_history(&mut self, ui: &mut egui::Ui) {
        let prev = self.car_previous_diags();
        if prev.is_empty() {
            ui.label(
                RichText::new("Ainda sem relatórios nesta viatura.")
                    .color(WHITE)
                    .size(12.0),
            );
            return;
        }
        section_title(ui, "Nesta viatura", false);
        let mut open: Option<(PathBuf, String)> = None;
        for d in &prev {
            if board_hit(ui, &format!("·  {}", d.label)) {
                open = Some((d.path.clone(), d.client.clone()));
            }
        }
        if let Some((path, client)) = open {
            self.load_saved_diag(&path, &client);
        }
    }

    fn ui_diag_autocom(&mut self, ui: &mut egui::Ui, edit: bool) {
        section_title(ui, "Autocom", !edit);
        ui.label(
            RichText::new("Abre o PDF (ou larga-o nesta janela).")
                .color(WHITE)
                .size(12.0),
        );
        let w = ui.available_width().max(220.0);
        if gold_button(ui, "Abrir PDF Autocom…", [w, 34.0]) {
            self.open_autocom_pdf();
        }
        if !self.scan.source_name.trim().is_empty() {
            ui.label(
                RichText::new(format!("Lido: {}", self.scan.source_name))
                    .color(BRASS)
                    .size(12.0),
            );
        }
        ui.collapsing("Outras origens", |ui| {
            ui.horizontal_wrapped(|ui| {
                if steel_button(ui, "Log FORScan…", [160.0, 28.0]) {
                    self.open_forscan_txt();
                }
                if steel_button(ui, "Ler vLinker", [140.0, 28.0]) {
                    self.begin_read_vlinker(false);
                }
                if steel_button(ui, "Simular", [110.0, 28.0]) {
                    self.begin_read_vlinker(true);
                }
            });
        });
    }

    fn ui_diag_oficina(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Oficina (não sai no PDF)", |ui| {
            let est_before = self.car.estado_oficina.clone();
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Estado").color(BRASS));
                let mut est = self.car.estado_oficina.clone();
                egui::ComboBox::from_id_salt("diag-car-estado")
                    .selected_text(if est.is_empty() {
                        "(escolher)".to_string()
                    } else {
                        est.clone()
                    })
                    .show_ui(ui, |ui| {
                        for e in ESTADOS_OFICINA {
                            ui.selectable_value(&mut est, (*e).to_string(), *e);
                        }
                    });
                self.car.estado_oficina = est;
            });
            if est_before != self.car.estado_oficina {
                let c = self.car.clone();
                self.persist_car(c, false, "Estado da viatura guardado.");
            }
            ui.label(RichText::new("Recado interno").color(BRASS).size(12.0));
            let rec = stretch_multiline(ui, &mut self.car.recado);
            if rec.lost_focus() {
                let prev = self
                    .cars
                    .iter()
                    .find(|c| c.label() == self.car.label())
                    .map(|c| c.recado.as_str())
                    .unwrap_or("");
                if prev != self.car.recado {
                    let c = self.car.clone();
                    self.persist_car(c, true, "Recado guardado.");
                }
            }
        });
    }

    fn ui_diag_identity(&mut self, ui: &mut egui::Ui, edit: bool) {
        section_title(ui, "Viatura", false);
        ui.label(
            RichText::new(self.car.label())
                .color(GOLD)
                .size(14.0)
                .strong(),
        );
        ro_pair(ui, "Matrícula", &self.car.matricula);
        ro_pair(ui, "VIN", &self.car.vin);
        let veic = format!("{} {}", self.car.marca, self.car.modelo);
        if !veic.trim().is_empty() {
            ro_pair(ui, "Modelo", veic.trim());
        }
        if !self.scan.dtcs.is_empty() {
            ui.label(
                RichText::new(format!("{} códigos", self.scan.dtcs.len()))
                    .color(BRASS)
                    .size(12.0),
            );
        }
        ui.horizontal(|ui| {
            ui.label("Km:");
            let w = (ui.available_width() - 8.0).max(80.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.scan.km)
                    .desired_width(w)
                    .interactive(edit),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Data:");
            let w = (ui.available_width() - 8.0).max(80.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.scan.data)
                    .desired_width(w)
                    .interactive(edit),
            );
        });
        if edit {
            let w = ui.available_width().max(140.0);
            if steel_button(ui, "Trocar viatura", [w, 26.0]) {
                self.car = Carro::default();
                self.selected_car.clear();
                self.set_status(true, "Escolhe outra viatura. O Autocom fica.");
            }
        }
        ui.add_space(8.0);
        self.ui_diag_history(ui);
    }

    fn ui_diag_work(&mut self, ui: &mut egui::Ui, edit: bool, saude: bool) {
        if edit {
            self.ui_diag_autocom(ui, edit);
            ui.add_space(8.0);
        }
        self.ui_diag_codes(ui, edit);
        ui.add_space(8.0);
        section_title(ui, "Carta", !edit);
        let carta_w = ui.available_width().max(160.0);
        ui.add_enabled(
            edit,
            egui::TextEdit::multiline(&mut self.scan.sumario)
                .desired_width(carta_w)
                .desired_rows(8)
                .hint_text("O que viste, o que aconselhas, próximo passo…"),
        );
        ui.add_space(8.0);
        self.ui_diag_stars(ui, saude);
        ui.add_space(6.0);
        self.ui_diag_oficina(ui);
    }

    fn ui_diag_visit(&mut self, ui: &mut egui::Ui, edit: bool, saude: bool) {
        let avail = ui.available_width();
        let wide = avail >= 820.0;
        if wide {
            let gap = 16.0_f32;
            let left = (avail * 0.32).max(240.0);
            let right = (avail - left - gap).max(320.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_min_width(left);
                    ui.set_max_width(left);
                    ui.set_width(left);
                    self.ui_diag_identity(ui, edit);
                });
                ui.add_space(gap);
                ui.vertical(|ui| {
                    ui.set_min_width(right);
                    ui.set_max_width(right);
                    ui.set_width(right);
                    self.ui_diag_work(ui, edit, saude);
                });
            });
        } else {
            self.ui_diag_identity(ui, edit);
            ui.add_space(8.0);
            self.ui_diag_work(ui, edit, saude);
        }
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);
        self.ui_diag_footer(ui);
    }

    fn ui_diagnostico(&mut self, ui: &mut egui::Ui) {
        let edit = self.can_edit_relatorio();
        let saude = self.can_edit_saude();
        let door = ui_doors(ui, &mut self.desk_gate);
        if door == Some(DeskGate::Form) {
            self.scan = Scan::default();
            self.diag_pdf_saved = false;
            self.last_diag_pdf = None;
            self.selected_diag.clear();
            self.selected_car.clear();
            self.diag_client.clear();
            self.car = Carro::default();
            self.mark_clean();
        }
        if self.desk_gate == DeskGate::Menu {
            ui.label(
                RichText::new("Abrir um relatório gravado, nova visita (escolhe o carro), ou apagar. Procurar fica em Abrir. Podes largar o PDF Autocom nesta janela.")
                    .color(WHITE)
                    .size(13.0),
            );
            return;
        }
        if self.desk_gate == DeskGate::Abrir || self.desk_gate == DeskGate::Apagar {
            self.ui_diag_picker(ui);
            return;
        }
        if !self.car_can_persist(&self.car) {
            section_title(ui, "Viatura desta visita", false);
            ui.label(
                RichText::new("Escolhe o carro. Depois entra o Autocom e a carta.")
                    .color(WHITE)
                    .size(13.0),
            );
            if !self.scan.source_name.trim().is_empty() {
                ui.label(
                    RichText::new(format!(
                        "Já li {} ({} códigos).",
                        self.scan.source_name,
                        self.scan.dtcs.len()
                    ))
                    .color(BRASS)
                    .size(12.0),
                );
            }
            self.ui_diag_car_list(ui);
            return;
        }
        self.ui_diag_visit(ui, edit, saude);
    }

    fn car_previous_diags(&self) -> Vec<cars::SavedDiag> {
        let client = self.diag_client.trim().to_lowercase();
        let plate = diag_parse::norm_id(&self.scan.matricula);
        let plate2 = diag_parse::norm_id(&self.car.matricula);
        let vin = diag_parse::norm_id(&self.scan.vin);
        let current = self.selected_diag.clone();
        self.saved_diags
            .iter()
            .filter(|d| {
                d.path.display().to_string() != current
                    && diag_belongs(&d.client, &d.label, &client, &plate, &plate2, &vin)
            })
            .take(5)
            .cloned()
            .collect()
    }

    fn open_autocom_pdf(&mut self) {
        if self.busy.is_some() || !self.can_edit_relatorio() {
            return;
        }
        let path = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .pick_file();
        let Some(path) = path else {
            return;
        };
        self.begin_read_autocom(path);
    }

    fn open_forscan_txt(&mut self) {
        if self.busy.is_some() || !self.can_edit_relatorio() {
            return;
        }
        let path = rfd::FileDialog::new()
            .add_filter("Texto FORScan", &["txt", "log"])
            .pick_file();
        let Some(path) = path else {
            return;
        };
        self.begin_read_autocom(path);
    }

    fn load_saved_diag(&mut self, path: &Path, client: &str) {
        let ext = path
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let json_sib = path.with_extension("json");
        let house_json = if ext == "json" {
            path.to_path_buf()
        } else if json_sib.is_file() {
            json_sib.clone()
        } else {
            PathBuf::new()
        };
        let loaded = if house_json.is_file() {
            match fs::read_to_string(&house_json) {
                Ok(s) => serde_json::from_str::<Scan>(&s).map_err(|e| anyhow::anyhow!("{e}")),
                Err(e) => Err(anyhow::anyhow!("{e}")),
            }
            .or_else(|_| {
                if ext == "pdf" {
                    diag_parse::parse_autocom_pdf(path)
                } else {
                    let pdf = path.with_extension("pdf");
                    if pdf.is_file() {
                        diag_parse::parse_autocom_pdf(&pdf)
                    } else {
                        Err(anyhow::anyhow!("JSON inválido e sem PDF irmão"))
                    }
                }
            })
        } else if ext == "pdf" {
            diag_parse::parse_autocom_pdf(path)
        } else {
            Err(anyhow::anyhow!("não encontro JSON nem PDF"))
        };
        match loaded {
            Ok(mut scan) => {
                if scan.titulo.trim().is_empty() {
                    scan.titulo = diag_parse::default_titulo(&scan);
                }
                scan.dtcs = diag_parse::enrich_dtcs(scan.dtcs);
                self.scan = scan;
                self.diag_client = client.to_string();
                self.selected_diag = path.display().to_string();
                self.bind_car_from_scan();
                let house = house_json.is_file();
                let pdf = if ext == "pdf" {
                    path.to_path_buf()
                } else {
                    path.with_extension("pdf")
                };
                if house && pdf.is_file() {
                    self.last_diag_pdf = Some(pdf);
                    self.diag_pdf_saved = true;
                } else {
                    self.last_diag_pdf = None;
                    self.diag_pdf_saved = false;
                }
                self.desk_gate = DeskGate::Form;
                self.mark_clean();
                self.set_status(
                    true,
                    format!("Aberto «{}» ({})", self.scan.titulo, client),
                );
            }
            Err(e) => self.set_status(false, format!("Não li o diagnóstico: {e:#}")),
        }
    }

    fn bind_car_from_scan(&mut self) {
        if let Some(car) =
            cars::find_car(&self.root_path(), &self.scan.vin, &self.scan.matricula)
        {
            self.car = car;
            self.car.ensure_sistemas();
            cars::tidy_car(&mut self.car);
            self.selected_car = self.car.label();
            if self.diag_client.trim().is_empty() {
                self.diag_client = self.car.dono.clone();
            }
            return;
        }
        let dono = self.diag_client.clone();
        let mut c = Carro::blank(&dono);
        c.matricula = self.scan.matricula.clone();
        c.vin = self.scan.vin.clone();
        c.km = self.scan.km.clone();
        let bits = diag_parse::split_veiculo(&self.scan.veiculo);
        c.marca = bits.marca;
        c.modelo = bits.modelo;
        c.versao = bits.versao;
        c.ano = bits.ano;
        c.ensure_sistemas();
        self.car = c;
        self.selected_car = self.car.label();
    }

    fn suggest_diag_client(&self) -> Option<String> {
        let vin = diag_parse::norm_id(&self.scan.vin);
        let plate = diag_parse::norm_id(&self.scan.matricula);
        if vin.is_empty() && plate.is_empty() {
            return None;
        }
        for name in &self.client_names {
            let dir = client_dir(&self.root_path(), name);
            if client_matches_scan(&dir, &vin, &plate) {
                return Some(name.clone());
            }
        }
        None
    }

    fn save_vanguarda_diag(&mut self) {
        if self.busy.is_some() {
            return;
        }
        if !self.can_edit_relatorio() {
            self.set_status(false, "Este departamento só lê relatórios.");
            return;
        }
        self.fill_scan_from_car();
        let cliente = self.diag_client.trim().to_string();
        if cliente.is_empty() {
            self.set_status(false, "Escolhe a viatura ou o cliente.");
            return;
        }
        if !self.car_can_persist(&self.car)
            && self.scan.vin.is_empty()
            && self.scan.dtcs.is_empty()
        {
            self.set_status(false, "Escolhe uma viatura ou importa um Autocom.");
            return;
        }
        if let Some(u) = &self.session {
            self.scan.mecanico = u.nome.clone();
        }
        let root = self.root_path();
        let scan = self.scan.clone();
        let car = self.car.clone();
        let mecanico = self
            .session
            .as_ref()
            .map(|u| u.nome.clone())
            .unwrap_or_default();
        let (tx, rx) = mpsc::channel();
        self.start_job(BusyKind::GravarRelatorio, rx);
        let style = self.doc_style.clone();
        std::thread::spawn(move || {
            writers::set_house_style(style);
            let r = persist_vanguarda_diag(&root, &cliente, scan, car, &mecanico);
            let msg = match r {
                Ok((pdf, car)) => JobOut::Saved {
                    pdf: Ok(pdf),
                    car,
                },
                Err(e) => JobOut::Saved {
                    pdf: Err(e),
                    car: None,
                },
            };
            let _ = tx.send(msg);
        });
    }

    fn begin_consolidate(&mut self) {
        self.conflicts = cars::scan_conflicts(&self.root_path());
        let has_hard = self.conflicts.iter().any(|c| !c.current.is_empty());
        if !has_hard {
            match cars::apply_conflicts(&self.root_path(), &self.conflicts) {
                Ok(n) => {
                    self.refresh_lists();
                    self.set_status(
                        true,
                        format!("Consolidado: {n} campos vazios preenchidos."),
                    );
                }
                Err(e) => self.set_status(false, format!("{e:#}")),
            }
            return;
        }
        self.consolidate_pick = false;
        self.show_consolidate = true;
    }

    fn ui_consolidate(&mut self, ctx: &egui::Context) {
        let mut open = self.show_consolidate;
        egui::Window::new("Consolidar informação")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.label("Há campos já preenchidos com valores diferentes.");
                ui.label("Os vazios são sempre preenchidos.");
                ui.add_space(8.0);
                if ui.button("Só preencher vazios").clicked() {
                    for c in &mut self.conflicts {
                        c.apply = c.current.is_empty();
                    }
                    self.finish_consolidate();
                }
                if gold_button(ui, "Substituir todos os conflitos", [280.0, 28.0]) {
                    for c in &mut self.conflicts {
                        c.apply = true;
                    }
                    self.finish_consolidate();
                }
                ui.checkbox(&mut self.consolidate_pick, "Escolher campos");
                if self.consolidate_pick {
                    egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                        for c in &mut self.conflicts {
                            if c.current.is_empty() {
                                continue;
                            }
                            ui.checkbox(
                                &mut c.apply,
                                format!(
                                    "{} / {}  está «{}»  →  «{}» ({})",
                                    c.client, c.field, c.current, c.incoming, c.source
                                ),
                            );
                        }
                    });
                    if gold_button(ui, "Aplicar escolhidos", [200.0, 28.0]) {
                        for c in &mut self.conflicts {
                            if c.current.is_empty() {
                                c.apply = true;
                            }
                        }
                        self.finish_consolidate();
                    }
                }
            });
        self.show_consolidate = open && self.show_consolidate;
    }

    fn finish_consolidate(&mut self) {
        match cars::apply_conflicts(&self.root_path(), &self.conflicts) {
            Ok(n) => {
                self.refresh_lists();
                self.set_status(true, format!("Consolidado: {n} alterações."));
            }
            Err(e) => self.set_status(false, format!("{e:#}")),
        }
        self.show_consolidate = false;
    }

    fn export_tab(&mut self) {
        let pdf = self.export_pdf_path();
        let Some(pdf) = pdf else {
            self.set_status(false, "Ainda não há PDF para exportar neste separador.");
            return;
        };
        self.begin_pdf_out(pdf, true);
    }

    fn export_pdf_path(&self) -> Option<PathBuf> {
        match self.mode {
            Mode::Inicio => None,
            Mode::Diagnostico => {
                let client_media = if self.diag_client.trim().is_empty() {
                    None
                } else {
                    Some(
                        client_dir(&self.root_path(), self.diag_client.trim())
                            .join("Media")
                            .join("Diagnosticos"),
                    )
                };
                let car_media = if self.car.dono.trim().is_empty() {
                    None
                } else {
                    Some(
                        cars::car_dir(&self.root_path(), &self.car)
                            .join("Media")
                            .join("Diagnosticos"),
                    )
                };
                client_media
                    .as_ref()
                    .and_then(|p| newest_pdf_prefix(p, "Diagnostico_"))
                    .or_else(|| {
                        car_media
                            .as_ref()
                            .and_then(|p| newest_pdf_prefix(p, "Diagnostico_"))
                    })
            }
            Mode::Carro => {
                let dir = cars::car_dir(&self.root_path(), &self.car);
                newest_pdf_prefix(&dir, "Ficha_Viatura_")
            }
            Mode::Cliente => {
                let dir = self.current_person_dir()?;
                newest_pdf_prefix(&dir, "Ficha_Cliente_")
            }
            Mode::Staff => {
                let dir = self.current_person_dir()?;
                newest_pdf_prefix(&dir, "Ficha_Staff_")
            }
            Mode::Documentos | Mode::Sistema | Mode::Guias | Mode::Care | Mode::Interiores | Mode::Trabalho | Mode::Agenda | Mode::Stock => None,
        }
    }
}

fn newest_pdf_prefix(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let rd = fs::read_dir(dir).ok()?;
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()).map(|x| x.eq_ignore_ascii_case("pdf")) != Some(true)
        {
            continue;
        }
        let name = p.file_name()?.to_string_lossy();
        if !name.starts_with(prefix) {
            continue;
        }
        let t = e.metadata().and_then(|m| m.modified()).ok()?;
        if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
            best = Some((t, p));
        }
    }
    best.map(|(_, p)| p)
}

fn client_matches_scan(dir: &Path, vin: &str, plate: &str) -> bool {
    if let Ok(s) = fs::read_to_string(dir.join("ficha.json")) {
        if let Ok(f) = serde_json::from_str::<ClientFicha>(&s) {
            if !vin.is_empty() && diag_parse::norm_id(&f.veiculo_vin) == vin {
                return true;
            }
            if !plate.is_empty() && diag_parse::norm_id(&f.veiculo_matricula) == plate {
                return true;
            }
        }
    }
    let diag_dir = dir.join("Media").join("Diagnosticos");
    let Ok(rd) = fs::read_dir(&diag_dir) else {
        return false;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        let Ok(s) = fs::read_to_string(&p) else {
            continue;
        };
        let Ok(scan) = serde_json::from_str::<Scan>(&s) else {
            continue;
        };
        if !vin.is_empty() && diag_parse::norm_id(&scan.vin) == vin {
            return true;
        }
        if !plate.is_empty() && diag_parse::norm_id(&scan.matricula) == plate {
            return true;
        }
    }
    false
}

fn metal_chip<R>(
    ui: &mut egui::Ui,
    tex: Option<egui::TextureId>,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    metal_chip_sized(ui, tex, false, add)
}

fn metal_chip_fill<R>(
    ui: &mut egui::Ui,
    tex: Option<egui::TextureId>,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    metal_chip_sized(ui, tex, true, add)
}

fn metal_chip_sized<R>(
    ui: &mut egui::Ui,
    tex: Option<egui::TextureId>,
    fill: bool,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let bg_idx = ui.painter().add(egui::Shape::Noop);
    let pad = 11.0_f32;
    let max = ui.available_rect_before_wrap().shrink(pad);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(max)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    let min_h = if fill { max.height().max(120.0) } else { 0.0 };
    child.set_min_size(Vec2::new(max.width(), min_h));
    let ret = add(&mut child);
    let content = child.min_rect();
    let mut widget = content.expand(10.0);
    widget.set_left(max.left());
    widget.set_right(max.right());
    let rounding = 6.0;
    let mut shapes: Vec<egui::Shape> = Vec::new();
    let _ = tex;
    shapes.push(egui::Shape::rect_filled(widget, rounding, PANEL));
    let t0 = ui
        .ctx()
        .data(|d| d.get_temp::<f64>(egui::Id::new("v-tab-t0")))
        .unwrap_or(0.0);
    let now = ui.ctx().input(|i| i.time);
    let fade = if t0 <= 0.0 {
        1.0
    } else {
        ((now - t0) / 0.10).clamp(0.0, 1.0) as f32
    };
    if fade < 1.0 {
        ui.ctx().request_repaint();
    }
    let stroke = Color32::from_rgba_unmultiplied(0x3A, 0x3A, 0x3C, (220.0 * fade) as u8);
    shapes.push(egui::Shape::rect_stroke(
        widget,
        rounding,
        egui::Stroke::new(1.0_f32, stroke),
        egui::StrokeKind::Inside,
    ));
    ui.painter().set(bg_idx, egui::Shape::Vec(shapes));
    ui.allocate_rect(widget.expand(1.0), egui::Sense::hover());
    ret
}

fn ui_doors(ui: &mut egui::Ui, gate: &mut DeskGate) -> Option<DeskGate> {
    ui.label(
        RichText::new("O que queres fazer?")
            .color(WHITE)
            .size(13.0),
    );
    let mut hit = None;
    ui.horizontal_wrapped(|ui| {
        let abrir = *gate == DeskGate::Abrir;
        let novo = *gate == DeskGate::Form;
        let apagar = *gate == DeskGate::Apagar;
        if if abrir {
            gold_button(ui, "Abrir", [110.0, 30.0])
        } else {
            steel_button(ui, "Abrir", [110.0, 30.0])
        } {
            *gate = DeskGate::Abrir;
            hit = Some(DeskGate::Abrir);
        }
        if if novo {
            gold_button(ui, "Novo", [110.0, 30.0])
        } else {
            steel_button(ui, "Novo", [110.0, 30.0])
        } {
            *gate = DeskGate::Form;
            hit = Some(DeskGate::Form);
        }
        if if apagar {
            gold_button(ui, "Apagar", [110.0, 30.0])
        } else {
            steel_button(ui, "Apagar", [110.0, 30.0])
        } {
            *gate = DeskGate::Apagar;
            hit = Some(DeskGate::Apagar);
        }
    });
    ui.add_space(6.0);
    hit
}

fn shop_tab(ui: &mut egui::Ui, current: &mut Mode, this: Mode, label: &str) -> bool {
    let on = *current == this;
    let id = ui.id().with(("tab-k", label));
    let now = ui.ctx().input(|i| i.time);
    let target = if on { 1.0_f32 } else { 0.0 };
    let prev = ui.ctx().data(|d| d.get_temp::<f32>(id)).unwrap_or(target);
    let dt = ui.ctx().input(|i| i.unstable_dt as f32).clamp(0.0, 0.05);
    let k = prev + (target - prev) * (1.0 - (-dt / 0.12).exp());
    ui.ctx().data_mut(|d| d.insert_temp(id, k));
    if (k - target).abs() > 0.01 {
        ui.ctx().request_repaint();
    }
    let fill = Color32::from_rgb(
        (0x16 as f32 + (0xB8 as f32 - 0x16 as f32) * k) as u8,
        (0x17 as f32 + (0xA0 as f32 - 0x17 as f32) * k) as u8,
        (0x18 as f32 + (0x78 as f32 - 0x18 as f32) * k) as u8,
    );
    let color = if k > 0.5 { DARK } else { WHITE };
    let hov_id = id.with("hov");
    let hovered = ui.ctx().data(|d| d.get_temp::<bool>(hov_id)).unwrap_or(false);
    let hov_k = {
        let hid = id.with("hovk");
        let target = if hovered { 1.0_f32 } else { 0.0 };
        let prev = ui.ctx().data(|d| d.get_temp::<f32>(hid)).unwrap_or(target);
        let dt = ui.ctx().input(|i| i.unstable_dt as f32).clamp(0.0, 0.05);
        let nk = prev + (target - prev) * (1.0 - (-dt / 0.08).exp());
        ui.ctx().data_mut(|d| d.insert_temp(hid, nk));
        nk
    };
    let stroke_c = if on {
        BRASS
    } else if hovered {
        Color32::from_rgb(0xC8, 0xB4, 0x90)
    } else {
        LINE
    };
    let resp = ui.add(
        egui::Button::new(
            RichText::new(label)
                .color(color)
                .size(14.0 + 0.6 * hov_k)
                .family(fonts::heading_family())
                .strong(),
        )
        .fill(fill)
        .stroke(egui::Stroke::new(if hovered { 1.6_f32 } else { 1.0_f32 }, stroke_c))
        .corner_radius(4.0),
    );
    ui.ctx()
        .data_mut(|d| d.insert_temp(hov_id, resp.hovered()));
    if resp.hovered() {
        ui.ctx().request_repaint();
    }
    if on {
        let r = resp.rect;
        ui.painter().hline(
            r.x_range(),
            r.bottom() - 1.0,
            egui::Stroke::new(2.0_f32, DARK),
        );
    }
    let _ = now;
    if resp.clicked() {
        *current = this;
        true
    } else {
        false
    }
}

fn red_button(ui: &mut egui::Ui, label: &str, size: [f32; 2]) -> bool {
    ui.add_sized(
        size,
        egui::Button::new(RichText::new(label).color(WHITE).size(13.0).strong())
            .fill(RED)
            .stroke(egui::Stroke::new(1.0_f32, GOLD))
            .corner_radius(4.0),
    )
    .clicked()
}

fn steel_button(ui: &mut egui::Ui, label: &str, size: [f32; 2]) -> bool {
    let id = ui.id().with(("steel", label));
    let now = ui.ctx().input(|i| i.time);
    let t0 = ui.ctx().data(|d| d.get_temp::<f64>(id)).unwrap_or(0.0);
    let flash = ((0.12 - (now - t0)).max(0.0) / 0.12) as f32;
    let hover = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(id.with("hov")))
        .unwrap_or(false);
    let stroke_w = 1.0 + 1.5 * flash + if hover { 0.4 } else { 0.0 };
    let resp = ui.add_sized(
        size,
        egui::Button::new(RichText::new(label).color(WHITE).size(13.0).strong())
            .fill(PANEL)
            .stroke(egui::Stroke::new(stroke_w, LINE))
            .corner_radius(4.0),
    );
    ui.ctx()
        .data_mut(|d| d.insert_temp(id.with("hov"), resp.hovered()));
    let clicked = resp.clicked();
    if clicked {
        ui.ctx().data_mut(|d| d.insert_temp(id, now));
        ui.ctx().request_repaint();
    } else if flash > 0.0 || resp.hovered() {
        ui.ctx().request_repaint();
    }
    clicked
}

fn board_hit(ui: &mut egui::Ui, line: &str) -> bool {
    board_hit_wide(ui, line, 0.0)
}

fn board_hit_wide(ui: &mut egui::Ui, line: &str, width: f32) -> bool {
    let id = ui.id().with(("board", line));
    let now = ui.ctx().input(|i| i.time);
    let t0 = ui.ctx().data(|d| d.get_temp::<f64>(id)).unwrap_or(0.0);
    let flash = ((0.12 - (now - t0)).max(0.0) / 0.12) as f32;
    let hov_id = id.with("hov");
    let hovered = ui.ctx().data(|d| d.get_temp::<bool>(hov_id)).unwrap_or(false);
    let stroke = if hovered || flash > 0.0 {
        egui::Stroke::new(1.0 + 1.2 * flash, GOLD)
    } else {
        egui::Stroke::new(0.0_f32, GOLD)
    };
    let btn = egui::Button::new(RichText::new(line).color(WHITE).size(13.0))
        .fill(Color32::TRANSPARENT)
        .stroke(stroke)
        .corner_radius(3.0);
    let r = if width > 8.0 {
        ui.add_sized([width, 28.0], btn)
    } else {
        ui.add(btn)
    };
    ui.ctx()
        .data_mut(|d| d.insert_temp(hov_id, r.hovered()));
    if r.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        ui.ctx().request_repaint();
    }
    if r.clicked() {
        ui.ctx().data_mut(|d| d.insert_temp(id, now));
        ui.ctx().request_repaint();
    } else if flash > 0.0 {
        ui.ctx().request_repaint();
    }
    r.clicked()
}

fn star_hit(ui: &mut egui::Ui, row: usize, star: u8, on: bool, enabled: bool) -> bool {
    let id = ui.id().with(("star", row, star));
    let now = ui.ctx().input(|i| i.time);
    let t0 = ui.ctx().data(|d| d.get_temp::<f64>(id)).unwrap_or(0.0);
    let pop = ((0.12 - (now - t0)).max(0.0) / 0.12) as f32;
    let size = 16.0 + 3.0 * pop;
    let mark = if on { "★" } else { "☆" };
    let resp = ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(mark).color(GOLD).size(size))
            .fill(DARK)
            .stroke(egui::Stroke::new(0.0_f32, GOLD))
            .min_size(Vec2::new(22.0, 22.0)),
    );
    let clicked = resp.clicked();
    if clicked {
        ui.ctx().data_mut(|d| d.insert_temp(id, now));
        ui.ctx().request_repaint();
    } else if pop > 0.0 {
        ui.ctx().request_repaint();
    }
    clicked
}

fn gold_button(ui: &mut egui::Ui, label: &str, size: [f32; 2]) -> bool {
    let id = ui.id().with(label);
    let now = ui.ctx().input(|i| i.time);
    let t0 = ui.ctx().data(|d| d.get_temp::<f64>(id)).unwrap_or(0.0);
    let flash = ((0.14 - (now - t0)).max(0.0) / 0.14) as f32;
    let hover = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(id.with("hov")))
        .unwrap_or(false);
    let brass = if hover && flash <= 0.0 {
        Color32::from_rgb(0xE2, 0xC0, 0x78)
    } else {
        Color32::from_rgb(
            (0xD4 as f32 + (255.0 - 0xD4 as f32) * flash) as u8,
            (0xB0 as f32 + (240.0 - 0xB0 as f32) * flash) as u8,
            (0x6A as f32 + (200.0 - 0x6A as f32) * flash) as u8,
        )
    };
    let scale = 1.0 - 0.07 * flash;
    let sized = [size[0] * scale, size[1] * (0.98 + 0.02 * (1.0 - flash))];
    let resp = ui.add_sized(
        sized,
        egui::Button::new(RichText::new(label).color(DARK).strong())
            .fill(brass)
            .stroke(egui::Stroke::new(if hover { 1.5_f32 } else { 1.0_f32 }, GOLD))
            .corner_radius(5.0),
    );
    ui.ctx()
        .data_mut(|d| d.insert_temp(id.with("hov"), resp.hovered()));
    let clicked = resp.clicked();
    if clicked {
        ui.ctx().data_mut(|d| d.insert_temp(id, now));
        ui.ctx().request_repaint();
    } else if flash > 0.0 || resp.hovered() {
        ui.ctx().request_repaint();
    }
    clicked
}

fn grid_fields(
    ui: &mut egui::Ui,
    id: &str,
    editable: bool,
    fields: &mut [(&str, &mut String)],
) {
    let avail = ui.available_width();
    let label_w = 150.0_f32.min(avail * 0.38).max(96.0);
    let field_w = (avail - label_w - 16.0).max(80.0);
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([12.0, 6.0])
        .min_col_width(label_w)
        .striped(true)
        .show(ui, |ui| {
            for (label, value) in fields.iter_mut() {
                ui.add_sized(
                    [label_w, 20.0],
                    egui::Label::new(RichText::new(*label).color(BRASS)),
                );
                let mut edit = egui::TextEdit::singleline(*value)
                    .desired_width(field_w)
                    .text_color(WHITE)
                    .background_color(DARK);
                if let Some(h) = format::field_hint(label) {
                    edit = edit.hint_text(h);
                }
                let resp = ui.add_enabled(editable, edit);
                if editable && resp.lost_focus() {
                    **value = format::format_named(label, value);
                }
                ui.end_row();
            }
        });
}

fn section_title(ui: &mut egui::Ui, title: &str, locked: bool) {
    ui.add_space(10.0);
    let left = ui.cursor().left();
    let right = ui.max_rect().right();
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title)
                .family(fonts::heading_family())
                .color(BRASS)
                .size(20.0)
                .strong(),
        );
        if locked {
            ui.label(RichText::new("só leitura").color(GOLD).size(11.0));
        }
    });
    let y = ui.cursor().top() - 2.0;
    ui.painter().hline(
        left..=right,
        y,
        egui::Stroke::new(1.0_f32, LINE),
    );
    ui.add_space(6.0);
}

fn ro_pair(ui: &mut egui::Ui, k: &str, v: &str) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [140.0, 18.0],
            egui::Label::new(RichText::new(k).color(GOLD)),
        );
        let show = if v.trim().is_empty() { "—" } else { v };
        ui.label(RichText::new(show).color(WHITE));
    });
}

fn diag_belongs(
    d_client: &str,
    d_label: &str,
    client: &str,
    plate: &str,
    plate2: &str,
    vin: &str,
) -> bool {
    let lab = d_label.to_uppercase().replace('-', "");
    let same_client = !client.is_empty() && d_client.to_lowercase() == client;
    let hit_plate = (!plate.is_empty() && lab.contains(plate))
        || (!plate2.is_empty() && lab.contains(plate2));
    let hit_vin = !vin.is_empty() && lab.contains(vin);
    same_client || hit_plate || hit_vin
}

fn parse_euro(s: &str) -> f32 {
    let t = s.replace("€", "").replace(' ', "").replace(',', ".");
    t.parse::<f32>().unwrap_or(0.0)
}

fn filter_people_names(root: &Path, names: Vec<String>) -> Vec<String> {
    let skip: Vec<String> = auth::load_users(root)
        .into_iter()
        .filter(|u| u.software)
        .map(|u| u.nome)
        .collect();
    names
        .into_iter()
        .filter(|n| !skip.iter().any(|s| s.eq_ignore_ascii_case(n)))
        .collect()
}

fn load_consumo_log(root: &Path, nome: &str) -> ConsumoLog {
    if nome.trim().is_empty() {
        return ConsumoLog::default();
    }
    let p = staff_dir(root, nome).join("consumo.json");
    fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| ConsumoLog::blank(nome))
}

fn draw_consumo_bars(ui: &mut egui::Ui, log: &ConsumoLog, grow: f32) {
    let mut totals: Vec<(&str, f32)> = CONSUMO_TIPOS
        .iter()
        .map(|t| {
            let sum: f32 = log
                .entradas
                .iter()
                .filter(|e| e.tipo == *t)
                .map(|e| parse_euro(&e.custo_interno))
                .sum();
            (*t, sum)
        })
        .filter(|(_, v)| *v > 0.01)
        .collect();
    if totals.is_empty() {
        let n = log.entradas.len() as f32;
        if n <= 0.0 {
            ui.label(RichText::new("Ainda sem linhas.").color(WHITE).size(12.0));
            return;
        }
        totals.push(("Linhas", n));
    }
    let max = totals.iter().map(|(_, v)| *v).fold(0.0_f32, f32::max).max(1.0);
    let w = ui.available_width().min(420.0);
    for (name, val) in totals {
        ui.label(RichText::new(format!("{name}  ·  {val:.2} €")).color(WHITE).size(11.0));
        let bar_w = (w * (val / max) * grow).max(2.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(bar_w, 10.0), Sense::hover());
        ui.painter().rect_filled(rect, 2.0, GOLD);
        ui.add_space(4.0);
    }
}

fn tidy_client_placeholders(f: &mut ClientFicha) {
    f.nif = format::blank_placeholder(&f.nif);
    f.data_nascimento = format::blank_placeholder(&f.data_nascimento);
    f.telemovel = format::blank_placeholder(&f.telemovel);
    f.email = format::blank_placeholder(&f.email);
    f.morada = format::blank_placeholder(&f.morada);
    f.codigo_postal = format::blank_placeholder(&f.codigo_postal);
    f.localidade = format::blank_placeholder(&f.localidade);
    f.veiculo_marca = format::blank_placeholder(&f.veiculo_marca);
    f.veiculo_modelo = format::blank_placeholder(&f.veiculo_modelo);
    f.veiculo_versao = format::blank_placeholder(&f.veiculo_versao);
    f.veiculo_matricula = format::blank_placeholder(&f.veiculo_matricula);
    f.veiculo_ano = format::blank_placeholder(&f.veiculo_ano);
    f.veiculo_cor = format::blank_placeholder(&f.veiculo_cor);
    f.veiculo_vin = format::blank_placeholder(&f.veiculo_vin);
    f.veiculo_km = format::blank_placeholder(&f.veiculo_km);
    f.pneus = format::blank_placeholder(&f.pneus);
    f.observacoes_veiculo = format::blank_placeholder(&f.observacoes_veiculo);
    f.forma_pagamento = format::blank_placeholder(&f.forma_pagamento);
    f.fatura_com_nif = format::blank_placeholder(&f.fatura_com_nif);
}

fn stretch_multiline(ui: &mut egui::Ui, value: &mut String) -> egui::Response {
    stretch_multiline_enabled(ui, value, true)
}

fn stretch_multiline_enabled(
    ui: &mut egui::Ui,
    value: &mut String,
    enabled: bool,
) -> egui::Response {
    let w = ui.available_width();
    ui.add_enabled(
        enabled,
        egui::TextEdit::multiline(value)
            .desired_width(w)
            .desired_rows(4),
    )
}

#[cfg(test)]
mod export_pick {
    use super::*;

    #[test]
    fn diagnostico_prefix_not_ficha() {
        let dir = std::env::temp_dir().join(format!("v-exp-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Ficha_Cliente_X.pdf"), b"%PDF-ficha").unwrap();
        fs::write(dir.join("Diagnostico_Y.pdf"), b"%PDF-diag").unwrap();
        let hit = newest_pdf_prefix(&dir, "Diagnostico_").unwrap();
        assert!(hit
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("Diagnostico_"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn previous_scans_match_plate_or_client() {
        assert!(super::diag_belongs(
            "Ana",
            "Diagnostico_60_HU_86_20-08-2026  —  Ana",
            "ana",
            "60HU86",
            "",
            "",
        ));
        assert!(super::diag_belongs(
            "Joana",
            "P161A — Joana",
            "",
            "60HU86",
            "",
            "WVGZZZ1TZ9W034244",
        ) == false);
        assert!(super::diag_belongs(
            "Joana",
            "WVGZZZ1TZ9W034244 Touran",
            "",
            "",
            "",
            "WVGZZZ1TZ9W034244",
        ));
    }
}
