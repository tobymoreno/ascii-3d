use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{
    canvas::Canvas,
    curves::CubicBezier3,
    math::{Mat4, Vec3},
    projection::ObliqueProjector,
};

#[derive(Debug, Deserialize)]
pub struct GlyphAsset {
    pub name: String,
    pub version: u32,
    #[serde(rename = "type")]
    pub glyph_type: String,
    pub paths: Vec<GlyphPath>,
    pub sampling: GlyphSampling,
}

#[derive(Debug, Deserialize)]
pub struct GlyphSampling {
    pub default_segments_per_curve: usize,
}

#[derive(Debug, Deserialize)]
pub struct GlyphPath {
    pub id: String,
    pub closed: bool,
    pub segments: Vec<GlyphSegment>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum GlyphSegment {
    #[serde(rename = "line")]
    Line { from: [f32; 3], to: [f32; 3] },

    #[serde(rename = "cubic_bezier")]
    CubicBezier {
        p0: [f32; 3],
        p1: [f32; 3],
        p2: [f32; 3],
        p3: [f32; 3],
    },
}

#[derive(Debug, Deserialize)]
pub struct GlyphMetadata {
    pub name: String,
    pub version: u32,
    pub display: GlyphDisplay,
}

#[derive(Debug, Deserialize)]
pub struct GlyphDisplay {
    pub stroke_character: char,
    pub control_polygon_character: char,
    pub control_point_character: char,
    pub anchor_point_character: char,
    pub show_strokes: bool,
    pub show_control_points: bool,
    pub show_control_polygons: bool,
    pub show_anchor_points: bool,
    pub show_labels: bool,
}

#[derive(Debug, Deserialize)]
pub struct WordAsset {
    pub name: String,
    pub version: u32,
    #[serde(rename = "type")]
    pub word_type: String,
    pub children: Vec<WordChild>,
}

#[derive(Debug, Deserialize)]
pub struct WordChild {
    pub id: String,
    #[serde(rename = "type")]
    pub child_type: String,
    pub glyph_asset: String,
    pub metadata_asset: String,
    pub character: String,
    pub local_transform: TransformConfig,
}

#[derive(Debug, Deserialize)]
pub struct WordMetadata {
    pub name: String,
    pub version: u32,
    pub display: WordDisplay,
}

#[derive(Debug, Deserialize)]
pub struct WordDisplay {
    pub stroke_character: char,
    pub show_word_bounds: bool,
    pub word_bounds_character: char,
    pub show_glyph_bounds: bool,
    pub glyph_bounds_character: char,
    pub show_baseline: bool,
    pub baseline_character: char,
    pub show_origin: bool,
    pub origin_character: char,
    pub show_child_labels: bool,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct TransformConfig {
    pub translation: [f32; 3],
    pub rotation_degrees: [f32; 3],
    pub scale: [f32; 3],
}

pub fn asset_path(relative_path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path)
}

pub fn read_json<T>(relative_path: &str) -> io::Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let path = asset_path(relative_path);

    let text = fs::read_to_string(&path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to read {}: {}", path.display(), error),
        )
    })?;

    serde_json::from_str(&text)
        .map_err(|error| io::Error::other(format!("failed to parse {}: {}", path.display(), error)))
}

pub fn vec3(value: [f32; 3]) -> Vec3 {
    Vec3::new(value[0], value[1], value[2])
}

pub fn transform_matrix(config: TransformConfig) -> Mat4 {
    Mat4::translation_vec3(vec3(config.translation))
        * Mat4::rotation_z(config.rotation_degrees[2].to_radians())
        * Mat4::rotation_y(config.rotation_degrees[1].to_radians())
        * Mat4::rotation_x(config.rotation_degrees[0].to_radians())
        * Mat4::scale(config.scale[0], config.scale[1], config.scale[2])
}

fn draw_transformed_line(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    transform: Mat4,
    from: Vec3,
    to: Vec3,
    character: char,
) {
    let from = transform.transform_point(from);
    let to = transform.transform_point(to);

    canvas.draw_line(projector.project(from), projector.project(to), character);
}

fn draw_glyph(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    glyph: &GlyphAsset,
    metadata: &GlyphMetadata,
    transform: Mat4,
) -> io::Result<()> {
    if glyph.version != 1 || glyph.glyph_type != "bezier_glyph_3d" {
        return Err(io::Error::other(format!(
            "unsupported glyph '{}' version {} type {}",
            glyph.name, glyph.version, glyph.glyph_type,
        )));
    }

    let display = &metadata.display;

    for path in &glyph.paths {
        for segment in &path.segments {
            match segment {
                GlyphSegment::Line { from, to } => {
                    if display.show_strokes {
                        draw_transformed_line(
                            canvas,
                            projector,
                            transform,
                            vec3(*from),
                            vec3(*to),
                            display.stroke_character,
                        );
                    }

                    if display.show_anchor_points {
                        let from = transform.transform_point(vec3(*from));
                        let to = transform.transform_point(vec3(*to));

                        canvas.set(projector.project(from), display.anchor_point_character);
                        canvas.set(projector.project(to), display.anchor_point_character);
                    }
                }

                GlyphSegment::CubicBezier { p0, p1, p2, p3 } => {
                    let curve = CubicBezier3::new(vec3(*p0), vec3(*p1), vec3(*p2), vec3(*p3));

                    if display.show_control_polygons {
                        draw_transformed_line(
                            canvas,
                            projector,
                            transform,
                            curve.p0,
                            curve.p1,
                            display.control_polygon_character,
                        );
                        draw_transformed_line(
                            canvas,
                            projector,
                            transform,
                            curve.p1,
                            curve.p2,
                            display.control_polygon_character,
                        );
                        draw_transformed_line(
                            canvas,
                            projector,
                            transform,
                            curve.p2,
                            curve.p3,
                            display.control_polygon_character,
                        );
                    }

                    if display.show_strokes {
                        let sampled = curve.sample(glyph.sampling.default_segments_per_curve);

                        for (start, end) in sampled.line_segments() {
                            draw_transformed_line(
                                canvas,
                                projector,
                                transform,
                                start,
                                end,
                                display.stroke_character,
                            );
                        }
                    }

                    if display.show_control_points {
                        for point in [curve.p0, curve.p1, curve.p2, curve.p3] {
                            let point = transform.transform_point(point);
                            canvas.set(projector.project(point), display.control_point_character);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn draw_word_helpers(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    metadata: &WordMetadata,
    transform: Mat4,
) {
    let display = &metadata.display;

    let origin = transform.transform_point(Vec3::new(0.0, 0.0, 0.0));
    let baseline_end = transform.transform_point(Vec3::new(1.0, 0.0, 0.0));

    if display.show_baseline {
        canvas.draw_line(
            projector.project(origin),
            projector.project(baseline_end),
            display.baseline_character,
        );
    }

    if display.show_word_bounds {
        let p0 = transform.transform_point(Vec3::new(0.0, 0.0, 0.0));
        let p1 = transform.transform_point(Vec3::new(1.0, 0.0, 0.0));
        let p2 = transform.transform_point(Vec3::new(1.0, 1.0, 0.0));
        let p3 = transform.transform_point(Vec3::new(0.0, 1.0, 0.0));

        canvas.draw_line(
            projector.project(p0),
            projector.project(p1),
            display.word_bounds_character,
        );
        canvas.draw_line(
            projector.project(p1),
            projector.project(p2),
            display.word_bounds_character,
        );
        canvas.draw_line(
            projector.project(p2),
            projector.project(p3),
            display.word_bounds_character,
        );
        canvas.draw_line(
            projector.project(p3),
            projector.project(p0),
            display.word_bounds_character,
        );
    }

    if display.show_origin {
        canvas.set(projector.project(origin), display.origin_character);
    }
}

pub fn render_word(
    canvas: &mut Canvas,
    projector: &ObliqueProjector,
    word: &WordAsset,
    metadata: &WordMetadata,
    word_transform: Mat4,
) -> io::Result<()> {
    if word.version != 1 || word.word_type != "bezier_word_3d" {
        return Err(io::Error::other(format!(
            "unsupported word '{}' version {} type {}",
            word.name, word.version, word.word_type,
        )));
    }

    draw_word_helpers(canvas, projector, metadata, word_transform);

    for child in &word.children {
        let glyph: GlyphAsset = read_json(&child.glyph_asset)?;
        let glyph_metadata: GlyphMetadata = read_json(&child.metadata_asset)?;

        let child_transform = word_transform * transform_matrix(child.local_transform);

        draw_glyph(canvas, projector, &glyph, &glyph_metadata, child_transform)?;

        if metadata.display.show_child_labels {
            let label_position = child_transform.transform_point(Vec3::new(0.0, 1.1, 0.0));
            let label = format!("{}:{}", child.character, child.id);

            canvas.draw_text(projector.project(label_position), &label);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{GlyphAsset, GlyphMetadata, WordAsset, WordMetadata, read_json};

    #[test]
    fn p_glyph_asset_loads() {
        let glyph: GlyphAsset =
            read_json("assets/glyphs/P.simple.glyph.json").expect("P glyph should load");

        assert_eq!(glyph.version, 1);
        assert_eq!(glyph.glyph_type, "bezier_glyph_3d");
        assert_eq!(glyph.paths.len(), 2);
    }

    #[test]
    fn p_glyph_metadata_loads() {
        let metadata: GlyphMetadata =
            read_json("assets/glyphs/P.simple.metadata.json").expect("P metadata should load");

        assert_eq!(metadata.display.stroke_character, '*');
    }

    #[test]
    fn single_p_word_asset_loads() {
        let word: WordAsset =
            read_json("assets/words/single_p.word.json").expect("single_p should load");

        assert_eq!(word.version, 1);
        assert_eq!(word.children.len(), 1);
        assert_eq!(word.children[0].character, "P");
    }

    #[test]
    fn single_p_word_metadata_loads() {
        let metadata: WordMetadata = read_json("assets/words/single_p.metadata.json")
            .expect("single_p metadata should load");

        assert_eq!(metadata.display.origin_character, 'O');
    }
}
