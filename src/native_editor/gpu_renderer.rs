use egui_wgpu::wgpu::{self, util::DeviceExt};

const SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) light: f32,
    @location(3) bands: vec3<f32>,
    @location(4) thresholds: vec2<f32>,
};
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) light: f32,
    @location(2) bands: vec3<f32>,
    @location(3) thresholds: vec2<f32>,
};
@vertex fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = input.position;
    output.color = input.color;
    output.light = input.light;
    output.bands = input.bands;
    output.thresholds = input.thresholds;
    return output;
}
@fragment fn fs_fill(input: VertexOutput) -> @location(0) vec4<f32> {
    var band = input.bands.z;
    if (input.light < input.thresholds.x) {
        band = input.bands.x;
    } else if (input.light < input.thresholds.y) {
        band = input.bands.y;
    }
    return vec4<f32>(input.color.rgb * band, input.color.a);
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
    pub(crate) light: f32,
    pub(crate) bands: [f32; 3],
    pub(crate) thresholds: [f32; 2],
}
impl GpuVertex {
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
            light,
            bands,
            thresholds,
        }
    }
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 2,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: std::mem::size_of::<[f32; 9]>() as wgpu::BufferAddress,
                    shader_location: 3,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: std::mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 4,
                },
            ],
        }
    }
}

pub(crate) struct GpuViewportCallback {
    fill_vertices: Vec<GpuVertex>,
    hull_vertices: Vec<GpuVertex>,
    line_vertices: Vec<GpuVertex>,
    target_format: wgpu::TextureFormat,
}
impl GpuViewportCallback {
    pub(crate) fn new(
        fill_vertices: Vec<GpuVertex>,
        hull_vertices: Vec<GpuVertex>,
        line_vertices: Vec<GpuVertex>,
        target_format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            fill_vertices,
            hull_vertices,
            line_vertices,
            target_format,
        }
    }
}

struct DynamicVertices {
    buffer: wgpu::Buffer,
    capacity: usize,
    count: u32,
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
        }
    }
    fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        vertices: &[GpuVertex],
    ) {
        if vertices.len() > self.capacity {
            self.capacity = vertices.len().next_power_of_two();
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (self.capacity * std::mem::size_of::<GpuVertex>()) as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !vertices.is_empty() {
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(vertices));
        }
        self.count = vertices.len() as u32;
    }
}

struct GpuViewportResources {
    hull_pipeline: wgpu::RenderPipeline,
    fill_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    hulls: DynamicVertices,
    fills: DynamicVertices,
    lines: DynamicVertices,
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
        format: wgpu::TextureFormat,
        hulls: &[GpuVertex],
        fills: &[GpuVertex],
        lines: &[GpuVertex],
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ascii-3d toon shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ascii-3d toon pipeline layout"),
            bind_group_layouts: &[],
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
                "fs_line",
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
            hulls: DynamicVertices::new(device, "ascii-3d toon hull vertices", hulls),
            fills: DynamicVertices::new(device, "ascii-3d toon fill vertices", fills),
            lines: DynamicVertices::new(device, "ascii-3d toon stroke vertices", lines),
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
        if resources.get::<GpuViewportResources>().is_none() {
            resources.insert(GpuViewportResources::new(
                device,
                self.target_format,
                &self.hull_vertices,
                &self.fill_vertices,
                &self.line_vertices,
            ));
        }
        if let Some(r) = resources.get_mut::<GpuViewportResources>() {
            r.hulls.update(
                device,
                queue,
                "ascii-3d toon hull vertices",
                &self.hull_vertices,
            );
            r.fills.update(
                device,
                queue,
                "ascii-3d toon fill vertices",
                &self.fill_vertices,
            );
            r.lines.update(
                device,
                queue,
                "ascii-3d toon stroke vertices",
                &self.line_vertices,
            );
        }
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
        if r.lines.count > 0 {
            pass.set_pipeline(&r.line_pipeline);
            pass.set_vertex_buffer(0, r.lines.buffer.slice(..));
            pass.draw(0..r.lines.count, 0..1);
        }
    }
}
