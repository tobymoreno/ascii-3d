use crate::{
    geojson::GeoJsonMap,
    math::{Mat4, Vec3},
};

use super::gpu_renderer::GpuVertex;

const CAMERA_NEAR: f32 = 0.1;
const MAX_CAMERA_DISTANCE: f32 = 250.0;
const STROKE_DEPTH_BIAS_NDC: f32 = 1.0e-4;
const GEOJSON_LIGHT_DIRECTION: Vec3 = Vec3::new(-0.55, 0.20, 0.81);

fn lerp(left: f32, right: f32, amount: f32) -> f32 {
    left + (right - left) * amount
}

fn sigmoid(value: f32) -> f32 {
    1.0 / (1.0 + (-value).exp())
}

fn modulate_line_color(base: [f32; 4], local_position: Vec3) -> [f32; 4] {
    if base[3] <= 0.001 {
        return base;
    }

    let normal = local_position.normalized();
    let light_direction = GEOJSON_LIGHT_DIRECTION.normalized();
    let lighting = normal.dot(light_direction).clamp(-1.0, 1.0);
    let amount = sigmoid(lighting * 3.0);

    let darker = [
        (base[0] * 0.94).clamp(0.0, 1.0),
        (base[1] * 0.96).clamp(0.0, 1.0),
        (base[2] * 0.98).clamp(0.0, 1.0),
    ];

    let lighter = [
        (base[0] * 1.05).clamp(0.0, 1.0),
        (base[1] * 1.06).clamp(0.0, 1.0),
        (base[2] * 1.08).clamp(0.0, 1.0),
    ];

    [
        lerp(darker[0], lighter[0], amount),
        lerp(darker[1], lighter[1], amount),
        lerp(darker[2], lighter[2], amount),
        (base[3] * lerp(0.88, 1.10, amount)).clamp(0.30, 0.56),
    ]
}

#[derive(Clone, Copy)]
pub(crate) struct GeoJsonLineStyle {
    pub(crate) inner_color: [f32; 4],
    pub(crate) outer_color: [f32; 4],
    pub(crate) inner_width_pixels: f32,
    pub(crate) outer_width_pixels: f32,
    pub(crate) shade_bands: [f32; 3],
    pub(crate) band_thresholds: [f32; 2],
}

pub(crate) fn append_geojson_lines(
    vertices: &mut Vec<GpuVertex>,
    map: &GeoJsonMap,
    model_view: Mat4,
    viewport_width: f32,
    viewport_height: f32,
    focal_length: f32,
    style: GeoJsonLineStyle,
) {
    for &(local_start, local_end) in &map.segments {
        if segment_is_back_facing(local_start, local_end, model_view) {
            continue;
        }

        let view_start = model_view.transform_point(local_start);
        let view_end = model_view.transform_point(local_end);
        let (Some(start), Some(end)) = (
            project_point_clip(view_start, viewport_width, viewport_height, focal_length),
            project_point_clip(view_end, viewport_width, viewport_height, focal_length),
        ) else {
            continue;
        };

        let outer_start_color = modulate_line_color(style.outer_color, local_start);
        let outer_end_color = modulate_line_color(style.outer_color, local_end);
        let inner_start_color = modulate_line_color(style.inner_color, local_start);
        let inner_end_color = modulate_line_color(style.inner_color, local_end);

        if style.outer_width_pixels > 0.0
            && (outer_start_color[3] > 0.001 || outer_end_color[3] > 0.001)
        {
            push_screen_space_stroke(
                vertices,
                start,
                end,
                style.outer_width_pixels,
                viewport_width,
                viewport_height,
                outer_start_color,
                outer_end_color,
                style.shade_bands,
                style.band_thresholds,
            );
        }

        if style.inner_width_pixels > 0.0
            && (inner_start_color[3] > 0.001 || inner_end_color[3] > 0.001)
        {
            push_screen_space_stroke(
                vertices,
                start,
                end,
                style.inner_width_pixels,
                viewport_width,
                viewport_height,
                inner_start_color,
                inner_end_color,
                style.shade_bands,
                style.band_thresholds,
            );
        }
    }
}

fn segment_is_back_facing(local_start: Vec3, local_end: Vec3, model_view: Mat4) -> bool {
    let midpoint = (local_start + local_end) * 0.5;
    let view_midpoint = model_view.transform_point(midpoint);
    let view_normal = model_view
        .transform_vector(midpoint.normalized())
        .normalized();
    let to_camera = (view_midpoint * -1.0).normalized();
    view_normal.dot(to_camera) <= 0.0
}

fn project_point_clip(point: Vec3, width: f32, height: f32, focal_length: f32) -> Option<[f32; 4]> {
    if point.z >= -CAMERA_NEAR {
        return None;
    }

    let depth = -point.z;
    let screen_x = width * 0.5 + point.x / depth * focal_length;
    let screen_y = height * 0.5 - point.y / depth * focal_length;
    let far = (MAX_CAMERA_DISTANCE * 4.0).max(CAMERA_NEAR + 1.0);
    let depth_ndc = (far / (far - CAMERA_NEAR)
        - (far * CAMERA_NEAR) / ((far - CAMERA_NEAR) * depth))
        .clamp(0.0, 1.0);
    let x_ndc = screen_x / width * 2.0 - 1.0;
    let y_ndc = 1.0 - screen_y / height * 2.0;

    Some([x_ndc * depth, y_ndc * depth, depth_ndc * depth, depth])
}

fn bias_clip_toward_camera(position: [f32; 4]) -> [f32; 4] {
    [
        position[0],
        position[1],
        (position[2] / position[3] - STROKE_DEPTH_BIAS_NDC) * position[3],
        position[3],
    ]
}

#[allow(clippy::too_many_arguments)]
fn push_screen_space_stroke(
    vertices: &mut Vec<GpuVertex>,
    start: [f32; 4],
    end: [f32; 4],
    width_pixels: f32,
    viewport_width: f32,
    viewport_height: f32,
    start_color: [f32; 4],
    end_color: [f32; 4],
    shade_bands: [f32; 3],
    band_thresholds: [f32; 2],
) {
    if start[3].abs() <= f32::EPSILON || end[3].abs() <= f32::EPSILON {
        return;
    }

    let start = bias_clip_toward_camera(start);
    let end = bias_clip_toward_camera(end);
    let start_ndc = [start[0] / start[3], start[1] / start[3]];
    let end_ndc = [end[0] / end[3], end[1] / end[3]];
    let dx_pixels = (end_ndc[0] - start_ndc[0]) * viewport_width * 0.5;
    let dy_pixels = (end_ndc[1] - start_ndc[1]) * viewport_height * 0.5;
    let length = (dx_pixels * dx_pixels + dy_pixels * dy_pixels).sqrt();
    if length <= f32::EPSILON {
        return;
    }

    let half_width = width_pixels.max(0.08) * 0.5;
    let offset_x_pixels = -dy_pixels / length * half_width;
    let offset_y_pixels = dx_pixels / length * half_width;
    let offset_ndc = [
        offset_x_pixels * 2.0 / viewport_width.max(1.0),
        offset_y_pixels * 2.0 / viewport_height.max(1.0),
    ];

    let start_a = [
        start[0] + offset_ndc[0] * start[3],
        start[1] + offset_ndc[1] * start[3],
        start[2],
        start[3],
    ];
    let start_b = [
        start[0] - offset_ndc[0] * start[3],
        start[1] - offset_ndc[1] * start[3],
        start[2],
        start[3],
    ];
    let end_a = [
        end[0] + offset_ndc[0] * end[3],
        end[1] + offset_ndc[1] * end[3],
        end[2],
        end[3],
    ];
    let end_b = [
        end[0] - offset_ndc[0] * end[3],
        end[1] - offset_ndc[1] * end[3],
        end[2],
        end[3],
    ];

    for (position, color) in [
        (start_a, start_color),
        (start_b, start_color),
        (end_a, end_color),
        (start_b, start_color),
        (end_b, end_color),
        (end_a, end_color),
    ] {
        vertices.push(GpuVertex::new(
            position,
            color,
            1.0,
            shade_bands,
            band_thresholds,
        ));
    }
}
