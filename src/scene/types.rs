use bytemuck::{Pod, Zeroable};
pub use glam::Vec2;

/// RGBA color with f32 components (0.0 - 1.0)
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn white() -> Self {
        Self::rgb(1.0, 1.0, 1.0)
    }

    pub const fn black() -> Self {
        Self::rgb(0.0, 0.0, 0.0)
    }

    pub const fn transparent() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    /// Parse a hex color string (e.g., "#ef4444" or "ef4444")
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }

        let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;

        Some(Self::rgb(r, g, b))
    }

    /// Convert to hex string (e.g., "#ef4444")
    pub fn to_hex(&self) -> String {
        let r = (self.r * 255.0).round() as u8;
        let g = (self.g * 255.0).round() as u8;
        let b = (self.b * 255.0).round() as u8;
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    }

    /// Convert to array for GPU upload
    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::black()
    }
}

/// 2D transform with position, scale, rotation, and anchor point
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform2D {
    pub position: Vec2,
    pub scale: Vec2,
    pub rotation: f32, // radians
    pub anchor: Vec2,  // transform origin (local coordinates)
}

impl Transform2D {
    pub fn new(position: Vec2, scale: Vec2, rotation: f32, anchor: Vec2) -> Self {
        Self {
            position,
            scale,
            rotation,
            anchor,
        }
    }

    pub fn identity() -> Self {
        Self {
            position: Vec2::ZERO,
            scale: Vec2::ONE,
            rotation: 0.0,
            anchor: Vec2::ZERO,
        }
    }

    pub fn from_position(position: Vec2) -> Self {
        Self {
            position,
            ..Self::identity()
        }
    }

    /// Builder method to set position
    pub fn with_position(mut self, position: Vec2) -> Self {
        self.position = position;
        self
    }

    /// Builder method to set scale
    pub fn with_scale(mut self, scale: Vec2) -> Self {
        self.scale = scale;
        self
    }

    /// Builder method to set rotation (in radians)
    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Builder method to set anchor point
    pub fn with_anchor(mut self, anchor: Vec2) -> Self {
        self.anchor = anchor;
        self
    }

    /// Apply this transform to a point
    pub fn transform_point(&self, point: Vec2) -> Vec2 {
        // Translate to anchor, scale, rotate, translate back, then apply position
        let p = point - self.anchor;
        let scaled = p * self.scale;
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();
        let rotated = Vec2::new(
            scaled.x * cos_r - scaled.y * sin_r,
            scaled.x * sin_r + scaled.y * cos_r,
        );
        rotated + self.anchor + self.position
    }

    /// Get the 3x3 transformation matrix (as 4x4 for GPU compatibility)
    pub fn to_matrix(&self) -> glam::Mat4 {
        let translation = glam::Mat4::from_translation(glam::Vec3::new(
            self.position.x + self.anchor.x,
            self.position.y + self.anchor.y,
            0.0,
        ));
        let rotation = glam::Mat4::from_rotation_z(self.rotation);
        let scale = glam::Mat4::from_scale(glam::Vec3::new(self.scale.x, self.scale.y, 1.0));
        let anchor_offset = glam::Mat4::from_translation(glam::Vec3::new(-self.anchor.x, -self.anchor.y, 0.0));

        translation * rotation * scale * anchor_offset
    }

    /// Get the transformation matrix as a raw array for GPU uniform buffers
    pub fn to_matrix4(&self) -> [[f32; 4]; 4] {
        self.to_matrix().to_cols_array_2d()
    }
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::identity()
    }
}

/// Stroke styling for shape outlines
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeStyle {
    pub color: Color,
    pub width: f32,
}

impl StrokeStyle {
    pub fn new(color: Color, width: f32) -> Self {
        Self { color, width }
    }
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            color: Color::black(),
            width: 1.0,
        }
    }
}

/// Complete styling for a shape (fill and/or stroke)
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ShapeStyle {
    pub fill: Option<Color>,
    pub stroke: Option<StrokeStyle>,
}

impl ShapeStyle {
    pub fn new(fill: Option<Color>, stroke: Option<StrokeStyle>) -> Self {
        Self { fill, stroke }
    }

    pub fn fill_only(color: Color) -> Self {
        Self {
            fill: Some(color),
            stroke: None,
        }
    }

    pub fn stroke_only(stroke: StrokeStyle) -> Self {
        Self {
            fill: None,
            stroke: Some(stroke),
        }
    }

    pub fn fill_and_stroke(fill: Color, stroke: StrokeStyle) -> Self {
        Self {
            fill: Some(fill),
            stroke: Some(stroke),
        }
    }
}

/// Axis-aligned bounding box using Vec2
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BBox {
    pub min: Vec2,
    pub max: Vec2,
}

impl BBox {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    pub fn from_points(points: &[Vec2]) -> Option<Self> {
        if points.is_empty() {
            return None;
        }

        let mut min = points[0];
        let mut max = points[0];

        for &p in &points[1..] {
            min = min.min(p);
            max = max.max(p);
        }

        Some(Self { min, max })
    }

    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    pub fn intersects(&self, other: &BBox) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    /// Expand to include another bounding box
    pub fn union(&self, other: &BBox) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Expand by a margin
    pub fn expand(&self, margin: f32) -> Self {
        Self {
            min: self.min - Vec2::splat(margin),
            max: self.max + Vec2::splat(margin),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_hex_parsing() {
        let color = Color::from_hex("#ef4444").unwrap();
        assert!((color.r - 0.937).abs() < 0.01);
        assert!((color.g - 0.267).abs() < 0.01);
        assert!((color.b - 0.267).abs() < 0.01);
    }

    #[test]
    fn test_color_hex_roundtrip() {
        let original = "#3b82f6";
        let color = Color::from_hex(original).unwrap();
        let hex = color.to_hex();
        assert_eq!(hex, original);
    }

    #[test]
    fn test_transform_identity() {
        let t = Transform2D::identity();
        let point = Vec2::new(10.0, 20.0);
        let transformed = t.transform_point(point);
        assert_eq!(transformed, point);
    }

    #[test]
    fn test_transform_translation() {
        let t = Transform2D::from_position(Vec2::new(5.0, 10.0));
        let point = Vec2::new(10.0, 20.0);
        let transformed = t.transform_point(point);
        assert_eq!(transformed, Vec2::new(15.0, 30.0));
    }

    #[test]
    fn test_bbox_from_points() {
        let points = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 5.0),
            Vec2::new(5.0, 15.0),
        ];
        let bbox = BBox::from_points(&points).unwrap();
        assert_eq!(bbox.min, Vec2::new(0.0, 0.0));
        assert_eq!(bbox.max, Vec2::new(10.0, 15.0));
    }
}
