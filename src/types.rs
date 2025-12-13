use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Dimensions {
    pub width: f64,
    pub height: f64,
}

impl Dimensions {
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl BoundingBox {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }

    pub fn from_points_and_dimensions(point: Point, dimensions: Dimensions) -> Self {
        Self {
            x: point.x,
            y: point.y,
            width: dimensions.width,
            height: dimensions.height,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Polygon {
    pub points: String,
    pub fill: String,
    pub stroke: String,
    pub stroke_width: f64,
}

impl Polygon {
    pub fn new(points: String, fill: String, stroke: String, stroke_width: f64) -> Self {
        Self {
            points,
            fill,
            stroke,
            stroke_width,
        }
    }

    pub fn default_triangle_1() -> Self {
        Self {
            points: "230,220 260,220 245,250".to_string(),
            fill: "#ef4444".to_string(),
            stroke: "#000000".to_string(),
            stroke_width: 1.0,
        }
    }

    pub fn default_triangle_2() -> Self {
        Self {
            points: "270,230 300,230 285,260".to_string(),
            fill: "#3b82f6".to_string(),
            stroke: "#000000".to_string(),
            stroke_width: 1.0,
        }
    }

    pub fn default_triangle_3() -> Self {
        Self {
            points: "240,270 270,270 255,300".to_string(),
            fill: "#22c55e".to_string(),
            stroke: "#000000".to_string(),
            stroke_width: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GuidelineType {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Guideline {
    pub guideline_type: GuidelineType,
    pub pos: f64,
    pub start: f64,
    pub end: f64,
}

impl Guideline {
    pub fn new(guideline_type: GuidelineType, pos: f64, start: f64, end: f64) -> Self {
        Self {
            guideline_type,
            pos,
            start,
            end,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ResizeTransform {
    pub width: f64,
    pub height: f64,
    pub anchor_x: f64,
    pub anchor_y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SelectionRect {
    pub start: Point,
    pub current: Point,
}

impl SelectionRect {
    pub fn new(start: Point, current: Point) -> Self {
        Self { start, current }
    }

    pub fn to_bounding_box(&self) -> BoundingBox {
        let x = self.start.x.min(self.current.x);
        let y = self.start.y.min(self.current.y);
        let width = (self.current.x - self.start.x).abs();
        let height = (self.current.y - self.start.y).abs();
        BoundingBox::new(x, y, width, height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleName {
    Right,
    Bottom,
    Left,
    Top,
    BottomRight,
    BottomLeft,
    TopRight,
    TopLeft,
}

impl HandleName {
    pub fn to_kebab_case(&self) -> &'static str {
        match self {
            HandleName::Right => "right",
            HandleName::Bottom => "bottom",
            HandleName::Left => "left",
            HandleName::Top => "top",
            HandleName::BottomRight => "bottom-right",
            HandleName::BottomLeft => "bottom-left",
            HandleName::TopRight => "top-right",
            HandleName::TopLeft => "top-left",
        }
    }
}

impl HandleName {
    pub fn cursor(&self) -> &'static str {
        match self {
            HandleName::Right => "ew-resize",
            HandleName::Left => "ew-resize",
            HandleName::Top => "ns-resize",
            HandleName::Bottom => "ns-resize",
            HandleName::TopLeft => "nwse-resize",
            HandleName::BottomRight => "nwse-resize",
            HandleName::TopRight => "nesw-resize",
            HandleName::BottomLeft => "nesw-resize",
        }
    }

    pub fn is_corner(&self) -> bool {
        matches!(
            self,
            HandleName::TopLeft
                | HandleName::TopRight
                | HandleName::BottomLeft
                | HandleName::BottomRight
        )
    }

    pub fn calc_position(&self, bbox: &BoundingBox) -> Point {
        match self {
            HandleName::Right => Point::new(bbox.x + bbox.width, bbox.y + bbox.height / 2.0),
            HandleName::Left => Point::new(bbox.x, bbox.y + bbox.height / 2.0),
            HandleName::Top => Point::new(bbox.x + bbox.width / 2.0, bbox.y),
            HandleName::Bottom => Point::new(bbox.x + bbox.width / 2.0, bbox.y + bbox.height),
            HandleName::TopLeft => Point::new(bbox.x, bbox.y),
            HandleName::TopRight => Point::new(bbox.x + bbox.width, bbox.y),
            HandleName::BottomLeft => Point::new(bbox.x, bbox.y + bbox.height),
            HandleName::BottomRight => Point::new(bbox.x + bbox.width, bbox.y + bbox.height),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FlipState {
    pub x: bool,
    pub y: bool,
}

impl FlipState {
    pub fn new(x: bool, y: bool) -> Self {
        Self { x, y }
    }

    pub fn none() -> Self {
        Self { x: false, y: false }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn new(role: String, content: String) -> Self {
        Self { role, content }
    }

    pub fn user(content: String) -> Self {
        Self {
            role: "user".to_string(),
            content,
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            role: "assistant".to_string(),
            content,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Design,
    Chat,
}
