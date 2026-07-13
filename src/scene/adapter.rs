use super::{
    AxisDocument, BehaviorDocument, GroupDocument, NodeDocument, ObjectDocument,
    ObjectKindDocument, SceneDocument, SphereGuideDocument, TransformDocument,
};
use crate::render::{
    GreatCircle, RenderAxis, RenderBehavior, RenderCamera, RenderDisplay, RenderGeoJsonMapOverlay,
    RenderGroup, RenderLighting, RenderMeshObject, RenderNode, RenderObject, RenderObjectNode,
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
    let uses_legacy_mesh = document.groups.is_empty() && !document.mesh_asset.trim().is_empty();
    let has_configured_mesh = document.groups.iter().any(group_document_contains_mesh);
    let mesh_only_scene = document.quads.is_empty() && (uses_legacy_mesh || has_configured_mesh);

    let mut scene = RenderScene::new(
        document.name,
        RenderDisplay {
            world_scale: document.display.world_scale,
        },
    );

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

    if !document.groups.is_empty() {
        scene.groups = document
            .groups
            .into_iter()
            .map(group_document_to_render_group)
            .collect();
        return scene;
    }

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

        scene
            .objects
            .push(RenderObject::QuadGroup(quad_group.clone()));
        root_group
            .children
            .push(RenderNode::Object(RenderObjectNode::new(
                "quads",
                "Quads",
                RenderObject::QuadGroup(quad_group),
            )));
    }

    if uses_legacy_mesh {
        let mut earth_group = RenderGroup::new("earth", "Earth");
        earth_group
            .behaviors
            .push(RenderBehavior::Spin(RenderSpinBehavior::new(
                RenderAxis::Y,
                15.0,
            )));
        earth_group
            .children
            .push(RenderNode::Object(RenderObjectNode::new(
                "mesh",
                "Mesh",
                RenderObject::Mesh(RenderMeshObject {
                    mesh_asset: document.mesh_asset,
                    transform: RenderTransform::default(),
                    backface_cull: false,
                }),
            )));

        if let Some(map_overlay) = document.map_overlay {
            earth_group
                .children
                .push(RenderNode::Object(RenderObjectNode::new(
                    "map",
                    "Map",
                    RenderObject::GeoJsonMap(RenderGeoJsonMapOverlay {
                        asset: map_overlay.asset,
                        visible: map_overlay.visible,
                        radius_scale: map_overlay.radius_scale,
                    }),
                )));
        }

        root_group.children.push(RenderNode::Group(earth_group));
    }

    scene.groups.push(root_group);
    scene
}

fn group_document_contains_mesh(group: &GroupDocument) -> bool {
    group.children.iter().any(|child| match child {
        NodeDocument::Group(child_group) => group_document_contains_mesh(child_group),
        NodeDocument::Object(object) => {
            matches!(object.object, ObjectKindDocument::Mesh { .. })
        }
    })
}

fn group_document_to_render_group(document: GroupDocument) -> RenderGroup {
    let mut group = RenderGroup::new(document.id, document.name);
    group.transform = transform_document_to_render_transform(document.transform);
    group.visible = document.visible;
    group.editor_composite = document.editor_composite;
    group.behaviors = document
        .behaviors
        .into_iter()
        .map(behavior_document_to_render_behavior)
        .collect();
    group.children = document
        .children
        .into_iter()
        .map(node_document_to_render_node)
        .collect();
    group
}

fn node_document_to_render_node(document: NodeDocument) -> RenderNode {
    match document {
        NodeDocument::Group(group) => RenderNode::Group(group_document_to_render_group(group)),
        NodeDocument::Object(object) => {
            RenderNode::Object(object_document_to_render_object_node(object))
        }
    }
}

fn object_document_to_render_object_node(document: ObjectDocument) -> RenderObjectNode {
    let object = match document.object {
        ObjectKindDocument::Mesh {
            asset,
            backface_cull,
        } => RenderObject::Mesh(RenderMeshObject {
            mesh_asset: asset,
            transform: RenderTransform::default(),
            backface_cull,
        }),
        ObjectKindDocument::GeoJsonMap {
            asset,
            radius_scale,
        } => RenderObject::GeoJsonMap(RenderGeoJsonMapOverlay {
            asset,
            visible: true,
            radius_scale,
        }),
        ObjectKindDocument::SphereGuide {
            guide,
            marker,
            radius_scale,
        } => RenderObject::SphereGuide(RenderSphereGuide {
            kind: sphere_guide_document_to_render_kind(guide),
            marker,
            visible: true,
            radius_scale,
        }),
    };

    let mut node = RenderObjectNode::new(document.id, document.name, object);
    node.transform = transform_document_to_render_transform(document.transform);
    node.visible = document.visible;
    node.behaviors = document
        .behaviors
        .into_iter()
        .map(behavior_document_to_render_behavior)
        .collect();
    node
}

fn sphere_guide_document_to_render_kind(document: SphereGuideDocument) -> RenderSphereGuideKind {
    match document {
        SphereGuideDocument::Equator => RenderSphereGuideKind::GreatCircle(GreatCircle::EquatorY0),
        SphereGuideDocument::MeridianX => {
            RenderSphereGuideKind::GreatCircle(GreatCircle::MeridianX0)
        }
        SphereGuideDocument::MeridianZ => {
            RenderSphereGuideKind::GreatCircle(GreatCircle::MeridianZ0)
        }
        SphereGuideDocument::Latitude { degrees } => RenderSphereGuideKind::Latitude(degrees),
    }
}

fn behavior_document_to_render_behavior(document: BehaviorDocument) -> RenderBehavior {
    match document {
        BehaviorDocument::Spin {
            axis,
            degrees_per_second,
            enabled,
        } => RenderBehavior::Spin(RenderSpinBehavior {
            axis: axis_document_to_render_axis(axis),
            degrees_per_second,
            enabled,
        }),
    }
}

fn axis_document_to_render_axis(axis: AxisDocument) -> RenderAxis {
    match axis {
        AxisDocument::X => RenderAxis::X,
        AxisDocument::Y => RenderAxis::Y,
        AxisDocument::Z => RenderAxis::Z,
    }
}

fn transform_document_to_render_transform(document: TransformDocument) -> RenderTransform {
    RenderTransform {
        position: document.position,
        rotation_degrees: document.rotation_degrees,
        scale: document.scale,
    }
}

#[cfg(test)]
mod tests {
    use super::scene_document_to_render_scene;
    use crate::{
        render::{RenderNode, RenderObject},
        scene::{
            DisplayDocument, GroupDocument, NodeDocument, ObjectDocument, ObjectKindDocument,
            SceneDocument, TransformDocument,
        },
    };

    fn base_document() -> SceneDocument {
        SceneDocument {
            name: "test".to_string(),
            mesh_asset: String::new(),
            display: DisplayDocument {
                world_scale: 1.0,
                rotation_y_degrees_per_turn: None,
            },
            lighting: None,
            map_overlay: None,
            quads: Vec::new(),
            groups: Vec::new(),
        }
    }

    #[test]
    fn adapter_wraps_empty_legacy_scene_in_root_group() {
        let scene = scene_document_to_render_scene(base_document());
        assert_eq!(scene.groups[0].id, "root");
    }

    #[test]
    fn adapter_builds_config_defined_composite_group() {
        let mut document = base_document();
        document.groups.push(GroupDocument {
            id: "root".to_string(),
            name: "Root".to_string(),
            transform: TransformDocument::default(),
            visible: true,
            editor_composite: false,
            behaviors: Vec::new(),
            children: vec![NodeDocument::Group(GroupDocument {
                id: "graticule".to_string(),
                name: "Graticule".to_string(),
                transform: TransformDocument::default(),
                visible: true,
                editor_composite: true,
                behaviors: Vec::new(),
                children: Vec::new(),
            })],
        });

        let scene = scene_document_to_render_scene(document);
        let RenderNode::Group(graticule) = &scene.groups[0].children[0] else {
            panic!("expected Graticule group");
        };
        assert!(graticule.editor_composite);
    }

    #[test]
    fn adapter_builds_config_defined_mesh_object() {
        let mut document = base_document();
        document.groups.push(GroupDocument {
            id: "root".to_string(),
            name: "Root".to_string(),
            transform: TransformDocument::default(),
            visible: true,
            editor_composite: false,
            behaviors: Vec::new(),
            children: vec![NodeDocument::Object(ObjectDocument {
                id: "mesh".to_string(),
                name: "Mesh".to_string(),
                transform: TransformDocument::default(),
                visible: true,
                behaviors: Vec::new(),
                object: ObjectKindDocument::Mesh {
                    asset: "sphere.obj".to_string(),
                    backface_cull: false,
                },
            })],
        });

        let scene = scene_document_to_render_scene(document);
        let RenderNode::Object(node) = &scene.groups[0].children[0] else {
            panic!("expected object");
        };
        let RenderObject::Mesh(mesh) = &node.object else {
            panic!("expected mesh");
        };
        assert_eq!(mesh.mesh_asset, "sphere.obj");
    }
}
