use crate::gpu::vertex::{Mesh, Vertex};
use crate::scene::{Color, Shape, ShapeGeometry, Transform2D, Vec2};
use lyon::geom::point;
use lyon::path::Path;
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers,
};
use std::collections::HashMap;

/// Tessellation tolerance in pixels - lower values produce smoother curves
/// Default is 0.25, we use 0.1 for higher quality rendering
const TESSELLATION_TOLERANCE: f32 = 0.1;

/// Convert an SVG elliptical arc to cubic bezier curves
/// Based on the SVG arc implementation algorithm
fn arc_to_beziers(
    from: Vec2,
    rx: f32,
    ry: f32,
    x_rotation: f32,
    large_arc: bool,
    sweep: bool,
    to: Vec2,
) -> Vec<(Vec2, Vec2, Vec2)> {
    // Handle degenerate cases
    if from == to {
        return vec![];
    }

    let mut rx = rx.abs();
    let mut ry = ry.abs();

    if rx == 0.0 || ry == 0.0 {
        // Treat as line
        return vec![];
    }

    let phi = x_rotation.to_radians();
    let cos_phi = phi.cos();
    let sin_phi = phi.sin();

    // Step 1: Compute (x1', y1')
    let dx = (from.x - to.x) / 2.0;
    let dy = (from.y - to.y) / 2.0;
    let x1_prime = cos_phi * dx + sin_phi * dy;
    let y1_prime = -sin_phi * dx + cos_phi * dy;

    // Step 2: Compute (cx', cy')
    let rx_sq = rx * rx;
    let ry_sq = ry * ry;
    let x1_prime_sq = x1_prime * x1_prime;
    let y1_prime_sq = y1_prime * y1_prime;

    // Check if radii are large enough
    let lambda = x1_prime_sq / rx_sq + y1_prime_sq / ry_sq;
    if lambda > 1.0 {
        let lambda_sqrt = lambda.sqrt();
        rx *= lambda_sqrt;
        ry *= lambda_sqrt;
    }

    let rx_sq = rx * rx;
    let ry_sq = ry * ry;

    let num = rx_sq * ry_sq - rx_sq * y1_prime_sq - ry_sq * x1_prime_sq;
    let den = rx_sq * y1_prime_sq + ry_sq * x1_prime_sq;

    let sq = if den == 0.0 { 0.0 } else { (num / den).max(0.0).sqrt() };
    let sq = if large_arc == sweep { -sq } else { sq };

    let cx_prime = sq * rx * y1_prime / ry;
    let cy_prime = -sq * ry * x1_prime / rx;

    // Step 3: Compute (cx, cy) from (cx', cy')
    let cx = cos_phi * cx_prime - sin_phi * cy_prime + (from.x + to.x) / 2.0;
    let cy = sin_phi * cx_prime + cos_phi * cy_prime + (from.y + to.y) / 2.0;

    // Step 4: Compute theta1 and dtheta
    fn angle(ux: f32, uy: f32, vx: f32, vy: f32) -> f32 {
        let n = (ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt();
        if n == 0.0 {
            return 0.0;
        }
        let c = (ux * vx + uy * vy) / n;
        let c = c.clamp(-1.0, 1.0);
        let angle = c.acos();
        if ux * vy - uy * vx < 0.0 { -angle } else { angle }
    }

    let theta1 = angle(1.0, 0.0, (x1_prime - cx_prime) / rx, (y1_prime - cy_prime) / ry);
    let mut dtheta = angle(
        (x1_prime - cx_prime) / rx,
        (y1_prime - cy_prime) / ry,
        (-x1_prime - cx_prime) / rx,
        (-y1_prime - cy_prime) / ry,
    );

    if !sweep && dtheta > 0.0 {
        dtheta -= 2.0 * std::f32::consts::PI;
    } else if sweep && dtheta < 0.0 {
        dtheta += 2.0 * std::f32::consts::PI;
    }

    // Convert arc to bezier curves
    // Split into segments of at most 90 degrees
    let num_segments = ((dtheta.abs() / (std::f32::consts::PI / 2.0)).ceil() as usize).max(1);
    let segment_angle = dtheta / num_segments as f32;

    let mut curves = Vec::new();
    let mut current_theta = theta1;

    for _ in 0..num_segments {
        let theta2 = current_theta + segment_angle;

        // Compute bezier control points for this arc segment
        let t = (segment_angle / 4.0).tan();
        let alpha = segment_angle.sin() * ((4.0 + 3.0 * t * t).sqrt() - 1.0) / 3.0;

        let cos_t1 = current_theta.cos();
        let sin_t1 = current_theta.sin();
        let cos_t2 = theta2.cos();
        let sin_t2 = theta2.sin();

        // Points on the unit circle
        let p1x = cos_t1;
        let p1y = sin_t1;
        let p2x = cos_t2;
        let p2y = sin_t2;

        // Control points
        let c1x = p1x - alpha * sin_t1;
        let c1y = p1y + alpha * cos_t1;
        let c2x = p2x + alpha * sin_t2;
        let c2y = p2y - alpha * cos_t2;

        // Transform from unit circle to actual ellipse
        fn transform_point(px: f32, py: f32, rx: f32, ry: f32, cos_phi: f32, sin_phi: f32, cx: f32, cy: f32) -> Vec2 {
            let x = rx * px;
            let y = ry * py;
            Vec2::new(
                cos_phi * x - sin_phi * y + cx,
                sin_phi * x + cos_phi * y + cy,
            )
        }

        let ctrl1 = transform_point(c1x, c1y, rx, ry, cos_phi, sin_phi, cx, cy);
        let ctrl2 = transform_point(c2x, c2y, rx, ry, cos_phi, sin_phi, cx, cy);
        let end = transform_point(p2x, p2y, rx, ry, cos_phi, sin_phi, cx, cy);

        curves.push((ctrl1, ctrl2, end));
        current_theta = theta2;
    }

    curves
}

/// Tessellator for converting shapes to GPU-renderable triangles
/// Includes a cache to avoid re-tessellating unchanged shapes
pub struct Tessellator {
    fill_tessellator: FillTessellator,
    stroke_tessellator: StrokeTessellator,
    /// Cache of tessellated meshes by shape ID
    mesh_cache: HashMap<u64, Mesh>,
}

impl Default for Tessellator {
    fn default() -> Self {
        Self::new()
    }
}

impl Tessellator {
    pub fn new() -> Self {
        Self {
            fill_tessellator: FillTessellator::new(),
            stroke_tessellator: StrokeTessellator::new(),
            mesh_cache: HashMap::new(),
        }
    }

    /// Clear the mesh cache
    pub fn clear_cache(&mut self) {
        self.mesh_cache.clear();
    }

    /// Remove a specific shape from the cache
    pub fn invalidate_shape(&mut self, shape_id: u64) {
        self.mesh_cache.remove(&shape_id);
    }

    /// Get or create a cached mesh for a shape
    /// Uses the shape's dirty flag to determine if re-tessellation is needed
    /// IMPORTANT: This tessellates with identity transform - the actual transform
    /// is applied in the shader via uniform
    pub fn get_or_tessellate_shape(&mut self, shape: &Shape) -> &Mesh {
        let shape_id = shape.id;

        // Check if we need to re-tessellate
        if shape.dirty || !self.mesh_cache.contains_key(&shape_id) {
            let mesh = self.tessellate_shape_at_origin(shape);
            self.mesh_cache.insert(shape_id, mesh);
        }

        self.mesh_cache.get(&shape_id).unwrap()
    }

    /// Tessellate a shape at origin (without applying shape's transform)
    /// The transform will be applied in the shader
    fn tessellate_shape_at_origin(&mut self, shape: &Shape) -> Mesh {
        let mut mesh = Mesh::new();
        let identity = Transform2D::identity();

        // Tessellate fill if present
        if let Some(fill_color) = shape.style.fill {
            if let Some(fill_mesh) = self.tessellate_geometry_fill(&shape.geometry, &identity, fill_color) {
                mesh.extend(&fill_mesh);
            }
        }

        // Tessellate stroke if present
        if let Some(stroke) = shape.style.stroke {
            if let Some(stroke_mesh) = self.tessellate_geometry_stroke(
                &shape.geometry,
                &identity,
                stroke.color,
                stroke.width,
            ) {
                mesh.extend(&stroke_mesh);
            }
        }

        mesh
    }

    /// Tessellate a shape into a mesh (includes shape's transform baked in)
    /// Use get_or_tessellate_shape for cached version without transform
    pub fn tessellate_shape(&mut self, shape: &Shape) -> Mesh {
        let mut mesh = Mesh::new();

        // Tessellate fill if present
        if let Some(fill_color) = shape.style.fill {
            if let Some(fill_mesh) = self.tessellate_geometry_fill(&shape.geometry, &shape.transform, fill_color) {
                mesh.extend(&fill_mesh);
            }
        }

        // Tessellate stroke if present
        if let Some(stroke) = shape.style.stroke {
            if let Some(stroke_mesh) = self.tessellate_geometry_stroke(
                &shape.geometry,
                &shape.transform,
                stroke.color,
                stroke.width,
            ) {
                mesh.extend(&stroke_mesh);
            }
        }

        mesh
    }

    /// Tessellate multiple shapes into a single mesh (legacy method)
    /// For better performance, use get_or_tessellate_shape with per-shape rendering
    pub fn tessellate_shapes(&mut self, shapes: &[Shape]) -> Mesh {
        let mut mesh = Mesh::new();
        for shape in shapes {
            let shape_mesh = self.tessellate_shape(shape);
            mesh.extend(&shape_mesh);
        }
        mesh
    }

    /// Tessellate geometry fill
    fn tessellate_geometry_fill(
        &mut self,
        geometry: &ShapeGeometry,
        transform: &Transform2D,
        color: Color,
    ) -> Option<Mesh> {
        match geometry {
            ShapeGeometry::Polygon { points } => {
                self.tessellate_polygon_fill(points, transform, color)
            }
            ShapeGeometry::Rectangle {
                width,
                height,
                corner_radius,
            } => self.tessellate_rectangle_fill(*width, *height, *corner_radius, transform, color),
            ShapeGeometry::Ellipse { rx, ry } => {
                self.tessellate_ellipse_fill(*rx, *ry, transform, color)
            }
            ShapeGeometry::Path { commands } => {
                self.tessellate_path_fill(commands, transform, color)
            }
        }
    }

    /// Tessellate geometry stroke
    fn tessellate_geometry_stroke(
        &mut self,
        geometry: &ShapeGeometry,
        transform: &Transform2D,
        color: Color,
        width: f32,
    ) -> Option<Mesh> {
        match geometry {
            ShapeGeometry::Polygon { points } => {
                self.tessellate_polygon_stroke(points, transform, color, width)
            }
            ShapeGeometry::Rectangle {
                width: w,
                height: h,
                corner_radius,
            } => self.tessellate_rectangle_stroke(*w, *h, *corner_radius, transform, color, width),
            ShapeGeometry::Ellipse { rx, ry } => {
                self.tessellate_ellipse_stroke(*rx, *ry, transform, color, width)
            }
            ShapeGeometry::Path { commands } => {
                self.tessellate_path_stroke(commands, transform, color, width)
            }
        }
    }

    /// Tessellate a polygon fill
    fn tessellate_polygon_fill(
        &mut self,
        points: &[Vec2],
        transform: &Transform2D,
        color: Color,
    ) -> Option<Mesh> {
        if points.len() < 3 {
            return None;
        }

        // Build path from points
        let mut builder = Path::builder();
        let first = transform.transform_point(points[0]);
        builder.begin(point(first.x, first.y));
        for p in &points[1..] {
            let transformed = transform.transform_point(*p);
            builder.line_to(point(transformed.x, transformed.y));
        }
        builder.close();
        let path = builder.build();

        // Tessellate
        let mut buffers: VertexBuffers<Vertex, u32> = VertexBuffers::new();
        let color_arr = color.to_array();

        let result = self.fill_tessellator.tessellate_path(
            &path,
            &FillOptions::tolerance(TESSELLATION_TOLERANCE),
            &mut BuffersBuilder::new(&mut buffers, |vertex: FillVertex| Vertex {
                position: [vertex.position().x, vertex.position().y],
                color: color_arr,
            }),
        );

        if result.is_ok() {
            Some(Mesh {
                vertices: buffers.vertices,
                indices: buffers.indices,
            })
        } else {
            None
        }
    }

    /// Tessellate a polygon stroke
    fn tessellate_polygon_stroke(
        &mut self,
        points: &[Vec2],
        transform: &Transform2D,
        color: Color,
        width: f32,
    ) -> Option<Mesh> {
        if points.len() < 2 {
            return None;
        }

        // Build path from points
        let mut builder = Path::builder();
        let first = transform.transform_point(points[0]);
        builder.begin(point(first.x, first.y));
        for p in &points[1..] {
            let transformed = transform.transform_point(*p);
            builder.line_to(point(transformed.x, transformed.y));
        }
        builder.close();
        let path = builder.build();

        // Tessellate stroke
        let mut buffers: VertexBuffers<Vertex, u32> = VertexBuffers::new();
        let color_arr = color.to_array();

        let result = self.stroke_tessellator.tessellate_path(
            &path,
            &StrokeOptions::tolerance(TESSELLATION_TOLERANCE).with_line_width(width),
            &mut BuffersBuilder::new(&mut buffers, |vertex: StrokeVertex| Vertex {
                position: [vertex.position().x, vertex.position().y],
                color: color_arr,
            }),
        );

        if result.is_ok() {
            Some(Mesh {
                vertices: buffers.vertices,
                indices: buffers.indices,
            })
        } else {
            None
        }
    }

    /// Tessellate a rectangle fill
    fn tessellate_rectangle_fill(
        &mut self,
        width: f32,
        height: f32,
        corner_radius: f32,
        transform: &Transform2D,
        color: Color,
    ) -> Option<Mesh> {
        let mut builder = Path::builder();

        if corner_radius <= 0.0 {
            // Simple rectangle
            let corners = [
                Vec2::new(0.0, 0.0),
                Vec2::new(width, 0.0),
                Vec2::new(width, height),
                Vec2::new(0.0, height),
            ];
            let first = transform.transform_point(corners[0]);
            builder.begin(point(first.x, first.y));
            for corner in &corners[1..] {
                let transformed = transform.transform_point(*corner);
                builder.line_to(point(transformed.x, transformed.y));
            }
            builder.close();
        } else {
            // Rounded rectangle
            let r = corner_radius.min(width / 2.0).min(height / 2.0);

            // Start at top-left after curve
            let start = transform.transform_point(Vec2::new(r, 0.0));
            builder.begin(point(start.x, start.y));

            // Top edge to top-right corner
            let p = transform.transform_point(Vec2::new(width - r, 0.0));
            builder.line_to(point(p.x, p.y));

            // Top-right corner (approximate arc with quadratic)
            let ctrl = transform.transform_point(Vec2::new(width, 0.0));
            let end = transform.transform_point(Vec2::new(width, r));
            builder.quadratic_bezier_to(point(ctrl.x, ctrl.y), point(end.x, end.y));

            // Right edge to bottom-right corner
            let p = transform.transform_point(Vec2::new(width, height - r));
            builder.line_to(point(p.x, p.y));

            // Bottom-right corner
            let ctrl = transform.transform_point(Vec2::new(width, height));
            let end = transform.transform_point(Vec2::new(width - r, height));
            builder.quadratic_bezier_to(point(ctrl.x, ctrl.y), point(end.x, end.y));

            // Bottom edge to bottom-left corner
            let p = transform.transform_point(Vec2::new(r, height));
            builder.line_to(point(p.x, p.y));

            // Bottom-left corner
            let ctrl = transform.transform_point(Vec2::new(0.0, height));
            let end = transform.transform_point(Vec2::new(0.0, height - r));
            builder.quadratic_bezier_to(point(ctrl.x, ctrl.y), point(end.x, end.y));

            // Left edge to top-left corner
            let p = transform.transform_point(Vec2::new(0.0, r));
            builder.line_to(point(p.x, p.y));

            // Top-left corner
            let ctrl = transform.transform_point(Vec2::new(0.0, 0.0));
            let end = transform.transform_point(Vec2::new(r, 0.0));
            builder.quadratic_bezier_to(point(ctrl.x, ctrl.y), point(end.x, end.y));

            builder.close();
        }

        let path = builder.build();
        let mut buffers: VertexBuffers<Vertex, u32> = VertexBuffers::new();
        let color_arr = color.to_array();

        let result = self.fill_tessellator.tessellate_path(
            &path,
            &FillOptions::tolerance(TESSELLATION_TOLERANCE),
            &mut BuffersBuilder::new(&mut buffers, |vertex: FillVertex| Vertex {
                position: [vertex.position().x, vertex.position().y],
                color: color_arr,
            }),
        );

        if result.is_ok() {
            Some(Mesh {
                vertices: buffers.vertices,
                indices: buffers.indices,
            })
        } else {
            None
        }
    }

    /// Tessellate a rectangle stroke
    fn tessellate_rectangle_stroke(
        &mut self,
        width: f32,
        height: f32,
        corner_radius: f32,
        transform: &Transform2D,
        color: Color,
        stroke_width: f32,
    ) -> Option<Mesh> {
        // Reuse fill path building logic
        let mut builder = Path::builder();

        if corner_radius <= 0.0 {
            let corners = [
                Vec2::new(0.0, 0.0),
                Vec2::new(width, 0.0),
                Vec2::new(width, height),
                Vec2::new(0.0, height),
            ];
            let first = transform.transform_point(corners[0]);
            builder.begin(point(first.x, first.y));
            for corner in &corners[1..] {
                let transformed = transform.transform_point(*corner);
                builder.line_to(point(transformed.x, transformed.y));
            }
            builder.close();
        } else {
            let r = corner_radius.min(width / 2.0).min(height / 2.0);
            let start = transform.transform_point(Vec2::new(r, 0.0));
            builder.begin(point(start.x, start.y));

            let p = transform.transform_point(Vec2::new(width - r, 0.0));
            builder.line_to(point(p.x, p.y));
            let ctrl = transform.transform_point(Vec2::new(width, 0.0));
            let end = transform.transform_point(Vec2::new(width, r));
            builder.quadratic_bezier_to(point(ctrl.x, ctrl.y), point(end.x, end.y));

            let p = transform.transform_point(Vec2::new(width, height - r));
            builder.line_to(point(p.x, p.y));
            let ctrl = transform.transform_point(Vec2::new(width, height));
            let end = transform.transform_point(Vec2::new(width - r, height));
            builder.quadratic_bezier_to(point(ctrl.x, ctrl.y), point(end.x, end.y));

            let p = transform.transform_point(Vec2::new(r, height));
            builder.line_to(point(p.x, p.y));
            let ctrl = transform.transform_point(Vec2::new(0.0, height));
            let end = transform.transform_point(Vec2::new(0.0, height - r));
            builder.quadratic_bezier_to(point(ctrl.x, ctrl.y), point(end.x, end.y));

            let p = transform.transform_point(Vec2::new(0.0, r));
            builder.line_to(point(p.x, p.y));
            let ctrl = transform.transform_point(Vec2::new(0.0, 0.0));
            let end = transform.transform_point(Vec2::new(r, 0.0));
            builder.quadratic_bezier_to(point(ctrl.x, ctrl.y), point(end.x, end.y));

            builder.close();
        }

        let path = builder.build();
        let mut buffers: VertexBuffers<Vertex, u32> = VertexBuffers::new();
        let color_arr = color.to_array();

        let result = self.stroke_tessellator.tessellate_path(
            &path,
            &StrokeOptions::tolerance(TESSELLATION_TOLERANCE).with_line_width(stroke_width),
            &mut BuffersBuilder::new(&mut buffers, |vertex: StrokeVertex| Vertex {
                position: [vertex.position().x, vertex.position().y],
                color: color_arr,
            }),
        );

        if result.is_ok() {
            Some(Mesh {
                vertices: buffers.vertices,
                indices: buffers.indices,
            })
        } else {
            None
        }
    }

    /// Tessellate an ellipse fill
    fn tessellate_ellipse_fill(
        &mut self,
        rx: f32,
        ry: f32,
        transform: &Transform2D,
        color: Color,
    ) -> Option<Mesh> {
        // Approximate ellipse with bezier curves
        // Using 4 cubic bezier curves for a good approximation
        let k = 0.5522847498; // Magic number for circular arcs
        let kx = rx * k;
        let ky = ry * k;

        let mut builder = Path::builder();

        // Start at right
        let start = transform.transform_point(Vec2::new(rx, 0.0));
        builder.begin(point(start.x, start.y));

        // Right to bottom
        let ctrl1 = transform.transform_point(Vec2::new(rx, ky));
        let ctrl2 = transform.transform_point(Vec2::new(kx, ry));
        let end = transform.transform_point(Vec2::new(0.0, ry));
        builder.cubic_bezier_to(
            point(ctrl1.x, ctrl1.y),
            point(ctrl2.x, ctrl2.y),
            point(end.x, end.y),
        );

        // Bottom to left
        let ctrl1 = transform.transform_point(Vec2::new(-kx, ry));
        let ctrl2 = transform.transform_point(Vec2::new(-rx, ky));
        let end = transform.transform_point(Vec2::new(-rx, 0.0));
        builder.cubic_bezier_to(
            point(ctrl1.x, ctrl1.y),
            point(ctrl2.x, ctrl2.y),
            point(end.x, end.y),
        );

        // Left to top
        let ctrl1 = transform.transform_point(Vec2::new(-rx, -ky));
        let ctrl2 = transform.transform_point(Vec2::new(-kx, -ry));
        let end = transform.transform_point(Vec2::new(0.0, -ry));
        builder.cubic_bezier_to(
            point(ctrl1.x, ctrl1.y),
            point(ctrl2.x, ctrl2.y),
            point(end.x, end.y),
        );

        // Top to right
        let ctrl1 = transform.transform_point(Vec2::new(kx, -ry));
        let ctrl2 = transform.transform_point(Vec2::new(rx, -ky));
        let end = transform.transform_point(Vec2::new(rx, 0.0));
        builder.cubic_bezier_to(
            point(ctrl1.x, ctrl1.y),
            point(ctrl2.x, ctrl2.y),
            point(end.x, end.y),
        );

        builder.close();
        let path = builder.build();

        let mut buffers: VertexBuffers<Vertex, u32> = VertexBuffers::new();
        let color_arr = color.to_array();

        let result = self.fill_tessellator.tessellate_path(
            &path,
            &FillOptions::tolerance(TESSELLATION_TOLERANCE),
            &mut BuffersBuilder::new(&mut buffers, |vertex: FillVertex| Vertex {
                position: [vertex.position().x, vertex.position().y],
                color: color_arr,
            }),
        );

        if result.is_ok() {
            Some(Mesh {
                vertices: buffers.vertices,
                indices: buffers.indices,
            })
        } else {
            None
        }
    }

    /// Tessellate an ellipse stroke
    fn tessellate_ellipse_stroke(
        &mut self,
        rx: f32,
        ry: f32,
        transform: &Transform2D,
        color: Color,
        width: f32,
    ) -> Option<Mesh> {
        let k = 0.5522847498;
        let kx = rx * k;
        let ky = ry * k;

        let mut builder = Path::builder();

        let start = transform.transform_point(Vec2::new(rx, 0.0));
        builder.begin(point(start.x, start.y));

        let ctrl1 = transform.transform_point(Vec2::new(rx, ky));
        let ctrl2 = transform.transform_point(Vec2::new(kx, ry));
        let end = transform.transform_point(Vec2::new(0.0, ry));
        builder.cubic_bezier_to(
            point(ctrl1.x, ctrl1.y),
            point(ctrl2.x, ctrl2.y),
            point(end.x, end.y),
        );

        let ctrl1 = transform.transform_point(Vec2::new(-kx, ry));
        let ctrl2 = transform.transform_point(Vec2::new(-rx, ky));
        let end = transform.transform_point(Vec2::new(-rx, 0.0));
        builder.cubic_bezier_to(
            point(ctrl1.x, ctrl1.y),
            point(ctrl2.x, ctrl2.y),
            point(end.x, end.y),
        );

        let ctrl1 = transform.transform_point(Vec2::new(-rx, -ky));
        let ctrl2 = transform.transform_point(Vec2::new(-kx, -ry));
        let end = transform.transform_point(Vec2::new(0.0, -ry));
        builder.cubic_bezier_to(
            point(ctrl1.x, ctrl1.y),
            point(ctrl2.x, ctrl2.y),
            point(end.x, end.y),
        );

        let ctrl1 = transform.transform_point(Vec2::new(kx, -ry));
        let ctrl2 = transform.transform_point(Vec2::new(rx, -ky));
        let end = transform.transform_point(Vec2::new(rx, 0.0));
        builder.cubic_bezier_to(
            point(ctrl1.x, ctrl1.y),
            point(ctrl2.x, ctrl2.y),
            point(end.x, end.y),
        );

        builder.close();
        let path = builder.build();

        let mut buffers: VertexBuffers<Vertex, u32> = VertexBuffers::new();
        let color_arr = color.to_array();

        let result = self.stroke_tessellator.tessellate_path(
            &path,
            &StrokeOptions::tolerance(TESSELLATION_TOLERANCE).with_line_width(width),
            &mut BuffersBuilder::new(&mut buffers, |vertex: StrokeVertex| Vertex {
                position: [vertex.position().x, vertex.position().y],
                color: color_arr,
            }),
        );

        if result.is_ok() {
            Some(Mesh {
                vertices: buffers.vertices,
                indices: buffers.indices,
            })
        } else {
            None
        }
    }

    /// Tessellate a path fill
    fn tessellate_path_fill(
        &mut self,
        commands: &[crate::scene::PathCommand],
        transform: &Transform2D,
        color: Color,
    ) -> Option<Mesh> {
        use crate::scene::PathCommand;

        if commands.is_empty() {
            return None;
        }

        let mut builder = Path::builder();
        let mut started = false;
        let mut current_pos = Vec2::ZERO;

        for cmd in commands {
            match cmd {
                PathCommand::MoveTo(p) => {
                    if started {
                        builder.end(false);
                    }
                    let tp = transform.transform_point(*p);
                    builder.begin(point(tp.x, tp.y));
                    started = true;
                    current_pos = *p;
                }
                PathCommand::LineTo(p) => {
                    if started {
                        let tp = transform.transform_point(*p);
                        builder.line_to(point(tp.x, tp.y));
                        current_pos = *p;
                    }
                }
                PathCommand::QuadraticTo { control, to } => {
                    if started {
                        let ctrl = transform.transform_point(*control);
                        let end = transform.transform_point(*to);
                        builder.quadratic_bezier_to(point(ctrl.x, ctrl.y), point(end.x, end.y));
                        current_pos = *to;
                    }
                }
                PathCommand::CubicTo { ctrl1, ctrl2, to } => {
                    if started {
                        let c1 = transform.transform_point(*ctrl1);
                        let c2 = transform.transform_point(*ctrl2);
                        let end = transform.transform_point(*to);
                        builder.cubic_bezier_to(
                            point(c1.x, c1.y),
                            point(c2.x, c2.y),
                            point(end.x, end.y),
                        );
                        current_pos = *to;
                    }
                }
                PathCommand::ArcTo { rx, ry, x_rotation, large_arc, sweep, to } => {
                    if started {
                        // Convert arc to bezier curves
                        let beziers = arc_to_beziers(current_pos, *rx, *ry, *x_rotation, *large_arc, *sweep, *to);
                        if beziers.is_empty() {
                            // Degenerate arc, draw line instead
                            let tp = transform.transform_point(*to);
                            builder.line_to(point(tp.x, tp.y));
                        } else {
                            for (ctrl1, ctrl2, end) in beziers {
                                let c1 = transform.transform_point(ctrl1);
                                let c2 = transform.transform_point(ctrl2);
                                let e = transform.transform_point(end);
                                builder.cubic_bezier_to(
                                    point(c1.x, c1.y),
                                    point(c2.x, c2.y),
                                    point(e.x, e.y),
                                );
                            }
                        }
                        current_pos = *to;
                    }
                }
                PathCommand::Close => {
                    if started {
                        builder.close();
                        started = false;
                    }
                }
            }
        }

        if started {
            builder.end(false);
        }

        let path = builder.build();
        let mut buffers: VertexBuffers<Vertex, u32> = VertexBuffers::new();
        let color_arr = color.to_array();

        let result = self.fill_tessellator.tessellate_path(
            &path,
            &FillOptions::tolerance(TESSELLATION_TOLERANCE),
            &mut BuffersBuilder::new(&mut buffers, |vertex: FillVertex| Vertex {
                position: [vertex.position().x, vertex.position().y],
                color: color_arr,
            }),
        );

        if result.is_ok() && !buffers.vertices.is_empty() {
            Some(Mesh {
                vertices: buffers.vertices,
                indices: buffers.indices,
            })
        } else {
            None
        }
    }

    /// Tessellate a path stroke
    fn tessellate_path_stroke(
        &mut self,
        commands: &[crate::scene::PathCommand],
        transform: &Transform2D,
        color: Color,
        width: f32,
    ) -> Option<Mesh> {
        use crate::scene::PathCommand;

        if commands.is_empty() {
            return None;
        }

        let mut builder = Path::builder();
        let mut started = false;
        let mut current_pos = Vec2::ZERO;

        for cmd in commands {
            match cmd {
                PathCommand::MoveTo(p) => {
                    if started {
                        builder.end(false);
                    }
                    let tp = transform.transform_point(*p);
                    builder.begin(point(tp.x, tp.y));
                    started = true;
                    current_pos = *p;
                }
                PathCommand::LineTo(p) => {
                    if started {
                        let tp = transform.transform_point(*p);
                        builder.line_to(point(tp.x, tp.y));
                        current_pos = *p;
                    }
                }
                PathCommand::QuadraticTo { control, to } => {
                    if started {
                        let ctrl = transform.transform_point(*control);
                        let end = transform.transform_point(*to);
                        builder.quadratic_bezier_to(point(ctrl.x, ctrl.y), point(end.x, end.y));
                        current_pos = *to;
                    }
                }
                PathCommand::CubicTo { ctrl1, ctrl2, to } => {
                    if started {
                        let c1 = transform.transform_point(*ctrl1);
                        let c2 = transform.transform_point(*ctrl2);
                        let end = transform.transform_point(*to);
                        builder.cubic_bezier_to(
                            point(c1.x, c1.y),
                            point(c2.x, c2.y),
                            point(end.x, end.y),
                        );
                        current_pos = *to;
                    }
                }
                PathCommand::ArcTo { rx, ry, x_rotation, large_arc, sweep, to } => {
                    if started {
                        // Convert arc to bezier curves
                        let beziers = arc_to_beziers(current_pos, *rx, *ry, *x_rotation, *large_arc, *sweep, *to);
                        if beziers.is_empty() {
                            // Degenerate arc, draw line instead
                            let tp = transform.transform_point(*to);
                            builder.line_to(point(tp.x, tp.y));
                        } else {
                            for (ctrl1, ctrl2, end) in beziers {
                                let c1 = transform.transform_point(ctrl1);
                                let c2 = transform.transform_point(ctrl2);
                                let e = transform.transform_point(end);
                                builder.cubic_bezier_to(
                                    point(c1.x, c1.y),
                                    point(c2.x, c2.y),
                                    point(e.x, e.y),
                                );
                            }
                        }
                        current_pos = *to;
                    }
                }
                PathCommand::Close => {
                    if started {
                        builder.close();
                        started = false;
                    }
                }
            }
        }

        if started {
            builder.end(false);
        }

        let path = builder.build();
        let mut buffers: VertexBuffers<Vertex, u32> = VertexBuffers::new();
        let color_arr = color.to_array();

        let result = self.stroke_tessellator.tessellate_path(
            &path,
            &StrokeOptions::tolerance(TESSELLATION_TOLERANCE).with_line_width(width),
            &mut BuffersBuilder::new(&mut buffers, |vertex: StrokeVertex| Vertex {
                position: [vertex.position().x, vertex.position().y],
                color: color_arr,
            }),
        );

        if result.is_ok() && !buffers.vertices.is_empty() {
            Some(Mesh {
                vertices: buffers.vertices,
                indices: buffers.indices,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{ShapeStyle, StrokeStyle};

    #[test]
    fn test_tessellate_triangle() {
        let mut tessellator = Tessellator::new();
        let shape = Shape::new(
            ShapeGeometry::polygon(vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(100.0, 0.0),
                Vec2::new(50.0, 100.0),
            ]),
            ShapeStyle::fill_only(Color::rgb(1.0, 0.0, 0.0)),
        );

        let mesh = tessellator.tessellate_shape(&shape);
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.indices.len() % 3, 0); // Should be triangles
    }

    #[test]
    fn test_tessellate_rectangle() {
        let mut tessellator = Tessellator::new();
        let shape = Shape::new(
            ShapeGeometry::rectangle(100.0, 50.0),
            ShapeStyle::fill_only(Color::rgb(0.0, 1.0, 0.0)),
        );

        let mesh = tessellator.tessellate_shape(&shape);
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
    }

    #[test]
    fn test_tessellate_ellipse() {
        let mut tessellator = Tessellator::new();
        let shape = Shape::new(
            ShapeGeometry::ellipse(50.0, 30.0),
            ShapeStyle::fill_only(Color::rgb(0.0, 0.0, 1.0)),
        );

        let mesh = tessellator.tessellate_shape(&shape);
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
    }

    #[test]
    fn test_tessellate_with_stroke() {
        let mut tessellator = Tessellator::new();
        let shape = Shape::new(
            ShapeGeometry::rectangle(100.0, 50.0),
            ShapeStyle::fill_and_stroke(
                Color::rgb(1.0, 0.0, 0.0),
                StrokeStyle::new(Color::black(), 2.0),
            ),
        );

        let mesh = tessellator.tessellate_shape(&shape);
        assert!(!mesh.vertices.is_empty());
        // Should have both fill and stroke vertices
    }
}
