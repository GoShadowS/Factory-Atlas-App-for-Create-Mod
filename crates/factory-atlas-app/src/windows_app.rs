use crate::app_logic::{add_node, add_resource, connect_source_to_target};
use eframe::egui;
use factory_atlas_core::model::{
    AppSettings, FactoryNode, FactoryProject, FlowUnit, NodeKind, ResourceKind, SolverResult,
    ThemeMode, WarningSeverity,
};
use factory_atlas_core::project_format::{create_empty_project, now_iso_timestamp};
use factory_atlas_core::storage::{AppPaths, load_project_file, save_project_file};
use factory_atlas_core::worker_pool::{BackgroundJob, BackgroundOutput, WorkerPool};
use rfd::FileDialog;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

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
    settings: AppSettings,
    project: FactoryProject,
    project_path: Option<PathBuf>,
    dirty: bool,
    active_view: MainView,
    selected_node_id: Option<String>,
    canvas_offset: egui::Vec2,
    canvas_zoom: f32,
    catalog_search: String,
    show_node_dialog: bool,
    show_link_dialog: bool,
    solver_pool: WorkerPool,
    solver_task_id: Option<u64>,
    solver_result: Option<SolverResult>,
    project_revision: u64,
    last_solve_revision: u64,
    applied_theme: Option<ThemeMode>,
    resource_form: ResourceForm,
    node_form: NodeForm,
    selected_source_id: Option<String>,
    selected_target_id: Option<String>,
    pending_action: Option<PendingAction>,
    show_close_prompt: bool,
    status: String,
    status_is_error: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MainView {
    Workspace,
    Resources,
}

impl FactoryAtlasApp {
    fn new() -> Self {
        let paths = AppPaths::discover();
        let settings = paths.load_settings();
        let status = paths
            .ensure_dirs()
            .err()
            .unwrap_or_else(|| "Новый пустой проект. Создайте свои ресурсы и узлы.".to_owned());
        Self {
            paths,
            settings,
            project: create_empty_project("Новый проект"),
            project_path: None,
            dirty: false,
            active_view: MainView::Workspace,
            selected_node_id: None,
            canvas_offset: egui::vec2(30.0, 30.0),
            canvas_zoom: 1.0,
            catalog_search: String::new(),
            show_node_dialog: false,
            show_link_dialog: false,
            solver_pool: WorkerPool::new(Some(2)),
            solver_task_id: None,
            solver_result: None,
            project_revision: 1,
            last_solve_revision: 0,
            applied_theme: None,
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

    fn mark_project_changed(&mut self) {
        self.project_revision = self.project_revision.wrapping_add(1);
        self.project.updated_at = now_iso_timestamp();
        self.dirty = true;
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
                self.record_recent_project(&path);
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
                self.record_recent_project(&path);
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
                self.selected_node_id = None;
                self.solver_result = None;
                self.project_revision = self.project_revision.wrapping_add(1);
                self.set_status("Создан пустой проект.", false);
            }
            PendingAction::OpenProject(path) => match load_project_file(&path) {
                Ok((project, migration)) => {
                    self.project = project;
                    self.project_path = Some(path.clone());
                    self.dirty = false;
                    self.selected_source_id = None;
                    self.selected_target_id = None;
                    self.selected_node_id = None;
                    self.project_revision = self.project_revision.wrapping_add(1);
                    self.solver_result = None;
                    self.record_recent_project(&path);
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
                self.mark_project_changed();
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
                self.mark_project_changed();
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
                self.mark_project_changed();
                self.set_status("Связь добавлена.", false);
            }
            Err(error) => self.set_status(error, true),
        }
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let mut visuals = match self.settings.theme {
            ThemeMode::Dark => egui::Visuals::dark(),
            ThemeMode::Light => egui::Visuals::light(),
        };
        if self.settings.theme == ThemeMode::Dark {
            visuals.dark_mode = true;
            visuals.panel_fill = egui::Color32::from_rgb(18, 23, 26);
            visuals.window_fill = egui::Color32::from_rgb(25, 31, 34);
            visuals.faint_bg_color = egui::Color32::from_rgb(22, 28, 31);
            visuals.extreme_bg_color = egui::Color32::from_rgb(12, 16, 18);
            visuals.code_bg_color = egui::Color32::from_rgb(20, 26, 29);
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(37, 46, 50);
            visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(54, 65, 69);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(190, 104, 59);
            visuals.selection.bg_fill = egui::Color32::from_rgb(158, 87, 52);
            visuals.widgets.noninteractive.bg_stroke.color = egui::Color32::from_rgb(47, 59, 63);
        } else {
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(194, 105, 60);
            visuals.selection.bg_fill = egui::Color32::from_rgb(220, 160, 124);
        }
        visuals.window_corner_radius = egui::CornerRadius::same(10);
        visuals.menu_corner_radius = egui::CornerRadius::same(8);
        visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
        visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
        visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);
        ctx.set_visuals(visuals);

        ctx.global_style_mut(|style| {
            style.spacing.item_spacing = egui::vec2(9.0, 8.0);
            style.spacing.button_padding = egui::vec2(11.0, 7.0);
            style.spacing.window_margin = egui::Margin::same(16);
            style.visuals.override_text_color = Some(match self.settings.theme {
                ThemeMode::Dark => egui::Color32::from_rgb(225, 231, 231),
                ThemeMode::Light => egui::Color32::from_rgb(42, 50, 52),
            });
        });
        self.applied_theme = Some(self.settings.theme);
    }

    fn schedule_solver(&mut self) {
        if self.last_solve_revision == self.project_revision {
            return;
        }
        if let Some(task_id) = self.solver_task_id.take() {
            self.solver_pool.cancel_task(task_id);
        }
        self.last_solve_revision = self.project_revision;
        self.solver_result = None;
        self.solver_task_id = Some(self.solver_pool.submit(BackgroundJob::SolveProject {
            project: self.project.clone(),
        }));
    }

    fn poll_solver(&mut self) {
        for output in self.solver_pool.poll_outputs() {
            match output {
                BackgroundOutput::Solved { task_id, result }
                    if self.solver_task_id == Some(task_id) =>
                {
                    self.solver_result = Some(*result);
                    self.solver_task_id = None;
                }
                BackgroundOutput::Failed { task_id, error }
                    if self.solver_task_id == Some(task_id) =>
                {
                    self.set_status(format!("Расчёт не выполнен: {error}"), true);
                    self.solver_task_id = None;
                }
                _ => {}
            }
        }
    }

    fn record_recent_project(&mut self, path: &PathBuf) {
        let value = path.to_string_lossy().to_string();
        self.settings
            .recent_projects
            .retain(|entry| entry != &value);
        self.settings.recent_projects.insert(0, value);
        self.settings.recent_projects.truncate(8);
        if let Err(error) = self.paths.save_settings(&self.settings) {
            self.set_status(format!("Не удалось сохранить настройки: {error}"), true);
        }
    }

    fn toggle_theme(&mut self, ctx: &egui::Context) {
        self.settings.theme = match self.settings.theme {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
        };
        self.apply_theme(ctx);
        if let Err(error) = self.paths.save_settings(&self.settings) {
            self.set_status(format!("Не удалось сохранить тему: {error}"), true);
        }
    }

    fn draw_navigation(&mut self, ui: &mut egui::Ui) {
        ui.add_space(5.0);
        ui.label(
            egui::RichText::new("РАБОЧЕЕ МЕСТО")
                .size(10.0)
                .strong()
                .color(muted()),
        );
        ui.add_space(7.0);
        for (view, icon, label) in [
            (MainView::Workspace, "▦", "Граф"),
            (MainView::Resources, "◆", "Ресурсы"),
        ] {
            let selected = self.active_view == view;
            let text = egui::RichText::new(format!("{icon}   {label}"))
                .color(if selected { copper() } else { muted() })
                .strong();
            if ui
                .add_sized(
                    [ui.available_width(), 36.0],
                    egui::Button::new(text).selected(selected),
                )
                .clicked()
            {
                self.active_view = view;
            }
        }
        ui.add_space(14.0);
        ui.separator();
        ui.add_space(5.0);
        ui.label(
            egui::RichText::new("ПРОЕКТНЫЕ ДАННЫЕ")
                .size(10.0)
                .strong()
                .color(muted()),
        );
        ui.add_space(5.0);
        ui.label(format!("{} ресурса", self.project.custom_resources.len()));
        ui.label(format!("{} узлов", self.project.nodes.len()));
        ui.label(format!("{} связи", self.project.edges.len()));
        ui.add_space(14.0);
        ui.separator();
        ui.add_space(8.0);
        ui.small("Пустой проект");
        ui.small("Данные добавляются вручную.");
    }

    fn draw_catalog(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Каталог");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("＋")
                    .on_hover_text("Создать ресурс")
                    .clicked()
                {
                    self.active_view = MainView::Resources;
                }
            });
        });
        ui.add(
            egui::TextEdit::singleline(&mut self.catalog_search)
                .hint_text("Поиск по ресурсам")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(4.0);
        egui::ScrollArea::vertical()
            .id_salt("catalog-resources")
            .show(ui, |ui| {
                if self.project.custom_resources.is_empty() {
                    ui.label(egui::RichText::new("Здесь появятся ресурсы проекта.").color(muted()));
                }
                for resource in self.project.custom_resources.iter().filter(|resource| {
                    self.catalog_search.trim().is_empty()
                        || resource
                            .name_ru
                            .to_lowercase()
                            .contains(&self.catalog_search.to_lowercase())
                        || resource
                            .name_en
                            .to_lowercase()
                            .contains(&self.catalog_search.to_lowercase())
                        || resource
                            .id
                            .to_lowercase()
                            .contains(&self.catalog_search.to_lowercase())
                }) {
                    let (color, symbol) = match resource.kind {
                        ResourceKind::Item => (item_color(), "◆"),
                        ResourceKind::Fluid => (fluid_color(), "◉"),
                    };
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(symbol).color(color).strong());
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&resource.name_ru).strong());
                            ui.small(egui::RichText::new(&resource.id).color(muted()));
                        });
                    });
                }
            });
        ui.add_space(8.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.heading("Узлы");
            if ui
                .small_button("＋")
                .on_hover_text("Добавить узел")
                .clicked()
            {
                self.show_node_dialog = true;
            }
        });
        egui::ScrollArea::vertical()
            .id_salt("catalog-nodes")
            .max_height(220.0)
            .show(ui, |ui| {
                if self.project.nodes.is_empty() {
                    ui.label(egui::RichText::new("Граф пока пуст.").color(muted()));
                }
                for node in &self.project.nodes {
                    let selected = self.selected_node_id.as_deref() == Some(node.id.as_str());
                    let response = ui.add_sized(
                        [ui.available_width(), 42.0],
                        egui::Button::new(
                            egui::RichText::new(format!(
                                "{}  {}",
                                node_kind_icon(node.kind),
                                node.title
                            ))
                            .color(if selected {
                                copper()
                            } else {
                                ui.visuals().text_color()
                            }),
                        )
                        .selected(selected),
                    );
                    if response.clicked() {
                        self.selected_node_id = Some(node.id.clone());
                        self.active_view = MainView::Workspace;
                    }
                }
            });
    }

    fn draw_canvas(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading(egui::RichText::new(&self.project.name).size(20.0));
                ui.small(
                    egui::RichText::new(format!(
                        "{} · {} узлов · {} связей",
                        self.project.profile.name,
                        self.project.nodes.len(),
                        self.project.edges.len()
                    ))
                    .color(muted()),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Связать узлы").clicked() {
                    self.show_link_dialog = true;
                }
                if ui.button("＋  Узел").clicked() {
                    self.show_node_dialog = true;
                }
                if ui.small_button("−").clicked() {
                    self.canvas_zoom = (self.canvas_zoom / 1.15).clamp(0.45, 2.4);
                }
                ui.label(format!("{}%", (self.canvas_zoom * 100.0) as u32));
                if ui.small_button("＋").clicked() {
                    self.canvas_zoom = (self.canvas_zoom * 1.15).clamp(0.45, 2.4);
                }
            });
        });
        ui.add_space(8.0);
        let size = ui.available_size().max(egui::vec2(200.0, 200.0));
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        let canvas_bg = if self.settings.theme == ThemeMode::Dark {
            egui::Color32::from_rgb(14, 19, 21)
        } else {
            egui::Color32::from_rgb(244, 246, 245)
        };
        painter.rect_filled(rect, egui::CornerRadius::same(9), canvas_bg);
        painter.rect_stroke(
            rect,
            egui::CornerRadius::same(9),
            egui::Stroke::new(1.0, border_color()),
            egui::StrokeKind::Inside,
        );

        let grid_step = (26.0 * self.canvas_zoom).max(13.0);
        let grid_color = if self.settings.theme == ThemeMode::Dark {
            egui::Color32::from_rgb(29, 38, 41)
        } else {
            egui::Color32::from_rgb(226, 231, 229)
        };
        let offset = self.canvas_offset;
        let first_x = rect.left() + (offset.x % grid_step);
        let first_y = rect.top() + (offset.y % grid_step);
        let mut gx = first_x;
        while gx < rect.right() {
            painter.line_segment(
                [egui::pos2(gx, rect.top()), egui::pos2(gx, rect.bottom())],
                egui::Stroke::new(0.6, grid_color),
            );
            gx += grid_step;
        }
        let mut gy = first_y;
        while gy < rect.bottom() {
            painter.line_segment(
                [egui::pos2(rect.left(), gy), egui::pos2(rect.right(), gy)],
                egui::Stroke::new(0.6, grid_color),
            );
            gy += grid_step;
        }

        if response.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            let ctrl = ui.input(|input| input.modifiers.ctrl);
            if ctrl && scroll != 0.0 {
                self.canvas_zoom = (self.canvas_zoom * (1.0 + scroll * 0.001)).clamp(0.45, 2.4);
            }
        }
        if response.dragged_by(egui::PointerButton::Middle) {
            self.canvas_offset += response.drag_delta();
        }

        let node_positions: HashMap<String, egui::Pos2> = self
            .project
            .nodes
            .iter()
            .map(|node| {
                let p = rect.min
                    + self.canvas_offset
                    + egui::vec2(node.x as f32, node.y as f32) * self.canvas_zoom;
                (node.id.clone(), p)
            })
            .collect();
        for edge in &self.project.edges {
            let (Some(from), Some(to)) = (
                node_positions.get(&edge.from_node_id),
                node_positions.get(&edge.to_node_id),
            ) else {
                continue;
            };
            let start = *from + egui::vec2(188.0 * self.canvas_zoom, 43.0 * self.canvas_zoom);
            let end = *to + egui::vec2(0.0, 43.0 * self.canvas_zoom);
            let color = if edge.is_byproduct_port == Some(true) {
                byproduct_color()
            } else {
                flow_color(edge.resource_kind)
            };
            painter.line_segment([start, end], egui::Stroke::new(2.3, color));
            let direction = (end - start).normalized();
            let head = end - direction * 7.0;
            let side = egui::vec2(-direction.y, direction.x) * 4.0;
            painter.add(egui::Shape::convex_polygon(
                vec![end, head + side, head - side],
                color,
                egui::Stroke::NONE,
            ));
        }

        let mut moved = false;
        for index in 0..self.project.nodes.len() {
            let node = &self.project.nodes[index];
            let Some(position) = node_positions.get(&node.id).copied() else {
                continue;
            };
            let node_rect =
                egui::Rect::from_min_size(position, egui::vec2(196.0, 88.0) * self.canvas_zoom);
            let response = ui.interact(
                node_rect,
                egui::IdSalt::new(node.id.clone()),
                egui::Sense::click_and_drag(),
            );
            let selected = self.selected_node_id.as_deref() == Some(node.id.as_str());
            if response.clicked() {
                self.selected_node_id = Some(node.id.clone());
            }
            if response.dragged() {
                let delta = response.drag_delta() / self.canvas_zoom;
                self.project.nodes[index].x += f64::from(delta.x);
                self.project.nodes[index].y += f64::from(delta.y);
                moved = true;
            }
            let accent = node_accent(node.kind, node.resource_kind);
            let fill = if selected {
                egui::Color32::from_rgb(41, 47, 47)
            } else {
                egui::Color32::from_rgb(29, 37, 40)
            };
            painter.rect_filled(node_rect, egui::CornerRadius::same(9), fill);
            painter.rect_stroke(
                node_rect,
                egui::CornerRadius::same(9),
                egui::Stroke::new(
                    if selected { 2.0 } else { 1.0 },
                    if selected { copper() } else { border_color() },
                ),
                egui::StrokeKind::Inside,
            );
            painter.rect_filled(
                egui::Rect::from_min_size(node_rect.min, egui::vec2(4.0, node_rect.height())),
                egui::CornerRadius::same(3),
                accent,
            );
            let text_pos = node_rect.min + egui::vec2(14.0, 12.0) * self.canvas_zoom;
            painter.text(
                text_pos,
                egui::Align2::LEFT_TOP,
                format!(
                    "{}  {}",
                    node_kind_icon(node.kind),
                    node_kind_name(node.kind).to_uppercase()
                ),
                egui::FontId::proportional((10.0 * self.canvas_zoom).clamp(9.0, 12.0)),
                accent,
            );
            painter.text(
                text_pos + egui::vec2(0.0, 23.0 * self.canvas_zoom),
                egui::Align2::LEFT_TOP,
                &node.title,
                egui::FontId::proportional((15.0 * self.canvas_zoom).clamp(12.0, 17.0)),
                egui::Color32::from_rgb(236, 239, 237),
            );
            let detail = node
                .resource_id
                .as_ref()
                .map(|id| {
                    let name = self
                        .project
                        .custom_resources
                        .iter()
                        .find(|resource| &resource.id == id)
                        .map(|resource| resource.name_ru.as_str())
                        .unwrap_or(id.as_str());
                    if let Some(rate) = node.source_rate_per_min.or(node.target_rate_per_min) {
                        format!("{name}  ·  {rate:.1} /мин")
                    } else {
                        name.to_owned()
                    }
                })
                .or_else(|| node.note_text.clone())
                .unwrap_or_else(|| "Узел производственной цепочки".to_owned());
            painter.text(
                text_pos + egui::vec2(0.0, 48.0 * self.canvas_zoom),
                egui::Align2::LEFT_TOP,
                detail,
                egui::FontId::proportional((11.0 * self.canvas_zoom).clamp(9.0, 12.0)),
                muted(),
            );
        }
        if moved {
            self.mark_project_changed();
        }

        if self.project.nodes.is_empty() {
            let center = rect.center();
            painter.text(
                center + egui::vec2(0.0, -32.0),
                egui::Align2::CENTER_CENTER,
                "Рабочая область готова",
                egui::FontId::proportional(19.0),
                egui::Color32::from_rgb(207, 215, 214),
            );
            painter.text(
                center + egui::vec2(0.0, -3.0),
                egui::Align2::CENTER_CENTER,
                "Добавьте собственный ресурс, затем узлы цепочки.",
                egui::FontId::proportional(13.0),
                muted(),
            );
            painter.text(
                center + egui::vec2(0.0, 32.0),
                egui::Align2::CENTER_CENTER,
                "＋  Создать ресурс     ·     ＋  Добавить узел",
                egui::FontId::proportional(13.0),
                copper(),
            );
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    if pos.y > center.y + 18.0 && pos.x < center.x {
                        self.active_view = MainView::Resources;
                    } else if pos.y > center.y + 18.0 {
                        self.show_node_dialog = true;
                    }
                }
            }
        }
        ui.horizontal(|ui| {
            ui.small(
                egui::RichText::new(
                    "Колесо: прокрутка · Ctrl+колесо: масштаб · средняя кнопка: перемещение",
                )
                .color(muted()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.colored_label(item_color(), "◆  Предметы");
                ui.colored_label(fluid_color(), "◉  Жидкости");
                ui.colored_label(byproduct_color(), "↗  Побочный продукт");
            });
        });
    }

    fn draw_inspector(&mut self, ui: &mut egui::Ui) {
        ui.heading("Свойства");
        let selected_id = self.selected_node_id.clone();
        let Some(id) = selected_id else {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Выберите узел на графе, чтобы посмотреть его свойства.")
                    .color(muted()),
            );
            ui.add_space(10.0);
            if ui.button("＋  Добавить узел").clicked() {
                self.show_node_dialog = true;
            }
            ui.separator();
            ui.label(egui::RichText::new("Проект").strong());
            ui.label(format!("{}", self.project.name));
            ui.small(format!(
                "{} ресурсов · {} машин · {} рецептов",
                self.project.custom_resources.len(),
                self.project.custom_machines.len(),
                self.project.custom_recipes.len()
            ));
            return;
        };
        let Some(index) = self.project.nodes.iter().position(|node| node.id == id) else {
            self.selected_node_id = None;
            return;
        };
        let mut changed = false;
        let remove_node;
        {
            let node = &mut self.project.nodes[index];
            ui.label(
                egui::RichText::new(node_kind_name(node.kind).to_uppercase())
                    .size(10.0)
                    .strong()
                    .color(node_accent(node.kind, node.resource_kind)),
            );
            ui.label("Название");
            changed |= ui
                .add(egui::TextEdit::singleline(&mut node.title).desired_width(f32::INFINITY))
                .changed();
            if let Some(resource_id) = node.resource_id.as_deref() {
                let resource = self
                    .project
                    .custom_resources
                    .iter()
                    .find(|resource| resource.id == resource_id);
                ui.add_space(5.0);
                ui.label("Ресурс");
                if let Some(resource) = resource {
                    ui.horizontal(|ui| {
                        ui.colored_label(
                            flow_color(resource.kind),
                            resource_kind_icon(resource.kind),
                        );
                        ui.vertical(|ui| {
                            ui.label(&resource.name_ru);
                            ui.small(egui::RichText::new(&resource.id).color(muted()));
                        });
                    });
                }
            }
            if node.kind == NodeKind::Source || node.kind == NodeKind::Target {
                let rate = if node.kind == NodeKind::Source {
                    &mut node.source_rate_per_min
                } else {
                    &mut node.target_rate_per_min
                };
                ui.add_space(5.0);
                ui.label("Темп в минуту");
                let mut value = (*rate)
                    .map(|rate| rate.to_string())
                    .unwrap_or_else(|| "0".to_owned());
                if ui.add(egui::TextEdit::singleline(&mut value)).changed() {
                    if let Ok(parsed) = value.parse::<f64>() {
                        if parsed.is_finite() && parsed >= 0.0 {
                            *rate = Some(parsed);
                            changed = true;
                        }
                    }
                }
            }
            if let Some(note) = node.note_text.as_mut() {
                ui.label("Заметка");
                changed |= ui
                    .add(egui::TextEdit::multiline(note).desired_rows(4))
                    .changed();
            }
            ui.separator();
            ui.small(format!("ID: {}", node.id));
            remove_node = ui
                .button(egui::RichText::new("Удалить узел").color(error_color()))
                .clicked();
        }
        if changed {
            self.mark_project_changed();
        }
        if remove_node {
            self.project.nodes.remove(index);
            self.project
                .edges
                .retain(|edge| edge.from_node_id != id && edge.to_node_id != id);
            self.selected_node_id = None;
            self.mark_project_changed();
        }
    }

    fn draw_analysis(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Расчёт и баланс");
            if self.solver_task_id.is_some() {
                ui.spinner();
                ui.small("Обновление модели…");
            }
            if let Some(result) = &self.solver_result {
                ui.small(format!(
                    "{} узлов · {} компонентов · {:.1} мс",
                    result.stats.total_nodes,
                    result.stats.component_count,
                    result.stats.solve_time_ms
                ));
            }
        });
        if let Some(result) = &self.solver_result {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Машин: {}", result.stats.total_machines_rounded));
                ui.separator();
                ui.label(format!(
                    "Суммарный стресс: {:.0} SU",
                    result.stats.total_stress_su
                ));
                ui.separator();
                let errors = result
                    .warnings
                    .iter()
                    .filter(|warning| warning.severity == WarningSeverity::Error)
                    .count();
                let warnings = result
                    .warnings
                    .iter()
                    .filter(|warning| warning.severity == WarningSeverity::Warning)
                    .count();
                ui.colored_label(
                    if errors > 0 {
                        error_color()
                    } else {
                        fluid_color()
                    },
                    format!("{} ошибки", errors),
                );
                ui.colored_label(
                    if warnings > 0 {
                        byproduct_color()
                    } else {
                        muted()
                    },
                    format!("{} предупреждений", warnings),
                );
            });
            egui::ScrollArea::horizontal()
                .max_height(54.0)
                .show(ui, |ui| {
                    for warning in result.warnings.iter().take(8) {
                        let color = match warning.severity {
                            WarningSeverity::Error => error_color(),
                            WarningSeverity::Warning => byproduct_color(),
                            WarningSeverity::Info => muted(),
                        };
                        ui.colored_label(color, format!("• {}", warning.message_ru));
                    }
                    if result.warnings.is_empty() {
                        ui.small(
                            egui::RichText::new("Предупреждений нет. Расчёт обновляется в фоне.")
                                .color(muted()),
                        );
                    }
                });
        } else if self.solver_task_id.is_none() {
            ui.small(egui::RichText::new("Добавьте узлы и рецепты — здесь появятся баланс потоков, нагрузка машин и предупреждения.").color(muted()));
        }
    }

    fn draw_node_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_node_dialog {
            return;
        }
        let mut open = self.show_node_dialog;
        egui::Window::new("Добавить узел")
            .collapsible(false)
            .resizable(false)
            .default_width(430.0)
            .open(&mut open)
            .show(ctx, |ui| {
                self.draw_node_editor(ui);
            });
        self.show_node_dialog = open;
    }

    fn draw_link_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_link_dialog {
            return;
        }
        let mut open = self.show_link_dialog;
        egui::Window::new("Соединить узлы")
            .collapsible(false)
            .resizable(false)
            .default_width(390.0)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Создайте связь между источником и целью с одинаковым ресурсом.");
                self.draw_edge_editor(ui);
            });
        self.show_link_dialog = open;
    }

    fn draw_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(3.0);
            ui.label(
                egui::RichText::new("FA")
                    .size(16.0)
                    .strong()
                    .color(copper()),
            );
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Factory Atlas").strong().size(15.0));
                ui.small(
                    egui::RichText::new("ПЛАНИРОВЩИК ПРОИЗВОДСТВА")
                        .size(9.0)
                        .color(muted()),
                );
            });
            ui.separator();
            if ui
                .add(
                    egui::TextEdit::singleline(&mut self.project.name)
                        .desired_width(205.0)
                        .hint_text("Название проекта"),
                )
                .changed()
            {
                self.mark_project_changed();
            }
            if ui.button("Новый").clicked() {
                self.request_action(PendingAction::NewProject);
            }
            if ui.button("Открыть").clicked() {
                self.choose_project_to_open();
            }
            if ui.button("Сохранить").clicked() {
                self.save_current();
            }
            if ui.button("Сохранить как").clicked() {
                self.save_as();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let theme_label = if self.settings.theme == ThemeMode::Dark {
                    "☼ Светлая"
                } else {
                    "◐ Тёмная"
                };
                if ui.button(theme_label).clicked() {
                    self.toggle_theme(ui.ctx());
                }
                ui.label(
                    egui::RichText::new(if self.dirty {
                        "● Не сохранено"
                    } else {
                        "● Сохранено"
                    })
                    .color(if self.dirty {
                        byproduct_color()
                    } else {
                        fluid_color()
                    }),
                );
            });
        });
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
            ui.horizontal(|ui| {
                ui.colored_label(
                    node_accent(node.kind, node.resource_kind),
                    node_kind_icon(node.kind),
                );
                ui.label(&node.title);
            });
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
                    if ui.button("Сохранить").clicked()
                        && self.save_current()
                        && let Some(action) = self.pending_action.take()
                    {
                        self.apply_action(action);
                    }
                    if ui.button("Продолжить без сохранения").clicked()
                        && let Some(action) = self.pending_action.take()
                    {
                        self.apply_action(action);
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
        let ctx = ui.ctx().clone();
        if self.applied_theme != Some(self.settings.theme) {
            self.apply_theme(&ctx);
        }
        self.poll_solver();
        self.schedule_solver();

        egui::Panel::top("factory-atlas-toolbar")
            .exact_size(55.0)
            .show_inside(ui, |ui| self.draw_toolbar(ui));
        egui::Panel::bottom("factory-atlas-status")
            .exact_size(28.0)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(
                        if self.status_is_error {
                            error_color()
                        } else {
                            muted()
                        },
                        &self.status,
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let rect = ui.ctx().input(|input| input.content_rect());
                        ui.small(format!(
                            "{} × {}",
                            rect.width() as u32,
                            rect.height() as u32
                        ));
                    });
                });
            });
        if self.active_view == MainView::Workspace {
            egui::Panel::bottom("factory-atlas-analysis")
                .resizable(true)
                .default_size(132.0)
                .size_range(88.0..=260.0)
                .show_inside(ui, |ui| self.draw_analysis(ui));
        }
        egui::Panel::left("factory-atlas-navigation")
            .exact_size(166.0)
            .show_inside(ui, |ui| {
                ui.add_space(7.0);
                self.draw_navigation(ui);
            });
        if self.active_view == MainView::Workspace {
            egui::Panel::left("factory-atlas-catalog")
                .resizable(true)
                .default_size(238.0)
                .size_range(185.0..=370.0)
                .show_inside(ui, |ui| self.draw_catalog(ui));
            egui::Panel::right("factory-atlas-inspector")
                .resizable(true)
                .default_size(270.0)
                .size_range(215.0..=390.0)
                .show_inside(ui, |ui| self.draw_inspector(ui));
            egui::CentralPanel::default().show(ui, |ui| self.draw_canvas(ui));
        } else {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.heading("Каталог ресурсов");
                ui.label(egui::RichText::new("Создавайте собственные предметы и жидкости. Они принадлежат только этому проекту.").color(muted()));
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| self.draw_resource_editor(ui));
            });
        }
        self.draw_node_dialog(&ctx);
        self.draw_link_dialog(&ctx);
        self.draw_pending_action_dialog(&ctx);
        self.draw_close_dialog(&ctx);
    }
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

fn copper() -> egui::Color32 {
    egui::Color32::from_rgb(226, 142, 91)
}
fn muted() -> egui::Color32 {
    egui::Color32::from_rgb(137, 151, 153)
}
fn border_color() -> egui::Color32 {
    egui::Color32::from_rgb(50, 63, 67)
}
fn item_color() -> egui::Color32 {
    egui::Color32::from_rgb(107, 178, 228)
}
fn fluid_color() -> egui::Color32 {
    egui::Color32::from_rgb(77, 190, 177)
}
fn byproduct_color() -> egui::Color32 {
    egui::Color32::from_rgb(233, 183, 92)
}
fn error_color() -> egui::Color32 {
    egui::Color32::from_rgb(235, 111, 99)
}

fn flow_color(kind: ResourceKind) -> egui::Color32 {
    match kind {
        ResourceKind::Item => item_color(),
        ResourceKind::Fluid => fluid_color(),
    }
}

fn node_accent(kind: NodeKind, resource_kind: Option<ResourceKind>) -> egui::Color32 {
    match kind {
        NodeKind::Source => egui::Color32::from_rgb(115, 190, 144),
        NodeKind::Target => egui::Color32::from_rgb(231, 174, 93),
        NodeKind::Note => egui::Color32::from_rgb(173, 157, 220),
        NodeKind::Byproduct => byproduct_color(),
        NodeKind::Machine | NodeKind::Recipe => resource_kind.map(flow_color).unwrap_or(copper()),
    }
}

fn node_kind_icon(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Source => "◉",
        NodeKind::Machine => "⚙",
        NodeKind::Recipe => "≋",
        NodeKind::Byproduct => "↗",
        NodeKind::Target => "◎",
        NodeKind::Note => "▤",
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
        Box::new(|creation_context| {
            let app = FactoryAtlasApp::new();
            let ctx = creation_context.egui_ctx.clone();
            app.solver_pool
                .set_wakeup(Arc::new(move || ctx.request_repaint()));
            Ok(Box::new(app))
        }),
    )
}
