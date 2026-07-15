use crate::{
    render::{
        DEFAULT_LIGHT_RAY_DIRECTION, Frame, GeoJsonMapAsset, Mat4, Projection, RenderNode,
        RenderObject, RenderQuad, RenderQuadGroup, RenderScene, RenderSphereGuideKind,
        RenderTransform, SphereGuidePoint, Vec3, draw_line, draw_line_overlay, fill_triangle,
        great_circle_points, land_fill_char, latitude_circle_points, point_in_polygon,
        prepare_frame_mesh, shade_ascii_lambert, surface_to_light_from_ray_direction,
        visit_geojson_segments, visit_lon_lat_samples, visit_prepared_triangles,
    },
    viewer::ViewerState,
};

use std::{collections::HashMap, sync::Arc};

pub const MIN_VIEW_SCENE_WIDTH: usize = 96;
pub const MIN_VIEW_SCENE_HEIGHT: usize = 34;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewerViewport {
    pub width: usize,
    pub height: usize,
    pub cell_aspect_ratio: f32,
}

impl ViewerViewport {
    pub fn terminal(width: usize, height: usize) -> Self {
        Self::with_cell_aspect_ratio(width, height, Projection::terminal_cell_aspect_ratio())
    }

    pub const fn with_cell_aspect_ratio(
        width: usize,
        height: usize,
        cell_aspect_ratio: f32,
    ) -> Self {
        Self {
            width,
            height,
            cell_aspect_ratio,
        }
    }

    pub fn clamped(self) -> Self {
        Self {
            width: self.width.max(MIN_VIEW_SCENE_WIDTH),
            height: self.height.max(MIN_VIEW_SCENE_HEIGHT),
            cell_aspect_ratio: self.cell_aspect_ratio,
        }
    }
}

fn marker_char(marker: &str) -> char {
    marker.chars().next().unwrap_or('#')
}

fn shade_char(color: Option<&str>, marker: char) -> char {
    match color.unwrap_or_default().to_ascii_lowercase().as_str() {
        "#e56a2d" => '@',
        "#e0b23a" => '#',
        "#76a9f7" => '*',
        _ => marker,
    }
}

fn screen_project(
    scene: &RenderScene,
    viewport: ViewerViewport,
    point: Vec3,
) -> Option<(i32, i32, f32)> {
    let camera = scene.active_camera()?;

    Projection::with_camera(
        viewport.width,
        viewport.height,
        camera.projection.camera_distance,
        camera.projection.near_clip,
        viewport.cell_aspect_ratio,
        camera.projection.vertical_center_ratio,
    )
    .project_xyz(point.x, point.y, point.z)
}

fn draw_axes(frame: &mut Frame, scene: &RenderScene, viewport: ViewerViewport, world: Mat4) {
    let Some(origin) = screen_project(
        scene,
        viewport,
        world.transform_point(Vec3::new(0.0, 0.0, 0.0)),
    ) else {
        return;
    };

    if let Some(x) = screen_project(
        scene,
        viewport,
        world.transform_point(Vec3::new(2.0, 0.0, 0.0)),
    ) {
        draw_line_overlay(frame, origin, x, 'x');
    }

    if let Some(y) = screen_project(
        scene,
        viewport,
        world.transform_point(Vec3::new(0.0, 2.0, 0.0)),
    ) {
        draw_line_overlay(frame, origin, y, 'y');
    }

    if let Some(z) = screen_project(
        scene,
        viewport,
        world.transform_point(Vec3::new(0.0, 0.0, 2.0)),
    ) {
        draw_line_overlay(frame, origin, z, 'z');
    }
}

fn find_quad_group_in_nodes<'a>(nodes: &'a [RenderNode]) -> Option<&'a RenderQuadGroup> {
    for node in nodes {
        match node {
            RenderNode::Group(group) => {
                if let Some(quad_group) = find_quad_group_in_nodes(&group.children) {
                    return Some(quad_group);
                }
            }
            RenderNode::Object(object_node) => {
                if !object_node.visible {
                    continue;
                }

                if let RenderObject::QuadGroup(group) = &object_node.object {
                    return Some(group);
                }
            }
        }
    }

    None
}

fn find_quad_group(scene: &RenderScene) -> Option<&RenderQuadGroup> {
    scene
        .groups
        .iter()
        .filter(|group| group.visible)
        .find_map(|group| find_quad_group_in_nodes(&group.children))
        .or_else(|| {
            scene.objects.iter().find_map(|object| match object {
                RenderObject::QuadGroup(group) => Some(group),
                _ => None,
            })
        })
}

fn quad_matrix(scene: &RenderScene, quad: &RenderQuad, state: &ViewerState) -> Mat4 {
    let root = Mat4::translation(Vec3::new(state.origin_x, state.origin_y, state.origin_z))
        * Mat4::rotation_x(state.rotation_x_degrees.to_radians())
        * Mat4::rotation_y(state.rotation_y_degrees.to_radians())
        * Mat4::rotation_z(state.rotation_z_degrees.to_radians())
        * Mat4::scale(
            scene.display.world_scale * state.zoom,
            scene.display.world_scale * state.zoom,
            scene.display.world_scale * state.zoom,
        );

    root * Mat4::translation(Vec3::new(
        quad.position[0],
        quad.position[1],
        quad.position[2],
    )) * Mat4::rotation_z(quad.rotation_z_degrees.to_radians())
        * Mat4::scale(quad.size[0], quad.size[1], 1.0)
}

fn render_transform_matrix(transform: RenderTransform) -> Mat4 {
    Mat4::translation(Vec3::new(
        transform.position[0],
        transform.position[1],
        transform.position[2],
    )) * Mat4::rotation_x(transform.rotation_degrees[0].to_radians())
        * Mat4::rotation_y(transform.rotation_degrees[1].to_radians())
        * Mat4::rotation_z(transform.rotation_degrees[2].to_radians())
        * Mat4::scale(transform.scale[0], transform.scale[1], transform.scale[2])
}

fn viewer_world_matrix(scene: &RenderScene, state: &ViewerState) -> Mat4 {
    Mat4::translation(Vec3::new(state.origin_x, state.origin_y, state.origin_z))
        * Mat4::rotation_x(state.rotation_x_degrees.to_radians())
        * Mat4::rotation_y(state.rotation_y_degrees.to_radians())
        * Mat4::rotation_z(state.rotation_z_degrees.to_radians())
        * Mat4::scale(
            scene.display.world_scale * state.zoom,
            scene.display.world_scale * state.zoom,
            scene.display.world_scale * state.zoom,
        )
}

fn screen_signed_area(a: (i32, i32, f32), b: (i32, i32, f32), c: (i32, i32, f32)) -> i128 {
    let abx = i128::from(b.0) - i128::from(a.0);
    let aby = i128::from(b.1) - i128::from(a.1);
    let acx = i128::from(c.0) - i128::from(a.0);
    let acy = i128::from(c.1) - i128::from(a.1);
    abx * acy - aby * acx
}

fn mesh_shade_char(scene: &RenderScene, normal: Vec3) -> char {
    let light_ray_direction = scene
        .lighting
        .as_ref()
        .map(|lighting| lighting.primary_light_direction)
        .unwrap_or(DEFAULT_LIGHT_RAY_DIRECTION);
    let surface_to_light = surface_to_light_from_ray_direction(light_ray_direction);

    shade_ascii_lambert(normal, surface_to_light, 0.18, 0.82)
}

fn draw_mesh_asset(
    frame: &mut Frame,
    scene: &RenderScene,
    viewport: ViewerViewport,
    mesh: &crate::mesh::Mesh,
    world: Mat4,
    backface_cull: bool,
) {
    let prepared = prepare_frame_mesh(
        mesh,
        |position| {
            let point = world.transform_point(Vec3::from_array(position));
            point.to_array()
        },
        Some,
        |camera| screen_project(scene, viewport, Vec3::from_array(camera)),
    );

    // The viewer projection uses its historical screen-space winding
    // convention. Keep shared vertex preparation and triangle traversal, but
    // perform culling after projection so handedness matches this viewport.
    visit_prepared_triangles(mesh, &prepared, false, |triangle| {
        if backface_cull {
            let [a, b, c] = triangle.screen;
            if screen_signed_area(a, b, c) >= 0 {
                return;
            }
        }

        let normal = Vec3::from_array(triangle.world_normal).normalized();
        fill_triangle(
            frame,
            triangle.screen[0],
            triangle.screen[1],
            triangle.screen[2],
            mesh_shade_char(scene, normal),
        );
    });
}

fn draw_geojson_map_asset(
    frame: &mut Frame,
    scene: &RenderScene,
    viewport: ViewerViewport,
    map: &GeoJsonMapAsset,
    radius_scale: f32,
    world: Mat4,
) {
    let center_world = world.transform_point(Vec3::new(0.0, 0.0, 0.0));
    let Some(center_depth) = screen_project(scene, viewport, center_world).map(|point| point.2)
    else {
        return;
    };

    for line in &map.lines {
        draw_lon_lat_fill(
            frame,
            scene,
            viewport,
            &line.points_lon_lat,
            radius_scale * 0.999,
            world,
            center_depth,
        );
    }

    for line in &map.lines {
        draw_lon_lat_line(
            frame,
            scene,
            viewport,
            &line.points_lon_lat,
            line.marker,
            radius_scale,
            world,
            center_depth,
        );
    }
}

fn draw_lon_lat_fill(
    frame: &mut Frame,
    scene: &RenderScene,
    viewport: ViewerViewport,
    points_lon_lat: &[(f32, f32)],
    radius: f32,
    world: Mat4,
    center_depth: f32,
) {
    let polygon =
        projected_lon_lat_polygon(scene, viewport, points_lon_lat, radius, world, center_depth);

    if polygon.len() < 3 {
        return;
    }

    let min_x = polygon
        .iter()
        .map(|point| point.0)
        .min()
        .unwrap_or(0)
        .max(0);
    let max_x = polygon
        .iter()
        .map(|point| point.0)
        .max()
        .unwrap_or(0)
        .min(viewport.width as i32 - 1);
    let min_y = polygon
        .iter()
        .map(|point| point.1)
        .min()
        .unwrap_or(0)
        .max(0);
    let max_y = polygon
        .iter()
        .map(|point| point.1)
        .max()
        .unwrap_or(0)
        .min(viewport.height as i32 - 1);

    if min_x > max_x || min_y > max_y {
        return;
    }

    let fill_depth = polygon
        .iter()
        .map(|point| point.2)
        .fold(f32::INFINITY, f32::min);

    if !fill_depth.is_finite() {
        return;
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if !point_in_polygon(x as f32 + 0.5, y as f32 + 0.5, &polygon) {
                continue;
            }

            if let Some(ch) = land_fill_char(x, y) {
                frame.set(x, y, fill_depth + 0.03, ch);
            }
        }
    }
}

fn projected_lon_lat_polygon(
    scene: &RenderScene,
    viewport: ViewerViewport,
    points_lon_lat: &[(f32, f32)],
    radius: f32,
    world: Mat4,
    center_depth: f32,
) -> Vec<(i32, i32, f32)> {
    let mut polygon = Vec::new();

    if points_lon_lat.len() < 3 {
        return polygon;
    }

    visit_lon_lat_samples(points_lon_lat, radius, |local| {
        let world_point = world.transform_point(Vec3::new(local[0], local[1], local[2]));

        if let Some(projected) = screen_project(scene, viewport, world_point) {
            if projected.2 >= center_depth {
                return;
            }

            if polygon
                .last()
                .map(|last: &(i32, i32, f32)| last.0 != projected.0 || last.1 != projected.1)
                .unwrap_or(true)
            {
                polygon.push(projected);
            }
        }
    });

    polygon
}

fn draw_lon_lat_line(
    frame: &mut Frame,
    scene: &RenderScene,
    viewport: ViewerViewport,
    points_lon_lat: &[(f32, f32)],
    marker: char,
    radius: f32,
    world: Mat4,
    center_depth: f32,
) {
    let map = GeoJsonMapAsset {
        lines: vec![crate::render::MapLine {
            name: "inline".to_string(),
            marker,
            points_lon_lat: points_lon_lat.to_vec(),
        }],
    };

    visit_geojson_segments(
        &map,
        radius,
        |local| {
            world
                .transform_point(Vec3::new(local[0], local[1], local[2]))
                .to_array()
        },
        |world_point| {
            screen_project(
                scene,
                viewport,
                Vec3::new(world_point[0], world_point[1], world_point[2]),
            )
            .is_some_and(|projected| projected.2 < center_depth)
        },
        |segment_marker, from, to| {
            let from = screen_project(scene, viewport, Vec3::new(from[0], from[1], from[2]));
            let to = screen_project(scene, viewport, Vec3::new(to[0], to[1], to[2]));

            if let (Some(from), Some(to)) = (from, to) {
                draw_line_overlay(frame, from, to, segment_marker);
            }
        },
    );
}

fn draw_sphere_guide_points(
    frame: &mut Frame,
    scene: &RenderScene,
    viewport: ViewerViewport,
    points: &[SphereGuidePoint],
    marker: char,
    radius_scale: f32,
    world: Mat4,
) {
    let mut previous = None;

    for point in points {
        let local = Vec3::new(
            point.x * radius_scale,
            point.y * radius_scale,
            point.z * radius_scale,
        );
        let world_point = world.transform_point(local);

        if world_point.z > 0.10 {
            previous = None;
            continue;
        }

        if let Some(current) = screen_project(scene, viewport, world_point) {
            if let Some(prev) = previous {
                draw_line_overlay(frame, prev, current, marker);
            }

            previous = Some(current);
        } else {
            previous = None;
        }
    }
}

fn draw_sphere_guide(
    frame: &mut Frame,
    scene: &RenderScene,
    viewport: ViewerViewport,
    guide: &crate::render::RenderSphereGuide,
    world: Mat4,
) {
    if !guide.visible {
        return;
    }

    match guide.kind {
        RenderSphereGuideKind::GreatCircle(circle) => {
            draw_sphere_guide_points(
                frame,
                scene,
                viewport,
                &great_circle_points(circle, 96),
                guide.marker,
                guide.radius_scale,
                world,
            );
        }
        RenderSphereGuideKind::Latitude(latitude_degrees) => {
            draw_sphere_guide_points(
                frame,
                scene,
                viewport,
                &latitude_circle_points(latitude_degrees, 96),
                guide.marker,
                guide.radius_scale,
                world,
            );
        }
    }
}

fn draw_meshes_from_nodes(
    frame: &mut Frame,
    scene: &RenderScene,
    viewport: ViewerViewport,
    nodes: &[RenderNode],
    meshes: &HashMap<String, Arc<crate::mesh::Mesh>>,
    maps: &HashMap<String, GeoJsonMapAsset>,
    parent_world: Mat4,
) -> usize {
    let mut count = 0;

    for node in nodes {
        match node {
            RenderNode::Group(group) => {
                if group.visible {
                    let group_world = parent_world * render_transform_matrix(group.transform);
                    count += draw_meshes_from_nodes(
                        frame,
                        scene,
                        viewport,
                        &group.children,
                        meshes,
                        maps,
                        group_world,
                    );
                }
            }
            RenderNode::Object(object_node) => {
                if !object_node.visible {
                    continue;
                }

                let object_world = parent_world * render_transform_matrix(object_node.transform);

                match &object_node.object {
                    RenderObject::Mesh(mesh_object) => {
                        let Some(mesh) = meshes.get(&mesh_object.mesh_asset) else {
                            continue;
                        };

                        let mesh_world =
                            object_world * render_transform_matrix(mesh_object.transform);
                        draw_mesh_asset(
                            frame,
                            scene,
                            viewport,
                            mesh,
                            mesh_world,
                            mesh_object.backface_cull,
                        );
                        count += 1;
                    }
                    RenderObject::GeoJsonMap(map_object) => {
                        if !map_object.visible {
                            continue;
                        }

                        let Some(map) = maps.get(&map_object.asset) else {
                            continue;
                        };

                        draw_geojson_map_asset(
                            frame,
                            scene,
                            viewport,
                            map,
                            map_object.radius_scale,
                            object_world,
                        );
                    }
                    RenderObject::SphereGuide(guide) => {
                        draw_sphere_guide(frame, scene, viewport, guide, object_world);
                    }
                    RenderObject::QuadGroup(_) => {}
                }
            }
        }
    }

    count
}

fn draw_meshes(
    frame: &mut Frame,
    scene: &RenderScene,
    viewport: ViewerViewport,
    meshes: &HashMap<String, Arc<crate::mesh::Mesh>>,
    maps: &HashMap<String, GeoJsonMapAsset>,
    state: &ViewerState,
) -> usize {
    let viewer_world = viewer_world_matrix(scene, state);

    scene
        .groups
        .iter()
        .filter(|group| group.visible)
        .map(|group| {
            let group_world = viewer_world * render_transform_matrix(group.transform);
            draw_meshes_from_nodes(
                frame,
                scene,
                viewport,
                &group.children,
                meshes,
                maps,
                group_world,
            )
        })
        .sum()
}

pub fn draw_render_scene(
    frame: &mut Frame,
    viewport: ViewerViewport,
    scene: &RenderScene,
    meshes: &HashMap<String, Arc<crate::mesh::Mesh>>,
    maps: &HashMap<String, GeoJsonMapAsset>,
    state: &ViewerState,
) {
    let viewport = viewport.clamped();

    frame.clear();

    let mesh_count = draw_meshes(frame, scene, viewport, meshes, maps, state);

    let Some(quad_group) = find_quad_group(scene) else {
        frame.draw_text(
            2,
            1,
            &format!(
                "view-scene: {} | meshes={} | groups={} | objects={}",
                scene.name,
                mesh_count,
                scene.groups.len(),
                scene.objects.len()
            ),
        );
        frame.draw_text(
            2,
            2,
            &format!(
                "rot x/y/z = {:+.1}/{:+.1}/{:+.1} | zoom {:.2}",
                state.rotation_x_degrees,
                state.rotation_y_degrees,
                state.rotation_z_degrees,
                state.zoom
            ),
        );
        frame.draw_text(
            2,
            3,
            &format!(
                "origin x/y/z = {:+.1}/{:+.1}/{:+.1} | axes {} | fps {:>5.1} | frame {:>5.2} ms",
                state.origin_x,
                state.origin_y,
                state.origin_z,
                if state.show_axes { "on" } else { "off" },
                state.fps,
                state.frame_time_ms
            ),
        );
        frame.draw_text(
            2,
            viewport.height.saturating_sub(2),
            "controls: a axes on | A axes off | arrows origin | PgUp/PgDn z | +/- scale object/origin | camera dolly | x/y/z rotate | 0 origin | r reset | q quit",
        );
        return;
    };

    let root = Mat4::translation(Vec3::new(state.origin_x, state.origin_y, state.origin_z))
        * Mat4::rotation_x(state.rotation_x_degrees.to_radians())
        * Mat4::rotation_y(state.rotation_y_degrees.to_radians())
        * Mat4::rotation_z(state.rotation_z_degrees.to_radians())
        * Mat4::scale(
            scene.display.world_scale * state.zoom,
            scene.display.world_scale * state.zoom,
            scene.display.world_scale * state.zoom,
        );

    let local_corners = [
        Vec3::new(-0.5, -0.5, 0.0),
        Vec3::new(0.5, -0.5, 0.0),
        Vec3::new(0.5, 0.5, 0.0),
        Vec3::new(-0.5, 0.5, 0.0),
    ];

    for quad in &quad_group.quads {
        let world = quad_matrix(scene, quad, state);
        let projected = local_corners
            .map(|corner| screen_project(scene, viewport, world.transform_point(corner)));

        let Some(p0) = projected[0] else { continue };
        let Some(p1) = projected[1] else { continue };
        let Some(p2) = projected[2] else { continue };
        let Some(p3) = projected[3] else { continue };

        let fill = shade_char(quad.color.as_deref(), marker_char(&quad.marker));

        fill_triangle(frame, p0, p1, p2, fill);
        fill_triangle(frame, p0, p2, p3, fill);

        draw_line(frame, p0, p1, '+');
        draw_line(frame, p1, p2, '+');
        draw_line(frame, p2, p3, '+');
        draw_line(frame, p3, p0, '+');
    }

    if state.show_axes {
        draw_axes(frame, scene, viewport, root);
    }

    frame.draw_text(
        2,
        1,
        &format!(
            "view-scene: {} | quads={} | meshes={} | groups={} | objects={}",
            scene.name,
            quad_group.quads.len(),
            mesh_count,
            scene.groups.len(),
            scene.objects.len()
        ),
    );
    frame.draw_text(
        2,
        2,
        &format!(
            "rot x/y/z = {:+.1}/{:+.1}/{:+.1} | zoom {:.2}",
            state.rotation_x_degrees,
            state.rotation_y_degrees,
            state.rotation_z_degrees,
            state.zoom
        ),
    );
    frame.draw_text(
        2,
        3,
        &format!(
            "origin x/y/z = {:+.1}/{:+.1}/{:+.1} | axes {} | fps {:>5.1} | frame {:>5.2} ms",
            state.origin_x,
            state.origin_y,
            state.origin_z,
            if state.show_axes { "on" } else { "off" },
            state.fps,
            state.frame_time_ms
        ),
    );
    frame.draw_text(
        2,
        viewport.height.saturating_sub(2),
        "controls: a axes on | A axes off | arrows origin | PgUp/PgDn z | +/- scale object/origin | camera dolly | x/y/z rotate | 0 origin | r reset | q quit",
    );
}

#[cfg(test)]
mod overflow_tests {
    use super::screen_signed_area;

    #[test]
    fn screen_signed_area_handles_extreme_i32_coordinates() {
        let area = screen_signed_area(
            (i32::MIN, i32::MIN, 0.0),
            (i32::MAX, i32::MIN, 0.0),
            (i32::MIN, i32::MAX, 0.0),
        );

        assert!(area > 0);
    }
}
