use yew::prelude::*;
use crate::types::Polygon;

/// Represents a path shape in the layers panel
#[derive(Clone, PartialEq)]
pub struct PathShapeInfo {
    pub name: String,
    pub color: String,
}

#[derive(Properties, PartialEq)]
pub struct LayersPanelProps {
    pub polygons: Vec<Polygon>,
    pub selected_ids: Vec<usize>,
    pub on_select: Callback<usize>,
    #[prop_or_default]
    pub path_shapes: Vec<PathShapeInfo>,
}

#[function_component(LayersPanel)]
pub fn layers_panel(props: &LayersPanelProps) -> Html {
    html! {
        <div class="w-64 flex-none bg-white border-r border-gray-300 p-4 overflow-y-auto">
            <h2 class="text-lg font-semibold pb-3 mb-4 border-b border-gray-200">{"Layers"}</h2>
            <div class="space-y-2">
                // Polygons
                {
                    props.polygons.iter().enumerate().map(|(idx, polygon)| {
                        let is_selected = props.selected_ids.contains(&idx);
                        let on_select = props.on_select.clone();
                        let onclick = Callback::from(move |_| {
                            on_select.emit(idx);
                        });

                        html! {
                            <div
                                key={idx}
                                {onclick}
                                class={classes!(
                                    "flex",
                                    "items-center",
                                    "gap-2",
                                    "p-2",
                                    "rounded",
                                    "cursor-pointer",
                                    "border",
                                    "border-gray-200",
                                    "hover:bg-gray-100",
                                    "hover:border-gray-300",
                                    if is_selected { "bg-blue-100 border-blue-300" } else { "bg-white" }
                                )}
                            >
                                <div
                                    class="w-6 h-6 rounded border border-gray-300"
                                    style={format!("background-color: {}", polygon.fill)}
                                />
                                <span class="text-sm">
                                    {format!("Polygon {}", idx)}
                                </span>
                            </div>
                        }
                    }).collect::<Html>()
                }

                // Path shapes (flat, same style as polygons)
                {
                    props.path_shapes.iter().enumerate().map(|(idx, shape)| {
                        html! {
                            <div
                                key={format!("path-{}", idx)}
                                class="flex items-center gap-2 p-2 rounded border border-gray-200 bg-white hover:bg-gray-100 hover:border-gray-300 cursor-pointer"
                            >
                                <div class="w-6 h-6 rounded border border-gray-300 flex items-center justify-center"
                                     style={format!("background-color: {}", shape.color)}>
                                    // Path icon (simple bezier curve SVG)
                                    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5">
                                        <path d="M2 12 Q 7 2, 12 7" stroke-linecap="round"/>
                                    </svg>
                                </div>
                                <span class="text-sm">
                                    {&shape.name}
                                </span>
                            </div>
                        }
                    }).collect::<Html>()
                }
            </div>
        </div>
    }
}
