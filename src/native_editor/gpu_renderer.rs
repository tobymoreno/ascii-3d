use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use egui_wgpu::wgpu::{
    self,
    util::{DeviceExt, TextureDataOrder},
};

const TERRAIN_WIDTH: u32 = 1024;
const TERRAIN_HEIGHT: u32 = 512;
const TERRAIN_RGBA: &[u8] = include_bytes!("../../assets/maps/earth_terrain_1024x512.rgba");

const SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) light: f32,
    @location(4) bands: vec3<f32>,
    @location(5) thresholds: vec2<f32>,
    @location(6) local_normal: vec3<f32>,
    @location(7) terrain_surface: f32,
};
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) light: f32,
    @location(3) bands: vec3<f32>,
    @location(4) thresholds: vec2<f32>,
    @location(5) local_normal: vec3<f32>,
    @location(6) terrain_surface: f32,
};
@group(0) @binding(0) var terrain_texture: texture_2d<f32>;
@group(0) @binding(1) var terrain_sampler: sampler;
@vertex fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = input.position;
    output.color = input.color;
    output.normal = input.normal;
    output.light = input.light;
    output.bands = input.bands;
    output.thresholds = input.thresholds;
    output.local_normal = input.local_normal;
    output.terrain_surface = input.terrain_surface;
    return output;
}
@fragment fn fs_fill(input: VertexOutput) -> @location(0) vec4<f32> {
    // The Earth sphere is shaded continuously per fragment. The sphere triangles
    // only define coverage; the interpolated analytic normal drives sunlight,
    // so no UV-sphere columns, latitude rings, or toon bands appear in the ocean.
    if (input.terrain_surface > 0.5) {
        let local_normal = normalize(input.local_normal);
        let view_normal = normalize(input.normal);
        let pi = 3.141592653589793;
        let longitude = atan2(local_normal.x, local_normal.z);
        let latitude = asin(clamp(local_normal.y, -1.0, 1.0));
        let uv = vec2<f32>(fract(longitude / (2.0 * pi) + 0.5), 0.5 - latitude / pi);
        let terrain = textureSample(terrain_texture, terrain_sampler, uv);

        // Sun comes from the upper-left of the viewport. A wrapped diffuse term
        // keeps the far side readable while producing a smooth light-to-dark ocean.
        let sun_direction = normalize(vec3<f32>(-0.82, 0.20, 0.54));
        let sun_dot = dot(view_normal, sun_direction);
        let wrapped_light = smoothstep(-0.42, 0.88, sun_dot);

        let ocean_brightness = mix(0.58, 1.10, wrapped_light);
        var ocean_color = input.color.rgb * ocean_brightness;

        // Feather the inside edge of the globe instead of drawing a hard line.
        // The broad smoothstep transition remains independent of sphere mesh edges.
        let limb = 1.0 - clamp(abs(view_normal.z), 0.0, 1.0);
        let inner_rim = smoothstep(0.58, 0.98, limb);
        ocean_color += vec3<f32>(0.045, 0.070, 0.100) * inner_rim;

        // Preserve the baked terrain palette while giving land only a restrained
        // version of the same continuous sunlight gradient.
        let land_brightness = mix(0.88, 1.06, wrapped_light);
        let land_color = terrain.rgb * land_brightness;
        let surface_color = mix(ocean_color, land_color, terrain.a);
        return vec4<f32>(surface_color, input.color.a);
    }

    var surface_color = input.color;
    var lighting = input.light;
    if (dot(input.normal, input.normal) > 0.000001) {
        let normal = normalize(input.normal);
        let light_direction = normalize(vec3<f32>(-0.35, 0.75, 0.55));
        let diffuse = max(dot(normal, light_direction), 0.0);

        let flat_material =
            abs(input.bands.x - input.bands.y) < 0.0001 &&
            abs(input.bands.y - input.bands.z) < 0.0001;
        if (flat_material) {
            lighting = input.bands.x;
        } else {
            lighting = 0.16 + diffuse * 0.84;
        }
    }

    let transition_width = 0.085;
    let dark_to_mid = smoothstep(
        input.thresholds.x - transition_width,
        input.thresholds.x + transition_width,
        lighting,
    );
    let mid_to_light = smoothstep(
        input.thresholds.y - transition_width,
        input.thresholds.y + transition_width,
        lighting,
    );
    let dark_mid = mix(input.bands.x, input.bands.y, dark_to_mid);
    let band = mix(dark_mid, input.bands.z, mid_to_light);
    return vec4<f32>(surface_color.rgb * band, surface_color.a);
}
@fragment fn fs_hull(input: VertexOutput) -> @location(0) vec4<f32> {
    if (input.terrain_surface > 0.5) {
        let normal = normalize(input.normal);
        let limb = 1.0 - clamp(abs(normal.z), 0.0, 1.0);
        let broad_glow = smoothstep(0.18, 0.94, limb);
        let edge_glow = smoothstep(0.70, 1.0, limb);
        let alpha = input.color.a * (0.18 * broad_glow + 0.82 * edge_glow);
        return vec4<f32>(input.color.rgb, alpha);
    }
    return input.color;
}
@fragment fn fs_atmosphere(input: VertexOutput) -> @location(0) vec4<f32> {
    let uv = input.normal.xy;
    let radius = length(uv);
    if (radius < 0.93 || radius > 1.12) {
        discard;
    }

    let safe_uv = normalize(uv + vec2<f32>(0.00001, 0.0));
    let sun_axis = normalize(vec2<f32>(-0.94, 0.34));
    let sun = clamp(dot(safe_uv, sun_axis) * 0.5 + 0.5, 0.0, 1.0);
    let sun_boost = mix(0.82, 1.22, sun);

    let angle = atan2(safe_uv.y, safe_uv.x);
    let angular_variation = 0.92 + 0.08 * (0.5 + 0.5 * sin(angle * 3.0 - 0.6));

    let ring_distance = abs(radius - 1.0);
    let soft_band = 1.0 - smoothstep(0.0, 0.085, ring_distance);
    let core_band = 1.0 - smoothstep(0.0, 0.030, ring_distance);

    let inner_side = 1.0 - smoothstep(0.93, 1.0, radius);
    let outer_side = smoothstep(1.0, 1.12, radius);

    let color =
        vec3<f32>(0.56, 0.78, 1.00) * inner_side * 0.030 +
        vec3<f32>(0.66, 0.86, 1.00) * soft_band * (0.18 + 0.14 * sun_boost) * angular_variation +
        vec3<f32>(0.83, 0.94, 1.00) * core_band * (0.08 + 0.08 * sun_boost);

    let alpha = input.color.a * (
        inner_side * 0.05 +
        soft_band * (0.28 + 0.16 * sun_boost) * angular_variation +
        core_band * 0.10 +
        outer_side * 0.04
    );

    return vec4<f32>(color, alpha);
}
@fragment fn fs_line(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct GpuVertex {
    pub(crate) position: [f32; 4],
    pub(crate) color: [f32; 4],
    pub(crate) normal: [f32; 3],
    pub(crate) light: f32,
    pub(crate) bands: [f32; 3],
    pub(crate) thresholds: [f32; 2],
    pub(crate) local_normal: [f32; 3],
    pub(crate) terrain_surface: f32,
}
impl GpuVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 8] = wgpu::vertex_attr_array![
        0 => Float32x4,
        1 => Float32x4,
        2 => Float32x3,
        3 => Float32,
        4 => Float32x3,
        5 => Float32x2,
        6 => Float32x3,
        7 => Float32,
    ];
    pub(crate) fn new(
        position: [f32; 4],
        color: [f32; 4],
        light: f32,
        bands: [f32; 3],
        thresholds: [f32; 2],
    ) -> Self {
        Self {
            position,
            color,
            normal: [0.0; 3],
            light,
            bands,
            thresholds,
            local_normal: [0.0; 3],
            terrain_surface: 0.0,
        }
    }

    pub(crate) fn with_normal(
        position: [f32; 4],
        color: [f32; 4],
        normal: [f32; 3],
        bands: [f32; 3],
        thresholds: [f32; 2],
    ) -> Self {
        Self {
            position,
            color,
            normal,
            light: 1.0,
            bands,
            thresholds,
            local_normal: [0.0; 3],
            terrain_surface: 0.0,
        }
    }

    pub(crate) fn with_terrain_surface(
        position: [f32; 4],
        color: [f32; 4],
        view_normal: [f32; 3],
        local_normal: [f32; 3],
        bands: [f32; 3],
        thresholds: [f32; 2],
    ) -> Self {
        Self {
            position,
            color,
            normal: view_normal,
            light: 1.0,
            bands,
            thresholds,
            local_normal,
            terrain_surface: 1.0,
        }
    }

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct UploadStats {
    inner: Arc<UploadStatsInner>,
}

#[derive(Default)]
struct UploadStatsInner {
    hull_bytes: AtomicU64,
    fill_bytes: AtomicU64,
    line_bytes: AtomicU64,
    geo_bytes: AtomicU64,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct UploadSnapshot {
    pub(crate) hull_bytes: u64,
    pub(crate) fill_bytes: u64,
    pub(crate) line_bytes: u64,
    pub(crate) geo_bytes: u64,
}

impl UploadSnapshot {
    pub(crate) fn total_bytes(self) -> u64 {
        self.hull_bytes + self.fill_bytes + self.line_bytes + self.geo_bytes
    }
}

impl UploadStats {
    pub(crate) fn snapshot(&self) -> UploadSnapshot {
        UploadSnapshot {
            hull_bytes: self.inner.hull_bytes.load(Ordering::Relaxed),
            fill_bytes: self.inner.fill_bytes.load(Ordering::Relaxed),
            line_bytes: self.inner.line_bytes.load(Ordering::Relaxed),
            geo_bytes: self.inner.geo_bytes.load(Ordering::Relaxed),
        }
    }

    fn record(&self, snapshot: UploadSnapshot) {
        self.inner
            .hull_bytes
            .store(snapshot.hull_bytes, Ordering::Relaxed);
        self.inner
            .fill_bytes
            .store(snapshot.fill_bytes, Ordering::Relaxed);
        self.inner
            .line_bytes
            .store(snapshot.line_bytes, Ordering::Relaxed);
        self.inner
            .geo_bytes
            .store(snapshot.geo_bytes, Ordering::Relaxed);
    }
}

fn vertex_fingerprint(vertices: &[GpuVertex]) -> u64 {
    let bytes: &[u8] = bytemuck::cast_slice(vertices);
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash ^ (vertices.len() as u64)
}

pub(crate) struct GpuViewportCallback {
    fill_vertices: Vec<GpuVertex>,
    hull_vertices: Vec<GpuVertex>,
    line_vertices: Vec<GpuVertex>,
    overlay_line_vertices: Vec<GpuVertex>,
    geo_fill_vertices: Vec<GpuVertex>,
    atmosphere_vertices: Vec<GpuVertex>,
    geo_revision: u64,
    target_format: wgpu::TextureFormat,
    upload_stats: UploadStats,
}
impl GpuViewportCallback {
    pub(crate) fn new(
        fill_vertices: Vec<GpuVertex>,
        hull_vertices: Vec<GpuVertex>,
        line_vertices: Vec<GpuVertex>,
        overlay_line_vertices: Vec<GpuVertex>,
        geo_fill_vertices: Vec<GpuVertex>,
        atmosphere_vertices: Vec<GpuVertex>,
        geo_revision: u64,
        target_format: wgpu::TextureFormat,
        upload_stats: UploadStats,
    ) -> Self {
        Self {
            fill_vertices,
            hull_vertices,
            line_vertices,
            overlay_line_vertices,
            geo_fill_vertices,
            atmosphere_vertices,
            geo_revision,
            target_format,
            upload_stats,
        }
    }
}

struct DynamicVertices {
    buffer: wgpu::Buffer,
    capacity: usize,
    count: u32,
    fingerprint: u64,
}
impl DynamicVertices {
    fn new(device: &wgpu::Device, label: &str, vertices: &[GpuVertex]) -> Self {
        let initial = if vertices.is_empty() {
            vec![GpuVertex::new(
                [0.0; 4],
                [0.0; 4],
                1.0,
                [1.0; 3],
                [0.0, 1.0],
            )]
        } else {
            vertices.to_vec()
        };
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(&initial),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            buffer,
            capacity: initial.len(),
            count: vertices.len() as u32,
            fingerprint: vertex_fingerprint(vertices),
        }
    }
    fn update_if_changed(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        vertices: &[GpuVertex],
    ) -> u64 {
        let fingerprint = vertex_fingerprint(vertices);
        if self.count as usize == vertices.len() && self.fingerprint == fingerprint {
            return 0;
        }

        if vertices.len() > self.capacity {
            self.capacity = vertices.len().next_power_of_two();
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (self.capacity * std::mem::size_of::<GpuVertex>()) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        let bytes: &[u8] = bytemuck::cast_slice(vertices);
        if !bytes.is_empty() {
            queue.write_buffer(&self.buffer, 0, bytes);
        }
        self.count = vertices.len() as u32;
        self.fingerprint = fingerprint;
        bytes.len() as u64
    }
}

struct GpuViewportResources {
    hull_pipeline: wgpu::RenderPipeline,
    fill_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    overlay_line_pipeline: wgpu::RenderPipeline,
    geo_fill_pipeline: wgpu::RenderPipeline,
    atmosphere_pipeline: wgpu::RenderPipeline,
    terrain_bind_group: wgpu::BindGroup,
    hulls: DynamicVertices,
    fills: DynamicVertices,
    lines: DynamicVertices,
    overlay_lines: DynamicVertices,
    geo_fills: DynamicVertices,
    atmospheres: DynamicVertices,
    geo_revision: u64,
}
impl GpuViewportResources {
    fn pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
        format: wgpu::TextureFormat,
        label: &str,
        topology: wgpu::PrimitiveTopology,
        fragment_entry: &str,
        cull_mode: Option<wgpu::Face>,
        write_depth: bool,
        compare: wgpu::CompareFunction,
        bias: i32,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[GpuVertex::layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: Some(write_depth),
                depth_compare: Some(compare),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: bias,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some(fragment_entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        })
    }

    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        hulls: &[GpuVertex],
        fills: &[GpuVertex],
        lines: &[GpuVertex],
        overlay_lines: &[GpuVertex],
        geo_fills: &[GpuVertex],
        atmosphere_vertices: &[GpuVertex],
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ascii-3d toon shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let terrain_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ascii-3d terrain texture layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let terrain_texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("ascii-3d terrain map"),
                size: wgpu::Extent3d {
                    width: TERRAIN_WIDTH,
                    height: TERRAIN_HEIGHT,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            TextureDataOrder::LayerMajor,
            TERRAIN_RGBA,
        );
        let terrain_view = terrain_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let terrain_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ascii-3d terrain sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let terrain_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ascii-3d terrain bind group"),
            layout: &terrain_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&terrain_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&terrain_sampler),
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ascii-3d toon pipeline layout"),
            bind_group_layouts: &[Some(&terrain_layout)],
            immediate_size: 0,
        });
        Self {
            hull_pipeline: Self::pipeline(
                device,
                &layout,
                &shader,
                format,
                "ascii-3d toon hull pipeline",
                wgpu::PrimitiveTopology::TriangleList,
                "fs_hull",
                Some(wgpu::Face::Front),
                false,
                wgpu::CompareFunction::LessEqual,
                0,
            ),
            fill_pipeline: Self::pipeline(
                device,
                &layout,
                &shader,
                format,
                "ascii-3d toon fill pipeline",
                wgpu::PrimitiveTopology::TriangleList,
                "fs_fill",
                None,
                true,
                wgpu::CompareFunction::LessEqual,
                0,
            ),
            line_pipeline: Self::pipeline(
                device,
                &layout,
                &shader,
                format,
                "ascii-3d toon stroke pipeline",
                wgpu::PrimitiveTopology::TriangleList,
                "fs_line",
                None,
                false,
                wgpu::CompareFunction::LessEqual,
                0,
            ),
            overlay_line_pipeline: Self::pipeline(
                device,
                &layout,
                &shader,
                format,
                "ascii-3d overlay line pipeline",
                wgpu::PrimitiveTopology::TriangleList,
                "fs_line",
                None,
                false,
                wgpu::CompareFunction::Always,
                0,
            ),
            geo_fill_pipeline: Self::pipeline(
                device,
                &layout,
                &shader,
                format,
                "ascii-3d geojson fill pipeline",
                wgpu::PrimitiveTopology::TriangleList,
                "fs_fill",
                None,
                false,
                wgpu::CompareFunction::LessEqual,
                1,
            ),
            atmosphere_pipeline: Self::pipeline(
                device,
                &layout,
                &shader,
                format,
                "ascii-3d atmosphere pipeline",
                wgpu::PrimitiveTopology::TriangleList,
                "fs_atmosphere",
                None,
                false,
                wgpu::CompareFunction::Always,
                0,
            ),
            terrain_bind_group,
            hulls: DynamicVertices::new(device, "ascii-3d toon hull vertices", hulls),
            fills: DynamicVertices::new(device, "ascii-3d toon fill vertices", fills),
            lines: DynamicVertices::new(device, "ascii-3d toon stroke vertices", lines),
            overlay_lines: DynamicVertices::new(
                device,
                "ascii-3d overlay line vertices",
                overlay_lines,
            ),
            geo_fills: DynamicVertices::new(device, "ascii-3d geojson fill vertices", geo_fills),
            atmospheres: DynamicVertices::new(
                device,
                "ascii-3d atmosphere vertices",
                atmosphere_vertices,
            ),
            geo_revision: 0,
        }
    }
}

impl egui_wgpu::CallbackTrait for GpuViewportCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let inserted = resources.get::<GpuViewportResources>().is_none();
        if inserted {
            resources.insert(GpuViewportResources::new(
                device,
                queue,
                self.target_format,
                &self.hull_vertices,
                &self.fill_vertices,
                &self.line_vertices,
                &self.overlay_line_vertices,
                &self.geo_fill_vertices,
                &self.atmosphere_vertices,
            ));
        }

        let mut uploads = UploadSnapshot::default();
        if inserted {
            uploads.hull_bytes =
                (self.hull_vertices.len() * std::mem::size_of::<GpuVertex>()) as u64;
            uploads.fill_bytes =
                (self.fill_vertices.len() * std::mem::size_of::<GpuVertex>()) as u64;
            uploads.line_bytes = ((self.line_vertices.len() + self.overlay_line_vertices.len())
                * std::mem::size_of::<GpuVertex>()) as u64;
            uploads.geo_bytes =
                (self.geo_fill_vertices.len() * std::mem::size_of::<GpuVertex>()) as u64;
            uploads.hull_bytes +=
                (self.atmosphere_vertices.len() * std::mem::size_of::<GpuVertex>()) as u64;
        } else if let Some(r) = resources.get_mut::<GpuViewportResources>() {
            uploads.hull_bytes = r.hulls.update_if_changed(
                device,
                queue,
                "ascii-3d toon hull vertices",
                &self.hull_vertices,
            );
            uploads.fill_bytes = r.fills.update_if_changed(
                device,
                queue,
                "ascii-3d toon fill vertices",
                &self.fill_vertices,
            );
            uploads.line_bytes = r.lines.update_if_changed(
                device,
                queue,
                "ascii-3d toon stroke vertices",
                &self.line_vertices,
            );
            uploads.line_bytes += r.overlay_lines.update_if_changed(
                device,
                queue,
                "ascii-3d overlay line vertices",
                &self.overlay_line_vertices,
            );
            uploads.hull_bytes += r.atmospheres.update_if_changed(
                device,
                queue,
                "ascii-3d atmosphere vertices",
                &self.atmosphere_vertices,
            );
            if r.geo_revision != self.geo_revision {
                uploads.geo_bytes = r.geo_fills.update_if_changed(
                    device,
                    queue,
                    "ascii-3d geojson fill vertices",
                    &self.geo_fill_vertices,
                );
                r.geo_revision = self.geo_revision;
            }
        }
        self.upload_stats.record(uploads);
        Vec::new()
    }
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let Some(r) = resources.get::<GpuViewportResources>() else {
            return;
        };
        pass.set_bind_group(0, &r.terrain_bind_group, &[]);
        if r.hulls.count > 0 {
            pass.set_pipeline(&r.hull_pipeline);
            pass.set_vertex_buffer(0, r.hulls.buffer.slice(..));
            pass.draw(0..r.hulls.count, 0..1);
        }
        if r.fills.count > 0 {
            pass.set_pipeline(&r.fill_pipeline);
            pass.set_vertex_buffer(0, r.fills.buffer.slice(..));
            pass.draw(0..r.fills.count, 0..1);
        }
        if r.geo_fills.count > 0 {
            pass.set_pipeline(&r.geo_fill_pipeline);
            pass.set_vertex_buffer(0, r.geo_fills.buffer.slice(..));
            pass.draw(0..r.geo_fills.count, 0..1);
        }
        if r.atmospheres.count > 0 {
            pass.set_pipeline(&r.atmosphere_pipeline);
            pass.set_vertex_buffer(0, r.atmospheres.buffer.slice(..));
            pass.draw(0..r.atmospheres.count, 0..1);
        }
        if r.lines.count > 0 {
            pass.set_pipeline(&r.line_pipeline);
            pass.set_vertex_buffer(0, r.lines.buffer.slice(..));
            pass.draw(0..r.lines.count, 0..1);
        }
        if r.overlay_lines.count > 0 {
            pass.set_pipeline(&r.overlay_line_pipeline);
            pass.set_vertex_buffer(0, r.overlay_lines.buffer.slice(..));
            pass.draw(0..r.overlay_lines.count, 0..1);
        }
    }
}
