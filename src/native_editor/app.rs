use std::error::Error;

use crate::editor_core::{EditorEntry, EditorSession};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NativeEditorTarget {
    Scene,
    Camera,
    Light,
}

impl NativeEditorTarget {
    pub(crate) const fn label(&self) -> &'static str {
        match self {
            Self::Scene => "Scene",
            Self::Camera => "Camera",
            Self::Light => "Light",
        }
    }
}

pub(crate) struct NativeEditorApp {
    pub(crate) session: EditorSession<NativeEditorTarget>,
    pub(crate) status: String,
}

impl Default for NativeEditorApp {
    fn default() -> Self {
        Self {
            session: EditorSession::new(
                vec![
                    EditorEntry::new(NativeEditorTarget::Scene, Some(true)),
                    EditorEntry::new(NativeEditorTarget::Camera, None),
                    EditorEntry::new(NativeEditorTarget::Light, Some(true)),
                ],
                NativeEditorTarget::Scene,
            ),
            status: "Native editor shell ready".to_owned(),
        }
    }
}

impl eframe::App for NativeEditorApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        super::gui::draw(self, ui, frame);
    }
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ascii-3d Native Editor")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 520.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "ascii-3d Native Editor",
        options,
        Box::new(|creation_context| {
            creation_context.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(NativeEditorApp::default()))
        }),
    )?;

    Ok(())
}
