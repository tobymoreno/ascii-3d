use super::SceneDocument;
use crate::render::{
    RenderCamera, RenderDisplay, RenderGeoJsonMapOverlay, RenderLighting, RenderObject,
    RenderOverlay, RenderProjectionConfig, RenderQuad, RenderQuadGroup, RenderScene,
    RenderTransform,
};

const DEFAULT_CAMERA_ID: &str = "default";

const DEFAULT_CAMERA_DISTANCE: f32 = 8.0;
const DEFAULT_NEAR_CLIP: f32 = 0.25;
const DEFAULT_VERTICAL_CENTER_RATIO: f32 = 0.52;

pub fn scene_document_to_render_scene(document: SceneDocument) -> RenderScene {
    let mut scene = RenderScene::new(
        document.name,
        RenderDisplay {
            world_scale: document.display.world_scale,
        },
    );

    scene.cameras.push(RenderCamera {
        id: DEFAULT_CAMERA_ID.to_string(),
        transform: RenderTransform::default(),
        projection: RenderProjectionConfig {
            camera_distance: DEFAULT_CAMERA_DISTANCE,
            near_clip: DEFAULT_NEAR_CLIP,
            vertical_center_ratio: DEFAULT_VERTICAL_CENTER_RATIO,
        },
    });
    scene.active_camera_id = Some(DEFAULT_CAMERA_ID.to_string());

    scene.lighting = document.lighting.map(|lighting| RenderLighting {
        primary_light_direction: lighting.primary_light_direction,
    });

    if !document.quads.is_empty() {
        scene.objects.push(RenderObject::QuadGroup(RenderQuadGroup {
            quads: document
                .quads
                .into_iter()
                .map(|quad| RenderQuad {
                    id: quad.id,
                    position: quad.position,
                    size: quad.size,
                    rotation_z_degrees: quad.rotation_z_degrees,
                    marker: quad.marker,
                    color: quad.color,
                })
                .collect(),
            transform: RenderTransform::default(),
        }));
    }

    if let Some(map_overlay) = document.map_overlay {
        scene
            .overlays
            .push(RenderOverlay::GeoJsonMap(RenderGeoJsonMapOverlay {
                asset: map_overlay.asset,
                visible: map_overlay.visible,
                radius_scale: map_overlay.radius_scale,
            }));
    }

    scene
}
