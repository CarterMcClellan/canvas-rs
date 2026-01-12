use crate::types::{BoundingBox, Guideline, GuidelineType, Point, Polygon};

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

fn parse_polygon_points(points_str: &str) -> Vec<Point> {
    points_str
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

fn calculate_polygon_bbox(polygon: &Polygon) -> BoundingBox {
    let points = parse_polygon_points(&polygon.points);

    if points.is_empty() {
        return BoundingBox::new(0.0, 0.0, 0.0, 0.0);
    }

    let xs: Vec<f64> = points.iter().map(|p| p.x).collect();
    let ys: Vec<f64> = points.iter().map(|p| p.y).collect();

    let min_x = xs.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_x = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_y = ys.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_y = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    BoundingBox::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

pub fn calculate_snap(
    proposed_box: &BoundingBox,
    polygons: &[Polygon],
    excluded_ids: &[usize],
    canvas_width: f64,
    canvas_height: f64,
    threshold: f64,
) -> SnapResult {
    // Calculate bounding boxes for non-excluded polygons
    let mut other_boxes: Vec<BoundingBox> = polygons
        .iter()
        .enumerate()
        .filter(|(i, _)| !excluded_ids.contains(i))
        .map(|(_, p)| calculate_polygon_bbox(p))
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

    #[test]
    fn test_parse_polygon_points() {
        let points_str = "100,100 200,100 150,200";
        let points = parse_polygon_points(points_str);
        assert_eq!(points.len(), 3);
        assert_eq!(points[0], Point::new(100.0, 100.0));
        assert_eq!(points[1], Point::new(200.0, 100.0));
        assert_eq!(points[2], Point::new(150.0, 200.0));
    }

    #[test]
    fn test_calculate_polygon_bbox() {
        let polygon = Polygon::new(
            "100,100 200,100 150,200".to_string(),
            "#ff0000".to_string(),
            "#000000".to_string(),
            1.0,
        );
        let bbox = calculate_polygon_bbox(&polygon);
        assert_eq!(bbox.x, 100.0);
        assert_eq!(bbox.y, 100.0);
        assert_eq!(bbox.width, 100.0);
        assert_eq!(bbox.height, 100.0);
    }
}
