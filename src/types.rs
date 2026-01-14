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
    Versions,
}
