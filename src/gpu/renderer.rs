use super::vertex::{Mesh, Uniforms, Vertex};
use crate::scene::Shape;
use std::collections::HashMap;
use wgpu::util::DeviceExt;
use web_sys::HtmlCanvasElement;

/// Maximum number of vertices we can render in a single draw call
const MAX_VERTICES: usize = 65536;
/// Maximum number of indices we can render in a single draw call
const MAX_INDICES: usize = MAX_VERTICES * 3;

/// Multiply two 4x4 matrices (column-major order)
/// Result = a * b
fn multiply_mat4(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut result = [[0.0f32; 4]; 4];
    for col in 0..4 {
        for row in 0..4 {
            result[col][row] =
                a[0][row] * b[col][0] +
                a[1][row] * b[col][1] +
                a[2][row] * b[col][2] +
                a[3][row] * b[col][3];
        }
    }
    result
}

/// MSAA sample count for anti-aliasing (1 = disabled, 4 = recommended)
const MSAA_SAMPLES: u32 = 4;

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
    msaa_texture: wgpu::Texture,
    msaa_view: wgpu::TextureView,
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
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
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

        // Create MSAA texture for anti-aliased rendering
        let (msaa_texture, msaa_view) =
            Self::create_msaa_texture(&device, width, height, surface_format);

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
                count: MSAA_SAMPLES,
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
            msaa_texture,
            msaa_view,
            width,
            height,
        })
    }

    /// Create MSAA texture for anti-aliased rendering
    fn create_msaa_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("MSAA Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: MSAA_SAMPLES,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    /// Resize the renderer when canvas size changes
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 && (width != self.width || height != self.height) {
            self.width = width;
            self.height = height;
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);

            // Recreate MSAA texture at new size
            let (msaa_texture, msaa_view) =
                Self::create_msaa_texture(&self.device, width, height, self.config.format);
            self.msaa_texture = msaa_texture;
            self.msaa_view = msaa_view;

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

        // Begin render pass - render to MSAA texture, resolve to swapchain
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Shape Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_view,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_color[0] as f64,
                            g: clear_color[1] as f64,
                            b: clear_color[2] as f64,
                            a: clear_color[3] as f64,
                        }),
                        store: wgpu::StoreOp::Discard, // MSAA samples discarded after resolve
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

    /// Render shapes with per-shape transform overrides
    /// This is the fast path for dragging/transforming selected shapes
    ///
    /// - `shape_meshes`: Pre-tessellated meshes for each shape (keyed by shape ID)
    /// - `shapes`: The shapes to render (for getting base transforms)
    /// - `transform_overrides`: Map of shape ID to transform matrix override
    /// - `clear_color`: Background color
    pub fn render_shapes_with_transforms(
        &mut self,
        shape_meshes: &HashMap<u64, Mesh>,
        shapes: &[Shape],
        transform_overrides: &HashMap<u64, [[f32; 4]; 4]>,
        clear_color: [f32; 4],
    ) -> Result<(), String> {
        // Get surface texture to render to
        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| format!("Failed to get surface texture: {e}"))?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Collect renderable shapes
        let renderable_shapes: Vec<_> = shapes
            .iter()
            .filter_map(|shape| {
                let mesh = shape_meshes.get(&shape.id)?;
                if mesh.is_empty()
                    || mesh.vertices.len() > MAX_VERTICES
                    || mesh.indices.len() > MAX_INDICES
                {
                    return None;
                }
                Some((shape, mesh))
            })
            .collect();

        let total_shapes = renderable_shapes.len();

        // First pass: clear the MSAA texture
        {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Clear Encoder"),
                });

            {
                let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Clear Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.msaa_view,
                        resolve_target: if total_shapes == 0 {
                            Some(&view) // Resolve immediately if no shapes
                        } else {
                            None
                        },
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
            }

            self.queue.submit(std::iter::once(encoder.finish()));
        }

        // Render each shape with its own transform to MSAA texture
        // Each shape needs its own submit because uniform buffers are shared
        for (i, (shape, mesh)) in renderable_shapes.iter().enumerate() {
            let is_last = i == total_shapes - 1;

            // Get transform - compose override with shape's base transform if available
            // The shape's base transform positions the shape in world space
            // The override applies additional translation/scale during drag operations
            let base_transform = shape.transform.to_matrix4();
            let model_transform = if let Some(override_transform) = transform_overrides.get(&shape.id) {
                // Compose: override * base (apply base first to get world position, then override)
                multiply_mat4(override_transform, &base_transform)
            } else {
                base_transform
            };

            // Update buffers
            let uniforms = Uniforms::orthographic(self.width as f32, self.height as f32)
                .with_model_transform(model_transform);
            self.queue
                .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
            self.queue
                .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&mesh.vertices));
            self.queue
                .write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&mesh.indices));

            // Create encoder and render pass for this shape
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Shape Encoder"),
                });

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Shape Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.msaa_view,
                        resolve_target: if is_last { Some(&view) } else { None },
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load, // Don't clear, preserve previous draws
                            store: if is_last {
                                wgpu::StoreOp::Discard // MSAA samples discarded after resolve
                            } else {
                                wgpu::StoreOp::Store
                            },
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
            }

            // Submit this shape's commands
            self.queue.submit(std::iter::once(encoder.finish()));
        }

        output.present();

        Ok(())
    }
}
