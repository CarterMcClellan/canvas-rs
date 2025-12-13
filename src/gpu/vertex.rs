use bytemuck::{Pod, Zeroable};

/// Vertex data for GPU rendering
/// Each vertex has a 2D position and RGBA color
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    pub const fn new(position: [f32; 2], color: [f32; 4]) -> Self {
        Self { position, color }
    }

    /// Vertex buffer layout descriptor for wgpu
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position attribute
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // Color attribute
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Uniform data passed to shaders
/// Contains the view-projection matrix for transforming vertices
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Uniforms {
    /// 4x4 view-projection matrix (column-major)
    pub view_proj: [[f32; 4]; 4],
}

impl Uniforms {
    /// Create uniforms for a 2D orthographic projection
    /// Maps canvas coordinates (0,0)-(width,height) to clip space (-1,-1)-(1,1)
    pub fn orthographic(width: f32, height: f32) -> Self {
        // Orthographic projection matrix
        // Maps (0, width) to (-1, 1) on X
        // Maps (0, height) to (1, -1) on Y (flip Y for screen coords)
        let view_proj = [
            [2.0 / width, 0.0, 0.0, 0.0],
            [0.0, -2.0 / height, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0, 1.0],
        ];
        Self { view_proj }
    }
}

/// A batch of vertices and indices ready for GPU upload
#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn with_capacity(vertex_capacity: usize, index_capacity: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(vertex_capacity),
            indices: Vec::with_capacity(index_capacity),
        }
    }

    /// Add a mesh to this mesh, offsetting indices appropriately
    pub fn extend(&mut self, other: &Mesh) {
        let index_offset = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&other.vertices);
        self.indices
            .extend(other.indices.iter().map(|i| i + index_offset));
    }

    /// Clear all vertices and indices
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }
}
