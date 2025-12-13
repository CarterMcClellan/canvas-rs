use yew::prelude::*;
use crate::types::Polygon;

#[derive(Properties, PartialEq)]
pub struct LayersPanelProps {
    pub polygons: Vec<Polygon>,
    pub selected_ids: Vec<usize>,
    pub on_select: Callback<usize>,
}

#[function_component(LayersPanel)]
pub fn layers_panel(props: &LayersPanelProps) -> Html {
    html! {
        <div class="w-64 flex-none bg-white border-r border-gray-300 p-4 overflow-y-auto">
            <h2 class="text-lg font-semibold pb-3 mb-4 border-b border-gray-200">{"Layers"}</h2>
            <div class="space-y-2">
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
            </div>
        </div>
    }
}
