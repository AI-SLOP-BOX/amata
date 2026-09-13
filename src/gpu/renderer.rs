use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
    tex_coord: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct GpuUniforms {
    resolution: [f32; 2],
    time: f32,
    zoom: f32,
    pan: [f32; 2],
    glow_radius: f32,
    glow_intensity: f32,
    blur_radius: f32,
    shadow_offset: [f32; 2],
    shadow_blur: f32,
    shadow_opacity: f32,
    opacity: f32,
    _padding: f32,
}

pub struct GpuRenderer {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    blur_pipeline: Option<wgpu::ComputePipeline>,
    blur_bind_group_layout: Option<wgpu::BindGroupLayout>,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl GpuRenderer {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Vector Path Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Vector Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Vector Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Vector Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 8,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                        wgpu::VertexAttribute {
                            offset: 24,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Blur compute pipeline
        let blur_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Blur Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba16Float,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let blur_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Blur Compute Pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Blur Pipeline Layout"),
                    bind_group_layouts: &[&blur_bind_group_layout],
                    push_constant_ranges: &[],
                }),
            ),
            module: &shader,
            entry_point: Some("cs_blur"),
            compilation_options: Default::default(),
            cache: None,
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: (1024 * 64) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: (1024 * 128) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[GpuUniforms {
                resolution: [1920.0, 1080.0],
                time: 0.0,
                zoom: 1.0,
                pan: [0.0, 0.0],
                glow_radius: 0.0,
                glow_intensity: 0.0,
                blur_radius: 0.0,
                shadow_offset: [0.0, 0.0],
                shadow_blur: 0.0,
                shadow_opacity: 0.0,
                opacity: 1.0,
                _padding: 0.0,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            device,
            queue,
            pipeline,
            bind_group_layout,
            blur_pipeline: Some(blur_pipeline),
            blur_bind_group_layout: Some(blur_bind_group_layout),
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn begin_frame(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    pub fn push_quad(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, color: [f32; 4]) {
        let base = self.vertices.len() as u32;
        self.vertices.push(Vertex {
            position: [x0, y0],
            color,
            tex_coord: [0.0, 0.0],
        });
        self.vertices.push(Vertex {
            position: [x1, y0],
            color,
            tex_coord: [1.0, 0.0],
        });
        self.vertices.push(Vertex {
            position: [x1, y1],
            color,
            tex_coord: [1.0, 1.0],
        });
        self.vertices.push(Vertex {
            position: [x0, y1],
            color,
            tex_coord: [0.0, 1.0],
        });
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn push_triangle(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: [f32; 4],
    ) {
        let base = self.vertices.len() as u32;
        self.vertices.push(Vertex {
            position: [x0, y0],
            color,
            tex_coord: [0.0, 0.0],
        });
        self.vertices.push(Vertex {
            position: [x1, y1],
            color,
            tex_coord: [1.0, 0.0],
        });
        self.vertices.push(Vertex {
            position: [x2, y2],
            color,
            tex_coord: [0.5, 1.0],
        });
        self.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    /// Triangulate a convex polygon and push to GPU buffers
    pub fn push_convex_polygon(&mut self, points: &[(f32, f32)], color: [f32; 4]) {
        if points.len() < 3 {
            return;
        }
        // Fan triangulation from first vertex
        let base = self.vertices.len() as u32;
        for (x, y) in points {
            self.vertices.push(Vertex {
                position: [*x, *y],
                color,
                tex_coord: [0.5, 0.5],
            });
        }
        for i in 1..points.len() as u32 - 1 {
            self.indices
                .extend_from_slice(&[base, base + i, base + i + 1]);
        }
    }

    /// Push a thick line segment as a quad
    pub fn push_thick_line(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        width: f32,
        color: [f32; 4],
    ) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.001 {
            return;
        }
        let nx = -dy / len * width * 0.5;
        let ny = dx / len * width * 0.5;

        let base = self.vertices.len() as u32;
        self.vertices.push(Vertex {
            position: [x0 + nx, y0 + ny],
            color,
            tex_coord: [0.0, 0.0],
        });
        self.vertices.push(Vertex {
            position: [x0 - nx, y0 - ny],
            color,
            tex_coord: [0.0, 1.0],
        });
        self.vertices.push(Vertex {
            position: [x1 - nx, y1 - ny],
            color,
            tex_coord: [1.0, 1.0],
        });
        self.vertices.push(Vertex {
            position: [x1 + nx, y1 + ny],
            color,
            tex_coord: [1.0, 0.0],
        });
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    /// Push a circle as a triangle fan
    pub fn push_circle(&mut self, cx: f32, cy: f32, radius: f32, segments: u32, color: [f32; 4]) {
        let base = self.vertices.len() as u32;
        // Center vertex
        self.vertices.push(Vertex {
            position: [cx, cy],
            color,
            tex_coord: [0.5, 0.5],
        });

        for i in 0..segments {
            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let x = cx + angle.cos() * radius;
            let y = cy + angle.sin() * radius;
            self.vertices.push(Vertex {
                position: [x, y],
                color,
                tex_coord: [0.5 + 0.5 * angle.cos(), 0.5 + 0.5 * angle.sin()],
            });
        }

        for i in 1..segments {
            self.indices
                .extend_from_slice(&[base, base + i, base + i + 1]);
        }
        // Close the fan
        self.indices
            .extend_from_slice(&[base, base + segments, base + 1]);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        render_pass: &mut wgpu::RenderPass,
        resolution: [f32; 2],
        zoom: f32,
        pan: [f32; 2],
        time: f32,
        glow_radius: f32,
        glow_intensity: f32,
        blur_radius: f32,
        shadow_offset: [f32; 2],
        shadow_blur: f32,
        shadow_opacity: f32,
        opacity: f32,
    ) {
        if self.vertices.is_empty() {
            return;
        }

        // Update uniforms
        let uniforms = GpuUniforms {
            resolution,
            time,
            zoom,
            pan,
            glow_radius,
            glow_intensity,
            blur_radius,
            shadow_offset,
            shadow_blur,
            shadow_opacity,
            opacity,
            _padding: 0.0,
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        // Upload vertex and index data
        self.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
        self.queue
            .write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices));

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Vector Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.uniform_buffer.as_entire_binding(),
            }],
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
    }

    /// Apply Gaussian blur via compute shader
    pub fn apply_blur(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        input: &wgpu::TextureView,
        output: &wgpu::TextureView,
        width: u32,
        height: u32,
        blur_radius: f32,
    ) {
        if let (Some(pipeline), Some(layout)) = (&self.blur_pipeline, &self.blur_bind_group_layout)
        {
            let uniforms = GpuUniforms {
                resolution: [width as f32, height as f32],
                time: 0.0,
                zoom: 1.0,
                pan: [0.0, 0.0],
                glow_radius: 0.0,
                glow_intensity: 0.0,
                blur_radius,
                shadow_offset: [0.0, 0.0],
                shadow_blur: 0.0,
                shadow_opacity: 0.0,
                opacity: 1.0,
                _padding: 0.0,
            };

            let uniform_buf = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Blur Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[uniforms]),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Blur Bind Group"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(input),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(output),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: uniform_buf.as_entire_binding(),
                    },
                ],
            });

            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Blur Compute Pass"),
                    timestamp_writes: None,
                });
                compute_pass.set_pipeline(pipeline);
                compute_pass.set_bind_group(0, &bind_group, &[]);
                compute_pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
            }
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }
}
