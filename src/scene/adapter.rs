use crate::render::GreatCircle;
use super::SceneDocument;
use crate::render::{
    RenderAxis, RenderBehavior, RenderCamera, RenderDisplay, RenderGeoJsonMapOverlay,
    RenderGroup, RenderLighting, RenderNode, RenderObject, RenderObjectNode,
    RenderProjectionConfig, RenderQuad, RenderQuadGroup, RenderScene, RenderSphereGuide,
    RenderSphereGuideKind, RenderSpinBehavior, RenderTransform,
};

const DEFAULT_CAMERA_ID: &str = "default";

const DEFAULT_CAMERA_DISTANCE: f32 = 8.0;
const DEFAULT_NEAR_CLIP: f32 = 0.25;
const DEFAULT_VERTICAL_CENTER_RATIO: f32 = 0.52;

const DEFAULT_MESH_CAMERA_DISTANCE: f32 = 34.0;
const DEFAULT_MESH_NEAR_CLIP: f32 = 1.0;
const DEFAULT_MESH_VERTICAL_CENTER_RATIO: f32 = 0.54;

pub fn scene_document_to_render_scene(document: SceneDocument) -> RenderScene {
    let mut scene = RenderScene::new(
        document.name,
        RenderDisplay {
            world_scale: document.display.world_scale,
        },
    );

    let mesh_only_scene = !document.mesh_asset.trim().is_empty() && document.quads.is_empty();

    let projection = if mesh_only_scene {
        RenderProjectionConfig {
            camera_distance: DEFAULT_MESH_CAMERA_DISTANCE,
            near_clip: DEFAULT_MESH_NEAR_CLIP,
            vertical_center_ratio: DEFAULT_MESH_VERTICAL_CENTER_RATIO,
        }
    } else {
        RenderProjectionConfig {
            camera_distance: DEFAULT_CAMERA_DISTANCE,
            near_clip: DEFAULT_NEAR_CLIP,
            vertical_center_ratio: DEFAULT_VERTICAL_CENTER_RATIO,
        }
    };

    scene.cameras.push(RenderCamera {
        id: DEFAULT_CAMERA_ID.to_string(),
        transform: RenderTransform::default(),
        projection,
    });
    scene.active_camera_id = Some(DEFAULT_CAMERA_ID.to_string());

    scene.lighting = document.lighting.map(|lighting| RenderLighting {
        primary_light_direction: lighting.primary_light_direction,
    });

    
    let mut root_group = RenderGroup::new("root", "Root");

    if !document.quads.is_empty() {
        let quad_group = RenderQuadGroup {
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
        };

        scene.objects.push(RenderObject::QuadGroup(quad_group.clone()));

        root_group.children.push(RenderNode::Object(RenderObjectNode::new(
            "quads",
            "Quads",
            RenderObject::QuadGroup(quad_group),
        )));
    }

    if mesh_only_scene {
        let mesh_object = RenderObject::Mesh(crate::render::RenderMeshObject {
            mesh_asset: document.mesh_asset,
            transform: RenderTransform::default(),
        });

        let mut earth_group = RenderGroup::new("earth", "Earth");
        earth_group
            .behaviors
            .push(RenderBehavior::Spin(RenderSpinBehavior::new(
                RenderAxis::Y,
                15.0,
            )));

        earth_group.children.push(RenderNode::Object(RenderObjectNode::new(
            "mesh",
            "Mesh",
            mesh_object,
        )));

        if let Some(map_overlay) = document.map_overlay {
            earth_group.children.push(RenderNode::Object(RenderObjectNode::new(
                "map",
                "Map",
                RenderObject::GeoJsonMap(RenderGeoJsonMapOverlay {
                    asset: map_overlay.asset,
                    visible: map_overlay.visible,
                    radius_scale: map_overlay.radius_scale,
                }),
            )));
        }

        let mut graticule_group = RenderGroup::new("graticule", "Graticule");
        graticule_group.editor_composite = true;

        graticule_group
            .children
            .push(RenderNode::Object(RenderObjectNode::new(
                "guide-equator",
                "Guide Equator",
                RenderObject::SphereGuide(RenderSphereGuide {
                    kind: RenderSphereGuideKind::GreatCircle(GreatCircle::EquatorY0),
                    marker: 'e',
                    visible: true,
                    radius_scale: 1.01,
                }),
            )));
        graticule_group
            .children
            .push(RenderNode::Object(RenderObjectNode::new(
                "guide-meridian-x",
                "Guide Meridian X",
                RenderObject::SphereGuide(RenderSphereGuide {
                    kind: RenderSphereGuideKind::GreatCircle(GreatCircle::MeridianX0),
                    marker: 'm',
                    visible: true,
                    radius_scale: 1.01,
                }),
            )));
        graticule_group
            .children
            .push(RenderNode::Object(RenderObjectNode::new(
                "guide-meridian-z",
                "Guide Meridian Z",
                RenderObject::SphereGuide(RenderSphereGuide {
                    kind: RenderSphereGuideKind::GreatCircle(GreatCircle::MeridianZ0),
                    marker: 'p',
                    visible: true,
                    radius_scale: 1.01,
                }),
            )));

        for (id, name, latitude_degrees, marker) in [
            ("guide-lat-60", "Guide Latitude 60", 60.0, 'N'),
            ("guide-lat-30", "Guide Latitude 30", 30.0, 'n'),
            ("guide-lat-15", "Guide Latitude 15", 15.0, '.'),
            ("guide-lat--30", "Guide Latitude -30", -30.0, 's'),
        ] {
            graticule_group
                .children
                .push(RenderNode::Object(RenderObjectNode::new(
                    id,
                    name,
                    RenderObject::SphereGuide(RenderSphereGuide {
                        kind: RenderSphereGuideKind::Latitude(latitude_degrees),
                        marker,
                        visible: true,
                        radius_scale: 1.012,
                    }),
                )));
        }

        earth_group
            .children
            .push(RenderNode::Group(graticule_group));
        root_group.children.push(RenderNode::Group(earth_group));
    }

    scene.groups.push(root_group);

    scene
}

#[cfg(test)]
mod tests {
    use super::scene_document_to_render_scene;
    use crate::{
        render::{RenderAxis, RenderBehavior, RenderNode, RenderObject},
        scene::{DisplayDocument, QuadDocument, SceneDocument},
    };

    #[test]
    fn adapter_wraps_empty_scene_in_root_group() {
        let scene = scene_document_to_render_scene(SceneDocument {
            name: "test".to_string(),
            mesh_asset: String::new(),
            display: DisplayDocument {
                world_scale: 1.0,
                rotation_y_degrees_per_turn: None,
            },
            lighting: None,
            map_overlay: None,
            quads: Vec::new(),
        });

        assert_eq!(scene.groups.len(), 1);
        assert_eq!(scene.groups[0].id, "root");
        assert_eq!(scene.groups[0].name, "Root");
        assert!(scene.groups[0].children.is_empty());
    }

    #[test]
    fn earth_guides_are_wrapped_in_editor_composite_graticule_group() {
        let scene = scene_document_to_render_scene(SceneDocument {
            name: "earth".to_string(),
            mesh_asset: "sphere.obj".to_string(),
            display: DisplayDocument {
                world_scale: 1.0,
                rotation_y_degrees_per_turn: None,
            },
            lighting: None,
            map_overlay: None,
            quads: Vec::new(),
        });

        let RenderNode::Group(earth) = &scene.groups[0].children[0] else {
            panic!("expected Earth group");
        };
        let graticule = earth
            .children
            .iter()
            .find_map(|node| match node {
                RenderNode::Group(group) if group.id == "graticule" => Some(group),
                _ => None,
            })
            .expect("expected Graticule group");

        assert_eq!(graticule.name, "Graticule");
        assert!(graticule.editor_composite);
        assert_eq!(graticule.children.len(), 7);
    }

    #[test]
    fn adapter_keeps_compatibility_objects_and_group_nodes() {
        let scene = scene_document_to_render_scene(SceneDocument {
            name: "test".to_string(),
            mesh_asset: String::new(),
            display: DisplayDocument {
                world_scale: 1.0,
                rotation_y_degrees_per_turn: None,
            },
            lighting: None,
            map_overlay: None,
            quads: vec![QuadDocument {
                id: "q1".to_string(),
                position: [0.0, 0.0, 0.0],
                size: [1.0, 1.0],
                rotation_z_degrees: 0.0,
                marker: "#".to_string(),
                color: None,
            }],
        });

        assert_eq!(scene.objects.len(), 1);
        assert_eq!(scene.groups.len(), 1);
        assert_eq!(scene.groups[0].children.len(), 1);

        let RenderNode::Object(node) = &scene.groups[0].children[0] else {
            panic!("expected object node");
        };

        let RenderObject::QuadGroup(group) = &node.object else {
            panic!("expected quad group object");
        };

        assert_eq!(node.id, "quads");
        assert_eq!(node.name, "Quads");
        assert_eq!(group.quads.len(), 1);
        assert_eq!(group.quads[0].id, "q1");
    }
    #[test]
    fn adapter_wraps_mesh_asset_as_object_node() {
        let scene = scene_document_to_render_scene(SceneDocument {
            name: "earth".to_string(),
            mesh_asset: "assets/models/sphere_uv_32x16.obj".to_string(),
            display: DisplayDocument {
                world_scale: 1.0,
                rotation_y_degrees_per_turn: None,
            },
            lighting: None,
            map_overlay: None,
            quads: Vec::new(),
        });

        assert_eq!(scene.groups.len(), 1);
        assert_eq!(scene.groups[0].children.len(), 1);

        let RenderNode::Group(earth_group) = &scene.groups[0].children[0] else {
            panic!("expected earth group");
        };

        assert_eq!(earth_group.id, "earth");
        assert_eq!(earth_group.name, "Earth");
        assert_eq!(earth_group.behaviors.len(), 1);
        assert!(earth_group.children.len() >= 1);

        let RenderBehavior::Spin(spin) = &earth_group.behaviors[0];

        assert_eq!(spin.axis, RenderAxis::Y);
        assert_eq!(spin.degrees_per_second, 15.0);
        assert!(spin.enabled);

        let RenderNode::Object(node) = &earth_group.children[0] else {
            panic!("expected mesh object node");
        };

        let RenderObject::Mesh(mesh) = &node.object else {
            panic!("expected mesh object");
        };

        assert_eq!(node.id, "mesh");
        assert_eq!(node.name, "Mesh");
        assert_eq!(mesh.mesh_asset, "assets/models/sphere_uv_32x16.obj");
    }

}

