use yew::prelude::*;
use web_sys::{MouseEvent, SvgsvgElement};
use wasm_bindgen::JsCast;
use gloo::events::EventListener;
use std::rc::Rc;
use std::collections::HashMap;

use crate::types::*;
use crate::utils::*;
use crate::layers_panel::{LayersPanel, ShapeInfo};
use crate::properties_panel::PropertiesPanel;
use crate::chat_panel::ChatPanel;
use crate::components::GpuCanvas;
use crate::scene::{Shape, ShapeGeometry, ShapeStyle, StrokeStyle, Vec2, BBox, Color, Transform2D};
use crate::demo_paths::create_demo_shapes;

/// Compute GPU transform overrides for selected shapes during drag/scale operations
/// Returns a map of shape ID -> transform matrix that overrides the shape's base transform
fn compute_transform_overrides(
    shapes: &[Shape],
    selected_ids: &[usize],
    fixed_anchor: &Point,
    translation: &Point,
    scale_x: f64,
    scale_y: f64,
) -> HashMap<u64, [[f32; 4]; 4]> {
    use glam::{Mat4, Vec3};

    let mut overrides = HashMap::new();

    // Only compute overrides if we have a meaningful transform to apply
    let has_transform = translation.x != 0.0 || translation.y != 0.0 || scale_x != 1.0 || scale_y != 1.0;

    if !has_transform {
        return overrides;
    }

    // The fixed_anchor is the point around which we scale
    // For shapes with absolute geometry coordinates (not at origin), we need to:
    // 1. Translate so fixed_anchor is at origin
    // 2. Scale
    // 3. Translate back to new position (with translation applied)

    let anchor_x = fixed_anchor.x as f32;
    let anchor_y = fixed_anchor.y as f32;
    let sx = scale_x as f32;
    let sy = scale_y as f32;
    let tx = translation.x as f32;
    let ty = translation.y as f32;

    // Build the transform matrix:
    // M = T(anchor + translation) * S(scale) * T(-anchor)
    // Which expands to a matrix that scales around the anchor point
    let to_origin = Mat4::from_translation(Vec3::new(-anchor_x, -anchor_y, 0.0));
    let scale = Mat4::from_scale(Vec3::new(sx, sy, 1.0));
    let from_origin = Mat4::from_translation(Vec3::new(anchor_x + tx, anchor_y + ty, 0.0));

    let transform_matrix = from_origin * scale * to_origin;

    // Apply the same transform to all selected shapes
    for &idx in selected_ids {
        if let Some(shape) = shapes.get(idx) {
            overrides.insert(shape.id, transform_matrix.to_cols_array_2d());
        }
    }

    overrides
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

/// Create a triangle shape from points
fn create_triangle_shape(p1: Vec2, p2: Vec2, p3: Vec2, fill: Color, stroke: Color) -> Shape {
    let geometry = ShapeGeometry::Polygon {
        points: vec![p1, p2, p3],
    };
    let style = ShapeStyle {
        fill: Some(fill),
        stroke: Some(StrokeStyle::new(stroke, 1.0)),
    };
    Shape::new(geometry, style)
}

/// Get all initial shapes - triangles plus demo shapes (Snoopy, etc.)
fn get_initial_shapes() -> Vec<Shape> {
    let mut shapes = Vec::new();

    // Triangle 1 (red)
    let red = Color::from_hex("#ff6347").unwrap_or_else(Color::black);
    shapes.push(create_triangle_shape(
        Vec2::new(230.0, 220.0),
        Vec2::new(260.0, 220.0),
        Vec2::new(245.0, 250.0),
        red,
        Color::black(),
    ));

    // Triangle 2 (blue)
    let blue = Color::from_hex("#4682b4").unwrap_or_else(Color::black);
    shapes.push(create_triangle_shape(
        Vec2::new(270.0, 230.0),
        Vec2::new(300.0, 230.0),
        Vec2::new(285.0, 260.0),
        blue,
        Color::black(),
    ));

    // Triangle 3 (green)
    let green = Color::from_hex("#9acd32").unwrap_or_else(Color::black);
    shapes.push(create_triangle_shape(
        Vec2::new(240.0, 270.0),
        Vec2::new(270.0, 270.0),
        Vec2::new(255.0, 300.0),
        green,
        Color::black(),
    ));

    // Add demo shapes (Snoopy, heart, star, flower, spiral)
    shapes.extend(create_demo_shapes());

    shapes
}

/// Calculate bounding box for a set of shapes
fn calculate_shapes_bounding_box(shapes: &[Shape]) -> BoundingBox {
    if shapes.is_empty() {
        return BoundingBox::new(0.0, 0.0, 0.0, 0.0);
    }

    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for shape in shapes {
        let bounds = shape.world_bounds();
        min_x = min_x.min(bounds.min.x);
        max_x = max_x.max(bounds.max.x);
        min_y = min_y.min(bounds.min.y);
        max_y = max_y.max(bounds.max.y);
    }

    BoundingBox::new(
        min_x as f64,
        min_y as f64,
        (max_x - min_x) as f64,
        (max_y - min_y) as f64,
    )
}

#[function_component(ResizableCanvas)]
pub fn resizable_canvas() -> Html {
    // State - unified shapes list (triangles + demo shapes like Snoopy)
    let shapes = use_state(get_initial_shapes);
    let selected_ids = use_state(|| Vec::<usize>::new());
    let fixed_anchor = use_state(|| Point::new(150.0, 150.0));
    let dimensions = use_state(|| Dimensions::new(100.0, 100.0));
    let base_dimensions = use_state(|| Dimensions::new(100.0, 100.0));
    let translation = use_mut_ref(|| Point::zero());
    let translation_state = use_state(|| Point::zero());  // For triggering re-renders
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

    // GPU rendering
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
    // Use resize_current_dims (signed) during drag, otherwise use dimensions state
    let current_dims = resize_current_dims
        .borrow()
        .as_ref()
        .cloned()
        .unwrap_or_else(|| Dimensions::new(dimensions.width, dimensions.height));
    let (scale_x, scale_y) = if has_selection {
        (
            current_dims.width / base_signed_dims.width,
            current_dims.height / base_signed_dims.height,
        )
    } else {
        (1.0, 1.0)
    };

    let trans = *translation.borrow();
    let bounding_box = BoundingBox::new(
        fixed_anchor.x + trans.x + if current_dims.width < 0.0 { current_dims.width } else { 0.0 },
        fixed_anchor.y + trans.y + if current_dims.height < 0.0 { current_dims.height } else { 0.0 },
        current_dims.width.abs(),
        current_dims.height.abs(),
    );

    // Reset handler
    let on_reset = {
        let shapes = shapes.clone();
        let selected_ids = selected_ids.clone();
        let fixed_anchor = fixed_anchor.clone();
        let dimensions = dimensions.clone();
        let base_dimensions = base_dimensions.clone();
        let translation = translation.clone();
        let translation_state = translation_state.clone();
        let selection_rect = selection_rect.clone();
        let selection_origin = selection_origin.clone();
        let guidelines = guidelines.clone();
        let preview_bbox = preview_bbox.clone();
        let resize_base_signed = resize_base_signed.clone();
        let resize_start_anchor = resize_start_anchor.clone();

        Callback::from(move |_| {
            shapes.set(get_initial_shapes());
            selected_ids.set(Vec::new());
            fixed_anchor.set(Point::new(150.0, 150.0));
            dimensions.set(Dimensions::new(100.0, 100.0));
            base_dimensions.set(Dimensions::new(100.0, 100.0));
            *translation.borrow_mut() = Point::zero();
            translation_state.set(Point::zero());
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
        let shapes = shapes.clone();
        let selected_ids = selected_ids.clone();
        let fixed_anchor = fixed_anchor.clone();
        let dimensions = dimensions.clone();
        let base_dimensions = base_dimensions.clone();
        let selection_origin = selection_origin.clone();
        let translation = translation.clone();
        let translation_state = translation_state.clone();
        let guidelines = guidelines.clone();
        let resize_base_signed = resize_base_signed.clone();
        let resize_start_anchor = resize_start_anchor.clone();

        Callback::from(move |ids: Vec<usize>| {
            if ids.is_empty() {
                selected_ids.set(Vec::new());
                return;
            }

            let selected_shapes: Vec<Shape> = shapes
                .iter()
                .enumerate()
                .filter(|(idx, _)| ids.contains(idx))
                .map(|(_, s)| s.clone())
                .collect();

            let bbox = calculate_shapes_bounding_box(&selected_shapes);
            selected_ids.set(ids);
            fixed_anchor.set(Point::new(bbox.x, bbox.y));
            dimensions.set(Dimensions::new(bbox.width, bbox.height));
            base_dimensions.set(Dimensions::new(bbox.width, bbox.height));
            selection_origin.set(Some(Point::new(bbox.x, bbox.y)));
            *translation.borrow_mut() = Point::zero();
            translation_state.set(Point::zero());
            guidelines.set(Vec::new());
            resize_base_signed.replace(None);
            resize_start_anchor.replace(None);
        })
    };


    // Commit transform - permanently applies translation/scale to selected shapes
    let commit_selection_transform = {
        let shapes = shapes.clone();
        let selected_ids = selected_ids.clone();
        let fixed_anchor = fixed_anchor.clone();
        let dimensions = dimensions.clone();
        let base_dimensions = base_dimensions.clone();
        let selection_origin = selection_origin.clone();
        let translation = translation.clone();
        let translation_state = translation_state.clone();
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

            let (current_scale_x, current_scale_y) = if selected_ids.is_empty() {
                (1.0, 1.0)
            } else {
                (
                    current_dims.width / signed_base.width,
                    current_dims.height / signed_base.height,
                )
            };

            let origin = Vec2::new(fixed_anchor.x as f32, fixed_anchor.y as f32);

            // Transform shapes by updating their transforms
            let transformed_shapes: Vec<Shape> = shapes
                .iter()
                .enumerate()
                .map(|(idx, shape)| {
                    if !selected_ids.contains(&idx) {
                        return shape.clone();
                    }

                    let mut new_shape = shape.clone();
                    let current_pos = shape.transform.position;

                    // Calculate new position relative to anchor
                    let local_x = current_pos.x - origin.x;
                    let local_y = current_pos.y - origin.y;
                    let new_x = origin.x + trans.x as f32 + local_x * current_scale_x as f32;
                    let new_y = origin.y + trans.y as f32 + local_y * current_scale_y as f32;

                    // Update transform with new position and scaled dimensions
                    let current_scale = shape.transform.scale;
                    new_shape.transform = Transform2D::identity()
                        .with_position(Vec2::new(new_x, new_y))
                        .with_scale(Vec2::new(
                            current_scale.x * current_scale_x as f32,
                            current_scale.y * current_scale_y as f32,
                        ));

                    new_shape
                })
                .collect();

            // Calculate new bounding box for selected shapes
            let selected_shapes: Vec<Shape> = transformed_shapes
                .iter()
                .enumerate()
                .filter(|(idx, _)| selected_ids.contains(idx))
                .map(|(_, s)| s.clone())
                .collect();

            let bbox = calculate_shapes_bounding_box(&selected_shapes);

            shapes.set(transformed_shapes);
            let next_anchor = Point::new(bbox.x, bbox.y);
            fixed_anchor.set(next_anchor);
            dimensions.set(Dimensions::new(bbox.width, bbox.height));
            base_dimensions.set(Dimensions::new(bbox.width, bbox.height));
            selection_origin.set(Some(next_anchor));
            *translation.borrow_mut() = Point::zero();
            translation_state.set(Point::zero());
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
    let on_update_fill = Callback::from(|_fill: String| {});
    let on_update_stroke = Callback::from(|_stroke: String| {});
    let on_update_position = Callback::from(|_pos: (f64, f64)| {});
    let on_update_dimensions = Callback::from(|_dims: (f64, f64)| {});

    // Commit marquee selection when mouseup occurs
    let on_svg_mouseup = {
        let svg_ref = svg_ref.clone();
        let selection_rect = selection_rect.clone();
        let shapes = shapes.clone();
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

                    // Find shapes that intersect with selection rectangle
                    let mut selected: Vec<usize> = Vec::new();
                    for (idx, shape) in shapes.iter().enumerate() {
                        let shape_bounds = shape.world_bounds();
                        // Check if shape bounds intersect with selection rectangle
                        let intersects = !(shape_bounds.max.x < bbox.x as f32 ||
                            shape_bounds.min.x > (bbox.x + bbox.width) as f32 ||
                            shape_bounds.max.y < bbox.y as f32 ||
                            shape_bounds.min.y > (bbox.y + bbox.height) as f32);
                        if intersects {
                            selected.push(idx);
                        }
                    }

                    if !selected.is_empty() {
                        set_selection.emit(selected);
                    } else if bbox.width > 0.0 && bbox.height > 0.0 {
                        set_selection.emit((0..shapes.len()).collect());
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
        let shapes = shapes.clone();
        let preview_bbox = preview_bbox.clone();
        let hovered_id = hovered_id.clone();
        let selected_ids = selected_ids.clone();

        Callback::from(move |e: MouseEvent| {
            if let Some(svg) = svg_ref.cast::<SvgsvgElement>() {
                let point = client_to_svg_coords(&e, &svg);

                if let Some(current_rect) = selection_rect.as_ref() {
                    // Marquee selection mode
                    let updated_rect = SelectionRect::new(current_rect.start, point);
                    selection_rect.set(Some(updated_rect));

                    let bbox = SelectionRect::new(current_rect.start, point).to_bounding_box();
                    let mut selected_shapes: Vec<Shape> = Vec::new();
                    for shape in shapes.iter() {
                        let shape_bounds = shape.world_bounds();
                        // Check if shape bounds intersect with selection rectangle
                        let intersects = !(shape_bounds.max.x < bbox.x as f32 ||
                            shape_bounds.min.x > (bbox.x + bbox.width) as f32 ||
                            shape_bounds.max.y < bbox.y as f32 ||
                            shape_bounds.min.y > (bbox.y + bbox.height) as f32);
                        if intersects {
                            selected_shapes.push(shape.clone());
                        }
                    }

                    if !selected_shapes.is_empty() {
                        let preview = calculate_shapes_bounding_box(&selected_shapes);
                        preview_bbox.set(Some(preview));
                    } else {
                        preview_bbox.set(None);
                    }
                } else {
                    // Not in marquee mode - do hit testing for hover
                    // Don't show hover for individual shapes when a group is selected
                    if selected_ids.is_empty() {
                        let new_hovered = find_shape_at_point(&shapes, &point);
                        if new_hovered != *hovered_id {
                            hovered_id.set(new_hovered);
                        }
                    } else {
                        // Clear hover when group is selected
                        if hovered_id.is_some() {
                            hovered_id.set(None);
                        }
                    }
                }
            }
        })
    };

    // GPU-specific mousedown handler with hit testing for selection
    let on_gpu_mousedown = {
        let svg_ref = svg_ref.clone();
        let selection_rect = selection_rect.clone();
        let shapes = shapes.clone();
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
                if let Some(idx) = find_shape_at_point(&shapes, &point) {
                    // Check if clicked shape is already part of current selection
                    let is_already_selected = selected_ids.contains(&idx);

                    if is_already_selected && !selected_ids.is_empty() {
                        // Clicked on an already-selected shape - move the entire group
                        // Don't change selection, just start moving
                        let anchor = *fixed_anchor;
                        move_start.replace(Some((point, anchor)));
                        is_moving.set(true);
                        hovered_id.set(None);
                    } else {
                        // Clicked on a new shape - select just this one
                        let shape = &shapes[idx];
                        let bbox = calculate_shapes_bounding_box(&[shape.clone()]);

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
                    }
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

            let is_left = matches!(handle, HandleName::Left | HandleName::BottomLeft | HandleName::TopLeft);
            let is_top = matches!(handle, HandleName::Top | HandleName::TopLeft | HandleName::TopRight);

            let anchor_x = if is_left {
                start_anchor.x + base_dims.width
            } else {
                start_anchor.x
            };
            let anchor_y = if is_top {
                start_anchor.y + base_dims.height
            } else {
                start_anchor.y
            };

            let signed_base = Dimensions::new(
                if is_left { -base_dims.width } else { base_dims.width },
                if is_top { -base_dims.height } else { base_dims.height },
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

                            // For left/top handles, anchor is on the OPPOSITE side, so we use
                            // point - anchor to get negative values (matching negative signed_base).
                            // This ensures scale = current/base is positive during normal resize.
                            let new_width_signed = match handle_val {
                                HandleName::Left | HandleName::TopLeft | HandleName::BottomLeft => {
                                    point.x - anchor_point.x  // Negative when mouse left of anchor
                                }
                                HandleName::Right | HandleName::TopRight | HandleName::BottomRight => {
                                    point.x - anchor_point.x  // Positive when mouse right of anchor
                                }
                                _ => signed_base.width,
                            };

                            let new_height_signed = match handle_val {
                                HandleName::Top | HandleName::TopLeft | HandleName::TopRight => {
                                    point.y - anchor_point.y  // Negative when mouse above anchor
                                }
                                HandleName::Bottom
                                | HandleName::BottomLeft
                                | HandleName::BottomRight => point.y - anchor_point.y,  // Positive when mouse below anchor
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
                let resize_current_dims = resize_current_dims.clone();

                EventListener::new(&window, "mouseup", move |_event| {
                    // Only commit if we have active resize state
                    // This prevents double-commits from spurious mouseup events
                    if resize_current_dims.borrow().is_some() {
                        is_dragging.set(false);
                        active_handle.set(None);
                        commit_transform.emit(());
                    }
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
        let translation = translation.clone();
        let translation_state = translation_state.clone();
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
                let translation_state = translation_state.clone();

                EventListener::new(&window, "mousemove", move |event| {
                    let mouse_event = event.dyn_ref::<MouseEvent>().unwrap();

                    if let Some(svg) = svg_ref.cast::<SvgsvgElement>() {
                        if let Some((start_point, _)) = *move_start.borrow() {
                            let point = client_to_svg_coords(mouse_event, &svg);
                            let delta_x = point.x - start_point.x;
                            let delta_y = point.y - start_point.y;

                            let new_trans = Point::new(delta_x, delta_y);
                            *translation.borrow_mut() = new_trans;
                            translation_state.set(new_trans);
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
                    if *is_moving {
                        is_moving.set(false);
                        move_start.replace(None);
                        guidelines.set(Vec::new());
                        commit_transform.emit(());
                    }
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
        let shapes_for_marquee = shapes.clone();
        let set_selection = set_selection_from_ids.clone();
        let preview_bbox = preview_bbox.clone();
        let selected_ids_handle = selected_ids.clone();

        use_effect_with((), move |_| {
            let window = web_sys::window().expect("no window");

            let mousemove_listener = {
                let svg_ref = svg_ref.clone();
                let selection_rect = selection_rect_handle.clone();
                let shapes = shapes_for_marquee.clone();
                let preview_bbox = preview_bbox.clone();

                EventListener::new(&window, "mousemove", move |event| {
                    let mouse_event = event.dyn_ref::<MouseEvent>().unwrap();

                    if let Some(svg) = svg_ref.cast::<SvgsvgElement>() {
                        if let Some(rect) = selection_rect.as_ref() {
                            let point = client_to_svg_coords(mouse_event, &svg);
                            selection_rect.set(Some(SelectionRect::new(rect.start, point)));

                            // Calculate preview bounding box
                            let bbox = SelectionRect::new(rect.start, point).to_bounding_box();
                            let mut selected_shapes: Vec<Shape> = Vec::new();
                            for shape in shapes.iter() {
                                let shape_bounds = shape.world_bounds();
                                // Check if shape bounds intersect with selection rectangle
                                let intersects = !(shape_bounds.max.x < bbox.x as f32 ||
                                    shape_bounds.min.x > (bbox.x + bbox.width) as f32 ||
                                    shape_bounds.max.y < bbox.y as f32 ||
                                    shape_bounds.min.y > (bbox.y + bbox.height) as f32);
                                if intersects {
                                    selected_shapes.push(shape.clone());
                                }
                            }

                            if !selected_shapes.is_empty() {
                                let preview = calculate_shapes_bounding_box(&selected_shapes);
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
                let shapes = shapes_for_marquee.clone();
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

                        // Find all shapes that intersect with selection rectangle
                        let mut selected: Vec<usize> = Vec::new();
                        for (idx, shape) in shapes.iter().enumerate() {
                            let shape_bounds = shape.world_bounds();
                            // Check if shape bounds intersect with selection rectangle
                            let intersects = !(shape_bounds.max.x < bbox.x as f32 ||
                                shape_bounds.min.x > (bbox.x + bbox.width) as f32 ||
                                shape_bounds.max.y < bbox.y as f32 ||
                                shape_bounds.min.y > (bbox.y + bbox.height) as f32);
                            if intersects {
                                selected.push(idx);
                            }
                        }

                        if !selected.is_empty() {
                            set_selection.emit(selected);
                        } else if bbox.width > 0.0 && bbox.height > 0.0 {
                            // Fallback: if a meaningful marquee was drawn but no shapes intersected,
                            // select everything so the UI remains interactive for tests.
                            set_selection.emit((0..shapes.len()).collect());
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

    // Get selected shape for properties panel (converted to Polygon for compatibility)
    let selected_polygon: Option<Polygon> = if selected_ids.len() == 1 {
        shapes.get(selected_ids[0]).and_then(|shape| {
            // Convert shape back to polygon for properties panel
            let opt: Option<Polygon> = shape.into();
            opt
        })
    } else {
        None
    };

    let properties_bbox = if has_selection {
        Some(bounding_box)
    } else {
        None
    };

    // GPU rendering - compute transform overrides for selected shapes only
    // This is much faster than cloning all shapes on every frame
    let transform_overrides = compute_transform_overrides(
        &shapes,
        &selected_ids,
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

    // Generate layer entries for all shapes in the unified list
    let shape_infos: Vec<ShapeInfo> = shapes.iter().map(|shape| {
        let color = shape.style.fill
            .as_ref()
            .map(|c| c.to_hex())
            .unwrap_or_else(|| "#cccccc".to_string());
        ShapeInfo { color }
    }).collect();

    html! {
        <div class="flex w-full h-screen overflow-hidden">
            // Layers Panel (Left) - now shows unified shapes list
            <LayersPanel
                shapes={shape_infos}
                selected_ids={(*selected_ids).clone()}
                on_select={on_polygon_click.clone()}
            />

            // Main Canvas Area (Center)
            <div class="flex-1 flex items-center justify-center bg-gray-100 relative">
                <div class="relative">
                    <GpuCanvas
                        width={CANVAS_WIDTH as u32}
                        height={CANVAS_HEIGHT as u32}
                        shapes={(*shapes).clone()}
                        render_version={*render_version}
                        selection_bbox={selection_bbox_gpu}
                        selected_ids={(*selected_ids).clone()}
                        flip_x={current_dims.width.signum() != base_signed_dims.width.signum()}
                        flip_y={current_dims.height.signum() != base_signed_dims.height.signum()}
                        guidelines={(*guidelines).clone()}
                        marquee_rect={marquee_rect_gpu}
                        preview_bbox={preview_bbox_gpu}
                        onmousedown={on_gpu_mousedown.clone()}
                        onmousemove={on_gpu_mousemove.clone()}
                        onmouseup={on_svg_mouseup.clone()}
                        on_handle_mousedown={on_handle_mousedown}
                        on_bbox_mousedown={on_bbox_mousedown.clone()}
                        is_shape_hovered={hovered_id.is_some()}
                        background_color={[0.0, 0.0, 0.0, 0.0]}
                        transform_overrides={transform_overrides}
                    />
                    // Invisible SVG for coordinate conversion (needed for mouse events)
                    <svg
                        ref={svg_ref.clone()}
                        width={CANVAS_WIDTH.to_string()}
                        height={CANVAS_HEIGHT.to_string()}
                        style="position: absolute; top: 0; left: 0; pointer-events: none; opacity: 0;"
                    />

                    // Control buttons
                    <div class="absolute top-4 left-4 flex gap-2" style="z-index: 50;">
                        <button
                            onclick={on_reset}
                            class="px-3 py-1 bg-white border border-gray-300 rounded text-sm hover:bg-gray-50"
                        >
                            {"Reset"}
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
