use crate::editor_core::EditorCommand;

use super::{
    app::{NativeEditorApp, NativeEditorTarget},
    gpu_renderer::GpuViewportCallback,
};

const WINDOW_FILL: egui::Color32 = egui::Color32::from_rgb(18, 20, 24);
const PANEL_FILL: egui::Color32 = egui::Color32::from_rgb(25, 28, 34);
const BAR_FILL: egui::Color32 = egui::Color32::from_rgb(22, 24, 29);
const BORDER_COLOR: egui::Color32 = egui::Color32::from_rgb(70, 76, 88);
const TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(225, 228, 234);
const MUTED_TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(150, 156, 168);
const HIERARCHY_WIDTH: f32 = 230.0;
const PROPERTIES_WIDTH: f32 = 280.0;
const VIEWPORT_MARGIN: f32 = 12.0;

pub(crate) fn draw(app: &mut NativeEditorApp, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    if ui.ctx().input(|input| input.key_pressed(egui::Key::Escape)) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        return;
    }

    if ui.ctx().input(|input| {
        input.key_pressed(egui::Key::F) && !input.modifiers.ctrl && !input.modifiers.alt
    }) {
        app.frame_selected();
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
                file_menu(app, ui);
                menu_button(ui, "Edit", &["Undo", "Redo", "Preferences"]);
                view_menu(app, ui);
                menu_button(ui, "Objects", &["Add", "Duplicate", "Delete"]);
                debug_menu(app, ui);
                menu_button(ui, "Help", &["Controls", "About"]);

                ui.separator();
                ui.colored_label(MUTED_TEXT_COLOR, "Phase 10 - toon-shaded wgpu viewport");
            });
        });

    if ui.ctx().input(|input| input.viewport().close_requested()) {
        app.status = "Closing native editor".to_owned();
    }
}

fn file_menu(app: &mut NativeEditorApp, ui: &mut egui::Ui) {
    ui.menu_button("File", |ui| {
        ui.add_enabled(false, egui::Button::new("New"));

        if ui.button("Open...").clicked() {
            ui.close();
            let mut dialog = rfd::FileDialog::new()
                .add_filter("A3D scene", &["a3d"])
                .set_title("Open A3D Scene");

            if let Some(parent) = app.scene_path().parent() {
                dialog = dialog.set_directory(parent);
            }

            match dialog.pick_file() {
                Some(path) => {
                    app.load_scene(path);
                }
                None => {
                    app.status = "Open canceled".to_owned();
                }
            }
        }

        ui.add_enabled(false, egui::Button::new("Save"));
        ui.separator();
        if ui.button("Exit").clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    });
}

fn view_menu(app: &mut NativeEditorApp, ui: &mut egui::Ui) {
    ui.menu_button("View", |ui| {
        if ui.button("Reset Camera").clicked() {
            app.reset_camera();
            ui.close();
        }
        if ui.button("Frame Selected    F").clicked() {
            app.frame_selected();
            ui.close();
        }
    });
}

fn debug_menu(app: &mut NativeEditorApp, ui: &mut egui::Ui) {
    ui.menu_button("Debug", |ui| {
        let mut show_wireframe = app.show_wireframe();
        if ui.checkbox(&mut show_wireframe, "Show Wireframe").changed() {
            app.set_show_wireframe(show_wireframe);
        }

        ui.separator();
        ui.label("Outline width");
        let mut outline_width = app.outline_pixel_width();
        if ui
            .add(egui::Slider::new(&mut outline_width, 0.5..=4.0).suffix(" px"))
            .changed()
        {
            app.set_outline_pixel_width(outline_width);
        }

        ui.label("Viewport background");
        let mut background = app.viewport_background_rgb();
        if ui.color_edit_button_srgb(&mut background).changed() {
            app.set_viewport_background_rgb(background);
        }

        if ui.button("Reset Viewport Style").clicked() {
            app.reset_viewport_style();
        }

        ui.separator();
        ui.add_enabled(false, egui::Button::new("GPU Info"));
        ui.add_enabled(false, egui::Button::new("Show Metrics"));
    });
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
            |ui| draw_viewport(app, ui),
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
                app.apply_editor_command(EditorCommand::SelectIndex(index));
                app.apply_editor_command(EditorCommand::InspectSelected);
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
                            app.apply_editor_command(EditorCommand::SetVisibility {
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
                    app.apply_editor_command(EditorCommand::ToggleGizmo(target.clone()));
                }
                ui.end_row();
            });

        ui.add_space(12.0);
        if ui.button("Make Active").clicked() {
            app.apply_editor_command(EditorCommand::Activate(target.clone()));
            app.status = format!("Activated {}", target.label());
        }

        ui.add_space(10.0);
        draw_target_details(app, ui, &target);

        ui.separator();
        ui.colored_label(
            MUTED_TEXT_COLOR,
            "Values are loaded from the A3D world. Editing arrives with transform controls.",
        );
    });
}

fn draw_target_details(app: &mut NativeEditorApp, ui: &mut egui::Ui, target: &NativeEditorTarget) {
    match target {
        NativeEditorTarget::Scene => {
            ui.label(egui::RichText::new("Scene data").strong());
            egui::Grid::new("native_editor_scene_data")
                .num_columns(2)
                .spacing([14.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Title");
                    ui.label(app.scene_title());
                    ui.end_row();

                    ui.label("Source");
                    ui.label(app.scene_path().display().to_string());
                    ui.end_row();

                    ui.label("Editor objects");
                    ui.label(app.editor_object_count().to_string());
                    ui.end_row();
                });

            ui.add_space(10.0);
            ui.label(egui::RichText::new("Viewport style").strong());
            let mut background = app.viewport_background_rgb();
            ui.horizontal(|ui| {
                ui.label("Background");
                if ui.color_edit_button_srgb(&mut background).changed() {
                    app.set_viewport_background_rgb(background);
                }
            });
            if ui.button("Reset Viewport Style").clicked() {
                app.reset_viewport_style();
            }
        }
        NativeEditorTarget::Camera => {
            let position = app.camera_position();
            let camera_target = app.camera_target();
            let angles = app.camera_angles_degrees();

            ui.label(egui::RichText::new("Editor camera").strong());
            egui::Grid::new("native_editor_camera_grid")
                .num_columns(2)
                .spacing([14.0, 8.0])
                .show(ui, |ui| {
                    vec3_row(ui, "Position", position);
                    vec3_row(ui, "Target", camera_target);

                    ui.label("Distance");
                    ui.monospace(format!("{:.3}", app.camera_distance()));
                    ui.end_row();

                    ui.label("Yaw / Pitch");
                    ui.monospace(format!("{:.1} / {:.1} deg", angles[0], angles[1]));
                    ui.end_row();
                });

            ui.add_space(8.0);
            if ui.button("Reset Camera").clicked() {
                app.reset_camera();
            }
        }
        NativeEditorTarget::Object(_) => {
            let Some(transform) = app.object_transform(target) else {
                ui.colored_label(MUTED_TEXT_COLOR, "Object data is unavailable.");
                return;
            };

            ui.label(egui::RichText::new("Transform").strong());
            egui::Grid::new("native_editor_transform_grid")
                .num_columns(2)
                .spacing([14.0, 8.0])
                .show(ui, |ui| {
                    transform_row(ui, "Position", transform.position);
                    transform_row(ui, "Rotation", transform.rotation_degrees);
                    transform_row(ui, "Scale", transform.scale);
                });

            ui.add_space(8.0);
            if ui.button("Frame Selected (F)").clicked() {
                app.frame_selected();
            }

            ui.add_space(12.0);
            ui.label(egui::RichText::new("Toon material").strong());
            let uses_fallback = app.object_uses_fallback_toon_material(target);
            if uses_fallback {
                ui.colored_label(
                    MUTED_TEXT_COLOR,
                    "Renderer fallback — editing creates an object override.",
                );
                ui.add_space(4.0);
            }
            if let Some(mut material) = app.object_toon_material(target) {
                let mut changed = false;

                ui.horizontal(|ui| {
                    ui.label("Base color");
                    changed |= ui.color_edit_button_rgb(&mut material.base_color).changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Outline color");
                    changed |= ui
                        .color_edit_button_rgb(&mut material.outline_color)
                        .changed();
                });
                changed |= ui
                    .add(
                        egui::Slider::new(&mut material.outline_width, 0.0..=4.0)
                            .text("Outline width"),
                    )
                    .changed();

                ui.label("Shade multipliers");
                changed |= ui
                    .add(egui::Slider::new(&mut material.shade_bands[0], 0.1..=1.0).text("Dark"))
                    .changed();
                changed |= ui
                    .add(egui::Slider::new(&mut material.shade_bands[1], 0.1..=1.0).text("Mid"))
                    .changed();
                changed |= ui
                    .add(egui::Slider::new(&mut material.shade_bands[2], 0.1..=1.2).text("Light"))
                    .changed();

                changed |= ui
                    .checkbox(&mut material.smooth_shading, "Smooth shading")
                    .changed();
                changed |= ui.checkbox(&mut material.line_only, "Line only").changed();
                if material.line_only {
                    ui.colored_label(
                        MUTED_TEXT_COLOR,
                        "Line only uses base-color strokes with an outline border.",
                    );
                }

                if changed {
                    app.set_object_toon_material(target, material);
                }
            }
        }
    }
}

fn vec3_row(ui: &mut egui::Ui, label: &str, value: crate::math::Vec3) {
    ui.label(label);
    ui.monospace(format!("{:.3}, {:.3}, {:.3}", value.x, value.y, value.z));
    ui.end_row();
}

fn transform_row(ui: &mut egui::Ui, label: &str, values: [f32; 3]) {
    ui.label(label);
    ui.monospace(format!(
        "{:.3}, {:.3}, {:.3}",
        values[0], values[1], values[2]
    ));
    ui.end_row();
}

fn draw_viewport(app: &mut NativeEditorApp, ui: &mut egui::Ui) {
    let available = ui.available_rect_before_wrap();
    let response = ui.allocate_rect(available, egui::Sense::click_and_drag());

    let background = app.viewport_background_rgb();
    ui.painter().rect_filled(
        response.rect,
        0.0,
        egui::Color32::from_rgb(background[0], background[1], background[2]),
    );
    ui.painter().rect_stroke(
        response.rect.shrink(0.5),
        0.0,
        egui::Stroke::new(1.0, BORDER_COLOR),
        egui::StrokeKind::Inside,
    );

    if response.hovered() {
        let scroll_delta = ui.ctx().input(|input| input.smooth_scroll_delta.y);
        if scroll_delta.abs() > f32::EPSILON {
            app.zoom_camera(scroll_delta);
        }
    }

    let pointer_delta = ui.ctx().input(|input| input.pointer.delta());
    if response.dragged_by(egui::PointerButton::Secondary) {
        let orbit_focus = ui.ctx().input(|input| input.modifiers.alt);
        if orbit_focus {
            app.orbit_focus([pointer_delta.x, pointer_delta.y]);
        } else {
            app.look_camera([pointer_delta.x, pointer_delta.y]);
        }
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if response.dragged_by(egui::PointerButton::Middle) {
        app.pan_camera([pointer_delta.x, pointer_delta.y]);
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }

    let render_rect = response.rect.shrink(VIEWPORT_MARGIN);
    let viewport_painter = ui.painter().with_clip_rect(render_rect);
    let (fill_vertices, hull_vertices, line_vertices, geo_fill_vertices) =
        app.viewport_gpu_geometry(render_rect.width(), render_rect.height());
    let triangle_count = fill_vertices.len() / 3;
    let edge_count = line_vertices.len() / 6;

    if let Some(target_format) = app.gpu_target_format() {
        viewport_painter.add(egui_wgpu::Callback::new_paint_callback(
            render_rect,
            GpuViewportCallback::new(
                fill_vertices,
                hull_vertices,
                line_vertices,
                geo_fill_vertices,
                target_format,
            ),
        ));
    }

    if triangle_count == 0 && edge_count == 0 {
        viewport_painter.text(
            render_rect.center(),
            egui::Align2::CENTER_CENTER,
            "No renderable mesh geometry",
            egui::FontId::proportional(16.0),
            MUTED_TEXT_COLOR,
        );
    } else {
        viewport_painter.text(
            render_rect.left_top(),
            egui::Align2::LEFT_TOP,
            format!(
                "{triangle_count} toon triangles + {edge_count} outline edges | Wheel: zoom  Middle: pan  Right: look  Alt+Right: orbit focus  F: frame selected"
            ),
            egui::FontId::monospace(12.0),
            MUTED_TEXT_COLOR,
        );
    }
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
                ui.separator();
                ui.colored_label(
                    MUTED_TEXT_COLOR,
                    format!("Objects: {}", app.editor_object_count()),
                );
                ui.separator();
                ui.colored_label(
                    MUTED_TEXT_COLOR,
                    format!("Meshes: {}", app.mesh_asset_count()),
                );
            });
        });
}
