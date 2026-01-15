use crate::types::Point;
use web_sys::{MouseEvent, SvgsvgElement};

pub fn client_to_svg_coords(event: &MouseEvent, svg_element: &SvgsvgElement) -> Point {
    // Get the bounding rectangle of the SVG element
    let rect = svg_element.get_bounding_client_rect();

    // Calculate SVG coordinates by subtracting the SVG's position from the event coordinates
    let x = event.client_x() as f64 - rect.left();
    let y = event.client_y() as f64 - rect.top();

    Point::new(x, y)
}

use crate::scene::{Shape, Vec2};

/// Find the ID of the topmost shape that contains the given point
/// Returns None if no shape contains the point
pub fn find_shape_at_point(shapes: &[Shape], point: &Point) -> Option<u64> {
    let vec2_point = Vec2::new(point.x as f32, point.y as f32);
    // Iterate in reverse to get topmost (last rendered) shape first
    for shape in shapes.iter().rev() {
        if shape.contains_point(vec2_point) {
            return Some(shape.id);
        }
    }
    None
}
