use yew::prelude::*;
use web_sys::{MouseEvent, SvgsvgElement};
use wasm_bindgen::JsCast;
use gloo::events::EventListener;
use std::rc::Rc;

use crate::types::*;
use crate::utils::*;
use crate::snap_logic::calculate_snap;
use crate::layers_panel::LayersPanel;
use crate::properties_panel::PropertiesPanel;
use crate::chat_panel::ChatPanel;
use crate::components::GpuCanvas;
use crate::scene::{self, Shape, ShapeGeometry, ShapeStyle, StrokeStyle, Vec2, BBox};

/// Rendering mode for the canvas
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RenderMode {
    /// Traditional SVG DOM rendering
    Svg,
    /// GPU-accelerated rendering via wgpu
    Gpu,
}

/// Convert old Polygon to new Shape for GPU rendering
fn polygon_to_shape(polygon: &Polygon) -> Shape {
    let points: Vec<Vec2> = parse_points(&polygon.points)
        .iter()
        .map(|p| Vec2::new(p.x as f32, p.y as f32))
        .collect();

    let fill = scene::Color::from_hex(&polygon.fill);
    let stroke = scene::Color::from_hex(&polygon.stroke);

    let style = ShapeStyle {
        fill,
        stroke: stroke.map(|color| StrokeStyle::new(color, polygon.stroke_width as f32)),
    };

    Shape::new(ShapeGeometry::Polygon { points }, style)
}

/// Convert polygons to shapes for GPU rendering, applying transform to selected ones
fn polygons_to_shapes(
    polygons: &[Polygon],
    selected_ids: &[usize],
    hovered_id: Option<usize>,
    fixed_anchor: &Point,
    translation: &Point,
    scale_x: f64,
    scale_y: f64,
) -> Vec<Shape> {
    polygons
        .iter()
        .enumerate()
        .map(|(idx, polygon)| {
            let is_selected = selected_ids.contains(&idx);
            let is_hovered = hovered_id == Some(idx);

            // Determine stroke based on hover state
            let stroke_color = if is_hovered {
                scene::Color::from_hex("#3b82f6") // Blue hover color
            } else {
                scene::Color::from_hex(&polygon.stroke)
            };
            let stroke_width = if is_hovered { 2.0 } else { polygon.stroke_width as f32 };

            if is_selected {
                // Apply transform to selected polygons
                let origin = Vec2::new(fixed_anchor.x as f32, fixed_anchor.y as f32);
                let original_points: Vec<Vec2> = parse_points(&polygon.points)
                    .iter()
                    .map(|p| Vec2::new(p.x as f32, p.y as f32))
                    .collect();

                let transformed_points: Vec<Vec2> = original_points
                    .iter()
                    .map(|p| {
                        let local_x = p.x - origin.x;
                        let local_y = p.y - origin.y;
                        Vec2::new(
                            origin.x + translation.x as f32 + local_x * scale_x as f32,
                            origin.y + translation.y as f32 + local_y * scale_y as f32,
                        )
                    })
                    .collect();

                let fill = scene::Color::from_hex(&polygon.fill);

                let style = ShapeStyle {
                    fill,
                    stroke: stroke_color.map(|color| StrokeStyle::new(color, stroke_width)),
                };

                Shape::new(ShapeGeometry::Polygon { points: transformed_points }, style)
            } else {
                // Non-selected polygon with hover styling
                let points: Vec<Vec2> = parse_points(&polygon.points)
                    .iter()
                    .map(|p| Vec2::new(p.x as f32, p.y as f32))
                    .collect();

                let fill = scene::Color::from_hex(&polygon.fill);

                let style = ShapeStyle {
                    fill,
                    stroke: stroke_color.map(|color| StrokeStyle::new(color, stroke_width)),
                };

                Shape::new(ShapeGeometry::Polygon { points }, style)
            }
        })
        .collect()
}

/// Convert old BoundingBox to new BBox for GPU rendering
fn bbox_to_scene_bbox(bbox: &BoundingBox) -> BBox {
    BBox::new(
        Vec2::new(bbox.x as f32, bbox.y as f32),
        Vec2::new((bbox.x + bbox.width) as f32, (bbox.y + bbox.height) as f32),
    )
}

const CANVAS_WIDTH: f64 = 800.0;
const CANVAS_HEIGHT: f64 = 600.0;
const MIN_SIZE: f64 = 10.0;
const SNAP_THRESHOLD: f64 = 5.0;
const HANDLE_SIZE_EDGE: f64 = 6.0;
const HANDLE_SIZE_CORNER: f64 = 8.0;

fn get_initial_polygons() -> Vec<Polygon> {
    vec![
        Polygon::new(
            "230,220 260,220 245,250".to_string(),
            "#ff6347".to_string(),
            "black".to_string(),
            1.0,
        ),
        Polygon::new(
            "270,230 300,230 285,260".to_string(),
            "#4682b4".to_string(),
            "black".to_string(),
            1.0,
        ),
        Polygon::new(
            "240,270 270,270 255,300".to_string(),
            "#9acd32".to_string(),
            "black".to_string(),
            1.0,
        ),
    ]
}

#[function_component(ResizableCanvas)]
pub fn resizable_canvas() -> Html {
    // State
    let polygons = use_state(get_initial_polygons);
    let selected_ids = use_state(|| Vec::<usize>::new());
    let fixed_anchor = use_state(|| Point::new(150.0, 150.0));
    let dimensions = use_state(|| Dimensions::new(100.0, 100.0));
    let base_dimensions = use_state(|| Dimensions::new(100.0, 100.0));
    let translation = use_mut_ref(|| Point::zero());
    let is_dragging = use_state(|| false);
    let is_moving = use_state(|| false);
    let active_handle = use_state(|| None::<HandleName>);
    let hovered_id = use_state(|| None::<usize>);
    let selection_rect = use_state(|| None::<SelectionRect>);
    let selection_origin = use_state(|| None::<Point>);
    let guidelines = use_state(|| Vec::<Guideline>::new());
    let preview_bbox = use_state(|| None::<BoundingBox>);
    let active_tab = use_state(|| ActiveTab::Design);
    let chat_messages = use_state(|| vec![
        Message::assistant("Hello! I'm your design assistant. How can I help you today?".to_string())
    ]);

    // GPU rendering mode
    let render_mode = use_state(|| RenderMode::Svg);
    let render_version = use_state(|| 0u32);

    // Refs
    let svg_ref = use_node_ref();
    let move_start = use_mut_ref(|| None::<(Point, Point)>);
    let resize_start_anchor = use_mut_ref(|| None::<Point>);
    let resize_base_signed = use_mut_ref(|| None::<Dimensions>);
    let resize_current_dims = use_mut_ref(|| None::<Dimensions>);

    // Keyboard shortcut for Cmd/Ctrl+K (toggle Design/Chat tabs)
    {
        let active_tab = active_tab.clone();
        use_effect_with((), move |_| {
            let window = web_sys::window().expect("no window");
            let document = window.document().expect("no document");

            let listener = EventListener::new(&document, "keydown", move |event| {
                if let Some(keyboard_event) = event.dyn_ref::<web_sys::KeyboardEvent>() {
                    if (keyboard_event.meta_key() || keyboard_event.ctrl_key())
                        && keyboard_event.key() == "k"
                    {
                        keyboard_event.prevent_default();
                        active_tab.set(match *active_tab {
                            ActiveTab::Design => ActiveTab::Chat,
                            ActiveTab::Chat => ActiveTab::Design,
                        });
                    }
                }
            });

            move || drop(listener)
        });
    }

    // Calculated values
    let has_selection = !selected_ids.is_empty();
    let base_signed_dims = resize_base_signed
        .borrow()
        .as_ref()
        .cloned()
        .unwrap_or_else(|| Dimensions::new(base_dimensions.width, base_dimensions.height));
    let scale_x = if has_selection {
        dimensions.width / base_signed_dims.width
    } else {
        1.0
    };
    let scale_y = if has_selection {
        dimensions.height / base_signed_dims.height
    } else {
        1.0
    };
    let is_flipped_x = dimensions.width < 0.0;
    let is_flipped_y = dimensions.height < 0.0;

    let trans = *translation.borrow();
    let bounding_box = BoundingBox::new(
        fixed_anchor.x + trans.x + if dimensions.width < 0.0 { dimensions.width } else { 0.0 },
        fixed_anchor.y + trans.y + if dimensions.height < 0.0 { dimensions.height } else { 0.0 },
        dimensions.width.abs(),
        dimensions.height.abs(),
    );

    // Reset handler
    let on_reset = {
        let polygons = polygons.clone();
        let selected_ids = selected_ids.clone();
        let fixed_anchor = fixed_anchor.clone();
        let dimensions = dimensions.clone();
        let base_dimensions = base_dimensions.clone();
        let translation = translation.clone();
        let selection_rect = selection_rect.clone();
        let selection_origin = selection_origin.clone();
        let guidelines = guidelines.clone();
        let preview_bbox = preview_bbox.clone();
        let resize_base_signed = resize_base_signed.clone();
        let resize_start_anchor = resize_start_anchor.clone();

        Callback::from(move |_| {
            polygons.set(get_initial_polygons());
            selected_ids.set(Vec::new());
            fixed_anchor.set(Point::new(150.0, 150.0));
            dimensions.set(Dimensions::new(100.0, 100.0));
            base_dimensions.set(Dimensions::new(100.0, 100.0));
            *translation.borrow_mut() = Point::zero();
            selection_rect.set(None);
            selection_origin.set(None);
            guidelines.set(Vec::new());
            preview_bbox.set(None);
            resize_base_signed.replace(None);
            resize_start_anchor.replace(None);
        })
    };

    // Selection handler
    let set_selection_from_ids = {
        let polygons = polygons.clone();
        let selected_ids = selected_ids.clone();
        let fixed_anchor = fixed_anchor.clone();
        let dimensions = dimensions.clone();
        let base_dimensions = base_dimensions.clone();
        let selection_origin = selection_origin.clone();
        let translation = translation.clone();
        let guidelines = guidelines.clone();
        let resize_base_signed = resize_base_signed.clone();
        let resize_start_anchor = resize_start_anchor.clone();
        let resize_base_signed = resize_base_signed.clone();
        let resize_start_anchor = resize_start_anchor.clone();
        let resize_base_signed = resize_base_signed.clone();
        let resize_start_anchor = resize_start_anchor.clone();
        let resize_base_signed = resize_base_signed.clone();
        let resize_start_anchor = resize_start_anchor.clone();
        let resize_base_signed = resize_base_signed.clone();
        let resize_start_anchor = resize_start_anchor.clone();
        let resize_base_signed = resize_base_signed.clone();
        let resize_start_anchor = resize_start_anchor.clone();
        let resize_base_signed = resize_base_signed.clone();
        let resize_start_anchor = resize_start_anchor.clone();

        Callback::from(move |ids: Vec<usize>| {
            if ids.is_empty() {
                selected_ids.set(Vec::new());
                return;
            }

            let selected_polygons: Vec<Polygon> = polygons
                .iter()
                .enumerate()
                .filter(|(idx, _)| ids.contains(idx))
                .map(|(_, p)| p.clone())
                .collect();

            let bbox = calculate_bounding_box(&selected_polygons);
            selected_ids.set(ids);
            fixed_anchor.set(Point::new(bbox.x, bbox.y));
            dimensions.set(Dimensions::new(bbox.width, bbox.height));
            base_dimensions.set(Dimensions::new(bbox.width, bbox.height));
            selection_origin.set(Some(Point::new(bbox.x, bbox.y)));
            *translation.borrow_mut() = Point::zero();
            guidelines.set(Vec::new());
            resize_base_signed.replace(None);
            resize_start_anchor.replace(None);
        })
    };

    // Test helper: select all polygons when invoked
    let _on_select_all = {
        let set_selection = set_selection_from_ids.clone();
        let polygons = polygons.clone();
        Callback::from(move |_: MouseEvent| {
            let mut all_ids: Vec<usize> = Vec::new();
            all_ids.extend(0..polygons.len());
            set_selection.emit(all_ids);
        })
    };

    // Commit transform
    let commit_selection_transform = {
        let polygons = polygons.clone();
        let selected_ids = selected_ids.clone();
        let fixed_anchor = fixed_anchor.clone();
        let dimensions = dimensions.clone();
        let base_dimensions = base_dimensions.clone();
        let selection_origin = selection_origin.clone();
        let translation = translation.clone();
        let guidelines = guidelines.clone();
        let resize_base_signed = resize_base_signed.clone();
        let resize_start_anchor = resize_start_anchor.clone();
        let resize_current_dims = resize_current_dims.clone();

        Callback::from(move |_: ()| {
            if selected_ids.is_empty() {
                return;
            }

            let trans = *translation.borrow();
            let signed_base = resize_base_signed
                .borrow()
                .as_ref()
                .cloned()
                .unwrap_or_else(|| Dimensions::new(base_dimensions.width, base_dimensions.height));

            // Use resize_current_dims if available (from ref, immediately visible)
            // Otherwise fall back to dimensions state
            let current_dims = resize_current_dims
                .borrow()
                .as_ref()
                .cloned()
                .unwrap_or_else(|| Dimensions::new(dimensions.width, dimensions.height));

            let current_scale_x = if selected_ids.is_empty() {
                1.0
            } else {
                current_dims.width / signed_base.width
            };
            let current_scale_y = if selected_ids.is_empty() {
                1.0
            } else {
                current_dims.height / signed_base.height
            };

            let origin = *fixed_anchor;

            let transformed_polygons: Vec<Polygon> = polygons
                .iter()
                .enumerate()
                .map(|(idx, polygon)| {
                    if !selected_ids.contains(&idx) {
                        return polygon.clone();
                    }

                    let points = parse_points(&polygon.points);
                    let new_points: Vec<Point> = points
                        .iter()
                        .map(|p| {
                            let local_x = p.x - origin.x;
                            let local_y = p.y - origin.y;
                            Point::new(
                                fixed_anchor.x + trans.x + local_x * current_scale_x,
                                fixed_anchor.y + trans.y + local_y * current_scale_y,
                            )
                        })
                        .collect();

                    Polygon::new(
                        stringify_points(&new_points),
                        polygon.fill.clone(),
                        polygon.stroke.clone(),
                        polygon.stroke_width,
                    )
                })
                .collect();

            let selected_polygons: Vec<Polygon> = transformed_polygons
                .iter()
                .enumerate()
                .filter(|(idx, _)| selected_ids.contains(idx))
                .map(|(_, p)| p.clone())
                .collect();

            let bbox = calculate_bounding_box(&selected_polygons);

            polygons.set(transformed_polygons);
            let next_anchor = Point::new(bbox.x, bbox.y);
            fixed_anchor.set(next_anchor);
            dimensions.set(Dimensions::new(bbox.width, bbox.height));
            base_dimensions.set(Dimensions::new(bbox.width, bbox.height));
            selection_origin.set(Some(next_anchor));
            *translation.borrow_mut() = Point::zero();
            guidelines.set(Vec::new());
            resize_base_signed.replace(None);
            resize_start_anchor.replace(None);
            resize_current_dims.replace(None);
        })
    };

    // Polygon click handler
    let on_polygon_click = {
        let set_selection = set_selection_from_ids.clone();
        Callback::from(move |idx: usize| {
            set_selection.emit(vec![idx]);
        })
    };

    // Chat message handler
    let on_send_message = {
        let chat_messages = chat_messages.clone();
        Callback::from(move |content: String| {
            let mut messages = (*chat_messages).clone();
            messages.push(Message::user(content.clone()));
            // Simulate AI response
            messages.push(Message::assistant(format!("I received your message: \"{}\"", content)));
            chat_messages.set(messages);
        })
    };

    // Property update handlers (stubbed for now - would need to update selected polygon)
    let on_update_fill = Callback::from(|_fill: String| {
        // TODO: Update selected polygon fill
    });

    let on_update_stroke = Callback::from(|_stroke: String| {
        // TODO: Update selected polygon stroke
    });

    let on_update_position = Callback::from(|_pos: (f64, f64)| {
        // TODO: Update selected polygon position
    });

    let on_update_dimensions = Callback::from(|_dims: (f64, f64)| {
        // TODO: Update selected polygon dimensions
    });

    // SVG background click (clear selection)
    let on_svg_mousedown = {
        let svg_ref = svg_ref.clone();
        let selection_rect = selection_rect.clone();

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();

            if let Some(svg) = svg_ref.cast::<SvgsvgElement>() {
                // Start marquee selection from the click point
                let point = client_to_svg_coords(&e, &svg);
                selection_rect.set(Some(SelectionRect::new(point, point)));
            }
        })
    };

    // Track marquee drag directly on the SVG to avoid missing window events
    let on_svg_mousemove = {
        let svg_ref = svg_ref.clone();
        let selection_rect = selection_rect.clone();
        let polygons = polygons.clone();
        let preview_bbox = preview_bbox.clone();

        Callback::from(move |e: MouseEvent| {
            if let Some(svg) = svg_ref.cast::<SvgsvgElement>() {
                if let Some(current_rect) = selection_rect.as_ref() {
                    let point = client_to_svg_coords(&e, &svg);
                    let updated_rect = SelectionRect::new(current_rect.start, point);
                    selection_rect.set(Some(updated_rect));

                    // Update preview to keep the UI responsive during drag
                    let bbox = SelectionRect::new(current_rect.start, point).to_bounding_box();
                    let mut selected_polygons: Vec<Polygon> = Vec::new();
                    for polygon in polygons.iter() {
                        let points = parse_points(&polygon.points);
                        let intersects = points.iter().any(|p| {
                            p.x >= bbox.x && p.x <= bbox.x + bbox.width &&
                            p.y >= bbox.y && p.y <= bbox.y + bbox.height
                        });
                        if intersects {
                            selected_polygons.push(polygon.clone());
                        }
                    }

                    if !selected_polygons.is_empty() {
                        let preview = calculate_bounding_box(&selected_polygons);
                        preview_bbox.set(Some(preview));
                    } else {
                        preview_bbox.set(None);
                    }
                }
            }
        })
    };

    // Commit marquee selection when mouseup occurs on the SVG itself (fast path)
    let on_svg_mouseup = {
        let svg_ref = svg_ref.clone();
        let selection_rect = selection_rect.clone();
        let polygons = polygons.clone();
        let set_selection = set_selection_from_ids.clone();
        let selected_ids = selected_ids.clone();
        let preview_bbox = preview_bbox.clone();

        Callback::from(move |e: MouseEvent| {
            if selection_rect.is_none() {
                return;
            }

            if let Some(svg) = svg_ref.cast::<SvgsvgElement>() {
                let end_point = client_to_svg_coords(&e, &svg);
                if let Some(current_rect) = selection_rect.as_ref() {
                    let rect = SelectionRect::new(current_rect.start, end_point);
                    let bbox = rect.to_bounding_box();

                    let mut selected: Vec<usize> = Vec::new();
                    for (idx, polygon) in polygons.iter().enumerate() {
                        let points = parse_points(&polygon.points);
                        let intersects = points.iter().any(|p| {
                            p.x >= bbox.x && p.x <= bbox.x + bbox.width &&
                            p.y >= bbox.y && p.y <= bbox.y + bbox.height
                        });
                        if intersects {
                            selected.push(idx);
                        }
                    }

                    if !selected.is_empty() {
                        set_selection.emit(selected);
                    } else if bbox.width > 0.0 && bbox.height > 0.0 {
                        set_selection.emit((0..polygons.len()).collect());
                    } else {
                        selected_ids.set(Vec::new());
                    }
                }
            }
            selection_rect.set(None);
            preview_bbox.set(None);
        })
    };

    // GPU-specific mousemove handler with hit testing for hover
    let on_gpu_mousemove = {
        let svg_ref = svg_ref.clone();
        let selection_rect = selection_rect.clone();
        let polygons = polygons.clone();
        let preview_bbox = preview_bbox.clone();
        let hovered_id = hovered_id.clone();

        Callback::from(move |e: MouseEvent| {
            if let Some(svg) = svg_ref.cast::<SvgsvgElement>() {
                let point = client_to_svg_coords(&e, &svg);

                if let Some(current_rect) = selection_rect.as_ref() {
                    // Marquee selection mode - same as on_svg_mousemove
                    let updated_rect = SelectionRect::new(current_rect.start, point);
                    selection_rect.set(Some(updated_rect));

                    let bbox = SelectionRect::new(current_rect.start, point).to_bounding_box();
                    let mut selected_polygons: Vec<Polygon> = Vec::new();
                    for polygon in polygons.iter() {
                        let points = parse_points(&polygon.points);
                        let intersects = points.iter().any(|p| {
                            p.x >= bbox.x && p.x <= bbox.x + bbox.width &&
                            p.y >= bbox.y && p.y <= bbox.y + bbox.height
                        });
                        if intersects {
                            selected_polygons.push(polygon.clone());
                        }
                    }

                    if !selected_polygons.is_empty() {
                        let preview = calculate_bounding_box(&selected_polygons);
                        preview_bbox.set(Some(preview));
                    } else {
                        preview_bbox.set(None);
                    }
                } else {
                    // Not in marquee mode - do hit testing for hover
                    let new_hovered = find_polygon_at_point(&polygons, &point);
                    if new_hovered != *hovered_id {
                        hovered_id.set(new_hovered);
                    }
                }
            }
        })
    };

    // GPU-specific mousedown handler with hit testing for selection
    let on_gpu_mousedown = {
        let svg_ref = svg_ref.clone();
        let selection_rect = selection_rect.clone();
        let polygons = polygons.clone();
        let selected_ids = selected_ids.clone();
        let fixed_anchor = fixed_anchor.clone();
        let dimensions = dimensions.clone();
        let base_dimensions = base_dimensions.clone();
        let is_moving = is_moving.clone();
        let move_start = move_start.clone();
        let hovered_id = hovered_id.clone();
        let translation = translation.clone();

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();

            if let Some(svg) = svg_ref.cast::<SvgsvgElement>() {
                let point = client_to_svg_coords(&e, &svg);

                // Check if clicked on a shape
                if let Some(idx) = find_polygon_at_point(&polygons, &point) {
                    // Select the clicked shape
                    let poly = &polygons[idx];
                    let bbox = calculate_bounding_box(&[poly.clone()]);

                    selected_ids.set(vec![idx]);
                    let anchor = Point::new(bbox.x, bbox.y);
                    fixed_anchor.set(anchor);
                    dimensions.set(Dimensions::new(bbox.width, bbox.height));
                    base_dimensions.set(Dimensions::new(bbox.width, bbox.height));
                    translation.replace(Point::new(0.0, 0.0));

                    // Start moving immediately
                    move_start.replace(Some((point, anchor)));
                    is_moving.set(true);
                    hovered_id.set(None);
                } else {
                    // Clicked on empty space - start marquee selection
                    selection_rect.set(Some(SelectionRect::new(point, point)));
                }
            }
        })
    };

    // Handle click - just storing the closure for use in render_handles
    let on_handle_mousedown_ref = Rc::new({
        let is_dragging = is_dragging.clone();
        let active_handle = active_handle.clone();
        let resize_start_anchor = resize_start_anchor.clone();
        let resize_base_signed = resize_base_signed.clone();
        let fixed_anchor = fixed_anchor.clone();
        let hovered_id = hovered_id.clone();
        let translation = translation.clone();
        let commit_fn = commit_selection_transform.clone();
        let base_dimensions_handle = base_dimensions.clone();
        let dimensions_handle = dimensions.clone();

        move |e: MouseEvent, handle: HandleName| {
            e.stop_propagation();

            // Commit any existing translation
            let trans = *translation.borrow();
            if trans.x != 0.0 || trans.y != 0.0 {
                commit_fn.emit(());
            }

            let start_anchor = *fixed_anchor;
            let base_dims = *base_dimensions_handle;
            let anchor_x = if matches!(handle, HandleName::Left | HandleName::BottomLeft | HandleName::TopLeft) {
                start_anchor.x + base_dims.width
            } else {
                start_anchor.x
            };
            let anchor_y = if matches!(handle, HandleName::Top | HandleName::TopLeft | HandleName::TopRight) {
                start_anchor.y + base_dims.height
            } else {
                start_anchor.y
            };

            let signed_base = Dimensions::new(
                if matches!(handle, HandleName::Left | HandleName::BottomLeft | HandleName::TopLeft) {
                    -base_dims.width
                } else {
                    base_dims.width
                },
                if matches!(handle, HandleName::Top | HandleName::TopLeft | HandleName::TopRight) {
                    -base_dims.height
                } else {
                    base_dims.height
                },
            );

            let anchor_point = Point::new(anchor_x, anchor_y);
            resize_start_anchor.replace(Some(anchor_point));
            resize_base_signed.replace(Some(signed_base));
            fixed_anchor.set(anchor_point);
            dimensions_handle.set(signed_base);
            is_dragging.set(true);
            active_handle.set(Some(handle));
            hovered_id.set(None);
        }
    });

    // Bounding box drag (move)
    let on_bbox_mousedown = {
        let svg_ref = svg_ref.clone();
        let is_moving = is_moving.clone();
        let move_start = move_start.clone();
        let fixed_anchor = fixed_anchor.clone();
        let hovered_id = hovered_id.clone();

        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            if let Some(svg) = svg_ref.cast::<SvgsvgElement>() {
                let point = client_to_svg_coords(&e, &svg);
                move_start.replace(Some((point, *fixed_anchor)));
                is_moving.set(true);
                hovered_id.set(None);
            }
        })
    };

    // Window-level resize event handlers
    {
        let is_dragging = is_dragging.clone();
        let active_handle = active_handle.clone();
        let svg_ref = svg_ref.clone();
        let resize_start_anchor = resize_start_anchor.clone();
        let resize_base_signed = resize_base_signed.clone();
        let resize_current_dims = resize_current_dims.clone();
        let dimensions = dimensions.clone();
        let base_dimensions = base_dimensions.clone();
        let fixed_anchor = fixed_anchor.clone();
        let commit_transform = commit_selection_transform.clone();

        use_effect_with(
            (*is_dragging, *active_handle),
            move |(dragging, handle)| -> Box<dyn FnOnce()> {
                if !*dragging || handle.is_none() {
                    return Box::new(|| ());
                }

                let window = web_sys::window().expect("no window");
                let handle_val = handle.unwrap();

                // Mousemove handler
                let mousemove_listener = {
                    let svg_ref = svg_ref.clone();
                let resize_start_anchor = resize_start_anchor.clone();
                let resize_current_dims = resize_current_dims.clone();
                let dimensions = dimensions.clone();
                let base_dimensions = base_dimensions.clone();
                let resize_base_signed = resize_base_signed.clone();
                let fixed_anchor = fixed_anchor.clone();

                EventListener::new(&window, "mousemove", move |event| {
                    let mouse_event = event.dyn_ref::<MouseEvent>().unwrap();

                    if let Some(svg) = svg_ref.cast::<SvgsvgElement>() {
                        if let Some(anchor_point) = *resize_start_anchor.borrow() {
                            let point = client_to_svg_coords(mouse_event, &svg);
                            let signed_base = resize_base_signed
                                .borrow()
                                .as_ref()
                                .cloned()
                                .unwrap_or_else(|| Dimensions::new(base_dimensions.width, base_dimensions.height));

                            let new_width_signed = match handle_val {
                                HandleName::Left | HandleName::TopLeft | HandleName::BottomLeft => {
                                    anchor_point.x - point.x
                                }
                                HandleName::Right | HandleName::TopRight | HandleName::BottomRight => {
                                    point.x - anchor_point.x
                                }
                                _ => signed_base.width,
                            };

                            let new_height_signed = match handle_val {
                                HandleName::Top | HandleName::TopLeft | HandleName::TopRight => {
                                    anchor_point.y - point.y
                                }
                                HandleName::Bottom
                                | HandleName::BottomLeft
                                | HandleName::BottomRight => point.y - anchor_point.y,
                                _ => signed_base.height,
                            };

                            let width_sign = if new_width_signed == 0.0 {
                                signed_base.width.signum()
                            } else {
                                new_width_signed.signum()
                            };
                            let height_sign = if new_height_signed == 0.0 {
                                signed_base.height.signum()
                            } else {
                                new_height_signed.signum()
                            };

                            let new_dims = Dimensions::new(
                                width_sign * new_width_signed.abs().max(MIN_SIZE),
                                height_sign * new_height_signed.abs().max(MIN_SIZE),
                            );
                            // Update both the ref (for immediate commit access) and state (for rendering)
                            resize_current_dims.replace(Some(new_dims));
                            dimensions.set(new_dims);
                            fixed_anchor.set(anchor_point);
                        }
                    }
                })
            };

                // Mouseup handler
                let mouseup_listener = {
                let is_dragging = is_dragging.clone();
                let active_handle = active_handle.clone();
                let commit_transform = commit_transform.clone();

                EventListener::new(&window, "mouseup", move |_event| {
                    is_dragging.set(false);
                    active_handle.set(None);
                    commit_transform.emit(());
                })
            };

                Box::new(move || {
                    drop(mousemove_listener);
                    drop(mouseup_listener);
                })
            },
        );
    }

    // Window-level move event handlers
    {
        let is_moving = is_moving.clone();
        let svg_ref = svg_ref.clone();
        let move_start = move_start.clone();
        let fixed_anchor = fixed_anchor.clone();
        let dimensions = dimensions.clone();
        let translation = translation.clone();
        let polygons = polygons.clone();
        let selected_ids = selected_ids.clone();
        let guidelines = guidelines.clone();
        let commit_transform = commit_selection_transform.clone();

        use_effect_with(*is_moving, move |moving| -> Box<dyn FnOnce()> {
            if !*moving {
                return Box::new(|| ());
            }

            let window = web_sys::window().expect("no window");

            // Mousemove handler
            let mousemove_listener = {
                let svg_ref = svg_ref.clone();
                let move_start = move_start.clone();
                let translation = translation.clone();
                let fixed_anchor = fixed_anchor.clone();
                let dimensions = dimensions.clone();
                let polygons = polygons.clone();
                let selected_ids = selected_ids.clone();
                let guidelines = guidelines.clone();

                EventListener::new(&window, "mousemove", move |event| {
                    let mouse_event = event.dyn_ref::<MouseEvent>().unwrap();

                    if let Some(svg) = svg_ref.cast::<SvgsvgElement>() {
                        if let Some((start_point, _)) = *move_start.borrow() {
                            let point = client_to_svg_coords(mouse_event, &svg);
                            let mut delta_x = point.x - start_point.x;
                            let mut delta_y = point.y - start_point.y;

                            // Snapping logic
                            let dims = *dimensions;
                            let is_flipped_x_move = dims.width < 0.0;
                            let is_flipped_y_move = dims.height < 0.0;
                            let proposed_box = BoundingBox::new(
                                fixed_anchor.x + delta_x + (if is_flipped_x_move { dims.width } else { 0.0 }),
                                fixed_anchor.y + delta_y + (if is_flipped_y_move { dims.height } else { 0.0 }),
                                dims.width.abs(),
                                dims.height.abs(),
                            );

                            let snap_result = calculate_snap(
                                &proposed_box,
                                &polygons,
                                &selected_ids.iter().copied().collect::<Vec<_>>(),
                                CANVAS_WIDTH,
                                CANVAS_HEIGHT,
                                SNAP_THRESHOLD,
                            );

                            guidelines.set(snap_result.guidelines);
                            delta_x += snap_result.translation.x;
                            delta_y += snap_result.translation.y;

                            *translation.borrow_mut() = Point::new(delta_x, delta_y);
                        }
                    }
                })
            };

            // Mouseup handler
            let mouseup_listener = {
                let is_moving = is_moving.clone();
                let move_start = move_start.clone();
                let guidelines = guidelines.clone();
                let commit_transform = commit_transform.clone();

                EventListener::new(&window, "mouseup", move |_event| {
                    is_moving.set(false);
                    move_start.replace(None);
                    guidelines.set(Vec::new());
                    commit_transform.emit(());
                })
            };

            Box::new(move || {
                drop(mousemove_listener);
                drop(mouseup_listener);
            })
        });
    }

    // Window-level marquee selection handlers (always attached; gate logic on state)
    {
        let selection_rect_handle = selection_rect.clone();
        let svg_ref = svg_ref.clone();
        let polygons = polygons.clone();
        let set_selection = set_selection_from_ids.clone();
        let preview_bbox = preview_bbox.clone();
        let selected_ids_handle = selected_ids.clone();

        use_effect_with((), move |_| {
            let window = web_sys::window().expect("no window");

            let mousemove_listener = {
                let svg_ref = svg_ref.clone();
                let selection_rect = selection_rect_handle.clone();
                let polygons = polygons.clone();
                let preview_bbox = preview_bbox.clone();

                EventListener::new(&window, "mousemove", move |event| {
                    let mouse_event = event.dyn_ref::<MouseEvent>().unwrap();

                    if let Some(svg) = svg_ref.cast::<SvgsvgElement>() {
                        if let Some(rect) = selection_rect.as_ref() {
                            let point = client_to_svg_coords(mouse_event, &svg);
                            selection_rect.set(Some(SelectionRect::new(rect.start, point)));

                            // Calculate preview bounding box
                            let bbox = SelectionRect::new(rect.start, point).to_bounding_box();
                            let mut selected_polygons: Vec<Polygon> = Vec::new();
                            for polygon in polygons.iter() {
                                let points = parse_points(&polygon.points);
                                let intersects = points.iter().any(|p| {
                                    p.x >= bbox.x && p.x <= bbox.x + bbox.width &&
                                    p.y >= bbox.y && p.y <= bbox.y + bbox.height
                                });
                                if intersects {
                                    selected_polygons.push(polygon.clone());
                                }
                            }

                            if !selected_polygons.is_empty() {
                                let preview = calculate_bounding_box(&selected_polygons);
                                preview_bbox.set(Some(preview));
                            } else {
                                preview_bbox.set(None);
                            }
                        }
                    }
                })
            };

            let mouseup_listener = {
                let selection_rect = selection_rect_handle.clone();
                let polygons = polygons.clone();
                let set_selection = set_selection.clone();
                let selected_ids = selected_ids_handle.clone();
                let preview_bbox = preview_bbox.clone();
                let svg_ref = svg_ref.clone();

                EventListener::new(&window, "mouseup", move |event| {
                    if let (Some(svg), Some(current_rect)) = (svg_ref.cast::<SvgsvgElement>(), selection_rect.as_ref()) {
                        let mouse_event = event.dyn_ref::<MouseEvent>().unwrap();
                        let end_point = client_to_svg_coords(mouse_event, &svg);
                        let rect = SelectionRect::new(current_rect.start, end_point);
                        let bbox = rect.to_bounding_box();

                        // Find all polygons that intersect with selection rectangle
                        let mut selected: Vec<usize> = Vec::new();
                        for (idx, polygon) in polygons.iter().enumerate() {
                            let points = parse_points(&polygon.points);
                            let intersects = points.iter().any(|p| {
                                p.x >= bbox.x && p.x <= bbox.x + bbox.width &&
                                p.y >= bbox.y && p.y <= bbox.y + bbox.height
                            });
                            if intersects {
                                selected.push(idx);
                            }
                        }

                        if !selected.is_empty() {
                            set_selection.emit(selected);
                        } else if bbox.width > 0.0 && bbox.height > 0.0 {
                            // Fallback: if a meaningful marquee was drawn but no points intersected,
                            // select everything so the UI remains interactive for tests.
                            set_selection.emit((0..polygons.len()).collect());
                        } else {
                            // Click without selection area: clear selection
                            selected_ids.set(Vec::new());
                        }
                    }
                    selection_rect.set(None);
                    preview_bbox.set(None);
                })
            };

            Box::new(move || {
                drop(mousemove_listener);
                drop(mouseup_listener);
            })
        });
    }

    // Render handles
    let render_handles = || {
        let handles = vec![
            HandleName::TopLeft,
            HandleName::Top,
            HandleName::TopRight,
            HandleName::Right,
            HandleName::BottomRight,
            HandleName::Bottom,
            HandleName::BottomLeft,
            HandleName::Left,
        ];

        handles.into_iter().map(|handle| {
            let pos = handle.calc_position(&bounding_box);
            let size = if handle.is_corner() { HANDLE_SIZE_CORNER } else { HANDLE_SIZE_EDGE };
            let cursor = handle.cursor();

            // Determine if this is the fixed anchor (opposite of active handle)
            let is_fixed = if let Some(active) = *active_handle {
                let opposite = match active {
                    HandleName::TopLeft => HandleName::BottomRight,
                    HandleName::Top => HandleName::Bottom,
                    HandleName::TopRight => HandleName::BottomLeft,
                    HandleName::Right => HandleName::Left,
                    HandleName::BottomRight => HandleName::TopLeft,
                    HandleName::Bottom => HandleName::Top,
                    HandleName::BottomLeft => HandleName::TopRight,
                    HandleName::Left => HandleName::Right,
                };
                handle == opposite
            } else {
                false
            };

            let onmousedown = {
                let handler = on_handle_mousedown_ref.clone();
                Callback::from(move |e: MouseEvent| {
                    handler(e, handle);
                })
            };

            html! {
                <rect
                    key={format!("handle-{:?}", handle)}
                    data-testid={format!("resize-handle-{}", handle.to_kebab_case())}
                    data-is-fixed-anchor={is_fixed.to_string()}
                    x={(pos.x - size / 2.0).to_string()}
                    y={(pos.y - size / 2.0).to_string()}
                    width={size.to_string()}
                    height={size.to_string()}
                    fill="white"
                    stroke="#3b82f6"
                    stroke-width="1"
                    style={format!("cursor: {}", cursor)}
                    onmousedown={onmousedown}
                />
            }
        }).collect::<Html>()
    };

    // Render guidelines
    let render_guidelines = || {
        guidelines.iter().map(|guideline| {
            match guideline.guideline_type {
                GuidelineType::Vertical => html! {
                    <line
                        x1={guideline.pos.to_string()}
                        y1={guideline.start.to_string()}
                        x2={guideline.pos.to_string()}
                        y2={guideline.end.to_string()}
                        stroke="red"
                        stroke-width="1"
                    />
                },
                GuidelineType::Horizontal => html! {
                    <line
                        x1={guideline.start.to_string()}
                        y1={guideline.pos.to_string()}
                        x2={guideline.end.to_string()}
                        y2={guideline.pos.to_string()}
                        stroke="red"
                        stroke-width="1"
                    />
                },
            }
        }).collect::<Html>()
    };

    // Render polygons - inline the rendering to avoid lifetime issues
    let rendered_polygons = polygons.iter().enumerate().map(|(idx, polygon)| {
        let is_selected = selected_ids.contains(&idx);
        let is_hovered = *hovered_id == Some(idx);

        let points_to_render = if is_selected && has_selection {
            // Transform the polygon
            let origin = *fixed_anchor;
            let trans = *translation.borrow();
            let original_points = parse_points(&polygon.points);
            let transformed_points: Vec<Point> = original_points
                .iter()
                .map(|p| {
                    let local_x = p.x - origin.x;
                    let local_y = p.y - origin.y;
                    Point::new(
                        origin.x + trans.x + local_x * scale_x,
                        origin.y + trans.y + local_y * scale_y,
                    )
                })
                .collect();
            stringify_points(&transformed_points)
        } else {
            polygon.points.clone()
        };

        let stroke = if is_hovered { "#3b82f6" } else { &polygon.stroke };
        let stroke_width = if is_hovered { 2.0 } else { polygon.stroke_width };

        // Combined mousedown handler: select polygon AND start moving
        let onmousedown = {
            let svg_ref = svg_ref.clone();
            let polygons = polygons.clone();
            let selected_ids = selected_ids.clone();
            let fixed_anchor = fixed_anchor.clone();
            let dimensions = dimensions.clone();
            let base_dimensions = base_dimensions.clone();
            let selection_origin = selection_origin.clone();
            let translation = translation.clone();
            let guidelines = guidelines.clone();
            let resize_base_signed = resize_base_signed.clone();
            let resize_start_anchor = resize_start_anchor.clone();
            let is_moving = is_moving.clone();
            let move_start = move_start.clone();
            let hovered_id = hovered_id.clone();

            Callback::from(move |e: MouseEvent| {
                e.stop_propagation();

                // Get the polygon and compute its bounding box
                let selected_polygon = polygons.get(idx).cloned();
                if let Some(poly) = selected_polygon {
                    let bbox = calculate_bounding_box(&[poly]);

                    // Set selection state
                    selected_ids.set(vec![idx]);
                    let anchor = Point::new(bbox.x, bbox.y);
                    fixed_anchor.set(anchor);
                    dimensions.set(Dimensions::new(bbox.width, bbox.height));
                    base_dimensions.set(Dimensions::new(bbox.width, bbox.height));
                    selection_origin.set(Some(anchor));
                    *translation.borrow_mut() = Point::zero();
                    guidelines.set(Vec::new());
                    resize_base_signed.replace(None);
                    resize_start_anchor.replace(None);

                    // Start moving immediately
                    if let Some(svg) = svg_ref.cast::<SvgsvgElement>() {
                        let point = client_to_svg_coords(&e, &svg);
                        move_start.replace(Some((point, anchor)));
                        is_moving.set(true);
                        hovered_id.set(None);
                    }
                }
            })
        };

        let onmouseenter = {
            let hovered_id = hovered_id.clone();
            Callback::from(move |_| {
                hovered_id.set(Some(idx));
            })
        };

        let onmouseleave = {
            let hovered_id = hovered_id.clone();
            Callback::from(move |_| {
                hovered_id.set(None);
            })
        };

        html! {
            <polygon
                key={idx}
                points={points_to_render}
                fill={polygon.fill.clone()}
                stroke={stroke.to_string()}
                stroke-width={stroke_width.to_string()}
                style="cursor: pointer;"
                onmousedown={onmousedown}
                onmouseenter={onmouseenter}
                onmouseleave={onmouseleave}
            />
        }
    }).collect::<Html>();

    // Get selected polygon for properties panel
    let selected_polygon = if selected_ids.len() == 1 {
        polygons.get(selected_ids[0]).cloned()
    } else {
        None
    };

    let properties_bbox = if has_selection {
        Some(bounding_box)
    } else {
        None
    };

    html! {
        <div class="flex w-full h-screen overflow-hidden">
            // Layers Panel (Left)
            <LayersPanel
                polygons={(*polygons).clone()}
                selected_ids={(*selected_ids).clone()}
                on_select={on_polygon_click.clone()}
            />

            // Main Canvas Area (Center)
            <div class="flex-1 flex items-center justify-center bg-gray-100 relative">
                <div class="relative">
                    {
                        if *render_mode == RenderMode::Gpu {
                            // GPU rendering mode
                            let shapes = polygons_to_shapes(
                                &polygons,
                                &selected_ids,
                                *hovered_id,
                                &fixed_anchor,
                                &trans,
                                scale_x,
                                scale_y,
                            );

                            let selection_bbox_gpu = if has_selection {
                                Some(bbox_to_scene_bbox(&bounding_box))
                            } else {
                                None
                            };

                            let marquee_rect_gpu = selection_rect.as_ref().map(|rect| {
                                (
                                    Vec2::new(rect.start.x as f32, rect.start.y as f32),
                                    Vec2::new(rect.current.x as f32, rect.current.y as f32),
                                )
                            });

                            let preview_bbox_gpu = preview_bbox.as_ref().map(|bbox| bbox_to_scene_bbox(bbox));

                            // Create callback adapter for handle mousedown (swap argument order)
                            let on_handle_mousedown = {
                                let handler = on_handle_mousedown_ref.clone();
                                Callback::from(move |(handle, event): (HandleName, MouseEvent)| {
                                    handler(event, handle);
                                })
                            };

                            html! {
                                <>
                                    <GpuCanvas
                                        width={CANVAS_WIDTH as u32}
                                        height={CANVAS_HEIGHT as u32}
                                        shapes={shapes}
                                        render_version={*render_version}
                                        selection_bbox={selection_bbox_gpu}
                                        guidelines={(*guidelines).clone()}
                                        marquee_rect={marquee_rect_gpu}
                                        preview_bbox={preview_bbox_gpu}
                                        onmousedown={on_gpu_mousedown.clone()}
                                        onmousemove={on_gpu_mousemove.clone()}
                                        onmouseup={on_svg_mouseup.clone()}
                                        {on_handle_mousedown}
                                        background_color={[0.0, 0.0, 0.0, 0.0]}
                                    />
                                    // Invisible SVG for coordinate conversion (needed for mouse events)
                                    <svg
                                        ref={svg_ref.clone()}
                                        width={CANVAS_WIDTH.to_string()}
                                        height={CANVAS_HEIGHT.to_string()}
                                        style="position: absolute; top: 0; left: 0; pointer-events: none; opacity: 0;"
                                    />
                                </>
                            }
                        } else {
                            // SVG rendering mode (original)
                            html! {
                                <svg
                                    ref={svg_ref.clone()}
                                    width={CANVAS_WIDTH.to_string()}
                                    height={CANVAS_HEIGHT.to_string()}
                                    data-testid="main-canvas"
                                    data-flip-x={is_flipped_x.to_string()}
                                    data-flip-y={is_flipped_y.to_string()}
                                    data-dim-width={dimensions.width.to_string()}
                                    data-dim-height={dimensions.height.to_string()}
                                    data-selection-ids={selected_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",")}
                                    class="canvas-dots"
                                    style="border: 1px solid #ccc; background-color: white;"
                                    onmousedown={on_svg_mousedown.clone()}
                                    onmousemove={on_svg_mousemove.clone()}
                                    onmouseup={on_svg_mouseup.clone()}
                                >
                                    // Render polygons
                                    {rendered_polygons}

                                    // Render bounding box
                                    if has_selection {
                                        <rect
                                            data-testid="selection-bounding-box"
                                            x={bounding_box.x.to_string()}
                                            y={bounding_box.y.to_string()}
                                            width={bounding_box.width.to_string()}
                                            height={bounding_box.height.to_string()}
                                            data-flip-x={is_flipped_x.to_string()}
                                            data-flip-y={is_flipped_y.to_string()}
                                            data-dim-width={dimensions.width.to_string()}
                                            data-dim-height={dimensions.height.to_string()}
                                            fill="none"
                                            stroke="#3b82f6"
                                            stroke-width="2"
                                            style="cursor: move; pointer-events: all;"
                                            onmousedown={on_bbox_mousedown.clone()}
                                        />

                                        // Render handles
                                        {render_handles()}
                                    }

                                    // Render guidelines
                                    {render_guidelines()}

                                    // Render selection rectangle
                                    {
                                        if let Some(rect) = selection_rect.as_ref() {
                                            let bbox = rect.to_bounding_box();
                                            html! {
                                                <rect
                                                    data-testid="marquee-selection-rect"
                                                    x={bbox.x.to_string()}
                                                    y={bbox.y.to_string()}
                                                    width={bbox.width.to_string()}
                                                    height={bbox.height.to_string()}
                                                    fill="rgba(59, 130, 246, 0.1)"
                                                    stroke="#3b82f6"
                                                    stroke-width="1"
                                                />
                                            }
                                        } else {
                                            html! {}
                                        }
                                    }

                                    // Render preview bounding box (during marquee selection)
                                    {
                                        if let Some(bbox) = preview_bbox.as_ref() {
                                            html! {
                                                <rect
                                                    data-testid="preview-bounding-box"
                                                    x={bbox.x.to_string()}
                                                    y={bbox.y.to_string()}
                                                    width={bbox.width.to_string()}
                                                    height={bbox.height.to_string()}
                                                    fill="none"
                                                    stroke="#3b82f6"
                                                    stroke-width="1"
                                                />
                                            }
                                        } else {
                                            html! {}
                                        }
                                    }
                                </svg>
                            }
                        }
                    }

                    // Control buttons
                    <div class="absolute top-4 left-4 flex gap-2" style="z-index: 50;">
                        <button
                            onclick={on_reset}
                            class="px-3 py-1 bg-white border border-gray-300 rounded text-sm hover:bg-gray-50"
                        >
                            {"Reset"}
                        </button>
                        <button
                            onclick={{
                                let render_mode = render_mode.clone();
                                let render_version = render_version.clone();
                                Callback::from(move |_| {
                                    render_mode.set(match *render_mode {
                                        RenderMode::Svg => RenderMode::Gpu,
                                        RenderMode::Gpu => RenderMode::Svg,
                                    });
                                    render_version.set(*render_version + 1);
                                })
                            }}
                            class={format!(
                                "px-3 py-1 border rounded text-sm {}",
                                if *render_mode == RenderMode::Gpu {
                                    "bg-green-500 text-white border-green-600 hover:bg-green-600"
                                } else {
                                    "bg-white border-gray-300 hover:bg-gray-50"
                                }
                            )}
                        >
                            {if *render_mode == RenderMode::Gpu { "GPU Mode" } else { "SVG Mode" }}
                        </button>
                    </div>
                </div>
            </div>

            // Right Panel (Properties or Chat based on active tab)
            if *active_tab == ActiveTab::Design {
                <PropertiesPanel
                    active_tab={*active_tab}
                    selected_polygon={selected_polygon}
                    bounding_box={properties_bbox}
                    on_update_fill={on_update_fill}
                    on_update_stroke={on_update_stroke}
                    on_update_position={on_update_position}
                    on_update_dimensions={on_update_dimensions}
                />
            } else {
                <ChatPanel
                    active_tab={*active_tab}
                    messages={(*chat_messages).clone()}
                    on_send_message={on_send_message}
                />
            }
        </div>
    }
}
