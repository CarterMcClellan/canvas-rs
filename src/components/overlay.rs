use crate::scene::{BBox, Vec2};
use crate::types::{Guideline, GuidelineType, HandleName};
use yew::prelude::*;

/// Props for the canvas overlay component
#[derive(Properties, Clone, PartialEq)]
pub struct OverlayProps {
    /// Selection bounding box (if any shapes selected)
    #[prop_or_default]
    pub selection_bbox: Option<BBox>,

    /// Snap guidelines to display
    #[prop_or_default]
    pub guidelines: Vec<Guideline>,

    /// Marquee selection rectangle (during drag)
    #[prop_or_default]
    pub marquee_rect: Option<(Vec2, Vec2)>,

    /// Preview bounding box (shapes that would be selected)
    #[prop_or_default]
    pub preview_bbox: Option<BBox>,

    /// Canvas width
    #[prop_or(800.0)]
    pub width: f64,

    /// Canvas height
    #[prop_or(600.0)]
    pub height: f64,

    /// Handle mouse down on resize handle
    #[prop_or_default]
    pub on_handle_mousedown: Callback<(HandleName, MouseEvent)>,
}

/// SVG overlay for UI controls (selection handles, guidelines, etc.)
/// This component renders on top of the GPU canvas
#[function_component(CanvasOverlay)]
pub fn canvas_overlay(props: &OverlayProps) -> Html {
    let handle_size = 8.0;
    let edge_handle_size = 6.0;

    // Render selection box and handles
    let selection_elements = if let Some(bbox) = &props.selection_bbox {
        let handles = [
            HandleName::TopLeft,
            HandleName::Top,
            HandleName::TopRight,
            HandleName::Right,
            HandleName::BottomRight,
            HandleName::Bottom,
            HandleName::BottomLeft,
            HandleName::Left,
        ];

        let handle_elements: Html = handles
            .iter()
            .map(|handle| {
                let pos = handle.calc_position_for_bbox(bbox);
                let size = if handle.is_corner() {
                    handle_size
                } else {
                    edge_handle_size
                };
                let half = size / 2.0;

                let on_handle_mousedown = props.on_handle_mousedown.clone();
                let handle_name = *handle;
                let onmousedown = Callback::from(move |e: MouseEvent| {
                    e.prevent_default();
                    e.stop_propagation();
                    on_handle_mousedown.emit((handle_name, e));
                });

                html! {
                    <rect
                        key={handle.to_kebab_case()}
                        x={format!("{}", pos.x - half)}
                        y={format!("{}", pos.y - half)}
                        width={format!("{}", size)}
                        height={format!("{}", size)}
                        fill="white"
                        stroke="#0d99ff"
                        stroke-width="1"
                        style={format!("cursor: {}; pointer-events: all;", handle.cursor())}
                        onmousedown={onmousedown}
                    />
                }
            })
            .collect();

        html! {
            <>
                // Selection bounding box
                <rect
                    x={format!("{}", bbox.min.x)}
                    y={format!("{}", bbox.min.y)}
                    width={format!("{}", bbox.width())}
                    height={format!("{}", bbox.height())}
                    fill="none"
                    stroke="#0d99ff"
                    stroke-width="1"
                />
                // Resize handles
                {handle_elements}
            </>
        }
    } else {
        html! {}
    };

    // Render snap guidelines
    let guideline_elements: Html = props
        .guidelines
        .iter()
        .enumerate()
        .map(|(i, guideline)| {
            match guideline.guideline_type {
                GuidelineType::Vertical => html! {
                    <line
                        key={format!("guideline-v-{}", i)}
                        x1={format!("{}", guideline.pos)}
                        y1={format!("{}", guideline.start)}
                        x2={format!("{}", guideline.pos)}
                        y2={format!("{}", guideline.end)}
                        stroke="#ff4444"
                        stroke-width="1"
                        stroke-dasharray="4,4"
                    />
                },
                GuidelineType::Horizontal => html! {
                    <line
                        key={format!("guideline-h-{}", i)}
                        x1={format!("{}", guideline.start)}
                        y1={format!("{}", guideline.pos)}
                        x2={format!("{}", guideline.end)}
                        y2={format!("{}", guideline.pos)}
                        stroke="#ff4444"
                        stroke-width="1"
                        stroke-dasharray="4,4"
                    />
                },
            }
        })
        .collect();

    // Render marquee selection rectangle
    let marquee_element = if let Some((start, current)) = &props.marquee_rect {
        let x = start.x.min(current.x);
        let y = start.y.min(current.y);
        let width = (current.x - start.x).abs();
        let height = (current.y - start.y).abs();

        html! {
            <rect
                x={format!("{}", x)}
                y={format!("{}", y)}
                width={format!("{}", width)}
                height={format!("{}", height)}
                fill="rgba(13, 153, 255, 0.1)"
                stroke="#0d99ff"
                stroke-width="1"
            />
        }
    } else {
        html! {}
    };

    // Render preview bounding box (shapes that would be selected)
    let preview_element = if let Some(bbox) = &props.preview_bbox {
        html! {
            <rect
                x={format!("{}", bbox.min.x)}
                y={format!("{}", bbox.min.y)}
                width={format!("{}", bbox.width())}
                height={format!("{}", bbox.height())}
                fill="none"
                stroke="#0d99ff"
                stroke-width="1"
                stroke-dasharray="2,2"
                opacity="0.5"
            />
        }
    } else {
        html! {}
    };

    html! {
        <svg
            style="position: absolute; top: 0; left: 0; z-index: 10; pointer-events: none;"
            width={format!("{}", props.width)}
            height={format!("{}", props.height)}
            viewBox={format!("0 0 {} {}", props.width, props.height)}
        >
            {selection_elements}
            {guideline_elements}
            {marquee_element}
            {preview_element}
        </svg>
    }
}

/// Extension trait for HandleName to work with BBox
impl HandleName {
    /// Calculate handle position for a BBox
    pub fn calc_position_for_bbox(&self, bbox: &BBox) -> Vec2 {
        match self {
            HandleName::Right => Vec2::new(bbox.max.x, (bbox.min.y + bbox.max.y) / 2.0),
            HandleName::Left => Vec2::new(bbox.min.x, (bbox.min.y + bbox.max.y) / 2.0),
            HandleName::Top => Vec2::new((bbox.min.x + bbox.max.x) / 2.0, bbox.min.y),
            HandleName::Bottom => Vec2::new((bbox.min.x + bbox.max.x) / 2.0, bbox.max.y),
            HandleName::TopLeft => Vec2::new(bbox.min.x, bbox.min.y),
            HandleName::TopRight => Vec2::new(bbox.max.x, bbox.min.y),
            HandleName::BottomLeft => Vec2::new(bbox.min.x, bbox.max.y),
            HandleName::BottomRight => Vec2::new(bbox.max.x, bbox.max.y),
        }
    }
}
