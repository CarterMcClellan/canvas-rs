use super::vertex::{Mesh, Uniforms, Vertex};
use wgpu::util::DeviceExt;
use web_sys::HtmlCanvasElement;

/// Maximum number of vertices we can render in a single draw call
const MAX_VERTICES: usize = 65536;
/// Maximum number of indices we can render in a single draw call
const MAX_INDICES: usize = MAX_VERTICES * 3;

/// GPU renderer using wgpu
/// Handles WebGL/WebGPU initialization and shape rendering
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

impl Renderer {
    /// Create a new renderer attached to an HTML canvas element
    pub async fn new(canvas: HtmlCanvasElement) -> Result<Self, String> {
        let width = canvas.width();
        let height = canvas.height();

        // Create wgpu instance - use WebGL2 only for browser compatibility
        // WebGPU has compatibility issues with wgpu 22.x and current Chrome
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        // Create surface from canvas
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.into()))
            .map_err(|e| format!("Failed to create surface: {e}"))?;

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("Failed to find a suitable GPU adapter")?;

        // Request device and queue with WebGL2-compatible limits
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Canvas Renderer Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| format!("Failed to create device: {e}"))?;

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);

        // Prefer non-sRGB format to avoid double gamma correction
        // (our hex colors are already in sRGB space, so we pass them through directly)
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| !f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        // Prefer premultiplied alpha for proper transparency compositing with the page
        let alpha_mode = if surface_caps
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::PreMultiplied)
        {
            wgpu::CompositeAlphaMode::PreMultiplied
        } else {
            surface_caps.alpha_modes[0]
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shape Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders.wgsl").into()),
        });

        // Create uniform buffer
        let uniforms = Uniforms::orthographic(width as f32, height as f32);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout and bind group
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create render pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shape Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // No culling for 2D
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Create vertex and index buffers with initial capacity
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: (MAX_VERTICES * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: (MAX_INDICES * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            uniform_bind_group,
            width,
            height,
        })
    }

    /// Resize the renderer when canvas size changes
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 && (width != self.width || height != self.height) {
            self.width = width;
            self.height = height;
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);

            // Update uniforms with new projection
            let uniforms = Uniforms::orthographic(width as f32, height as f32);
            self.queue
                .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }
    }

    /// Render a mesh to the canvas
    /// Clears with the given background color and draws all triangles
    pub fn render(&mut self, mesh: &Mesh, clear_color: [f32; 4]) -> Result<(), String> {
        if mesh.vertices.len() > MAX_VERTICES {
            return Err(format!(
                "Too many vertices: {} (max {})",
                mesh.vertices.len(),
                MAX_VERTICES
            ));
        }
        if mesh.indices.len() > MAX_INDICES {
            return Err(format!(
                "Too many indices: {} (max {})",
                mesh.indices.len(),
                MAX_INDICES
            ));
        }

        // Get surface texture to render to
        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| format!("Failed to get surface texture: {e}"))?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Upload vertex and index data
        if !mesh.is_empty() {
            self.queue
                .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&mesh.vertices));
            self.queue
                .write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&mesh.indices));
        }

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Begin render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Shape Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_color[0] as f64,
                            g: clear_color[1] as f64,
                            b: clear_color[2] as f64,
                            a: clear_color[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if !mesh.is_empty() {
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
            }
        }

        // Submit commands
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Get current canvas width
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get current canvas height
    pub fn height(&self) -> u32 {
        self.height
    }
}

/// Create a simple triangle mesh for testing
pub fn test_triangle() -> Mesh {
    let vertices = vec![
        Vertex::new([400.0, 100.0], [1.0, 0.0, 0.0, 1.0]), // Top - Red
        Vertex::new([200.0, 500.0], [0.0, 1.0, 0.0, 1.0]), // Bottom left - Green
        Vertex::new([600.0, 500.0], [0.0, 0.0, 1.0, 1.0]), // Bottom right - Blue
    ];
    let indices = vec![0, 1, 2];

    Mesh { vertices, indices }
}
