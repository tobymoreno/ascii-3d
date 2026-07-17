use crate::editor_core::EditorCommand;

use super::app::NativeEditorApp;

const WINDOW_FILL: egui::Color32 = egui::Color32::from_rgb(18, 20, 24);
const PANEL_FILL: egui::Color32 = egui::Color32::from_rgb(25, 28, 34);
const BAR_FILL: egui::Color32 = egui::Color32::from_rgb(22, 24, 29);
const VIEWPORT_FILL: egui::Color32 = egui::Color32::from_rgb(35, 39, 46);
const BORDER_COLOR: egui::Color32 = egui::Color32::from_rgb(70, 76, 88);
const TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(225, 228, 234);
const MUTED_TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(150, 156, 168);
const HIERARCHY_WIDTH: f32 = 230.0;
const PROPERTIES_WIDTH: f32 = 280.0;

pub(crate) fn draw(app: &mut NativeEditorApp, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    if ui.ctx().input(|input| input.key_pressed(egui::Key::Escape)) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        return;
    }

    configure_visuals(ui);

    egui::Frame::new()
        .fill(WINDOW_FILL)
        .inner_margin(egui::Margin::same(0))
        .show(ui, |ui| {
            draw_menu_bar(app, ui);
            draw_body(app, ui);
            draw_status_bar(app, ui);
        });
}

fn configure_visuals(ui: &mut egui::Ui) {
    let visuals = ui.visuals_mut();
    visuals.override_text_color = Some(TEXT_COLOR);
    visuals.panel_fill = WINDOW_FILL;
    visuals.window_fill = PANEL_FILL;
    visuals.faint_bg_color = PANEL_FILL;
    visuals.extreme_bg_color = WINDOW_FILL;
    visuals.widgets.noninteractive.fg_stroke.color = TEXT_COLOR;
    visuals.widgets.inactive.fg_stroke.color = TEXT_COLOR;
    visuals.widgets.hovered.fg_stroke.color = egui::Color32::WHITE;
    visuals.widgets.active.fg_stroke.color = egui::Color32::WHITE;
}

fn draw_menu_bar(app: &mut NativeEditorApp, ui: &mut egui::Ui) {
    egui::Frame::new()
        .fill(BAR_FILL)
        .inner_margin(egui::Margin::symmetric(8, 5))
        .show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                menu_button(ui, "File", &["New", "Open…", "Save", "Exit"]);
                menu_button(ui, "Edit", &["Undo", "Redo", "Preferences"]);
                menu_button(ui, "View", &["Reset View", "Frame Selection"]);
                menu_button(ui, "Objects", &["Add", "Duplicate", "Delete"]);
                menu_button(ui, "Debug", &["GPU Info", "Show Metrics"]);
                menu_button(ui, "Help", &["Controls", "About"]);

                ui.separator();
                ui.colored_label(
                    MUTED_TEXT_COLOR,
                    "Phase 4 shell — geometry rendering intentionally disabled",
                );
            });
        });

    if ui.ctx().input(|input| input.viewport().close_requested()) {
        app.status = "Closing native editor".to_owned();
    }
}

fn menu_button(ui: &mut egui::Ui, title: &str, items: &[&str]) {
    ui.menu_button(title, |ui| {
        for item in items {
            ui.add_enabled(false, egui::Button::new(*item));
        }
    });
}

fn draw_body(app: &mut NativeEditorApp, ui: &mut egui::Ui) {
    let body_height = (ui.available_height() - 29.0).max(120.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;

        ui.allocate_ui_with_layout(
            egui::vec2(HIERARCHY_WIDTH, body_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                egui::Frame::new()
                    .fill(PANEL_FILL)
                    .stroke(egui::Stroke::new(1.0, BORDER_COLOR))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(
                            HIERARCHY_WIDTH - 20.0,
                            (body_height - 20.0).max(0.0),
                        ));
                        draw_hierarchy(app, ui);
                    });
            },
        );

        let viewport_width =
            (ui.available_width() - PROPERTIES_WIDTH - ui.spacing().item_spacing.x).max(160.0);
        ui.allocate_ui_with_layout(
            egui::vec2(viewport_width, body_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| draw_viewport(ui),
        );

        ui.allocate_ui_with_layout(
            egui::vec2(PROPERTIES_WIDTH, body_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                egui::Frame::new()
                    .fill(PANEL_FILL)
                    .stroke(egui::Stroke::new(1.0, BORDER_COLOR))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(
                            PROPERTIES_WIDTH - 20.0,
                            (body_height - 20.0).max(0.0),
                        ));
                        draw_properties(app, ui);
                    });
            },
        );
    });
}

fn draw_hierarchy(app: &mut NativeEditorApp, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        ui.heading("Hierarchy");
        ui.separator();
        ui.add_space(3.0);

        let entries = app.session.entries().to_vec();
        let selected_index = app.session.selected_entry();

        for (index, entry) in entries.into_iter().enumerate() {
            let selected = selected_index == index;
            let response = ui.add_sized(
                [ui.available_width(), 24.0],
                egui::Button::selectable(selected, entry.target.label()),
            );

            if response.clicked() {
                app.session.apply(EditorCommand::SelectIndex(index));
                app.session.apply(EditorCommand::InspectSelected);
                app.status = format!("Selected {}", entry.target.label());
            }
        }
    });
}

fn draw_properties(app: &mut NativeEditorApp, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        ui.heading("Properties");
        ui.separator();
        ui.add_space(3.0);

        let Some(target) = app.session.inspected_target().cloned() else {
            ui.colored_label(MUTED_TEXT_COLOR, "Select an item in the hierarchy.");
            return;
        };

        ui.label(egui::RichText::new(target.label()).strong().size(17.0));
        ui.add_space(8.0);

        egui::Grid::new("native_editor_properties_grid")
            .num_columns(2)
            .spacing([14.0, 9.0])
            .show(ui, |ui| {
                ui.label("Active target");
                ui.label(if app.session.is_active(&target) {
                    "Yes"
                } else {
                    "No"
                });
                ui.end_row();

                ui.label("Visible");
                match app.session.visibility(&target) {
                    Some(mut visible) => {
                        if ui.checkbox(&mut visible, "").changed() {
                            app.session.apply(EditorCommand::SetVisibility {
                                target: target.clone(),
                                visible,
                            });
                        }
                    }
                    None => {
                        ui.colored_label(MUTED_TEXT_COLOR, "N/A");
                    }
                }
                ui.end_row();

                ui.label("Gizmo");
                let mut gizmo_visible = app.session.gizmo_visible(&target);
                if ui.checkbox(&mut gizmo_visible, "").changed() {
                    app.session
                        .apply(EditorCommand::ToggleGizmo(target.clone()));
                }
                ui.end_row();
            });

        ui.add_space(12.0);
        if ui.button("Make Active").clicked() {
            app.session.apply(EditorCommand::Activate(target.clone()));
            app.status = format!("Activated {}", target.label());
        }

        ui.separator();
        ui.colored_label(
            MUTED_TEXT_COLOR,
            "Transform controls arrive after the native viewport pipeline.",
        );
    });
}

fn draw_viewport(ui: &mut egui::Ui) {
    let available = ui.available_rect_before_wrap();
    let response = ui.allocate_rect(available, egui::Sense::hover());

    ui.painter().rect_filled(response.rect, 0.0, VIEWPORT_FILL);
    ui.painter().rect_stroke(
        response.rect.shrink(0.5),
        0.0,
        egui::Stroke::new(1.0, BORDER_COLOR),
        egui::StrokeKind::Inside,
    );

    let text =
        "Viewport\nwgpu-backed clear surface\nA3D rendering is intentionally not connected yet";
    ui.painter().text(
        response.rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(16.0),
        egui::Color32::from_rgb(190, 195, 205),
    );
}

fn draw_status_bar(app: &NativeEditorApp, ui: &mut egui::Ui) {
    egui::Frame::new()
        .fill(BAR_FILL)
        .inner_margin(egui::Margin::symmetric(8, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(&app.status);
                ui.separator();
                ui.colored_label(MUTED_TEXT_COLOR, "FPS: --");
                ui.separator();
                ui.colored_label(MUTED_TEXT_COLOR, "Renderer: wgpu");
            });
        });
}
