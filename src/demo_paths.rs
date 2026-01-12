//! Demo SVG paths for testing complex path rendering
//!
//! Contains Snoopy and other complex SVG path data

use crate::scene::{parse_svg_path, Shape, ShapeGeometry, ShapeStyle, StrokeStyle, Color, Transform2D, Vec2};

/// Create Snoopy character using SVG paths
/// Snoopy is rendered as multiple path shapes (body, ear, nose, etc.)
pub fn create_snoopy_shapes(offset_x: f32, offset_y: f32, scale: f32) -> Vec<Shape> {
    let mut shapes = Vec::new();

    // Snoopy's head (simple rounded shape - no self-intersections)
    let head_path = "M 20 60
        C 20 30 40 15 70 15
        C 95 15 105 35 105 55
        C 105 75 90 90 70 90
        C 50 90 20 85 20 60
        Z";

    let head_cmds = parse_svg_path(head_path);
    let head_shape = Shape::new(
        ShapeGeometry::Path { commands: head_cmds },
        ShapeStyle::fill_and_stroke(
            Color::rgb(1.0, 1.0, 1.0),
            StrokeStyle::new(Color::black(), 2.0),
        ),
    ).with_transform(Transform2D::identity()
        .with_position(Vec2::new(offset_x, offset_y))
        .with_scale(Vec2::new(scale, scale)));
    shapes.push(head_shape);

    // Snoopy's snout (elongated shape on the left)
    let snout_path = "M 20 55
        C 20 45 10 45 5 50
        C 0 55 0 65 5 70
        C 10 75 20 75 25 70
        C 30 65 30 55 20 55
        Z";

    let snout_cmds = parse_svg_path(snout_path);
    let snout_shape = Shape::new(
        ShapeGeometry::Path { commands: snout_cmds },
        ShapeStyle::fill_and_stroke(
            Color::rgb(1.0, 1.0, 1.0),
            StrokeStyle::new(Color::black(), 2.0),
        ),
    ).with_transform(Transform2D::identity()
        .with_position(Vec2::new(offset_x, offset_y))
        .with_scale(Vec2::new(scale, scale)));
    shapes.push(snout_shape);

    // Snoopy's body (below head)
    let body_path = "M 45 88
        C 35 90 30 100 30 115
        C 30 125 40 130 55 130
        L 75 130
        C 90 130 100 125 100 115
        C 100 100 90 90 80 88
        Z";

    let body_cmds = parse_svg_path(body_path);
    let body_shape = Shape::new(
        ShapeGeometry::Path { commands: body_cmds },
        ShapeStyle::fill_and_stroke(
            Color::rgb(1.0, 1.0, 1.0),
            StrokeStyle::new(Color::black(), 2.0),
        ),
    ).with_transform(Transform2D::identity()
        .with_position(Vec2::new(offset_x, offset_y))
        .with_scale(Vec2::new(scale, scale)));
    shapes.push(body_shape);

    // Snoopy's ear (black, floppy)
    let ear_path = "M 90 25
        C 100 20 115 25 120 40
        C 125 55 115 70 100 65
        C 95 63 92 55 95 45
        C 97 35 95 28 90 25
        Z";

    let ear_cmds = parse_svg_path(ear_path);
    let ear_shape = Shape::new(
        ShapeGeometry::Path { commands: ear_cmds },
        ShapeStyle::fill_only(Color::black()),
    ).with_transform(Transform2D::identity()
        .with_position(Vec2::new(offset_x, offset_y))
        .with_scale(Vec2::new(scale, scale)));
    shapes.push(ear_shape);

    // Snoopy's nose (black oval)
    let nose_shape = Shape::new(
        ShapeGeometry::Ellipse { rx: 7.0, ry: 5.0 },
        ShapeStyle::fill_only(Color::black()),
    ).with_transform(Transform2D::identity()
        .with_position(Vec2::new(offset_x + 5.0 * scale, offset_y + 60.0 * scale))
        .with_scale(Vec2::new(scale, scale)));
    shapes.push(nose_shape);

    // Snoopy's eye (small black dot)
    let eye_shape = Shape::new(
        ShapeGeometry::Ellipse { rx: 4.0, ry: 4.0 },
        ShapeStyle::fill_only(Color::black()),
    ).with_transform(Transform2D::identity()
        .with_position(Vec2::new(offset_x + 50.0 * scale, offset_y + 45.0 * scale))
        .with_scale(Vec2::new(scale, scale)));
    shapes.push(eye_shape);

    // Snoopy's collar (red band)
    let collar_path = "M 40 88
        Q 62 92 85 88
        L 85 95
        Q 62 99 40 95
        Z";

    let collar_cmds = parse_svg_path(collar_path);
    let collar_shape = Shape::new(
        ShapeGeometry::Path { commands: collar_cmds },
        ShapeStyle::fill_only(Color::rgb(0.9, 0.1, 0.1)),
    ).with_transform(Transform2D::identity()
        .with_position(Vec2::new(offset_x, offset_y))
        .with_scale(Vec2::new(scale, scale)));
    shapes.push(collar_shape);

    shapes
}

/// Create a heart shape using SVG path with arcs
pub fn create_heart_shape(x: f32, y: f32, size: f32, color: Color) -> Shape {
    // Heart using cubic beziers
    let heart_path = "M 50 30
        C 50 20 40 10 25 10
        C 10 10 0 25 0 40
        C 0 60 20 80 50 100
        C 80 80 100 60 100 40
        C 100 25 90 10 75 10
        C 60 10 50 20 50 30
        Z";

    let cmds = parse_svg_path(heart_path);
    let scale = size / 100.0;

    Shape::new(
        ShapeGeometry::Path { commands: cmds },
        ShapeStyle::fill_and_stroke(
            color,
            StrokeStyle::new(Color::rgb(0.5, 0.0, 0.0), 2.0),
        ),
    ).with_transform(Transform2D::identity()
        .with_position(Vec2::new(x, y))
        .with_scale(Vec2::new(scale, scale)))
}

/// Create a star shape using line commands
pub fn create_star_shape(x: f32, y: f32, outer_radius: f32, inner_radius: f32, points: u32, color: Color) -> Shape {
    use std::f32::consts::PI;

    let mut path = String::new();
    let angle_step = PI / points as f32;

    for i in 0..(points * 2) {
        let radius = if i % 2 == 0 { outer_radius } else { inner_radius };
        let angle = (i as f32) * angle_step - PI / 2.0;
        let px = radius * angle.cos();
        let py = radius * angle.sin();

        if i == 0 {
            path.push_str(&format!("M {} {} ", px, py));
        } else {
            path.push_str(&format!("L {} {} ", px, py));
        }
    }
    path.push('Z');

    let cmds = parse_svg_path(&path);

    Shape::new(
        ShapeGeometry::Path { commands: cmds },
        ShapeStyle::fill_and_stroke(
            color,
            StrokeStyle::new(Color::rgb(0.6, 0.4, 0.0), 2.0),
        ),
    ).with_transform(Transform2D::identity()
        .with_position(Vec2::new(x, y)))
}

/// Create a flower shape using arcs and beziers
pub fn create_flower_shape(x: f32, y: f32, size: f32) -> Vec<Shape> {
    let mut shapes = Vec::new();
    use std::f32::consts::PI;

    // Create 6 petals around center
    for i in 0..6 {
        let angle = (i as f32) * PI / 3.0;
        let petal_x = (size * 0.4) * angle.cos();
        let petal_y = (size * 0.4) * angle.sin();

        let petal_path = format!(
            "M 0 0
             Q {} {} {} {}
             Q {} {} 0 0
             Z",
            petal_x * 0.5 - petal_y * 0.3, petal_y * 0.5 + petal_x * 0.3,
            petal_x, petal_y,
            petal_x * 0.5 + petal_y * 0.3, petal_y * 0.5 - petal_x * 0.3
        );

        let cmds = parse_svg_path(&petal_path);
        let petal = Shape::new(
            ShapeGeometry::Path { commands: cmds },
            ShapeStyle::fill_and_stroke(
                Color::rgb(1.0, 0.4, 0.7),
                StrokeStyle::new(Color::rgb(0.8, 0.2, 0.5), 1.0),
            ),
        ).with_transform(Transform2D::identity()
            .with_position(Vec2::new(x, y)));
        shapes.push(petal);
    }

    // Center of flower
    let center = Shape::new(
        ShapeGeometry::Ellipse { rx: size * 0.15, ry: size * 0.15 },
        ShapeStyle::fill_only(Color::rgb(1.0, 0.9, 0.0)),
    ).with_transform(Transform2D::identity()
        .with_position(Vec2::new(x, y)));
    shapes.push(center);

    shapes
}

/// Create a spiral using arc commands
pub fn create_spiral_shape(x: f32, y: f32, turns: u32, color: Color) -> Shape {
    let mut path = String::from("M 0 0 ");
    let mut radius = 5.0f32;
    let mut angle = 0.0f32;

    for _ in 0..(turns * 4) {
        let next_angle = angle + std::f32::consts::PI / 2.0;
        let next_radius = radius + 5.0;

        let end_x = next_radius * next_angle.cos();
        let end_y = next_radius * next_angle.sin();

        // Use arc command for each quarter turn
        path.push_str(&format!(
            "A {} {} 0 0 1 {} {} ",
            (radius + next_radius) / 2.0,
            (radius + next_radius) / 2.0,
            end_x,
            end_y
        ));

        radius = next_radius;
        angle = next_angle;
    }

    let cmds = parse_svg_path(&path);

    Shape::new(
        ShapeGeometry::Path { commands: cmds },
        ShapeStyle::stroke_only(StrokeStyle::new(color, 3.0)),
    ).with_transform(Transform2D::identity()
        .with_position(Vec2::new(x, y)))
}

/// Create all demo shapes
pub fn create_demo_shapes() -> Vec<Shape> {
    let mut shapes = Vec::new();

    // Snoopy at position (400, 200) with 2x scale
    shapes.extend(create_snoopy_shapes(400.0, 150.0, 2.5));

    // Heart
    shapes.push(create_heart_shape(50.0, 350.0, 80.0, Color::rgb(1.0, 0.2, 0.3)));

    // Star
    shapes.push(create_star_shape(200.0, 400.0, 50.0, 20.0, 5, Color::rgb(1.0, 0.8, 0.0)));

    // Flower
    shapes.extend(create_flower_shape(650.0, 400.0, 60.0));

    // Spiral
    shapes.push(create_spiral_shape(550.0, 500.0, 3, Color::rgb(0.2, 0.5, 0.9)));

    shapes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_snoopy() {
        let shapes = create_snoopy_shapes(0.0, 0.0, 1.0);
        assert!(!shapes.is_empty());
        // Should have body, ear, nose, eye, collar
        assert!(shapes.len() >= 4);
    }

    #[test]
    fn test_create_heart() {
        let heart = create_heart_shape(0.0, 0.0, 100.0, Color::rgb(1.0, 0.0, 0.0));
        match heart.geometry {
            ShapeGeometry::Path { ref commands } => {
                assert!(!commands.is_empty());
            }
            _ => panic!("Expected Path geometry"),
        }
    }

    #[test]
    fn test_create_star() {
        let star = create_star_shape(0.0, 0.0, 50.0, 20.0, 5, Color::rgb(1.0, 1.0, 0.0));
        match star.geometry {
            ShapeGeometry::Path { ref commands } => {
                // 5-pointed star has 10 line segments + close
                assert!(commands.len() >= 10);
            }
            _ => panic!("Expected Path geometry"),
        }
    }
}
