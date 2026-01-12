use crate::components::overlay::CanvasOverlay;
use crate::gpu::{Renderer, Tessellator};
use crate::scene::{BBox, Shape, Vec2};
use crate::types::{Guideline, HandleName};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use web_sys::HtmlCanvasElement;
use yew::prelude::*;

/// Props for the GPU canvas component
#[derive(Properties, Clone, PartialEq)]
pub struct GpuCanvasProps {
    /// Width of the canvas in pixels
    #[prop_or(800)]
    pub width: u32,

    /// Height of the canvas in pixels
    #[prop_or(600)]
    pub height: u32,

    /// Shapes to render (passed directly rather than via SceneGraph for simpler reactivity)
    #[prop_or_default]
    pub shapes: Vec<Shape>,

    /// Render version - increment to trigger re-render
    #[prop_or(0)]
    pub render_version: u32,

    /// Selection bounding box
    #[prop_or_default]
    pub selection_bbox: Option<BBox>,

    /// Selected shape indices
    #[prop_or_default]
    pub selected_ids: Vec<usize>,

    /// Flip state for X axis
    #[prop_or(false)]
    pub flip_x: bool,

    /// Flip state for Y axis
    #[prop_or(false)]
    pub flip_y: bool,

    /// Snap guidelines
    #[prop_or_default]
    pub guidelines: Vec<Guideline>,

    /// Marquee selection rectangle
    #[prop_or_default]
    pub marquee_rect: Option<(Vec2, Vec2)>,

    /// Preview bounding box
    #[prop_or_default]
    pub preview_bbox: Option<BBox>,

    /// Mouse down callback
    #[prop_or_default]
    pub onmousedown: Callback<MouseEvent>,

    /// Mouse move callback
    #[prop_or_default]
    pub onmousemove: Callback<MouseEvent>,

    /// Mouse up callback
    #[prop_or_default]
    pub onmouseup: Callback<MouseEvent>,

    /// Handle mouse down callback
    #[prop_or_default]
    pub on_handle_mousedown: Callback<(HandleName, MouseEvent)>,

    /// Bounding box mouse down callback (for moving selection)
    #[prop_or_default]
    pub on_bbox_mousedown: Callback<MouseEvent>,

    /// Whether a shape is currently hovered (for cursor styling)
    #[prop_or(false)]
    pub is_shape_hovered: bool,

    /// Background color [r, g, b, a] (0.0 - 1.0)
    /// Default is white with full opacity to match SVG canvas
    #[prop_or([1.0, 1.0, 1.0, 1.0])]
    pub background_color: [f32; 4],

    /// Transform overrides for specific shapes (by shape ID)
    /// Used for efficient dragging/scaling without re-tessellation
    #[prop_or_default]
    pub transform_overrides: HashMap<u64, [[f32; 4]; 4]>,
}

/// State for the renderer
struct RendererState {
    renderer: Renderer,
    tessellator: Tessellator,
    /// Cached meshes by shape ID
    mesh_cache: HashMap<u64, crate::gpu::Mesh>,
    /// Track which shape IDs we've seen for cache invalidation
    known_shape_ids: Vec<u64>,
}

/// GPU-accelerated canvas component with SVG overlay
/// Renders shapes via wgpu and UI controls via SVG
#[function_component(GpuCanvas)]
pub fn gpu_canvas(props: &GpuCanvasProps) -> Html {
    let canvas_ref = use_node_ref();
    let renderer_state: UseStateHandle<Option<Rc<RefCell<RendererState>>>> = use_state(|| None);

    // Initialize renderer on mount
    {
        let canvas_ref = canvas_ref.clone();
        let renderer_state = renderer_state.clone();
        let width = props.width;
        let height = props.height;

        use_effect_with((), move |_| {
            let canvas_ref = canvas_ref.clone();

            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                // Set canvas size
                canvas.set_width(width);
                canvas.set_height(height);

                // Initialize renderer asynchronously
                wasm_bindgen_futures::spawn_local(async move {
                    match Renderer::new(canvas).await {
                        Ok(renderer) => {
                            let state = RendererState {
                                renderer,
                                tessellator: Tessellator::new(),
                                mesh_cache: HashMap::new(),
                                known_shape_ids: Vec::new(),
                            };
                            renderer_state.set(Some(Rc::new(RefCell::new(state))));
                        }
                        Err(e) => {
                            web_sys::console::error_1(&format!("Failed to create renderer: {}", e).into());
                        }
                    }
                });
            }

            || ()
        });
    }

    // Render when shapes change or renderer becomes available
    // Uses cached tessellation and per-shape transforms for efficient dragging
    {
        let renderer_state_clone = (*renderer_state).clone();
        let shapes = props.shapes.clone();
        let background_color = props.background_color;
        let transform_overrides = props.transform_overrides.clone();

        // Create a lightweight dependency: shape IDs, dirty flags, and transform overrides
        // This avoids cloning entire shape geometries
        let shape_deps: Vec<(u64, bool)> = shapes.iter().map(|s| (s.id, s.dirty)).collect();

        // For transform_overrides, we use the keys and a hash of values as dependency
        // This ensures the effect re-runs when transforms change
        let override_keys: Vec<u64> = transform_overrides.keys().copied().collect();
        let override_hash: u64 = transform_overrides.values()
            .flat_map(|m| m.iter().flat_map(|row| row.iter()))
            .map(|&f| f.to_bits() as u64)
            .fold(0u64, |acc, x| acc.wrapping_add(x));

        use_effect_with(
            (renderer_state_clone.is_some(), shape_deps, override_keys, override_hash),
            move |_| {
                if let Some(ref state) = renderer_state_clone {
                    let mut state = state.borrow_mut();

                    // Update mesh cache - only tessellate new or dirty shapes
                    let current_ids: Vec<u64> = shapes.iter().map(|s| s.id).collect();

                    // Remove meshes for shapes that no longer exist
                    state.mesh_cache.retain(|id, _| current_ids.contains(id));

                    // Tessellate new or dirty shapes (at origin - transform applied in shader)
                    for shape in &shapes {
                        let needs_tessellation = shape.dirty || !state.mesh_cache.contains_key(&shape.id);
                        if needs_tessellation {
                            let mesh = state.tessellator.get_or_tessellate_shape(shape).clone();
                            state.mesh_cache.insert(shape.id, mesh);
                        }
                    }

                    state.known_shape_ids = current_ids;

                    // Clone mesh cache to avoid borrow issues
                    // (This is a shallow clone of the HashMap, meshes are cloned but it's still
                    // much cheaper than re-tessellating everything on every frame)
                    let mesh_cache_snapshot = state.mesh_cache.clone();

                    // Render with per-shape transforms
                    if let Err(e) = state.renderer.render_shapes_with_transforms(
                        &mesh_cache_snapshot,
                        &shapes,
                        &transform_overrides,
                        background_color,
                    ) {
                        web_sys::console::error_1(&format!("Render error: {}", e).into());
                    }
                }
                || ()
            },
        );
    }

    // Mouse event handlers
    let onmousedown = {
        let callback = props.onmousedown.clone();
        Callback::from(move |e: MouseEvent| {
            callback.emit(e);
        })
    };

    let onmousemove = {
        let callback = props.onmousemove.clone();
        Callback::from(move |e: MouseEvent| {
            callback.emit(e);
        })
    };

    let onmouseup = {
        let callback = props.onmouseup.clone();
        Callback::from(move |e: MouseEvent| {
            callback.emit(e);
        })
    };

    // Determine cursor based on hover state
    let canvas_cursor = if props.is_shape_hovered { "pointer" } else { "default" };

    html! {
        <div
            class="canvas-dots"
            style={format!("position: relative; width: {}px; height: {}px; background-color: white; border: 1px solid #ccc;", props.width, props.height)}
        >
            // GPU canvas for shape rendering - transparent so container background shows through
            <canvas
                ref={canvas_ref}
                width={props.width.to_string()}
                height={props.height.to_string()}
                style={format!("display: block; cursor: {};", canvas_cursor)}
                {onmousedown}
                {onmousemove}
                {onmouseup}
            />

            // SVG overlay for UI controls
            <CanvasOverlay
                selection_bbox={props.selection_bbox.clone()}
                selected_ids={props.selected_ids.clone()}
                flip_x={props.flip_x}
                flip_y={props.flip_y}
                guidelines={props.guidelines.clone()}
                marquee_rect={props.marquee_rect.clone()}
                preview_bbox={props.preview_bbox.clone()}
                width={props.width as f64}
                height={props.height as f64}
                on_handle_mousedown={props.on_handle_mousedown.clone()}
                on_bbox_mousedown={props.on_bbox_mousedown.clone()}
            />
        </div>
    }
}

/// Helper function to get mouse position relative to canvas
pub fn get_canvas_mouse_position(event: &MouseEvent, canvas_ref: &NodeRef) -> Option<Vec2> {
    let canvas = canvas_ref.cast::<HtmlCanvasElement>()?;
    let rect = canvas.get_bounding_client_rect();

    let x = event.client_x() as f64 - rect.left();
    let y = event.client_y() as f64 - rect.top();

    Some(Vec2::new(x as f32, y as f32))
}
