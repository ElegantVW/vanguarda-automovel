use super::{ui_doors, DeskGate, TrabKind, *};

impl FichaApp {
    pub(super) fn save_quote(&mut self) -> bool {
        if !self.can_edit_ops() {
            self.set_status(false, "Este departamento não edita orçamentos.");
            return false;
        }
        if self.quote.cliente.trim().is_empty() {
            self.set_status(false, "O orçamento precisa de um cliente.");
            return false;
        }
        if self.quote.matricula.trim().is_empty() && self.quote.vin.trim().is_empty() {
            self.set_status(false, "Escolhe a viatura do cliente.");
            return false;
        }
        self.quote.iva_incluido = false;
        if self.quote.colaborador.trim().is_empty() {
            self.quote.colaborador = self.who();
        }
        self.bind_pdf_style();
        match db::open(&self.root_path()) {
            Ok(conn) => match db::save_quote(&conn, &mut self.quote) {
                Ok(_) => {
                    let dir = self.ops_dir();
                    let viatura = self.quote_veiculo_name();
                    match writers::write_quote_car(&dir, &self.quote, &viatura) {
                        Ok(w) => {
                            if let Some(p) = w.iter().find(|p| p.extension().and_then(|e| e.to_str()) == Some("pdf")) {
                                self.last_quote_pdf = Some(p.clone());
                            }
                            self.reload_ops();
                            self.mark_clean();
                            audit::append(
                                &self.root_path(),
                                &self.who(),
                                "guardar-orcamento",
                                &self.quote.numero,
                            );
                            self.set_status(true, format!("Orçamento {} gravado.", self.quote.numero));
                            true
                        }
                        Err(e) => {
                            self.set_status(false, format!("JSON gravado, PDF falhou: {e:#}"));
                            true
                        }
                    }
                }
                Err(e) => {
                    self.set_status(false, format!("{e:#}"));
                    false
                }
            },
            Err(e) => {
                self.set_status(false, format!("{e:#}"));
                false
            }
        }
    }

    pub(super) fn save_job(&mut self) -> bool {
        if !self.can_edit_ops()
            && !matches!(
                self.dept(),
                Some(Departamento::Oficina | Departamento::Care | Departamento::Interiores)
            )
        {
            self.set_status(false, "Este departamento não edita trabalhos.");
            return false;
        }
        if self.job.cliente.trim().is_empty() {
            self.set_status(false, "O trabalho precisa de um cliente.");
            return false;
        }
        if self.job.matricula.trim().is_empty() && self.job.vin.trim().is_empty() {
            self.set_status(false, "Escolhe a viatura.");
            return false;
        }
        self.bind_pdf_style();
        if self.job.estado == "entregue" && self.job.entregue.trim().is_empty() {
            self.job.entregue = today();
        }
        match db::open(&self.root_path()) {
            Ok(conn) => match db::save_job(&conn, &mut self.job) {
                Ok(_) => {
                    let dir = self.ops_dir();
                    let _ = writers::write_job(&dir, &self.job);
                    self.reload_ops();
                    self.mark_clean();
                    self.set_status(true, format!("Trabalho #{} gravado.", self.job.id));
                    true
                }
                Err(e) => {
                    self.set_status(false, format!("{e:#}"));
                    false
                }
            },
            Err(e) => {
                self.set_status(false, format!("{e:#}"));
                false
            }
        }
    }

    pub(super) fn save_sku_row(&mut self) -> bool {
        if !self.can_edit_ops() {
            self.set_status(false, "Este departamento não edita stock.");
            return false;
        }
        if self.sku.codigo.trim().is_empty() {
            self.set_status(false, "Código obrigatório.");
            return false;
        }
        if !self.sku_qty.trim().is_empty() {
            let t = self.sku_qty.replace(',', ".");
            if let Ok(q) = t.parse::<f64>() {
                self.sku.qty = q;
            }
        }
        match db::open(&self.root_path()) {
            Ok(conn) => match db::save_sku(&conn, &mut self.sku) {
                Ok(_) => {
                    self.reload_ops();
                    self.mark_clean();
                    self.set_status(true, format!("Stock {} gravado.", self.sku.codigo));
                    true
                }
                Err(e) => {
                    self.set_status(false, format!("{e:#}"));
                    false
                }
            },
            Err(e) => {
                self.set_status(false, format!("{e:#}"));
                false
            }
        }
    }

    pub(super) fn save_conta(&mut self) -> bool {
        if !self.can_edit_ops() {
            self.set_status(false, "Este departamento não edita contas.");
            return false;
        }
        if self.conta.cliente.trim().is_empty() {
            self.set_status(false, "A conta precisa de um cliente.");
            return false;
        }
        if self.conta.matricula.trim().is_empty() && self.conta.vin.trim().is_empty() {
            self.set_status(false, "Escolhe a viatura do cliente.");
            return false;
        }
        self.conta.iva_incluido = false;
        if self.conta.colaborador.trim().is_empty() {
            self.conta.colaborador = self.who();
        }
        self.bind_pdf_style();
        match db::open(&self.root_path()) {
            Ok(conn) => match db::save_conta(&conn, &mut self.conta) {
                Ok(_) => {
                    let dir = self.ops_dir();
                    let viatura = self.conta_veiculo_name();
                    match writers::write_conta_car(&dir, &self.conta, &viatura) {
                        Ok(w) => {
                            if let Some(p) = w
                                .iter()
                                .find(|p| p.extension().and_then(|e| e.to_str()) == Some("pdf"))
                            {
                                self.last_quote_pdf = Some(p.clone());
                            }
                            self.reload_ops();
                            self.mark_clean();
                            audit::append(
                                &self.root_path(),
                                &self.who(),
                                "guardar-conta",
                                &self.conta.numero,
                            );
                            self.set_status(true, format!("Conta {} gravada.", self.conta.numero));
                            true
                        }
                        Err(e) => {
                            self.set_status(false, format!("JSON gravado, PDF falhou: {e:#}"));
                            true
                        }
                    }
                }
                Err(e) => {
                    self.set_status(false, format!("{e:#}"));
                    false
                }
            },
            Err(e) => {
                self.set_status(false, format!("{e:#}"));
                false
            }
        }
    }

    fn conta_veiculo_name(&self) -> String {
        self.cars
            .iter()
            .find(|c| {
                (!self.conta.matricula.trim().is_empty()
                    && c.matricula == self.conta.matricula)
                    || (!self.conta.vin.trim().is_empty() && c.vin == self.conta.vin)
            })
            .map(|c| c.label())
            .unwrap_or_default()
    }

    fn emit_conta_from_quote(&mut self) {
        if !self.can_edit_ops() {
            return;
        }
        if self.quote.id == 0 && !self.save_quote() {
            return;
        }
        if self.quote.estado != "aceite" {
            self.set_status(false, "Aceita o orçamento antes de emitir a conta.");
            return;
        }
        let mut c = self.quote.clone();
        c.id = 0;
        c.numero.clear();
        c.estado = "emitida".into();
        c.parent_quote_id = Some(self.quote.id);
        c.created.clear();
        self.conta = c;
        if self.save_conta() {
            self.trab_kind = TrabKind::Conta;
            self.desk_gate = DeskGate::Form;
            self.set_status(true, format!("Conta {} emitida.", self.conta.numero));
        }
    }

    fn reset_conta_draft(&mut self) {
        self.conta = Quote::default();
        self.conta.colaborador = self.who();
        self.conta.estado = "rascunho".into();
        if let Some(d) = self.dept() {
            self.conta.tipo = match d {
                Departamento::Care => "care".into(),
                Departamento::Interiores => "pintura".into(),
                Departamento::Oficina => "oficina".into(),
                _ => "oficina".into(),
            };
        }
        self.last_quote_pdf = None;
        self.mark_clean();
    }

    fn open_job_desk(&mut self) {
        self.trab_kind = TrabKind::Job;
        self.desk_gate = DeskGate::Form;
    }

    fn accept_quote(&mut self) {
        if !self.can_edit_ops() {
            return;
        }
        if self.quote.id == 0 && !self.save_quote() {
            return;
        }
        if let Some(id) = self.quote.job_id {
            if let Some(j) = self.jobs.iter().find(|j| j.id == id).cloned() {
                self.job = j;
                self.open_job_desk();
                self.mark_clean();
                self.set_status(true, "Ordem já existia — aberta. Põe a data.");
                return;
            }
        }
        let mut j = Job::from_quote(&self.quote);
        self.quote.estado = "aceite".into();
        match db::open(&self.root_path()) {
            Ok(conn) => {
                if let Err(e) = db::save_job(&conn, &mut j) {
                    self.set_status(false, format!("{e:#}"));
                    return;
                }
                self.quote.job_id = Some(j.id);
                let _ = db::save_quote(&conn, &mut self.quote);
                let _ = writers::write_job(&self.ops_dir(), &j);
                self.job = j;
                self.reload_ops();
                self.open_job_desk();
                self.mark_clean();
                audit::append(
                    &self.root_path(),
                    &self.who(),
                    "aceitar-orcamento",
                    &self.quote.numero,
                );
                self.set_status(true, "Orçamento aceite — ordem criada. Põe a data.");
            }
            Err(e) => self.set_status(false, format!("{e:#}")),
        }
    }

    fn export_quote_pdf(&mut self) {
        if self.quote.cliente.trim().is_empty()
            || (self.quote.matricula.trim().is_empty() && self.quote.vin.trim().is_empty())
        {
            self.set_status(false, "Escolhe o cliente e a viatura.");
            return;
        }
        if self.quote.colaborador.trim().is_empty() {
            self.quote.colaborador = self.who();
        }
        if self.quote.numero.trim().is_empty() && !self.save_quote() {
            return;
        }
        self.bind_pdf_style();
        let dir = self.ops_dir();
        let viatura = self.quote_veiculo_name();
        match writers::write_quote_car(&dir, &self.quote, &viatura) {
            Ok(w) => {
                if let Some(p) = w
                    .iter()
                    .find(|p| p.extension().and_then(|e| e.to_str()) == Some("pdf"))
                {
                    self.last_quote_pdf = Some(p.clone());
                    self.finish_pdf_out(p, true);
                }
            }
            Err(e) => self.set_status(false, format!("PDF falhou: {e:#}")),
        }
    }

    pub(super) fn bind_quote_pdf_if_on_disk(&mut self) {
        if self.quote.numero.trim().is_empty() {
            self.last_quote_pdf = None;
            return;
        }
        let p = self.ops_dir().join(format!(
            "Orcamento_{}.pdf",
            crate::slug::slug(&self.quote.numero)
        ));
        self.last_quote_pdf = if p.is_file() { Some(p) } else { None };
    }

    pub(super) fn reset_quote_draft(&mut self) {
        self.quote = Quote::default();
        self.quote.colaborador = self.who();
        if let Some(d) = self.dept() {
            self.quote.tipo = match d {
                Departamento::Care => "care".into(),
                Departamento::Interiores => "pintura".into(),
                Departamento::Oficina => "oficina".into(),
                _ => "oficina".into(),
            };
        }
        self.line_desc.clear();
        self.line_eur.clear();
        self.peca_qty = "1".into();
        self.line_disc.clear();
        self.peca_custo.clear();
        self.peca_margem = "40".into();
        self.usd_edit.clear();
        self.mo_eur.clear();
        self.mo_qty = "1".into();
        self.mo_disc.clear();
        self.ops_line_err.clear();
        self.last_quote_pdf = None;
        self.mark_clean();
    }

    fn try_add_peca(&mut self) {
        let desc = self.line_desc.trim().to_string();
        let qty = self
            .peca_qty
            .replace(',', ".")
            .trim()
            .parse::<f64>()
            .unwrap_or(1.0);
        if ops::looks_usd(&self.peca_custo) && self.quote.usd_eur <= 0.0 {
            self.ops_line_err = "Indica o câmbio USD (1 USD = ? €) à direita.".into();
            self.set_status(false, self.ops_line_err.clone());
            return;
        }
        let custo = ops::parse_custo_cents(&self.peca_custo, self.quote.usd_eur);
        let margem = ops::parse_disc_pct(&self.peca_margem).unwrap_or(0.0);
        let mut cents = ops::parse_cents(&self.line_eur);
        if custo > 0 {
            cents = ops::sell_from_cost(custo, margem);
        }
        if desc.is_empty() || cents <= 0 {
            self.ops_line_err = "Indica a peça e o custo (ou o € de venda).".into();
            self.set_status(false, self.ops_line_err.clone());
            return;
        }
        self.ops_line_err.clear();
        let mut l = Line::peca(desc, qty, cents);
        l.custo_cents = custo;
        l.margem_pct = margem;
        l.custo_edit = self.peca_custo.trim().to_string();
        l.margem_edit = ops::fmt_disc_edit(margem);
        l.disc_pct = ops::parse_disc_pct(&self.line_disc).unwrap_or(0.0);
        l.disc_edit = ops::fmt_disc_edit(l.disc_pct);
        self.quote.linhas.push(l);
        self.quote.recompute();
        self.line_desc.clear();
        self.line_eur.clear();
        self.peca_qty = "1".into();
        self.line_disc.clear();
        self.peca_custo.clear();
        self.peca_margem = "40".into();
    }

    fn try_add_mao(&mut self) {
        let cents = ops::parse_cents(&self.mo_eur);
        if cents <= 0 {
            self.ops_line_err = "Indica as horas e o €/h.".into();
            self.set_status(false, self.ops_line_err.clone());
            return;
        }
        self.ops_line_err.clear();
        let qty = self
            .mo_qty
            .replace(',', ".")
            .trim()
            .parse::<f64>()
            .unwrap_or(1.0);
        let mut l = Line::mao("Mão de obra", qty, cents);
        l.disc_pct = ops::parse_disc_pct(&self.mo_disc).unwrap_or(0.0);
        l.disc_edit = ops::fmt_disc_edit(l.disc_pct);
        self.quote.linhas.push(l);
        self.quote.recompute();
        self.mo_eur.clear();
        self.mo_qty = "1".into();
        self.mo_disc.clear();
    }

    fn quote_fleet(&self) -> Vec<crate::model::Carro> {
        crate::cars::cars_for_owner(&self.cars, &self.quote.cliente)
    }

    fn quote_car_matches(c: &crate::model::Carro, q: &Quote) -> bool {
        let plate = crate::diag_parse::norm_id(&q.matricula);
        let vin = crate::diag_parse::norm_id(&q.vin);
        (!plate.is_empty() && crate::diag_parse::norm_id(&c.matricula) == plate)
            || (!vin.is_empty() && crate::diag_parse::norm_id(&c.vin) == vin)
    }

    fn bind_quote_car(&mut self, c: &crate::model::Carro) {
        self.quote.matricula = c.matricula.clone();
        self.quote.vin = c.vin.clone();
        self.pull_quote_notes_from_diag(false);
    }

    fn sync_quote_car(&mut self) {
        let fleet = self.quote_fleet();
        if fleet.iter().any(|c| Self::quote_car_matches(c, &self.quote)) {
            return;
        }
        if fleet.len() == 1 {
            self.bind_quote_car(&fleet[0]);
        } else {
            self.quote.matricula.clear();
            self.quote.vin.clear();
        }
    }

    fn quote_veiculo_name(&self) -> String {
        self.quote_fleet()
            .iter()
            .find(|c| Self::quote_car_matches(c, &self.quote))
            .map(|c| {
                let mm = [c.marca.trim(), c.modelo.trim()]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                if mm.is_empty() {
                    viatura_label(c)
                } else {
                    mm
                }
            })
            .unwrap_or_default()
    }

    fn ui_quote_viatura(&mut self, ui: &mut egui::Ui, edit: bool) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Viatura:");
            if self.quote.cliente.trim().is_empty() {
                ui.label(
                    RichText::new("Escolhe o cliente.")
                        .color(WHITE)
                        .size(13.0),
                );
                return;
            }
            let fleet = self.quote_fleet();
            if fleet.is_empty() {
                ui.label(
                    RichText::new("Este cliente não tem viatura. Cria em Carro.")
                        .color(WHITE)
                        .size(13.0),
                );
                return;
            }
            if edit && fleet.len() == 1 && !Self::quote_car_matches(&fleet[0], &self.quote) {
                self.bind_quote_car(&fleet[0]);
            }
            let current = fleet
                .iter()
                .find(|c| Self::quote_car_matches(c, &self.quote));
            let text = current
                .map(viatura_label)
                .unwrap_or_else(|| "(qual o carro?)".into());
            if !edit {
                ui.label(RichText::new(text).color(WHITE).size(13.0));
                return;
            }
            let mut pick_key = current
                .map(|c| format!("{}|{}", c.matricula, c.vin))
                .unwrap_or_default();
            let before = pick_key.clone();
            egui::ComboBox::from_id_salt("q-car")
                .selected_text(text)
                .width(280.0)
                .show_ui(ui, |ui| {
                    for c in &fleet {
                        let k = format!("{}|{}", c.matricula, c.vin);
                        ui.selectable_value(&mut pick_key, k, viatura_label(c));
                    }
                });
            if pick_key != before {
                if let Some(c) = fleet.iter().find(|c| {
                    format!("{}|{}", c.matricula, c.vin) == pick_key
                }) {
                    self.bind_quote_car(c);
                }
            }
        });
    }

    fn quote_people(&self) -> Vec<String> {
        let mut v: Vec<String> = crate::auth::activo(&self.root_path())
            .into_iter()
            .filter(|u| !u.software)
            .map(|u| u.nome)
            .collect();
        let me = self.who();
        if me != "—" && !me.is_empty() && !v.iter().any(|n| n == &me) {
            v.insert(0, me);
        }
        v
    }

    fn last_scan_for_quote(&self) -> Option<crate::diag_parse::Scan> {
        let client = self.quote.cliente.trim().to_lowercase();
        let plate = crate::diag_parse::norm_id(&self.quote.matricula);
        let vin = crate::diag_parse::norm_id(&self.quote.vin);
        if client.is_empty() && plate.is_empty() && vin.is_empty() {
            return None;
        }
        for d in &self.saved_diags {
            if !super::diag_belongs(&d.client, &d.label, &client, &plate, &plate, &vin) {
                continue;
            }
            let json = if d.is_pdf {
                d.path.with_extension("json")
            } else {
                d.path.clone()
            };
            if !json.is_file() {
                continue;
            }
            let Ok(s) = fs::read_to_string(&json) else {
                continue;
            };
            if let Ok(scan) = serde_json::from_str::<crate::diag_parse::Scan>(&s) {
                return Some(scan);
            }
        }
        None
    }

    fn pull_quote_notes_from_diag(&mut self, force: bool) {
        if !force && !self.quote.notas.trim().is_empty() {
            return;
        }
        match self.last_scan_for_quote() {
            Some(scan) => {
                let n = crate::diag_parse::notes_from_scan(&scan);
                if n.trim().is_empty() {
                    if force {
                        self.set_status(false, "O último relatório não tem notas.");
                    }
                    return;
                }
                self.quote.notas = n;
                if force {
                    self.set_status(true, "Notas do último relatório.");
                }
            }
            None if force => {
                self.set_status(false, "Não há relatório desta viatura.");
            }
            None => {}
        }
    }

    fn ui_quote_list(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Filtro:");
            ui.add(
                egui::TextEdit::singleline(&mut self.filter_ops)
                    .desired_width(160.0)
                    .hint_text("número, cliente"),
            );
        });
        let qf = self.filter_ops.to_lowercase();
        let mut pick_q: Option<Quote> = None;
        egui::ScrollArea::vertical()
            .id_salt("quote-list")
            .max_height(220.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                for q in &self.quotes {
                    if !qf.is_empty() && !q.label().to_lowercase().contains(&qf) {
                        continue;
                    }
                    let on = self.quote.id == q.id;
                    let line = format!("{}{}", if on { "●  " } else { "·  " }, q.label());
                    if board_hit(ui, &line) {
                        pick_q = Some(q.clone());
                    }
                }
            });
        if let Some(q) = pick_q {
            self.quote = q;
            self.bind_quote_pdf_if_on_disk();
            self.desk_gate = DeskGate::Form;
            self.mark_clean();
        }
    }

    fn ui_conta_list(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Filtro:");
            ui.add(
                egui::TextEdit::singleline(&mut self.filter_ops)
                    .desired_width(160.0)
                    .hint_text("número, cliente"),
            );
        });
        let qf = self.filter_ops.to_lowercase();
        let mut pick: Option<Quote> = None;
        egui::ScrollArea::vertical()
            .id_salt("conta-list")
            .max_height(220.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                for q in &self.contas {
                    if !qf.is_empty() && !q.label().to_lowercase().contains(&qf) {
                        continue;
                    }
                    let on = self.conta.id == q.id;
                    let line = format!("{}{}", if on { "●  " } else { "·  " }, q.label());
                    if board_hit(ui, &line) {
                        pick = Some(q.clone());
                    }
                }
            });
        if let Some(q) = pick {
            self.conta = q;
            self.desk_gate = DeskGate::Form;
            self.mark_clean();
        }
    }

    fn ui_conta_form(&mut self, ui: &mut egui::Ui, edit: bool) {
        std::mem::swap(&mut self.quote, &mut self.conta);
        self.ui_quote_form(ui, edit);
        std::mem::swap(&mut self.quote, &mut self.conta);
    }

    fn ui_quote_kind(&mut self, ui: &mut egui::Ui, mao: bool, edit: bool) {
        let usd = self.quote.usd_eur;
        let mut kill: Option<usize> = None;
        let mut dirty = false;
        for i in 0..self.quote.linhas.len() {
            if self.quote.linhas[i].is_mao() != mao {
                continue;
            }
            ui.push_id(("ql", i, mao), |ui| {
                ui.horizontal(|ui| {
                    let l = &mut self.quote.linhas[i];
                    if mao {
                        let name = if l.desc.trim().is_empty() {
                            "Mão de obra".to_string()
                        } else {
                            l.desc.clone()
                        };
                        ui.add_sized(
                            [120.0, 20.0],
                            egui::Label::new(RichText::new(name).color(WHITE).size(13.0)),
                        );
                        if ui
                            .add_enabled(
                                edit,
                                egui::DragValue::new(&mut l.qty)
                                    .speed(0.25)
                                    .range(0.01..=9_999.0)
                                    .max_decimals(2)
                                    .suffix(" h"),
                            )
                            .changed()
                        {
                            dirty = true;
                        }
                        if euro_drag(ui, &mut l.cents, edit) {
                            dirty = true;
                        }
                        ui.label(RichText::new("/h").color(WHITE).size(12.0));
                        if pct_box(ui, edit, "", &mut l.disc_edit, &mut l.disc_pct) {
                            dirty = true;
                        }
                        ui.add_sized(
                            [72.0, 20.0],
                            egui::Label::new(
                                RichText::new(ops::euro(l.total_cents()))
                                    .color(GOLD)
                                    .size(13.0),
                            ),
                        );
                    } else {
                        ui.add(
                            egui::TextEdit::singleline(&mut l.desc)
                                .desired_width(168.0)
                                .interactive(edit),
                        );
                        if ui
                            .add_enabled(
                                edit,
                                egui::DragValue::new(&mut l.qty)
                                    .speed(0.25)
                                    .range(0.01..=9_999.0)
                                    .max_decimals(2),
                            )
                            .changed()
                        {
                            dirty = true;
                        }
                        if edit
                            && custo_box(ui, true, usd, &mut l.custo_edit, &mut l.custo_cents, false)
                        {
                            l.apply_margin();
                            dirty = true;
                        }
                        if edit && pct_box(ui, true, "", &mut l.margem_edit, &mut l.margem_pct)
                        {
                            l.apply_margin();
                            dirty = true;
                        }
                        if euro_drag(ui, &mut l.cents, edit) {
                            dirty = true;
                        }
                        if pct_box(ui, edit, "", &mut l.disc_edit, &mut l.disc_pct) {
                            dirty = true;
                        }
                        ui.add_sized(
                            [72.0, 20.0],
                            egui::Label::new(
                                RichText::new(ops::euro(l.total_cents()))
                                    .color(GOLD)
                                    .size(13.0),
                            ),
                        );
                    }
                    if edit && ui.small_button("×").clicked() {
                        kill = Some(i);
                    }
                });
            });
        }
        if let Some(i) = kill {
            self.quote.linhas.remove(i);
            dirty = true;
        }
        if dirty {
            self.quote.recompute();
        }
    }

    fn job_fleet(&self) -> Vec<crate::model::Carro> {
        crate::cars::cars_for_owner(&self.cars, &self.job.cliente)
    }

    fn job_car_matches(c: &crate::model::Carro, j: &Job) -> bool {
        let plate = crate::diag_parse::norm_id(&j.matricula);
        let vin = crate::diag_parse::norm_id(&j.vin);
        (!plate.is_empty() && crate::diag_parse::norm_id(&c.matricula) == plate)
            || (!vin.is_empty() && crate::diag_parse::norm_id(&c.vin) == vin)
    }

    fn bind_job_car(&mut self, c: &crate::model::Carro) {
        self.job.matricula = c.matricula.clone();
        self.job.vin = c.vin.clone();
    }

    fn sync_job_car(&mut self) {
        let fleet = self.job_fleet();
        if fleet.iter().any(|c| Self::job_car_matches(c, &self.job)) {
            return;
        }
        if fleet.len() == 1 {
            self.bind_job_car(&fleet[0]);
        } else {
            self.job.matricula.clear();
            self.job.vin.clear();
        }
    }

    fn ui_job_viatura(&mut self, ui: &mut egui::Ui, edit: bool) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Viatura:");
            if self.job.cliente.trim().is_empty() {
                ui.label(
                    RichText::new("Escolhe o cliente.")
                        .color(WHITE)
                        .size(13.0),
                );
                return;
            }
            let fleet = self.job_fleet();
            if fleet.is_empty() {
                ui.label(
                    RichText::new("Este cliente não tem viatura. Cria em Carro.")
                        .color(WHITE)
                        .size(13.0),
                );
                return;
            }
            if edit && fleet.len() == 1 && !Self::job_car_matches(&fleet[0], &self.job) {
                self.bind_job_car(&fleet[0]);
            }
            let current = fleet
                .iter()
                .find(|c| Self::job_car_matches(c, &self.job));
            let text = current
                .map(viatura_label)
                .unwrap_or_else(|| "(qual o carro?)".into());
            if !edit {
                ui.label(RichText::new(text).color(WHITE).size(13.0));
                return;
            }
            let mut pick_key = current
                .map(|c| format!("{}|{}", c.matricula, c.vin))
                .unwrap_or_default();
            let before = pick_key.clone();
            egui::ComboBox::from_id_salt("j-car")
                .selected_text(text)
                .width(280.0)
                .show_ui(ui, |ui| {
                    for c in &fleet {
                        let k = format!("{}|{}", c.matricula, c.vin);
                        ui.selectable_value(&mut pick_key, k, viatura_label(c));
                    }
                });
            if pick_key != before {
                if let Some(c) = fleet
                    .iter()
                    .find(|c| format!("{}|{}", c.matricula, c.vin) == pick_key)
                {
                    self.bind_job_car(c);
                }
            }
        });
    }

    fn ui_job_list(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Filtro:");
            ui.add(
                egui::TextEdit::singleline(&mut self.filter_jobs)
                    .desired_width(160.0)
                    .hint_text("cliente, matrícula"),
            );
        });
        let qf = self.filter_jobs.to_lowercase();
        let mut pick_j: Option<Job> = None;
        egui::ScrollArea::vertical()
            .id_salt("job-list")
            .max_height(220.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                for j in &self.jobs {
                    if !qf.is_empty() && !j.label().to_lowercase().contains(&qf) {
                        continue;
                    }
                    let on = self.job.id == j.id;
                    let line = format!("{}{}", if on { "●  " } else { "·  " }, j.label());
                    if board_hit(ui, &line) {
                        pick_j = Some(j.clone());
                    }
                }
            });
        if let Some(j) = pick_j {
            self.job = j;
            self.desk_gate = DeskGate::Form;
            self.mark_clean();
        }
    }

    fn ui_job_form(&mut self, ui: &mut egui::Ui, edit: bool) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Cliente:");
            let mut cli = self.job.cliente.clone();
            egui::ComboBox::from_id_salt("j-cli")
                .selected_text(if cli.is_empty() {
                    "(obrigatório)".into()
                } else {
                    cli.clone()
                })
                .width(220.0)
                .show_ui(ui, |ui| {
                    for n in &self.client_names {
                        ui.selectable_value(&mut cli, n.clone(), n);
                    }
                });
            if cli != self.job.cliente {
                self.job.cliente = cli;
                self.sync_job_car();
            } else {
                self.job.cliente = cli;
            }
            ui.label("Departamento:");
            let mut t = self.job.tipo.clone();
            egui::ComboBox::from_id_salt("j-tipo")
                .selected_text(ops::tipo_label(&t))
                .show_ui(ui, |ui| {
                    for x in JOB_TIPOS {
                        ui.selectable_value(&mut t, (*x).to_string(), ops::tipo_label(x));
                    }
                });
            self.job.tipo = t;
            ui.label("Estado:");
            let mut e = self.job.estado.clone();
            egui::ComboBox::from_id_salt("j-est")
                .selected_text(ops::estado_label(&e))
                .show_ui(ui, |ui| {
                    for x in JOB_ESTADOS {
                        ui.selectable_value(&mut e, (*x).to_string(), ops::estado_label(x));
                    }
                });
            self.job.estado = e;
        });
        self.ui_job_viatura(ui, edit);
        ui.horizontal_wrapped(|ui| {
            ui.label("Agendado:");
            ui.add(
                egui::TextEdit::singleline(&mut self.job.agendado)
                    .desired_width(110.0)
                    .hint_text("AAAA-MM-DD"),
            );
            if edit && steel_button(ui, "Hoje", [70.0, 24.0]) {
                self.job.agendado = today();
            }
            ui.label("Previsão:");
            ui.label(RichText::new(ops::euro(self.job.previsao_cents)).color(GOLD));
            ui.checkbox(&mut self.job.pago, "Pago");
        });
        ui.label("Notas internas:");
        stretch_multiline(ui, &mut self.job.notas);
        ui.horizontal_wrapped(|ui| {
            if gold_button(ui, "Gravar trabalho", [170.0, 30.0]) {
                self.save_job();
            }
            if steel_button(ui, "CSV trabalhos", [140.0, 30.0]) {
                self.export_jobs_csv();
            }
        });
    }

    fn ui_quote_form(&mut self, ui: &mut egui::Ui, edit: bool) {
        if self.quote.colaborador.trim().is_empty() {
            self.quote.colaborador = self.who();
        }
        ui.horizontal_wrapped(|ui| {
            ui.label("Por:");
            let people = self.quote_people();
            let mut who = self.quote.colaborador.clone();
            egui::ComboBox::from_id_salt("q-por")
                .selected_text(if who.is_empty() {
                    "(quem)".into()
                } else {
                    who.clone()
                })
                .width(200.0)
                .show_ui(ui, |ui| {
                    for n in &people {
                        ui.selectable_value(&mut who, n.clone(), n);
                    }
                });
            self.quote.colaborador = who;
            ui.label("Para:");
            let mut cli = self.quote.cliente.clone();
            egui::ComboBox::from_id_salt("q-cli")
                .selected_text(if cli.is_empty() {
                    "(cliente)".into()
                } else {
                    cli.clone()
                })
                .width(220.0)
                .show_ui(ui, |ui| {
                    for n in &self.client_names {
                        ui.selectable_value(&mut cli, n.clone(), n);
                    }
                });
            if cli != self.quote.cliente {
                self.quote.cliente = cli;
                self.sync_quote_car();
            } else {
                self.quote.cliente = cli;
            }
        });
        self.ui_quote_viatura(ui, edit);
        ui.horizontal_wrapped(|ui| {
            ui.label("Departamento:");
            let mut t = self.quote.tipo.clone();
            egui::ComboBox::from_id_salt("q-tipo")
                .selected_text(ops::tipo_label(&t))
                .show_ui(ui, |ui| {
                    for x in JOB_TIPOS {
                        ui.selectable_value(&mut t, (*x).to_string(), ops::tipo_label(x));
                    }
                });
            self.quote.tipo = t;
            ui.label("Estado:");
            let mut e = self.quote.estado.clone();
            let estados: &[&str] = if self.trab_kind == TrabKind::Conta {
                CONTA_ESTADOS
            } else {
                QUOTE_ESTADOS
            };
            egui::ComboBox::from_id_salt("q-est")
                .selected_text(ops::estado_label(&e))
                .show_ui(ui, |ui| {
                    for x in estados {
                        ui.selectable_value(&mut e, (*x).to_string(), ops::estado_label(x));
                    }
                });
            self.quote.estado = e;
            if !self.quote.numero.trim().is_empty() {
                ui.label(
                    RichText::new(&self.quote.numero)
                        .color(GOLD)
                        .size(13.0),
                );
            }
        });
        ui.add_space(6.0);
        self.ui_quote_actions(ui, edit);
        ui.add_space(8.0);
        let avail = ui.available_width();
        let wide = edit && avail >= 900.0;
        if wide {
            let gap = 12.0_f32;
            let right = 250.0_f32.min(avail * 0.30).max(220.0);
            let left = (avail - right - gap).max(400.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width(left);
                    self.ui_quote_lines(ui, edit);
                });
                ui.add_space(gap);
                ui.vertical(|ui| {
                    ui.set_max_width(right);
                    self.ui_quote_house(ui, edit);
                });
            });
        } else {
            self.ui_quote_lines(ui, edit);
            if edit {
                ui.add_space(8.0);
                self.ui_quote_house(ui, edit);
            }
        }
    }

    fn ui_quote_actions(&mut self, ui: &mut egui::Ui, edit: bool) {
        let off = self.quote.discount_cents();
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!("Total  {}", ops::euro(self.quote.total_cents)))
                    .color(GOLD)
                    .size(18.0)
                    .strong(),
            );
            if off > 0 {
                ui.label(
                    RichText::new(format!("Desconto  −{}", ops::euro(off)))
                        .color(WHITE)
                        .size(13.0),
                );
            }
        });
        ui.add_space(4.0);
        let conta = self.trab_kind == TrabKind::Conta;
        ui.horizontal_wrapped(|ui| {
            if conta {
                if edit && gold_button(ui, "Nova conta", [140.0, 30.0]) {
                    self.reset_conta_draft();
                    self.quote = self.conta.clone();
                }
                if edit && gold_button(ui, "Gravar conta", [160.0, 30.0]) {
                    std::mem::swap(&mut self.quote, &mut self.conta);
                    self.save_conta();
                    std::mem::swap(&mut self.quote, &mut self.conta);
                }
                if gold_button(ui, "Exportar PDF", [160.0, 30.0]) {
                    std::mem::swap(&mut self.quote, &mut self.conta);
                    let _ = writers::write_conta_car(
                        &self.ops_dir(),
                        &self.conta,
                        &self.conta_veiculo_name(),
                    );
                    std::mem::swap(&mut self.quote, &mut self.conta);
                }
                if edit
                    && self.quote.estado != "paga"
                    && gold_button(ui, "Marcar paga", [140.0, 30.0])
                {
                    self.quote.estado = "paga".into();
                    std::mem::swap(&mut self.quote, &mut self.conta);
                    self.save_conta();
                    std::mem::swap(&mut self.quote, &mut self.conta);
                }
            } else {
                if edit && gold_button(ui, "Novo orçamento", [160.0, 30.0]) {
                    self.reset_quote_draft();
                }
                if edit && gold_button(ui, "Gravar orçamento", [180.0, 30.0]) {
                    self.save_quote();
                }
                if gold_button(ui, "Exportar PDF", [160.0, 30.0]) {
                    self.export_quote_pdf();
                }
                if edit && steel_button(ui, "Aceitar - trabalho", [180.0, 30.0]) {
                    self.accept_quote();
                }
                if edit
                    && self.quote.estado == "aceite"
                    && gold_button(ui, "Emitir conta", [160.0, 30.0])
                {
                    self.emit_conta_from_quote();
                }
                if steel_button(ui, "CSV orçamentos", [150.0, 30.0]) {
                    self.export_quotes_csv();
                }
            }
        });
    }

    fn ui_quote_lines(&mut self, ui: &mut egui::Ui, edit: bool) {
        ui.label(RichText::new("Mão de obra").color(GOLD).size(15.0).strong());
        if !self.quote.linhas.iter().any(|l| l.is_mao()) {
            ui.label(
                RichText::new("Ainda sem mão de obra.")
                    .color(WHITE)
                    .size(12.0),
            );
        }
        self.ui_quote_kind(ui, true, edit);
        if edit {
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.mo_qty)
                        .desired_width(60.0)
                        .hint_text("horas"),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.mo_eur)
                        .desired_width(80.0)
                        .hint_text("€/h"),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.mo_disc)
                        .desired_width(48.0)
                        .hint_text("%"),
                );
                if steel_button(ui, "Adicionar mão de obra", [180.0, 24.0]) {
                    self.try_add_mao();
                }
            });
        }
        ui.label(
            RichText::new(format!(
                "Subtotal mão de obra  {}",
                ops::euro(self.quote.mao_cents())
            ))
            .color(WHITE)
            .size(13.0),
        );
        ui.add_space(8.0);
        ui.label(RichText::new("Peças").color(GOLD).size(15.0).strong());
        if edit || self.quote.linhas.iter().any(|l| !l.is_mao()) {
            ui.horizontal(|ui| {
                for (w, t) in [
                    (168.0, "Peça"),
                    (48.0, "Qtd"),
                    (72.0, "Custo"),
                    (48.0, "Margem"),
                    (72.0, "Venda"),
                    (48.0, "%"),
                    (72.0, "Valor"),
                ] {
                    ui.add_sized(
                        [w, 16.0],
                        egui::Label::new(RichText::new(t).color(BRASS).size(11.0)),
                    );
                }
            });
        }
        if !self.quote.linhas.iter().any(|l| !l.is_mao()) {
            ui.label(
                RichText::new("Ainda sem peças.")
                    .color(WHITE)
                    .size(12.0),
            );
        }
        self.ui_quote_kind(ui, false, edit);
        if edit {
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.line_desc)
                        .desired_width(220.0)
                        .hint_text("peça"),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.peca_qty)
                        .desired_width(50.0)
                        .hint_text("qtd"),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.peca_custo)
                        .desired_width(72.0)
                        .hint_text("custo"),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.peca_margem)
                        .desired_width(48.0)
                        .hint_text("margem"),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.line_eur)
                        .desired_width(72.0)
                        .hint_text("venda"),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.line_disc)
                        .desired_width(48.0)
                        .hint_text("%"),
                );
                if steel_button(ui, "Adicionar peça", [140.0, 24.0]) {
                    self.try_add_peca();
                }
            });
        }
        ui.label(
            RichText::new(format!("Subtotal peças  {}", ops::euro(self.quote.pecas_cents())))
                .color(WHITE)
                .size(13.0),
        );
        if !self.ops_line_err.is_empty() {
            ui.label(
                RichText::new(&self.ops_line_err)
                    .color(RED)
                    .size(13.0),
            );
        }
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Notas").color(GOLD).size(15.0).strong());
            if edit && steel_button(ui, "Do último relatório", [180.0, 24.0]) {
                self.pull_quote_notes_from_diag(true);
            }
        });
        stretch_multiline(ui, &mut self.quote.notas);
    }

    fn ui_quote_house(&mut self, ui: &mut egui::Ui, edit: bool) {
        if !edit {
            return;
        }
        egui::Frame::new()
            .stroke(egui::Stroke::new(1.0_f32, GOLD))
            .corner_radius(6.0)
            .inner_margin(10.0)
            .fill(PANEL)
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width());
                self.ui_quote_house_inner(ui);
            });
    }

    fn ui_quote_house_inner(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("Contas da casa")
                .color(GOLD)
                .size(15.0)
                .strong(),
        );
        ui.label(
            RichText::new("não sai no PDF do cliente")
                .color(BRASS)
                .size(12.0),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("1 USD =").color(WHITE).size(13.0));
            if self.usd_edit.is_empty() && self.quote.usd_eur > 0.0 {
                self.usd_edit = format!("{:.2}", self.quote.usd_eur).replace('.', ",");
            }
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.usd_edit)
                    .desired_width(56.0)
                    .hint_text("0,92"),
            );
            ui.label(RichText::new("€").color(WHITE).size(13.0));
            if resp.changed() {
                let t = self.usd_edit.replace(',', ".").trim().to_string();
                self.quote.usd_eur = t.parse::<f64>().unwrap_or(0.0);
                if self.quote.usd_eur < 0.0 {
                    self.quote.usd_eur = 0.0;
                }
            }
        });
        ui.add_space(8.0);
        let custo = self.quote.pecas_custo_cents();
        let venda = self.quote.total_cents;
        let lucro = self.quote.lucro_cents();
        ui.label(
            RichText::new(format!("Custo peças  {}", ops::euro(custo)))
                .color(WHITE)
                .size(13.0),
        );
        ui.label(
            RichText::new(format!("Venda  {}", ops::euro(venda)))
                .color(WHITE)
                .size(13.0),
        );
        ui.label(
            RichText::new(format!("Lucro  {}", ops::euro(lucro)))
                .color(GOLD)
                .size(16.0)
                .strong(),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new("Custo x (1 + margem %) = venda. % vazio = sem cortesia.")
                .color(WHITE)
                .size(11.0),
        );
    }

    pub(super) fn ui_trabalho(&mut self, ui: &mut egui::Ui) {
        let edit = self.can_edit_ops();
        ui.horizontal_wrapped(|ui| {
            let q_on = self.trab_kind == TrabKind::Quote;
            let j_on = self.trab_kind == TrabKind::Job;
            let c_on = self.trab_kind == TrabKind::Conta;
            if if q_on {
                gold_button(ui, "Orçamento", [140.0, 30.0])
            } else {
                steel_button(ui, "Orçamento", [140.0, 30.0])
            } {
                self.trab_kind = TrabKind::Quote;
                self.desk_gate = DeskGate::Menu;
            }
            if if j_on {
                gold_button(ui, "Ordem de serviço", [180.0, 30.0])
            } else {
                steel_button(ui, "Ordem de serviço", [180.0, 30.0])
            } {
                self.trab_kind = TrabKind::Job;
                self.desk_gate = DeskGate::Menu;
            }
            if if c_on {
                gold_button(ui, "Conta", [120.0, 30.0])
            } else {
                steel_button(ui, "Conta", [120.0, 30.0])
            } {
                self.trab_kind = TrabKind::Conta;
                self.desk_gate = DeskGate::Menu;
            }
        });
        ui.add_space(4.0);
        let door = ui_doors(ui, &mut self.desk_gate);
        if door == Some(DeskGate::Form) {
            match self.trab_kind {
                TrabKind::Quote => self.reset_quote_draft(),
                TrabKind::Conta => self.reset_conta_draft(),
                TrabKind::Job => {
                    self.job = Job::default();
                    self.mark_clean();
                }
            }
        }
        if self.desk_gate == DeskGate::Menu {
            ui.label(
                RichText::new(match self.trab_kind {
                    TrabKind::Quote => {
                        "Abrir um orçamento gravado, fazer um novo, ou apagar. Procurar fica em Abrir."
                    }
                    TrabKind::Conta => {
                        "Abrir uma conta, emitir a partir de um orçamento aceite, ou criar uma nova. Sem NIF — não é fatura AT."
                    }
                    TrabKind::Job => {
                        "Abrir uma ordem, criar uma nova, ou apagar. Depois da aprovação: Aceitar no orçamento já traz o carro."
                    }
                })
                .color(WHITE)
                .size(13.0),
            );
            return;
        }
        match self.desk_gate {
            DeskGate::Abrir => match self.trab_kind {
                TrabKind::Quote => self.ui_quote_list(ui),
                TrabKind::Conta => self.ui_conta_list(ui),
                TrabKind::Job => self.ui_job_list(ui),
            },
            DeskGate::Apagar => match self.trab_kind {
                TrabKind::Quote => {
                    self.ui_quote_list(ui);
                    ui.label(
                        RichText::new("Apagar orçamentos ainda não está nesta versão. Anula o estado.")
                            .color(BRASS)
                            .size(12.0),
                    );
                }
                TrabKind::Conta => {
                    self.ui_conta_list(ui);
                    ui.label(
                        RichText::new("Para anular: estado Anulada e Gravar.")
                            .color(BRASS)
                            .size(12.0),
                    );
                }
                TrabKind::Job => {
                    self.ui_job_list(ui);
                    ui.label(
                        RichText::new("Para anular: estado Cancelado e Gravar.")
                            .color(BRASS)
                            .size(12.0),
                    );
                }
            },
            DeskGate::Form => {
                ui.set_min_height((ui.available_height() - 8.0).max(200.0));
                match self.trab_kind {
                    TrabKind::Quote => self.ui_quote_form(ui, edit),
                    TrabKind::Conta => self.ui_conta_form(ui, edit),
                    TrabKind::Job => self.ui_job_form(ui, edit),
                }
            }
            DeskGate::Menu => {}
        }
    }

    pub(super) fn ui_agenda(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "Agenda da semana", false);
        ui.label(
            RichText::new("Trabalhos com data. O correio (orcamentos@) continua no Thunderbird.")
                .color(WHITE)
                .size(13.0),
        );
        ui.add_space(8.0);
        let days = db::week_days();
        let names = ["Seg", "Ter", "Qua", "Qui", "Sex", "Sáb", "Dom"];
        let today = today();
        let mut open: Option<Job> = None;
        ui.horizontal_wrapped(|ui| {
            for (i, day) in days.iter().enumerate() {
                ui.vertical(|ui| {
                    ui.set_width(118.0);
                    let head = format!("{}  {}", names[i], &day[8..]);
                    ui.label(
                        RichText::new(head)
                            .color(if *day == today { GOLD } else { WHITE })
                            .strong(),
                    );
                    let list = db::jobs_on_day(&self.jobs, day);
                    if list.is_empty() {
                        ui.label(RichText::new("—").color(WHITE).size(12.0));
                    } else {
                        for j in list {
                            let lab = format!("{}  {}", j.cliente, ops::tipo_label(&j.tipo));
                            if board_hit(ui, &lab) {
                                open = Some(j);
                            }
                        }
                    }
                });
            }
        });
        if let Some(j) = open {
            self.job = j;
            self.mode = Mode::Trabalho;
            self.trab_kind = TrabKind::Job;
            self.desk_gate = DeskGate::Form;
            self.mark_clean();
        }
        ui.add_space(10.0);
        ui.label(RichText::new("Sem data").color(GOLD));
        for j in self.jobs.iter().filter(|j| j.agendado.trim().is_empty() && j.estado != "cancelado" && j.estado != "entregue") {
            ui.label(format!("·  {}", j.label()));
        }
    }

    pub(super) fn ui_stock(&mut self, ui: &mut egui::Ui) {
        let edit = self.can_edit_ops();
        section_title(ui, "Stock", !edit);
        ui.label(
            RichText::new("Stardust e peças da casa. PII fora. Conferir prateleira — o seed começa a zero.")
                .color(WHITE)
                .size(13.0),
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Filtro:");
            ui.add(
                egui::TextEdit::singleline(&mut self.filter_ops)
                    .desired_width(180.0)
                    .hint_text("código, nome"),
            );
        });
        let qf = self.filter_ops.to_lowercase();
        let mut pick: Option<Sku> = None;
        egui::ScrollArea::vertical()
            .id_salt("sku-list")
            .max_height(160.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                for s in &self.skus {
                    if !qf.is_empty()
                        && !s.label().to_lowercase().contains(&qf)
                    {
                        continue;
                    }
                    let on = self.sku.id == s.id;
                    let mark = if s.low() { "!" } else if on { "●" } else { "·" };
                    let line = format!("{mark}  {}", s.label());
                    if board_hit(ui, &line) {
                        pick = Some(s.clone());
                    }
                }
            });
        if let Some(s) = pick {
            self.sku = s;
            self.sku_qty = format!("{}", self.sku.qty);
            self.mark_clean();
        }
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.label("Código:");
            ui.add(egui::TextEdit::singleline(&mut self.sku.codigo).desired_width(120.0));
            ui.label("Nome:");
            ui.add(egui::TextEdit::singleline(&mut self.sku.nome).desired_width(220.0));
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Tipo:");
            let mut t = self.sku.tipo.clone();
            egui::ComboBox::from_id_salt("sku-tipo")
                .selected_text(&t)
                .show_ui(ui, |ui| {
                    for x in ["stardust", "peca", "consumo"] {
                        ui.selectable_value(&mut t, x.to_string(), x);
                    }
                });
            self.sku.tipo = t;
            ui.label("Qtd:");
            ui.add(egui::TextEdit::singleline(&mut self.sku_qty).desired_width(70.0));
            ui.label("Un:");
            ui.add(egui::TextEdit::singleline(&mut self.sku.unidade).desired_width(50.0));
            ui.label("Pedir:");
            ui.label(ops::euro(self.sku.pedir_cents));
        });
        ui.label("Notas:");
        stretch_multiline(ui, &mut self.sku.notas);
        ui.horizontal_wrapped(|ui| {
            if edit && gold_button(ui, "Guardar SKU", [140.0, 30.0]) {
                self.save_sku_row();
            }
            if edit && gold_button(ui, "Novo SKU", [120.0, 30.0]) {
                self.sku = Sku::default();
                self.sku_qty = "0".into();
                self.mark_clean();
            }
            if edit && steel_button(ui, "Importar peças Focus", [200.0, 30.0]) {
                self.import_focus_pecas();
            }
        });
    }

    fn import_focus_pecas(&mut self) {
        let p = crate::paths::interno_root(&self.root_path())
            .join("Catálogos")
            .join("PRECOS-E-VENDA.txt");
        let text = match fs::read_to_string(&p) {
            Ok(t) => t,
            Err(_) => {
                self.set_status(false, format!("Não li {}", p.display()));
                return;
            }
        };
        let items = ops::parse_precos_pecas(&text);
        if items.is_empty() {
            self.set_status(false, "O ficheiro não tinha linhas de peças.");
            return;
        }
        match db::open(&self.root_path()) {
            Ok(conn) => match db::upsert_sku_list(&conn, &items) {
                Ok(n) => {
                    self.reload_ops();
                    self.set_status(true, format!("{n} peças Focus no stock."));
                }
                Err(e) => self.set_status(false, format!("{e:#}")),
            },
            Err(e) => self.set_status(false, format!("{e:#}")),
        }
    }

    fn export_quotes_csv(&mut self) {
        let body = ops::quotes_csv(&self.quotes);
        let name = format!("Vanguarda-orcamentos-{}.csv", today());
        if let Some(p) = rfd::FileDialog::new().set_file_name(&name).save_file() {
            match fs::write(&p, body) {
                Ok(()) => self.set_status(true, format!("CSV: {}", p.display())),
                Err(e) => self.set_status(false, format!("{e}")),
            }
        }
    }

    fn export_jobs_csv(&mut self) {
        let body = ops::jobs_csv(&self.jobs);
        let name = format!("Vanguarda-trabalhos-{}.csv", today());
        if let Some(p) = rfd::FileDialog::new().set_file_name(&name).save_file() {
            match fs::write(&p, body) {
                Ok(()) => self.set_status(true, format!("CSV: {}", p.display())),
                Err(e) => self.set_status(false, format!("{e}")),
            }
        }
    }

}

fn viatura_label(c: &crate::model::Carro) -> String {
    let mm = [c.marca.trim(), c.modelo.trim()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let key = if !c.matricula.trim().is_empty() {
        c.matricula.trim()
    } else if !c.vin.trim().is_empty() {
        c.vin.trim()
    } else {
        "sem matrícula"
    };
    if mm.is_empty() {
        key.to_string()
    } else {
        format!("{mm}  ·  {key}")
    }
}

fn pct_box(
    ui: &mut egui::Ui,
    edit: bool,
    label: &str,
    buf: &mut String,
    pct: &mut f64,
) -> bool {
    if buf.is_empty() && *pct > 0.0 {
        *buf = ops::fmt_disc_edit(*pct);
    }
    if !label.is_empty() {
        ui.label(RichText::new(label).color(BRASS).size(12.0));
    }
    let resp = ui.add_enabled(
        edit,
        egui::TextEdit::singleline(buf)
            .desired_width(48.0)
            .hint_text("%"),
    );
    if resp.changed() {
        if let Some(p) = ops::parse_disc_pct(buf) {
            *pct = p;
            return true;
        }
    }
    false
}

fn custo_box(
    ui: &mut egui::Ui,
    edit: bool,
    usd_eur: f64,
    buf: &mut String,
    custo: &mut i64,
    show_label: bool,
) -> bool {
    if buf.is_empty() && *custo > 0 {
        *buf = format!("{},{:02}", *custo / 100, *custo % 100);
    }
    if show_label {
        ui.label(RichText::new("Custo").color(BRASS).size(12.0));
    }
    let resp = ui.add_enabled(
        edit,
        egui::TextEdit::singleline(buf)
            .desired_width(72.0)
            .hint_text("custo"),
    );
    if resp.changed() {
        if ops::looks_usd(buf) && usd_eur <= 0.0 {
            return false;
        }
        *custo = ops::parse_custo_cents(buf, usd_eur);
        true
    } else {
        false
    }
}

fn euro_drag(ui: &mut egui::Ui, cents: &mut i64, edit: bool) -> bool {
    let mut eur = *cents as f64 / 100.0;
    let resp = ui.add_enabled(
        edit,
        egui::DragValue::new(&mut eur)
            .prefix("€ ")
            .speed(0.5)
            .range(0.0..=9_999_999.0)
            .max_decimals(2)
            .custom_formatter(|n, _| {
                let c = (n * 100.0).round().abs() as i64;
                format!("{},{:02}", c / 100, c % 100)
            })
            .custom_parser(|s| s.replace(',', ".").parse::<f64>().ok()),
    );
    if resp.changed() {
        *cents = (eur * 100.0).round() as i64;
        true
    } else {
        false
    }
}

