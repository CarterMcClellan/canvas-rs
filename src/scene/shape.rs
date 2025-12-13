use super::types::{BBox, Color, ShapeStyle, StrokeStyle, Transform2D, Vec2};
use crate::types::Polygon;
use std::sync::atomic::{AtomicU64, Ordering};

/// Global shape ID counter
static NEXT_SHAPE_ID: AtomicU64 = AtomicU64::new(1);

fn generate_shape_id() -> u64 {
    NEXT_SHAPE_ID.fetch_add(1, Ordering::Relaxed)
}

/// Path command for arbitrary vector paths
#[derive(Clone, Debug, PartialEq)]
pub enum PathCommand {
    MoveTo(Vec2),
    LineTo(Vec2),
    QuadraticTo { control: Vec2, to: Vec2 },
    CubicTo { ctrl1: Vec2, ctrl2: Vec2, to: Vec2 },
    Close,
}

/// Geometry definition for different shape types
#[derive(Clone, Debug, PartialEq)]
pub enum ShapeGeometry {
    /// Polygon defined by a series of points
    Polygon { points: Vec<Vec2> },

    /// Rectangle with optional corner radius
    Rectangle {
        width: f32,
        height: f32,
        corner_radius: f32,
    },

    /// Ellipse defined by x and y radii
    Ellipse { rx: f32, ry: f32 },

    /// Arbitrary vector path
    Path { commands: Vec<PathCommand> },
}

impl ShapeGeometry {
    /// Create a polygon from points
    pub fn polygon(points: Vec<Vec2>) -> Self {
        Self::Polygon { points }
    }

    /// Create a rectangle
    pub fn rectangle(width: f32, height: f32) -> Self {
        Self::Rectangle {
            width,
            height,
            corner_radius: 0.0,
        }
    }

    /// Create a rounded rectangle
    pub fn rounded_rectangle(width: f32, height: f32, corner_radius: f32) -> Self {
        Self::Rectangle {
            width,
            height,
            corner_radius,
        }
    }

    /// Create an ellipse
    pub fn ellipse(rx: f32, ry: f32) -> Self {
        Self::Ellipse { rx, ry }
    }

    /// Create a circle (ellipse with equal radii)
    pub fn circle(radius: f32) -> Self {
        Self::Ellipse {
            rx: radius,
            ry: radius,
        }
    }

    /// Get the local bounding box (before transform)
    pub fn local_bounds(&self) -> BBox {
        match self {
            ShapeGeometry::Polygon { points } => {
                BBox::from_points(points).unwrap_or(BBox::new(Vec2::ZERO, Vec2::ZERO))
            }
            ShapeGeometry::Rectangle { width, height, .. } => {
                BBox::new(Vec2::ZERO, Vec2::new(*width, *height))
            }
            ShapeGeometry::Ellipse { rx, ry } => BBox::new(Vec2::new(-*rx, -*ry), Vec2::new(*rx, *ry)),
            ShapeGeometry::Path { commands } => {
                let points: Vec<Vec2> = commands
                    .iter()
                    .filter_map(|cmd| match cmd {
                        PathCommand::MoveTo(p) => Some(*p),
                        PathCommand::LineTo(p) => Some(*p),
                        PathCommand::QuadraticTo { to, .. } => Some(*to),
                        PathCommand::CubicTo { to, .. } => Some(*to),
                        PathCommand::Close => None,
                    })
                    .collect();
                BBox::from_points(&points).unwrap_or(BBox::new(Vec2::ZERO, Vec2::ZERO))
            }
        }
    }

    /// Get the points for polygon geometry (for compatibility)
    pub fn polygon_points(&self) -> Option<&[Vec2]> {
        match self {
            ShapeGeometry::Polygon { points } => Some(points),
            _ => None,
        }
    }
}

/// A shape in the scene graph
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    /// Unique identifier
    pub id: u64,

    /// The geometry of this shape
    pub geometry: ShapeGeometry,

    /// Transform applied to this shape
    pub transform: Transform2D,

    /// Visual style (fill and stroke)
    pub style: ShapeStyle,

    /// Whether this shape needs to be re-tessellated
    pub dirty: bool,
}

impl Shape {
    /// Create a new shape with auto-generated ID
    pub fn new(geometry: ShapeGeometry, style: ShapeStyle) -> Self {
        Self {
            id: generate_shape_id(),
            geometry,
            transform: Transform2D::identity(),
            style,
            dirty: true,
        }
    }

    /// Create a new shape with a specific ID
    pub fn with_id(id: u64, geometry: ShapeGeometry, style: ShapeStyle) -> Self {
        Self {
            id,
            geometry,
            transform: Transform2D::identity(),
            style,
            dirty: true,
        }
    }

    /// Set the transform
    pub fn with_transform(mut self, transform: Transform2D) -> Self {
        self.transform = transform;
        self
    }

    /// Get the world-space bounding box
    pub fn world_bounds(&self) -> BBox {
        let local = self.geometry.local_bounds();

        // Transform the corners of the local bounding box
        let corners = [
            self.transform.transform_point(local.min),
            self.transform
                .transform_point(Vec2::new(local.max.x, local.min.y)),
            self.transform.transform_point(local.max),
            self.transform
                .transform_point(Vec2::new(local.min.x, local.max.y)),
        ];

        BBox::from_points(&corners).unwrap()
    }

    /// Mark this shape as needing re-tessellation
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Clear the dirty flag
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Check if a point (in world coordinates) is inside this shape
    pub fn contains_point(&self, point: Vec2) -> bool {
        // Quick bounding box check first
        if !self.world_bounds().contains(point) {
            return false;
        }

        // For now, use bounding box hit testing
        // TODO: Implement proper point-in-polygon test
        true
    }
}

/// Convert from the old string-based Polygon type
impl From<&Polygon> for Shape {
    fn from(polygon: &Polygon) -> Self {
        let points = parse_svg_points(&polygon.points);
        let geometry = ShapeGeometry::Polygon { points };

        let fill = Color::from_hex(&polygon.fill);
        let stroke = Color::from_hex(&polygon.stroke);

        let style = ShapeStyle {
            fill,
            stroke: stroke.map(|color| StrokeStyle::new(color, polygon.stroke_width as f32)),
        };

        Shape::new(geometry, style)
    }
}

/// Convert back to the old Polygon type (for compatibility during migration)
impl From<&Shape> for Option<Polygon> {
    fn from(shape: &Shape) -> Self {
        match &shape.geometry {
            ShapeGeometry::Polygon { points } => {
                let points_str = stringify_points(points, &shape.transform);
                let fill = shape
                    .style
                    .fill
                    .map(|c| c.to_hex())
                    .unwrap_or_else(|| "#000000".to_string());
                let stroke = shape
                    .style
                    .stroke
                    .map(|s| s.color.to_hex())
                    .unwrap_or_else(|| "#000000".to_string());
                let stroke_width = shape.style.stroke.map(|s| s.width as f64).unwrap_or(1.0);

                Some(Polygon::new(points_str, fill, stroke, stroke_width))
            }
            _ => None, // Other geometry types can't convert to Polygon
        }
    }
}

/// Parse SVG-style point string to Vec2 array
/// Input format: "x1,y1 x2,y2 x3,y3"
pub fn parse_svg_points(points_str: &str) -> Vec<Vec2> {
    points_str
        .split_whitespace()
        .filter_map(|pair| {
            let mut coords = pair.split(',');
            let x = coords.next()?.parse::<f32>().ok()?;
            let y = coords.next()?.parse::<f32>().ok()?;
            Some(Vec2::new(x, y))
        })
        .collect()
}

/// Convert Vec2 points to SVG-style string, applying transform
pub fn stringify_points(points: &[Vec2], transform: &Transform2D) -> String {
    points
        .iter()
        .map(|p| {
            let transformed = transform.transform_point(*p);
            format!("{},{}", transformed.x.round(), transformed.y.round())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_svg_points() {
        let points = parse_svg_points("230,220 260,220 245,250");
        assert_eq!(points.len(), 3);
        assert_eq!(points[0], Vec2::new(230.0, 220.0));
        assert_eq!(points[1], Vec2::new(260.0, 220.0));
        assert_eq!(points[2], Vec2::new(245.0, 250.0));
    }

    #[test]
    fn test_stringify_points() {
        let points = vec![
            Vec2::new(230.0, 220.0),
            Vec2::new(260.0, 220.0),
            Vec2::new(245.0, 250.0),
        ];
        let transform = Transform2D::identity();
        let result = stringify_points(&points, &transform);
        assert_eq!(result, "230,220 260,220 245,250");
    }

    #[test]
    fn test_polygon_local_bounds() {
        let geometry = ShapeGeometry::polygon(vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(30.0, 0.0),
            Vec2::new(15.0, 30.0),
        ]);
        let bounds = geometry.local_bounds();
        assert_eq!(bounds.min, Vec2::new(0.0, 0.0));
        assert_eq!(bounds.max, Vec2::new(30.0, 30.0));
    }

    #[test]
    fn test_rectangle_local_bounds() {
        let geometry = ShapeGeometry::rectangle(100.0, 50.0);
        let bounds = geometry.local_bounds();
        assert_eq!(bounds.min, Vec2::new(0.0, 0.0));
        assert_eq!(bounds.max, Vec2::new(100.0, 50.0));
    }

    #[test]
    fn test_ellipse_local_bounds() {
        let geometry = ShapeGeometry::ellipse(20.0, 10.0);
        let bounds = geometry.local_bounds();
        assert_eq!(bounds.min, Vec2::new(-20.0, -10.0));
        assert_eq!(bounds.max, Vec2::new(20.0, 10.0));
    }
}
