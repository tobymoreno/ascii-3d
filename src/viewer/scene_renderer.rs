use crate::{
    render::{
        draw_line_overlay, great_circle_points, land_fill_char, latitude_circle_points,
        lerp_angle_degrees, lon_lat_to_sphere, point_in_polygon, segment_steps, Frame,
        GeoJsonMapAsset, MeshAsset, MeshVertex,
        Projection, RenderNode, RenderObject, RenderQuad, RenderQuadGroup, RenderScene,
        RenderSphereGuideKind, RenderTransform, SphereGuidePoint,
    },
    viewer::ViewerState,
};

use std::{
    collections::HashMap,
    io,
};

const WIDTH: usize = 96;
const HEIGHT: usize = 34;

pub const VIEW_SCENE_WIDTH: usize = WIDTH;
pub const VIEW_SCENE_HEIGHT: usize = HEIGHT;

#[derive(Debug, Clone, Copy)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    fn normalized(self) -> Self {
        let length = self.length();

        if length <= f32::EPSILON {
            return Self::new(0.0, 1.0, 0.0);
        }

        Self::new(self.x / length, self.y / length, self.z / length)
    }

    fn from_array(values: [f32; 3]) -> Self {
        Self::new(values[0], values[1], values[2])
    }
}

#[derive(Debug, Clone, Copy)]
struct Mat4 {
    m: [[f32; 4]; 4],
}

impl Mat4 {
    const fn identity() -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    fn translation(v: Vec3) -> Self {
        let mut result = Self::identity();
        result.m[0][3] = v.x;
        result.m[1][3] = v.y;
        result.m[2][3] = v.z;
        result
    }

    fn scale(x: f32, y: f32, z: f32) -> Self {
        Self {
            m: [
                [x, 0.0, 0.0, 0.0],
                [0.0, y, 0.0, 0.0],
                [0.0, 0.0, z, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    fn rotation_x(radians: f32) -> Self {
        let (s, c) = radians.sin_cos();

        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, c, -s, 0.0],
                [0.0, s, c, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    fn rotation_y(radians: f32) -> Self {
        let (s, c) = radians.sin_cos();

        Self {
            m: [
                [c, 0.0, s, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [-s, 0.0, c, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    fn rotation_z(radians: f32) -> Self {
        let (s, c) = radians.sin_cos();

        Self {
            m: [
                [c, -s, 0.0, 0.0],
                [s, c, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    fn transform_point(self, p: Vec3) -> Vec3 {
        Vec3::new(
            self.m[0][0] * p.x + self.m[0][1] * p.y + self.m[0][2] * p.z + self.m[0][3],
            self.m[1][0] * p.x + self.m[1][1] * p.y + self.m[1][2] * p.z + self.m[1][3],
            self.m[2][0] * p.x + self.m[2][1] * p.y + self.m[2][2] * p.z + self.m[2][3],
        )
    }
}

impl std::ops::Mul for Mat4 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut out = [[0.0; 4]; 4];

        for row in 0..4 {
            for col in 0..4 {
                out[row][col] = self.m[row][0] * rhs.m[0][col]
                    + self.m[row][1] * rhs.m[1][col]
                    + self.m[row][2] * rhs.m[2][col]
                    + self.m[row][3] * rhs.m[3][col];
            }
        }

        Self { m: out }
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

fn screen_project(scene: &RenderScene, point: Vec3) -> Option<(i32, i32, f32)> {
    let camera = scene.active_camera()?;

    Projection::terminal_with_camera(
        WIDTH,
        HEIGHT,
        camera.projection.camera_distance,
        camera.projection.near_clip,
        camera.projection.vertical_center_ratio,
    )
    .project_xyz(point.x, point.y, point.z)
}


fn edge(a: (f32, f32), b: (f32, f32), p: (f32, f32)) -> f32 {
    (p.0 - a.0) * (b.1 - a.1) - (p.1 - a.1) * (b.0 - a.0)
}

fn fill_triangle(
    frame: &mut Frame,
    a: (i32, i32, f32),
    b: (i32, i32, f32),
    c: (i32, i32, f32),
    ch: char,
) {
    let min_x = a.0.min(b.0).min(c.0).max(0);
    let max_x = a.0.max(b.0).max(c.0).min(WIDTH as i32 - 1);
    let min_y = a.1.min(b.1).min(c.1).max(0);
    let max_y = a.1.max(b.1).max(c.1).min(HEIGHT as i32 - 1);

    let af = (a.0 as f32, a.1 as f32);
    let bf = (b.0 as f32, b.1 as f32);
    let cf = (c.0 as f32, c.1 as f32);

    let area = edge(af, bf, cf);

    if area.abs() < f32::EPSILON {
        return;
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = (x as f32 + 0.5, y as f32 + 0.5);

            let w0 = edge(bf, cf, p) / area;
            let w1 = edge(cf, af, p) / area;
            let w2 = edge(af, bf, p) / area;

            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                let z = w0 * a.2 + w1 * b.2 + w2 * c.2;
                frame.set(x, y, z, ch);
            }
        }
    }
}

fn draw_line(frame: &mut Frame, a: (i32, i32, f32), b: (i32, i32, f32), ch: char) {
    let dx = (b.0 - a.0).abs();
    let dy = -(b.1 - a.1).abs();
    let sx = if a.0 < b.0 { 1 } else { -1 };
    let sy = if a.1 < b.1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = a.0;
    let mut y = a.1;
    let steps = dx.max(-dy).max(1) as f32;
    let mut step = 0.0;

    loop {
        let t = step / steps;
        let z = a.2 * (1.0 - t) + b.2 * t;
        frame.set(x, y, z - 0.001, ch);

        if x == b.0 && y == b.1 {
            break;
        }

        let e2 = 2 * err;

        if e2 >= dy {
            err += dy;
            x += sx;
        }

        if e2 <= dx {
            err += dx;
            y += sy;
        }

        step += 1.0;
    }
}


fn draw_axes(frame: &mut Frame, scene: &RenderScene, world: Mat4) {
    let Some(origin) = screen_project(scene, world.transform_point(Vec3::new(0.0, 0.0, 0.0))) else {
        return;
    };

    if let Some(x) = screen_project(scene, world.transform_point(Vec3::new(2.0, 0.0, 0.0))) {
        draw_line_overlay(frame, origin, x, 'x');
    }

    if let Some(y) = screen_project(scene, world.transform_point(Vec3::new(0.0, 2.0, 0.0))) {
        draw_line_overlay(frame, origin, y, 'y');
    }

    if let Some(z) = screen_project(scene, world.transform_point(Vec3::new(0.0, 0.0, 2.0))) {
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
    ))
    * Mat4::rotation_x(transform.rotation_degrees[0].to_radians())
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

fn transform_mesh_vertex(vertex: MeshVertex, world: Mat4) -> (Vec3, Vec3) {
    let position = world.transform_point(Vec3::from_array(vertex.position));
    let normal = Vec3::from_array(vertex.normal).normalized();

    (position, normal)
}

fn mesh_shade_char(normal: Vec3) -> char {
    let light = Vec3::new(-0.45, 0.7, -0.55).normalized();
    let brightness = (0.15 + normal.dot(light).max(0.0) * 0.75).clamp(0.0, 1.0);
    let ramp = b" .:-=+*#%@";
    let index = (brightness * (ramp.len() - 1) as f32).round() as usize;

    ramp[index.min(ramp.len() - 1)] as char
}

fn draw_mesh_asset(frame: &mut Frame, scene: &RenderScene, mesh: &MeshAsset, world: Mat4) {
    for triangle in &mesh.triangles {
        let (a, na) = transform_mesh_vertex(triangle.a, world);
        let (b, nb) = transform_mesh_vertex(triangle.b, world);
        let (c, nc) = transform_mesh_vertex(triangle.c, world);

        let Some(pa) = screen_project(scene, a) else { continue };
        let Some(pb) = screen_project(scene, b) else { continue };
        let Some(pc) = screen_project(scene, c) else { continue };

        let normal = Vec3::new(
            (na.x + nb.x + nc.x) / 3.0,
            (na.y + nb.y + nc.y) / 3.0,
            (na.z + nb.z + nc.z) / 3.0,
        )
        .normalized();

        fill_triangle(frame, pa, pb, pc, mesh_shade_char(normal));
    }
}


fn draw_geojson_map_asset(
    frame: &mut Frame,
    scene: &RenderScene,
    map: &GeoJsonMapAsset,
    radius_scale: f32,
    world: Mat4,
) {
    for line in &map.lines {
        draw_lon_lat_fill(frame, scene, &line.points_lon_lat, radius_scale * 0.999, world);
    }

    for line in &map.lines {
        draw_lon_lat_line(
            frame,
            scene,
            &line.points_lon_lat,
            line.marker,
            radius_scale,
            world,
        );
    }
}

fn draw_lon_lat_fill(
    frame: &mut Frame,
    scene: &RenderScene,
    points_lon_lat: &[(f32, f32)],
    radius: f32,
    world: Mat4,
) {
    let polygon = projected_lon_lat_polygon(scene, points_lon_lat, radius, world);

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
        .min(WIDTH as i32 - 1);
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
        .min(HEIGHT as i32 - 1);

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
    points_lon_lat: &[(f32, f32)],
    radius: f32,
    world: Mat4,
) -> Vec<(i32, i32, f32)> {
    let mut polygon = Vec::new();

    if points_lon_lat.len() < 3 {
        return polygon;
    }

    for pair in points_lon_lat.windows(2) {
        let (lon_a, lat_a) = pair[0];
        let (lon_b, lat_b) = pair[1];
        let steps = segment_steps(lon_a, lat_a, lon_b, lat_b);

        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let lon = lerp_angle_degrees(lon_a, lon_b, t);
            let lat = lat_a * (1.0 - t) + lat_b * t;

            let local = lon_lat_to_sphere(lon, lat, radius);
            let world_point = world.transform_point(Vec3::new(local.x, local.y, local.z));

            if world_point.z > 0.10 {
                continue;
            }

            if let Some(projected) = screen_project(scene, world_point) {
                if polygon
                    .last()
                    .map(|last: &(i32, i32, f32)| last.0 != projected.0 || last.1 != projected.1)
                    .unwrap_or(true)
                {
                    polygon.push(projected);
                }
            }
        }
    }

    polygon
}

fn draw_lon_lat_line(
    frame: &mut Frame,
    scene: &RenderScene,
    points_lon_lat: &[(f32, f32)],
    marker: char,
    radius: f32,
    world: Mat4,
) {
    if points_lon_lat.len() < 2 {
        return;
    }

    let mut previous = None;

    for pair in points_lon_lat.windows(2) {
        let (lon_a, lat_a) = pair[0];
        let (lon_b, lat_b) = pair[1];
        let steps = segment_steps(lon_a, lat_a, lon_b, lat_b);

        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let lon = lerp_angle_degrees(lon_a, lon_b, t);
            let lat = lat_a * (1.0 - t) + lat_b * t;

            let local = lon_lat_to_sphere(lon, lat, radius);
            let world_point = world.transform_point(Vec3::new(local.x, local.y, local.z));

            if world_point.z > 0.10 {
                previous = None;
                continue;
            }

            if let Some(current) = screen_project(scene, world_point) {
                if let Some(prev) = previous {
                    draw_line_overlay(frame, prev, current, marker);
                }
                previous = Some(current);
            } else {
                previous = None;
            }
        }
    }
}


fn draw_sphere_guide_points(
    frame: &mut Frame,
    scene: &RenderScene,
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

        if let Some(current) = screen_project(scene, world_point) {
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
    nodes: &[RenderNode],
    meshes: &HashMap<String, MeshAsset>,
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

                        let mesh_world = object_world * render_transform_matrix(mesh_object.transform);
                        draw_mesh_asset(frame, scene, mesh, mesh_world);
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
                            map,
                            map_object.radius_scale,
                            object_world,
                        );
                    }
                    RenderObject::SphereGuide(guide) => {
                        draw_sphere_guide(frame, scene, guide, object_world);
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
    meshes: &HashMap<String, MeshAsset>,
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
            draw_meshes_from_nodes(frame, scene, &group.children, meshes, maps, group_world)
        })
        .sum()
}

pub fn draw_render_scene(frame: &mut Frame, scene: &RenderScene, meshes: &HashMap<String, MeshAsset>, maps: &HashMap<String, GeoJsonMapAsset>, state: &ViewerState) {
    frame.clear();

    let mesh_count = draw_meshes(frame, scene, meshes, maps, state);

    let Some(quad_group) = find_quad_group(scene) else {
        frame.draw_text(2, 1, &format!("view-scene: {} | meshes={} | no quad group", scene.name, mesh_count));
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
        let projected = local_corners.map(|corner| screen_project(scene, world.transform_point(corner)));

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
        draw_axes(frame, scene, root);
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
            state.rotation_x_degrees, state.rotation_y_degrees, state.rotation_z_degrees, state.zoom
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
        HEIGHT - 2,
        "controls: a axes on | A axes off | arrows origin | PgUp/PgDn z | +/- zoom | x/y/z rotate | 0 origin | r reset | q quit",
    );
}

