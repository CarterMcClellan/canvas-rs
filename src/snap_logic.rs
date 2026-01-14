use crate::scene::Shape;
use crate::types::{BoundingBox, Guideline, GuidelineType, Point};

pub struct SnapResult {
    pub translation: Point,
    pub guidelines: Vec<Guideline>,
}

struct SnapCheck {
    dist: f64,
    snap_delta: f64,
    guideline_type: GuidelineType,
    pos: f64,
    start: f64,
    end: f64,
}

fn check_snap(
    value: f64,
    target: f64,
    guideline_type: GuidelineType,
    start: f64,
    end: f64,
    threshold: f64,
    current_min_dist: f64,
) -> Option<SnapCheck> {
    let dist = (value - target).abs();
    (dist < current_min_dist && dist < threshold).then(|| SnapCheck {
        dist,
        snap_delta: target - value,
        guideline_type,
        pos: target,
        start,
        end,
    })
}

pub fn calculate_snap(
    proposed_box: &BoundingBox,
    shapes: &[Shape],
    excluded_ids: &[usize],
    canvas_width: f64,
    canvas_height: f64,
    threshold: f64,
) -> SnapResult {
    // Calculate bounding boxes for non-excluded shapes
    let mut other_boxes: Vec<BoundingBox> = shapes
        .iter()
        .enumerate()
        .filter(|(i, _)| !excluded_ids.contains(i))
        .map(|(_, shape)| {
            let bbox = shape.world_bounds();
            BoundingBox::new(
                bbox.min.x as f64,
                bbox.min.y as f64,
                bbox.width() as f64,
                bbox.height() as f64,
            )
        })
        .collect();

    // Add canvas edges as a bounding box
    other_boxes.push(BoundingBox::new(0.0, 0.0, canvas_width, canvas_height));

    let mut guidelines = Vec::new();
    let mut snap_delta_x = 0.0;
    let mut snap_delta_y = 0.0;
    let mut min_dist_x = threshold;
    let mut min_dist_y = threshold;

    // Edges of the proposed box
    let edges_x = [
        proposed_box.x,                              // start
        proposed_box.x + proposed_box.width / 2.0,   // center
        proposed_box.x + proposed_box.width,         // end
    ];

    let edges_y = [
        proposed_box.y,                              // start
        proposed_box.y + proposed_box.height / 2.0,  // center
        proposed_box.y + proposed_box.height,        // end
    ];

    struct SnapMatch {
        target: f64,
        start: f64,
        end: f64,
    }

    let mut best_x_match: Option<SnapMatch> = None;
    let mut best_y_match: Option<SnapMatch> = None;

    for target_box in &other_boxes {
        let target_x = [
            target_box.x,
            target_box.x + target_box.width / 2.0,
            target_box.x + target_box.width,
        ];

        let target_y = [
            target_box.y,
            target_box.y + target_box.height / 2.0,
            target_box.y + target_box.height,
        ];

        // Vertical guides (horizontal movement)
        for &edge in &edges_x {
            for &target_val in &target_x {
                let start = proposed_box.y.min(target_box.y);
                let end = (proposed_box.y + proposed_box.height).max(target_box.y + target_box.height);

                if let Some(result) = check_snap(
                    edge,
                    target_val,
                    GuidelineType::Vertical,
                    start,
                    end,
                    threshold,
                    min_dist_x,
                ) {
                    min_dist_x = result.dist;
                    snap_delta_x = result.snap_delta;
                    best_x_match = Some(SnapMatch {
                        target: result.pos,
                        start: result.start,
                        end: result.end,
                    });
                }
            }
        }

        // Horizontal guides (vertical movement)
        for &edge in &edges_y {
            for &target_val in &target_y {
                let start = proposed_box.x.min(target_box.x);
                let end = (proposed_box.x + proposed_box.width).max(target_box.x + target_box.width);

                if let Some(result) = check_snap(
                    edge,
                    target_val,
                    GuidelineType::Horizontal,
                    start,
                    end,
                    threshold,
                    min_dist_y,
                ) {
                    min_dist_y = result.dist;
                    snap_delta_y = result.snap_delta;
                    best_y_match = Some(SnapMatch {
                        target: result.pos,
                        start: result.start,
                        end: result.end,
                    });
                }
            }
        }
    }

    if let Some(match_x) = best_x_match {
        guidelines.push(Guideline::new(
            GuidelineType::Vertical,
            match_x.target,
            match_x.start,
            match_x.end,
        ));
    }

    if let Some(match_y) = best_y_match {
        guidelines.push(Guideline::new(
            GuidelineType::Horizontal,
            match_y.target,
            match_y.start,
            match_y.end,
        ));
    }

    SnapResult {
        translation: Point::new(snap_delta_x, snap_delta_y),
        guidelines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{ShapeGeometry, ShapeStyle, Transform2D, Vec2};

    #[test]
    fn test_snap_to_shape_edge() {
        // Create a target shape at (100, 100) with 50x50 size
        let target = Shape::new(
            ShapeGeometry::rectangle(50.0, 50.0),
            ShapeStyle::default(),
        )
        .with_transform(Transform2D::from_position(Vec2::new(100.0, 100.0)));

        // Proposed box at (160, 100) - just outside snap threshold
        let proposed = BoundingBox::new(160.0, 100.0, 30.0, 30.0);
        let result = calculate_snap(&proposed, &[target.clone()], &[], 800.0, 600.0, 10.0);
        assert_eq!(result.translation.x, 0.0); // No snap - too far

        // Proposed box at (155, 100) - within threshold of target right edge (150)
        let proposed = BoundingBox::new(155.0, 100.0, 30.0, 30.0);
        let result = calculate_snap(&proposed, &[target], &[], 800.0, 600.0, 10.0);
        assert_eq!(result.translation.x, -5.0); // Snap to align left edge with right edge
        assert_eq!(result.guidelines.len(), 1);
    }

    #[test]
    fn test_snap_to_center() {
        // Create a target shape at (100, 100) with 50x50 size (center at 125, 125)
        let target = Shape::new(
            ShapeGeometry::rectangle(50.0, 50.0),
            ShapeStyle::default(),
        )
        .with_transform(Transform2D::from_position(Vec2::new(100.0, 100.0)));

        // Proposed box at (108, 200) with 30x30 size (center at 123)
        // Should snap center to 125
        let proposed = BoundingBox::new(108.0, 200.0, 30.0, 30.0);
        let result = calculate_snap(&proposed, &[target], &[], 800.0, 600.0, 10.0);
        assert_eq!(result.translation.x, 2.0); // Snap center 123 -> 125
    }

    #[test]
    fn test_snap_excludes_self() {
        // Create two shapes
        let shape1 = Shape::new(
            ShapeGeometry::rectangle(50.0, 50.0),
            ShapeStyle::default(),
        )
        .with_transform(Transform2D::from_position(Vec2::new(100.0, 100.0)));

        let shape2 = Shape::new(
            ShapeGeometry::rectangle(50.0, 50.0),
            ShapeStyle::default(),
        )
        .with_transform(Transform2D::from_position(Vec2::new(200.0, 100.0)));

        let shapes = vec![shape1, shape2];

        // Propose moving shape at index 0 to near shape at index 1
        // excluded_ids=[0] should prevent snapping to self
        let proposed = BoundingBox::new(145.0, 100.0, 50.0, 50.0);
        let result = calculate_snap(&proposed, &shapes, &[0], 800.0, 600.0, 10.0);

        // Should snap to shape at index 1 (right edge at 250), not to self
        // proposed right edge at 195, target left edge at 200 -> delta +5
        assert_eq!(result.translation.x, 5.0);
    }

    #[test]
    fn test_snap_to_canvas_edge() {
        // No other shapes, should snap to canvas edges
        let proposed = BoundingBox::new(5.0, 5.0, 30.0, 30.0);
        let result = calculate_snap(&proposed, &[], &[], 800.0, 600.0, 10.0);

        // Should snap to canvas origin (0, 0)
        assert_eq!(result.translation.x, -5.0);
        assert_eq!(result.translation.y, -5.0);
    }

    #[test]
    fn test_snap_to_canvas_horizontal_center() {
        // Test snapping to horizontal center of canvas (x=400 for 800px canvas)
        // Proposed box center at x=397 (box x=382, width=30, center=397)
        // Should snap center to x=400, delta = +3
        let proposed = BoundingBox::new(382.0, 100.0, 30.0, 30.0);
        let result = calculate_snap(&proposed, &[], &[], 800.0, 600.0, 10.0);

        // Proposed center is at 382 + 15 = 397, canvas center is 400
        // Snap delta should be 3.0 to align centers
        assert_eq!(result.translation.x, 3.0);
        assert_eq!(result.guidelines.len(), 1);

        // The guideline should be a vertical line at x=400
        let guideline = &result.guidelines[0];
        assert_eq!(guideline.guideline_type, GuidelineType::Vertical);
        assert_eq!(guideline.pos, 400.0);
    }

    #[test]
    fn test_snap_to_canvas_vertical_center() {
        // Test snapping to vertical center of canvas (y=300 for 600px canvas)
        // Proposed box center at y=297 (box y=282, height=30, center=297)
        // Should snap center to y=300, delta = +3
        let proposed = BoundingBox::new(100.0, 282.0, 30.0, 30.0);
        let result = calculate_snap(&proposed, &[], &[], 800.0, 600.0, 10.0);

        // Proposed center is at 282 + 15 = 297, canvas center is 300
        // Snap delta should be 3.0 to align centers
        assert_eq!(result.translation.y, 3.0);
        assert_eq!(result.guidelines.len(), 1);

        // The guideline should be a horizontal line at y=300
        let guideline = &result.guidelines[0];
        assert_eq!(guideline.guideline_type, GuidelineType::Horizontal);
        assert_eq!(guideline.pos, 300.0);
    }

    #[test]
    fn test_snap_to_both_canvas_centers() {
        // Test snapping to both horizontal and vertical centers
        // Proposed box at (382, 282) with size 30x30
        // Center would be at (397, 297), should snap to (400, 300)
        let proposed = BoundingBox::new(382.0, 282.0, 30.0, 30.0);
        let result = calculate_snap(&proposed, &[], &[], 800.0, 600.0, 10.0);

        assert_eq!(result.translation.x, 3.0);
        assert_eq!(result.translation.y, 3.0);
        assert_eq!(result.guidelines.len(), 2);
    }

    #[test]
    fn test_snap_to_canvas_right_edge() {
        // Test snapping to right edge of canvas (x=800)
        // Proposed box with right edge at x=795 (box x=765, width=30, right=795)
        // Should snap right edge to x=800, delta = +5
        let proposed = BoundingBox::new(765.0, 100.0, 30.0, 30.0);
        let result = calculate_snap(&proposed, &[], &[], 800.0, 600.0, 10.0);

        // Right edge at 765 + 30 = 795, canvas right edge is 800
        // Snap delta should be 5.0
        assert_eq!(result.translation.x, 5.0);
    }

    #[test]
    fn test_snap_to_canvas_bottom_edge() {
        // Test snapping to bottom edge of canvas (y=600)
        // Proposed box with bottom edge at y=595 (box y=565, height=30, bottom=595)
        // Should snap bottom edge to y=600, delta = +5
        let proposed = BoundingBox::new(100.0, 565.0, 30.0, 30.0);
        let result = calculate_snap(&proposed, &[], &[], 800.0, 600.0, 10.0);

        // Bottom edge at 565 + 30 = 595, canvas bottom edge is 600
        // Snap delta should be 5.0
        assert_eq!(result.translation.y, 5.0);
    }
}
