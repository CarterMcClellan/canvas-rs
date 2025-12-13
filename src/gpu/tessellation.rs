use crate::gpu::vertex::{Mesh, Vertex};
use crate::scene::{Color, Shape, ShapeGeometry, Transform2D, Vec2};
use lyon::geom::point;
use lyon::path::Path;
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers,
};

/// Tessellator for converting shapes to GPU-renderable triangles
pub struct Tessellator {
    fill_tessellator: FillTessellator,
    stroke_tessellator: StrokeTessellator,
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
        }
    }

    /// Tessellate a shape into a mesh
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

    /// Tessellate multiple shapes into a single mesh
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
            &FillOptions::default(),
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
            &StrokeOptions::default().with_line_width(width),
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
            &FillOptions::default(),
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
            &StrokeOptions::default().with_line_width(stroke_width),
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
            &FillOptions::default(),
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
            &StrokeOptions::default().with_line_width(width),
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

        for cmd in commands {
            match cmd {
                PathCommand::MoveTo(p) => {
                    if started {
                        builder.end(false);
                    }
                    let tp = transform.transform_point(*p);
                    builder.begin(point(tp.x, tp.y));
                    started = true;
                }
                PathCommand::LineTo(p) => {
                    if started {
                        let tp = transform.transform_point(*p);
                        builder.line_to(point(tp.x, tp.y));
                    }
                }
                PathCommand::QuadraticTo { control, to } => {
                    if started {
                        let ctrl = transform.transform_point(*control);
                        let end = transform.transform_point(*to);
                        builder.quadratic_bezier_to(point(ctrl.x, ctrl.y), point(end.x, end.y));
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
            &FillOptions::default(),
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

        for cmd in commands {
            match cmd {
                PathCommand::MoveTo(p) => {
                    if started {
                        builder.end(false);
                    }
                    let tp = transform.transform_point(*p);
                    builder.begin(point(tp.x, tp.y));
                    started = true;
                }
                PathCommand::LineTo(p) => {
                    if started {
                        let tp = transform.transform_point(*p);
                        builder.line_to(point(tp.x, tp.y));
                    }
                }
                PathCommand::QuadraticTo { control, to } => {
                    if started {
                        let ctrl = transform.transform_point(*control);
                        let end = transform.transform_point(*to);
                        builder.quadratic_bezier_to(point(ctrl.x, ctrl.y), point(end.x, end.y));
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
            &StrokeOptions::default().with_line_width(width),
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
