use crate::components::overlay::CanvasOverlay;
use crate::gpu::{Mesh, Renderer, Tessellator};
use crate::scene::{BBox, SceneGraph, Shape, Vec2};
use crate::types::{Guideline, HandleName};
use std::cell::RefCell;
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

    /// Background color [r, g, b, a] (0.0 - 1.0)
    #[prop_or([1.0, 1.0, 1.0, 1.0])]
    pub background_color: [f32; 4],
}

/// State for the renderer
struct RendererState {
    renderer: Renderer,
    tessellator: Tessellator,
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
    {
        let renderer_state_clone = (*renderer_state).clone();
        let shapes = props.shapes.clone();
        let background_color = props.background_color;
        let render_version = props.render_version;

        use_effect_with(
            (renderer_state_clone.is_some(), render_version, shapes.len()),
            move |_| {
                if let Some(ref state) = renderer_state_clone {
                    let mut state = state.borrow_mut();

                    // Tessellate shapes
                    let mesh = state.tessellator.tessellate_shapes(&shapes);

                    // Render
                    if let Err(e) = state.renderer.render(&mesh, background_color) {
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

    html! {
        <div
            class="relative"
            style={format!("width: {}px; height: {}px;", props.width, props.height)}
        >
            // GPU canvas for shape rendering
            <canvas
                ref={canvas_ref}
                width={props.width.to_string()}
                height={props.height.to_string()}
                style="border: 1px solid #e5e7eb;"
                {onmousedown}
                {onmousemove}
                {onmouseup}
            />

            // SVG overlay for UI controls
            <CanvasOverlay
                selection_bbox={props.selection_bbox.clone()}
                guidelines={props.guidelines.clone()}
                marquee_rect={props.marquee_rect.clone()}
                preview_bbox={props.preview_bbox.clone()}
                width={props.width as f64}
                height={props.height as f64}
                on_handle_mousedown={props.on_handle_mousedown.clone()}
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
