use crate::types::{BoundingBox, Point, Polygon};
use web_sys::{MouseEvent, SvgsvgElement};

pub fn parse_points(points_string: &str) -> Vec<Point> {
    points_string
        .split_whitespace()
        .filter_map(|pair| {
            let coords: Vec<f64> = pair.split(',').filter_map(|s| s.parse().ok()).collect();
            if coords.len() == 2 {
                Some(Point::new(coords[0], coords[1]))
            } else {
                None
            }
        })
        .collect()
}

pub fn stringify_points(points: &[Point]) -> String {
    points
        .iter()
        .map(|p| format!("{},{}", p.x, p.y))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn client_to_svg_coords(event: &MouseEvent, svg_element: &SvgsvgElement) -> Point {
    // Get the bounding rectangle of the SVG element
    let rect = svg_element.get_bounding_client_rect();

    // Calculate SVG coordinates by subtracting the SVG's position from the event coordinates
    let x = event.client_x() as f64 - rect.left();
    let y = event.client_y() as f64 - rect.top();

    Point::new(x, y)
}

pub fn calculate_bounding_box(polygons: &[Polygon]) -> BoundingBox {
    let all_points: Vec<Point> = polygons
        .iter()
        .flat_map(|p| parse_points(&p.points))
        .collect();

    if all_points.is_empty() {
        return BoundingBox::new(0.0, 0.0, 0.0, 0.0);
    }

    let xs: Vec<f64> = all_points.iter().map(|p| p.x).collect();
    let ys: Vec<f64> = all_points.iter().map(|p| p.y).collect();

    let min_x = xs.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_x = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_y = ys.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_y = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    BoundingBox::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

pub fn polygons_intersect_rect(polygon: &Polygon, rect: &BoundingBox) -> bool {
    let points = parse_points(&polygon.points);

    // Check if any polygon point is inside the rectangle
    for point in &points {
        if point.x >= rect.x
            && point.x <= rect.x + rect.width
            && point.y >= rect.y
            && point.y <= rect.y + rect.height
        {
            return true;
        }
    }

    // Check if any rectangle corner is inside the polygon (simplified check)
    // For a more complete solution, we would need full polygon containment tests
    false
}

/// Check if a point is inside a polygon using ray casting algorithm
pub fn point_in_polygon(point: &Point, polygon_points: &[Point]) -> bool {
    if polygon_points.len() < 3 {
        return false;
    }

    let mut inside = false;
    let n = polygon_points.len();
    let mut j = n - 1;

    for i in 0..n {
        let pi = &polygon_points[i];
        let pj = &polygon_points[j];

        if ((pi.y > point.y) != (pj.y > point.y))
            && (point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y) + pi.x)
        {
            inside = !inside;
        }
        j = i;
    }

    inside
}

/// Find the index of the topmost polygon that contains the given point
/// Returns None if no polygon contains the point
pub fn find_polygon_at_point(polygons: &[Polygon], point: &Point) -> Option<usize> {
    // Iterate in reverse to get topmost (last rendered) polygon first
    for (idx, polygon) in polygons.iter().enumerate().rev() {
        let points = parse_points(&polygon.points);
        if point_in_polygon(point, &points) {
            return Some(idx);
        }
    }
    None
}
