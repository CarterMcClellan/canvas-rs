use yew::prelude::*;
use web_sys::HtmlInputElement;
use crate::types::{ActiveTab, BoundingBox, Polygon};

#[derive(Properties, PartialEq)]
pub struct PropertiesPanelProps {
    pub active_tab: ActiveTab,
    pub selected_polygon: Option<Polygon>,
    pub bounding_box: Option<BoundingBox>,
    pub on_update_fill: Callback<String>,
    pub on_update_stroke: Callback<String>,
    pub on_update_position: Callback<(f64, f64)>,
    pub on_update_dimensions: Callback<(f64, f64)>,
}

#[function_component(PropertiesPanel)]
pub fn properties_panel(props: &PropertiesPanelProps) -> Html {
    if props.active_tab != ActiveTab::Design {
        return html! {};
    }

    let selected = props.selected_polygon.as_ref();
    let bbox = props.bounding_box.as_ref();

    html! {
        <>
            <h2 class="text-lg font-semibold pb-3 mb-4 border-b border-gray-200">{"Properties"}</h2>

            if selected.is_some() && bbox.is_some() {
                <div class="space-y-4">
                    // Fill Color
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">
                            {"Fill"}
                        </label>
                        <div class="flex gap-2">
                            <input
                                type="color"
                                value={selected.unwrap().fill.clone()}
                                oninput={
                                    let on_update = props.on_update_fill.clone();
                                    Callback::from(move |e: InputEvent| {
                                        if let Some(input) = e.target_dyn_into::<HtmlInputElement>() {
                                            on_update.emit(input.value());
                                        }
                                    })
                                }
                                class="w-12 h-8 rounded border border-gray-300 bg-white cursor-pointer"
                            />
                            <input
                                type="text"
                                value={selected.unwrap().fill.clone()}
                                oninput={
                                    let on_update = props.on_update_fill.clone();
                                    Callback::from(move |e: InputEvent| {
                                        if let Some(input) = e.target_dyn_into::<HtmlInputElement>() {
                                            on_update.emit(input.value());
                                        }
                                    })
                                }
                                class="flex-1 px-2 py-1 border border-gray-300 rounded text-sm bg-white text-gray-900"
                            />
                        </div>
                    </div>

                    // Stroke Color
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">
                            {"Stroke"}
                        </label>
                        <div class="flex gap-2">
                            <input
                                type="color"
                                value={selected.unwrap().stroke.clone()}
                                oninput={
                                    let on_update = props.on_update_stroke.clone();
                                    Callback::from(move |e: InputEvent| {
                                        if let Some(input) = e.target_dyn_into::<HtmlInputElement>() {
                                            on_update.emit(input.value());
                                        }
                                    })
                                }
                                class="w-12 h-8 rounded border border-gray-300 bg-white cursor-pointer"
                            />
                            <input
                                type="text"
                                value={selected.unwrap().stroke.clone()}
                                oninput={
                                    let on_update = props.on_update_stroke.clone();
                                    Callback::from(move |e: InputEvent| {
                                        if let Some(input) = e.target_dyn_into::<HtmlInputElement>() {
                                            on_update.emit(input.value());
                                        }
                                    })
                                }
                                class="flex-1 px-2 py-1 border border-gray-300 rounded text-sm bg-white text-gray-900"
                            />
                        </div>
                    </div>

                    // Position
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">
                            {"Position"}
                        </label>
                        <div class="grid grid-cols-2 gap-2">
                            <div>
                                <label class="block text-xs text-gray-500 mb-1">{"X"}</label>
                                <input
                                    type="number"
                                    value={bbox.unwrap().x.to_string()}
                                    oninput={
                                        let bbox = *bbox.unwrap();
                                        let on_update = props.on_update_position.clone();
                                        Callback::from(move |e: InputEvent| {
                                            if let Some(input) = e.target_dyn_into::<HtmlInputElement>() {
                                                if let Ok(x) = input.value().parse::<f64>() {
                                                    on_update.emit((x, bbox.y));
                                                }
                                            }
                                        })
                                    }
                                    class="w-full px-2 py-1 border border-gray-300 rounded text-sm bg-white text-gray-900"
                                />
                            </div>
                            <div>
                                <label class="block text-xs text-gray-500 mb-1">{"Y"}</label>
                                <input
                                    type="number"
                                    value={bbox.unwrap().y.to_string()}
                                    oninput={
                                        let bbox = *bbox.unwrap();
                                        let on_update = props.on_update_position.clone();
                                        Callback::from(move |e: InputEvent| {
                                            if let Some(input) = e.target_dyn_into::<HtmlInputElement>() {
                                                if let Ok(y) = input.value().parse::<f64>() {
                                                    on_update.emit((bbox.x, y));
                                                }
                                            }
                                        })
                                    }
                                    class="w-full px-2 py-1 border border-gray-300 rounded text-sm bg-white text-gray-900"
                                />
                            </div>
                        </div>
                    </div>

                    // Dimensions
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">
                            {"Dimensions"}
                        </label>
                        <div class="grid grid-cols-2 gap-2">
                            <div>
                                <label class="block text-xs text-gray-500 mb-1">{"Width"}</label>
                                <input
                                    type="number"
                                    value={bbox.unwrap().width.to_string()}
                                    oninput={
                                        let bbox = *bbox.unwrap();
                                        let on_update = props.on_update_dimensions.clone();
                                        Callback::from(move |e: InputEvent| {
                                            if let Some(input) = e.target_dyn_into::<HtmlInputElement>() {
                                                if let Ok(w) = input.value().parse::<f64>() {
                                                    on_update.emit((w, bbox.height));
                                                }
                                            }
                                        })
                                    }
                                    class="w-full px-2 py-1 border border-gray-300 rounded text-sm bg-white text-gray-900"
                                />
                            </div>
                            <div>
                                <label class="block text-xs text-gray-500 mb-1">{"Height"}</label>
                                <input
                                    type="number"
                                    value={bbox.unwrap().height.to_string()}
                                    oninput={
                                        let bbox = *bbox.unwrap();
                                        let on_update = props.on_update_dimensions.clone();
                                        Callback::from(move |e: InputEvent| {
                                            if let Some(input) = e.target_dyn_into::<HtmlInputElement>() {
                                                if let Ok(h) = input.value().parse::<f64>() {
                                                    on_update.emit((bbox.width, h));
                                                }
                                            }
                                        })
                                    }
                                    class="w-full px-2 py-1 border border-gray-300 rounded text-sm bg-white text-gray-900"
                                />
                            </div>
                        </div>
                    </div>
                </div>
            } else {
                <p class="text-sm text-gray-500">{"Select a shape to edit its properties"}</p>
            }
        </>
    }
}
