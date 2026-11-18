use crate::{add_node, add_resource, connect_source_to_target};
use eframe::egui;
use factory_atlas_core::model::FlowUnit;
use factory_atlas_core::model::{FactoryNode, FactoryProject, NodeKind, ResourceKind};
use factory_atlas_core::project_format::{create_empty_project, now_iso_timestamp};
use factory_atlas_core::storage::{AppPaths, load_project_file, save_project_file};
use rfd::FileDialog;
use std::path::PathBuf;

enum PendingAction {
    NewProject,
    OpenProject(PathBuf),
}

struct ResourceForm {
    id: String,
    name_ru: String,
    name_en: String,
    kind: ResourceKind,
}

impl Default for ResourceForm {
    fn default() -> Self {
        Self {
            id: String::new(),
            name_ru: String::new(),
            name_en: String::new(),
            kind: ResourceKind::Item,
        }
    }
}

struct NodeForm {
    kind: NodeKind,
    title: String,
    resource_id: Option<String>,
    note: String,
    rate_per_min: String,
}

impl Default for NodeForm {
    fn default() -> Self {
        Self {
            kind: NodeKind::Source,
            title: String::new(),
            resource_id: None,
            note: String::new(),
            rate_per_min: "60".to_owned(),
        }
    }
}

struct FactoryAtlasApp {
    paths: AppPaths,
    project: FactoryProject,
    project_path: Option<PathBuf>,
    dirty: bool,
    resource_form: ResourceForm,
    node_form: NodeForm,
    selected_source_id: Option<String>,
    selected_target_id: Option<String>,
    pending_action: Option<PendingAction>,
    show_close_prompt: bool,
    status: String,
    status_is_error: bool,
}

impl FactoryAtlasApp {
    fn new() -> Self {
        let paths = AppPaths::discover();
        let status = paths
            .ensure_dirs()
            .err()
            .unwrap_or_else(|| "Новый пустой проект. Создайте свои ресурсы и узлы.".to_owned());
        Self {
            paths,
            project: create_empty_project("Новый проект"),
            project_path: None,
            dirty: false,
            resource_form: ResourceForm::default(),
            node_form: NodeForm::default(),
            selected_source_id: None,
            selected_target_id: None,
            pending_action: None,
            show_close_prompt: false,
            status_is_error: false,
            status,
        }
    }

    fn set_status(&mut self, message: impl Into<String>, is_error: bool) {
        self.status = message.into();
        self.status_is_error = is_error;
    }

    fn save_current(&mut self) -> bool {
        let path = if let Some(path) = self.project_path.clone() {
            path
        } else {
            let dialog = FileDialog::new()
                .set_title("Сохранить проект Factory Atlas")
                .set_directory(&self.paths.projects_dir)
                .set_file_name(format!(
                    "{}.factoryatlas",
                    safe_file_name(&self.project.name)
                ))
                .add_filter("Factory Atlas", &["factoryatlas"])
                .add_filter("JSON", &["json"]);
            let Some(path) = dialog.save_file() else {
                return false;
            };
            with_project_extension(path)
        };

        self.project.updated_at = now_iso_timestamp();
        match save_project_file(&path, &self.project) {
            Ok(()) => {
                self.project_path = Some(path.clone());
                self.dirty = false;
                self.set_status(format!("Проект сохранён: {}", path.display()), false);
                true
            }
            Err(error) => {
                self.set_status(format!("Не удалось сохранить проект: {error}"), true);
                false
            }
        }
    }

    fn save_as(&mut self) -> bool {
        let dialog = FileDialog::new()
            .set_title("Сохранить проект как")
            .set_directory(&self.paths.projects_dir)
            .set_file_name(format!(
                "{}.factoryatlas",
                safe_file_name(&self.project.name)
            ))
            .add_filter("Factory Atlas", &["factoryatlas"])
            .add_filter("JSON", &["json"]);
        let Some(path) = dialog.save_file() else {
            return false;
        };
        let path = with_project_extension(path);
        self.project.updated_at = now_iso_timestamp();
        match save_project_file(&path, &self.project) {
            Ok(()) => {
                self.project_path = Some(path.clone());
                self.dirty = false;
                self.set_status(format!("Проект сохранён: {}", path.display()), false);
                true
            }
            Err(error) => {
                self.set_status(format!("Не удалось сохранить проект: {error}"), true);
                false
            }
        }
    }

    fn choose_project_to_open(&mut self) {
        let Some(path) = FileDialog::new()
            .set_title("Открыть проект Factory Atlas")
            .set_directory(&self.paths.projects_dir)
            .add_filter("Factory Atlas и JSON", &["factoryatlas", "json"])
            .pick_file()
        else {
            return;
        };
        self.request_action(PendingAction::OpenProject(path));
    }

    fn request_action(&mut self, action: PendingAction) {
        if self.dirty {
            self.pending_action = Some(action);
        } else {
            self.apply_action(action);
        }
    }

    fn apply_action(&mut self, action: PendingAction) {
        match action {
            PendingAction::NewProject => {
                self.project = create_empty_project("Новый проект");
                self.project_path = None;
                self.dirty = false;
                self.selected_source_id = None;
                self.selected_target_id = None;
                self.set_status("Создан пустой проект.", false);
            }
            PendingAction::OpenProject(path) => match load_project_file(&path) {
                Ok((project, migration)) => {
                    self.project = project;
                    self.project_path = Some(path.clone());
                    self.dirty = false;
                    self.selected_source_id = None;
                    self.selected_target_id = None;
                    let message = if migration == Some(1) {
                        format!("Проект открыт и мигрирован с v1: {}", path.display())
                    } else {
                        format!("Проект открыт: {}", path.display())
                    };
                    self.set_status(message, false);
                }
                Err(error) => {
                    self.set_status(format!("Не удалось открыть проект: {error}"), true);
                }
            },
        }
    }

    fn add_resource_from_form(&mut self) {
        match add_resource(
            &mut self.project,
            &self.resource_form.id,
            &self.resource_form.name_ru,
            &self.resource_form.name_en,
            self.resource_form.kind,
        ) {
            Ok(()) => {
                self.dirty = true;
                self.node_form.resource_id = Some(self.resource_form.id.trim().to_owned());
                self.set_status("Ресурс добавлен в проект.", false);
                self.resource_form = ResourceForm::default();
            }
            Err(error) => self.set_status(error, true),
        }
    }

    fn add_node_from_form(&mut self) {
        let resource_id = if matches!(self.node_form.kind, NodeKind::Source | NodeKind::Target) {
            self.node_form.resource_id.as_deref()
        } else {
            None
        };
        let rate = if matches!(self.node_form.kind, NodeKind::Source | NodeKind::Target) {
            match self.node_form.rate_per_min.trim().parse::<f64>() {
                Ok(rate) if rate.is_finite() && rate > 0.0 => Some(rate),
                _ => {
                    self.set_status("Введите положительный конечный темп.", true);
                    return;
                }
            }
        } else {
            None
        };
        match add_node(
            &mut self.project,
            self.node_form.kind,
            &self.node_form.title,
            resource_id,
            &self.node_form.note,
            rate,
        ) {
            Ok(_) => {
                self.dirty = true;
                self.node_form.title.clear();
                self.node_form.note.clear();
                self.set_status("Узел добавлен в проект.", false);
            }
            Err(error) => self.set_status(error, true),
        }
    }

    fn add_edge_from_selection(&mut self) {
        let (Some(source), Some(target)) = (
            self.selected_source_id.as_deref(),
            self.selected_target_id.as_deref(),
        ) else {
            self.set_status("Выберите узел источника и узел цели.", true);
            return;
        };
        match connect_source_to_target(&mut self.project, source, target) {
            Ok(()) => {
                self.dirty = true;
                self.set_status("Связь добавлена.", false);
            }
            Err(error) => self.set_status(error, true),
        }
    }

    fn draw_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.heading("Factory Atlas");
            ui.separator();
            if ui.button("Новый").clicked() {
                self.request_action(PendingAction::NewProject);
            }
            if ui.button("Открыть…").clicked() {
                self.choose_project_to_open();
            }
            if ui.button("Сохранить").clicked() {
                self.save_current();
            }
            if ui.button("Сохранить как…").clicked() {
                self.save_as();
            }
            ui.separator();
            ui.label(if self.dirty {
                "Изменения не сохранены"
            } else {
                "Сохранено"
            });
        });
    }

    fn draw_project_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Проект:");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.project.name)
                    .desired_width(340.0)
                    .hint_text("Название проекта"),
            );
            if response.changed() {
                self.project.updated_at = now_iso_timestamp();
                self.dirty = true;
            }
        });
        let file = self
            .project_path
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "ещё не сохранён".to_owned());
        ui.small(format!(
            "{} · {} узлов · {} связей · {} ресурсов · {}",
            self.project.profile.name,
            self.project.nodes.len(),
            self.project.edges.len(),
            self.project.custom_resources.len(),
            file
        ));
    }

    fn draw_resource_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Мои ресурсы");
        ui.label("Создайте ресурс, прежде чем добавлять узел источника или цели.");
        ui.add_space(4.0);
        ui.label("ID");
        ui.add(
            egui::TextEdit::singleline(&mut self.resource_form.id).hint_text("custom:iron_plate"),
        );
        ui.label("Название на русском");
        ui.add(
            egui::TextEdit::singleline(&mut self.resource_form.name_ru)
                .hint_text("Железная пластина"),
        );
        ui.label("Название на английском");
        ui.add(egui::TextEdit::singleline(&mut self.resource_form.name_en).hint_text("Iron Plate"));
        egui::ComboBox::from_id_salt("resource-kind")
            .selected_text(resource_kind_name(self.resource_form.kind))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.resource_form.kind, ResourceKind::Item, "Предмет");
                ui.selectable_value(
                    &mut self.resource_form.kind,
                    ResourceKind::Fluid,
                    "Жидкость",
                );
            });
        if ui.button("Добавить ресурс").clicked() {
            self.add_resource_from_form();
        }

        ui.separator();
        ui.label(format!(
            "В проекте: {}",
            self.project.custom_resources.len()
        ));
        for resource in &self.project.custom_resources {
            ui.horizontal(|ui| {
                ui.label(resource_kind_icon(resource.kind));
                ui.vertical(|ui| {
                    ui.label(&resource.name_ru);
                    ui.small(format!(
                        "{} · {}",
                        resource.id,
                        resource_kind_name(resource.kind)
                    ));
                });
            });
            ui.add_space(3.0);
        }
    }

    fn draw_node_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Узлы цепочки");
        egui::ComboBox::from_id_salt("node-kind")
            .selected_text(node_kind_name(self.node_form.kind))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.node_form.kind, NodeKind::Source, "Источник");
                ui.selectable_value(&mut self.node_form.kind, NodeKind::Target, "Цель");
                ui.selectable_value(&mut self.node_form.kind, NodeKind::Note, "Заметка");
            });
        ui.label("Название узла");
        ui.add(egui::TextEdit::singleline(&mut self.node_form.title).hint_text("Например, Рудник"));

        match self.node_form.kind {
            NodeKind::Source | NodeKind::Target => {
                egui::ComboBox::from_id_salt("node-resource")
                    .selected_text(
                        self.node_form
                            .resource_id
                            .as_deref()
                            .and_then(|id| {
                                self.project.custom_resources.iter().find(|r| r.id == id)
                            })
                            .map(|resource| format!("{} · {}", resource.name_ru, resource.id))
                            .unwrap_or_else(|| "Выберите созданный ресурс".to_owned()),
                    )
                    .show_ui(ui, |ui| {
                        for resource in &self.project.custom_resources {
                            ui.selectable_value(
                                &mut self.node_form.resource_id,
                                Some(resource.id.clone()),
                                format!("{} · {}", resource.name_ru, resource.id),
                            );
                        }
                    });
                let unit = self
                    .node_form
                    .resource_id
                    .as_deref()
                    .and_then(|id| self.project.custom_resources.iter().find(|r| r.id == id))
                    .map(|resource| {
                        FlowUnit::from_kind(resource.kind)
                            .label(factory_atlas_core::model::Locale::Ru)
                    })
                    .unwrap_or("ед./мин");
                ui.horizontal(|ui| {
                    ui.label(format!("Темп ({unit})"));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.node_form.rate_per_min)
                            .desired_width(100.0),
                    );
                });
            }
            NodeKind::Note => {
                ui.label("Текст заметки");
                ui.add(egui::TextEdit::multiline(&mut self.node_form.note).desired_rows(2));
            }
            _ => {}
        }
        if ui.button("Добавить узел").clicked() {
            self.add_node_from_form();
        }

        ui.separator();
        if self.project.nodes.is_empty() {
            ui.label("Пока нет узлов. Новый проект остаётся пустым, пока вы сами их не добавите.");
        }
        for node in &self.project.nodes {
            draw_node_row(ui, node, &self.project);
        }
    }

    fn draw_edge_editor(&mut self, ui: &mut egui::Ui) {
        let sources: Vec<&FactoryNode> = self
            .project
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Source)
            .collect();
        let targets: Vec<&FactoryNode> = self
            .project
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Target)
            .collect();
        ui.separator();
        ui.heading("Связать источник с целью");
        egui::ComboBox::from_id_salt("edge-source")
            .selected_text(selected_node_title(
                &sources,
                self.selected_source_id.as_deref(),
            ))
            .show_ui(ui, |ui| {
                for node in &sources {
                    ui.selectable_value(
                        &mut self.selected_source_id,
                        Some(node.id.clone()),
                        &node.title,
                    );
                }
            });
        egui::ComboBox::from_id_salt("edge-target")
            .selected_text(selected_node_title(
                &targets,
                self.selected_target_id.as_deref(),
            ))
            .show_ui(ui, |ui| {
                for node in &targets {
                    ui.selectable_value(
                        &mut self.selected_target_id,
                        Some(node.id.clone()),
                        &node.title,
                    );
                }
            });
        if ui.button("Добавить связь").clicked() {
            self.add_edge_from_selection();
        }
        ui.separator();
        ui.label(format!("Связей: {}", self.project.edges.len()));
        for edge in &self.project.edges {
            let from = self
                .project
                .nodes
                .iter()
                .find(|node| node.id == edge.from_node_id);
            let to = self
                .project
                .nodes
                .iter()
                .find(|node| node.id == edge.to_node_id);
            let from_label = from.map(|node| node.title.as_str()).unwrap_or("?");
            let to_label = to.map(|node| node.title.as_str()).unwrap_or("?");
            ui.label(format!("{from_label} → {to_label} · {}", edge.resource_id));
        }
    }

    fn draw_pending_action_dialog(&mut self, ctx: &egui::Context) {
        if self.pending_action.is_none() {
            return;
        }
        egui::Window::new("Несохранённые изменения")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Сохранить изменения перед продолжением?");
                ui.horizontal(|ui| {
                    if ui.button("Сохранить").clicked() && self.save_current() {
                        if let Some(action) = self.pending_action.take() {
                            self.apply_action(action);
                        }
                    }
                    if ui.button("Продолжить без сохранения").clicked() {
                        if let Some(action) = self.pending_action.take() {
                            self.apply_action(action);
                        }
                    }
                    if ui.button("Отмена").clicked() {
                        self.pending_action = None;
                    }
                });
            });
    }

    fn draw_close_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_close_prompt {
            return;
        }
        egui::Window::new("Несохранённые изменения")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Перед выходом сохраните изменения или отмените закрытие.");
                ui.horizontal(|ui| {
                    if ui.button("Сохранить и выйти").clicked() && self.save_current()
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui.button("Выйти без сохранения").clicked() {
                        self.dirty = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui.button("Остаться").clicked() {
                        self.show_close_prompt = false;
                    }
                });
            });
    }
}

impl eframe::App for FactoryAtlasApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let close_requested = ctx.input(|input| input.viewport().close_requested());
        if close_requested && self.dirty && !self.show_close_prompt {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.show_close_prompt = true;
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw_toolbar(ui);
        ui.separator();
        self.draw_project_header(ui);
        ui.separator();

        ui.columns(2, |columns| {
            egui::ScrollArea::vertical()
                .id_salt("left-column")
                .show(&mut columns[0], |ui| self.draw_resource_editor(ui));
            egui::ScrollArea::vertical()
                .id_salt("right-column")
                .show(&mut columns[1], |ui| {
                    self.draw_node_editor(ui);
                    self.draw_edge_editor(ui);
                });
        });

        ui.separator();
        let color = if self.status_is_error {
            ui.visuals().error_fg_color
        } else {
            ui.visuals().text_color()
        };
        ui.colored_label(color, &self.status);

        let ctx = ui.ctx().clone();
        self.draw_pending_action_dialog(&ctx);
        self.draw_close_dialog(&ctx);
    }
}

fn draw_node_row(ui: &mut egui::Ui, node: &FactoryNode, project: &FactoryProject) {
    ui.horizontal(|ui| {
        ui.label(node_kind_name(node.kind));
        ui.vertical(|ui| {
            ui.label(&node.title);
            if let Some(resource_id) = node.resource_id.as_deref() {
                let resource_name = project
                    .custom_resources
                    .iter()
                    .find(|resource| resource.id == resource_id)
                    .map(|resource| resource.name_ru.as_str())
                    .unwrap_or(resource_id);
                ui.small(format!("{resource_name} · {resource_id}"));
            }
        });
    });
}

fn selected_node_title(nodes: &[&FactoryNode], selected: Option<&str>) -> String {
    selected
        .and_then(|id| nodes.iter().find(|node| node.id == id))
        .map(|node| node.title.clone())
        .unwrap_or_else(|| "Выберите узел".to_owned())
}

fn node_kind_name(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Source => "Источник",
        NodeKind::Target => "Цель",
        NodeKind::Machine => "Машина",
        NodeKind::Recipe => "Рецепт",
        NodeKind::Byproduct => "Побочный продукт",
        NodeKind::Note => "Заметка",
    }
}

fn resource_kind_name(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Item => "Предмет · предм./мин",
        ResourceKind::Fluid => "Жидкость · мБ/мин",
    }
}

fn resource_kind_icon(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Item => "◆",
        ResourceKind::Fluid => "◉",
    }
}

fn safe_file_name(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|character| match character {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => character,
        })
        .collect();
    if safe.trim().is_empty() {
        "project".to_owned()
    } else {
        safe
    }
}

fn with_project_extension(path: PathBuf) -> PathBuf {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension)
            if extension.eq_ignore_ascii_case("factoryatlas")
                || extension.eq_ignore_ascii_case("json") =>
        {
            path
        }
        _ => path.with_extension("factoryatlas"),
    }
}

pub fn run() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_title("Factory Atlas — нативное приложение")
            .with_inner_size([1180.0, 820.0])
            .with_min_inner_size([920.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Factory Atlas",
        native_options,
        Box::new(|_creation_context| Ok(Box::new(FactoryAtlasApp::new()))),
    )
}
