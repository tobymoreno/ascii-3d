mod frame;
mod geojson_map;
mod geojson_pipeline;

pub use frame::Frame;
pub use geojson_map::{
    GeoJsonMapAsset, GeoPoint, MapLine, land_fill_char, lerp_angle_degrees, load_geojson_map_asset,
    lon_lat_to_sphere, point_in_polygon, segment_steps,
};
pub use geojson_pipeline::{visit_geojson_segments, visit_lon_lat_samples};
mod projection;
mod raster;
mod shading;

pub use projection::Projection;
pub use raster::{draw_line, fill_triangle, rasterize_triangle_clipped};
pub use shading::{
    DEFAULT_ASCII_SHADE_RAMP, DEFAULT_LIGHT_RAY_DIRECTION, lambert_brightness,
    shade_ascii_brightness, shade_ascii_lambert, surface_to_light_from_ray_direction,
};
mod lines;
mod math;

pub use lines::draw_line_overlay;
pub use math::{Mat4, Vec3};

mod mesh;
mod mesh_pipeline;
mod sphere_guides;

pub use mesh::{
    MeshAsset, MeshPrepareOptions, MeshTriangle, MeshVertex, load_obj_mesh, load_obj_mesh_from_str,
    load_obj_mesh_prepared, load_prepared_mesh,
};
pub use mesh_pipeline::{
    PreparedFrameMesh, PreparedMeshTriangle, ProjectedMeshVertex, prepare_frame_mesh,
    visit_prepared_triangles,
};
pub use sphere_guides::{
    GreatCircle, SphereGuidePoint, great_circle_points, latitude_circle_points,
};

mod model;

pub use model::{
    RenderAxis, RenderBehavior, RenderCamera, RenderDisplay, RenderGeoJsonMapOverlay, RenderGroup,
    RenderLighting, RenderMeshObject, RenderNode, RenderObject, RenderObjectNode, RenderOverlay,
    RenderProjectionConfig, RenderQuad, RenderQuadGroup, RenderScene, RenderSphereGuide,
    RenderSphereGuideKind, RenderSpinBehavior, RenderTextOverlay, RenderTransform,
    apply_render_behaviors_to_group, apply_render_behaviors_to_group_tree,
    apply_render_behaviors_to_object_node, apply_render_behaviors_to_scene,
};
