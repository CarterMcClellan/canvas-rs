use wasm_bindgen::JsCast;
use web_sys::{FocusEvent, HtmlInputElement, KeyboardEvent};
use yew::prelude::*;

use crate::scene::{LayerNode, LayerTree};

/// Shape type for icon display
#[derive(Clone, PartialEq, Debug)]
pub enum ShapeType {
    Rectangle,
    Ellipse,
    Circle,
    Polygon,
    Path,
}

/// Represents a shape in the layers panel
#[derive(Clone, PartialEq)]
pub struct ShapeInfo {
    pub id: u64,
    pub name: String,
    pub shape_type: ShapeType,
}

#[derive(Properties, PartialEq)]
pub struct LayersPanelProps {
    pub layer_tree: LayerTree,
    pub shapes: std::collections::HashMap<u64, ShapeInfo>,
    pub selected_ids: Vec<u64>,
    /// Callback to select shapes - receives a list of shape IDs to select
    pub on_select: Callback<Vec<u64>>,
    #[prop_or_default]
    pub on_rename: Option<Callback<(u64, String)>>,
    #[prop_or_default]
    pub on_toggle_expand: Option<Callback<u64>>,
    #[prop_or_default]
    pub on_group: Option<Callback<()>>,
    #[prop_or_default]
    pub on_ungroup: Option<Callback<u64>>,
}

/// Render a minimalist icon based on shape type
fn render_shape_icon(shape_type: &ShapeType) -> Html {
    let icon = match shape_type {
        ShapeType::Rectangle => html! {
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" class="text-gray-500">
                <rect x="2" y="3" width="12" height="10" rx="1" stroke="currentColor" stroke-width="1.5"/>
            </svg>
        },
        ShapeType::Circle => html! {
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" class="text-gray-500">
                <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5"/>
            </svg>
        },
        ShapeType::Ellipse => html! {
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" class="text-gray-500">
                <ellipse cx="8" cy="8" rx="6" ry="4" stroke="currentColor" stroke-width="1.5"/>
            </svg>
        },
        ShapeType::Polygon => html! {
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" class="text-gray-500">
                <path d="M8 2L14 13H2L8 2Z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
            </svg>
        },
        ShapeType::Path => html! {
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" class="text-gray-500">
                <path d="M2 12C4 4 12 4 14 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
        },
    };
    icon
}

/// Render a folder icon for groups
fn render_group_icon() -> Html {
    html! {
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" class="text-gray-500">
            <path d="M2 4C2 3.44772 2.44772 3 3 3H6L7 4H13C13.5523 4 14 4.44772 14 5V12C14 12.5523 13.5523 13 13 13H3C2.44772 13 2 12.5523 2 12V4Z" stroke="currentColor" stroke-width="1.5"/>
        </svg>
    }
}

/// Individual layer item component with inline editing
#[derive(Properties, PartialEq)]
struct LayerItemProps {
    pub shape_id: u64,
    pub shape: ShapeInfo,
    pub is_selected: bool,
    pub depth: usize,
    /// All shape IDs to select when this item is clicked (for group membership)
    pub select_ids: Vec<u64>,
    pub on_select: Callback<Vec<u64>>,
    pub on_rename: Option<Callback<(u64, String)>>,
}

#[function_component(LayerItem)]
fn layer_item(props: &LayerItemProps) -> Html {
    let editing = use_state(|| false);
    let edit_value = use_state(|| props.shape.name.clone());

    // Update edit_value when shape name changes
    {
        let edit_value = edit_value.clone();
        let name = props.shape.name.clone();
        use_effect_with(name.clone(), move |name| {
            edit_value.set(name.clone());
            || ()
        });
    }

    let select_ids = props.select_ids.clone();
    let on_select = props.on_select.clone();
    let onclick = {
        let editing = editing.clone();
        let select_ids = select_ids.clone();
        Callback::from(move |_| {
            if !*editing {
                on_select.emit(select_ids.clone());
            }
        })
    };

    let ondblclick = {
        let editing = editing.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            editing.set(true);
        })
    };

    let oninput = {
        let edit_value = edit_value.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) {
                edit_value.set(input.value());
            }
        })
    };

    let on_rename = props.on_rename.clone();
    let shape_id = props.shape_id;

    let finish_edit = {
        let editing = editing.clone();
        let edit_value = edit_value.clone();
        let on_rename = on_rename.clone();
        Callback::from(move |_| {
            if let Some(ref callback) = on_rename {
                callback.emit((shape_id, (*edit_value).clone()));
            }
            editing.set(false);
        })
    };

    let cancel_edit = {
        let editing = editing.clone();
        let edit_value = edit_value.clone();
        let original_name = props.shape.name.clone();
        Callback::from(move |_| {
            edit_value.set(original_name.clone());
            editing.set(false);
        })
    };

    let onkeydown = {
        let finish_edit = finish_edit.clone();
        let cancel_edit = cancel_edit.clone();
        Callback::from(move |e: KeyboardEvent| {
            match e.key().as_str() {
                "Enter" => finish_edit.emit(()),
                "Escape" => cancel_edit.emit(()),
                _ => {}
            }
        })
    };

    let onblur = {
        let finish_edit = finish_edit.clone();
        Callback::from(move |_: FocusEvent| {
            finish_edit.emit(());
        })
    };

    let indent_px = props.depth * 16;
    let box_style = format!("padding-left: {}px", indent_px + 12);

    html! {
        <div
            key={shape_id.to_string()}
            {onclick}
            style={box_style}
            class={classes!(
                "flex",
                "items-center",
                "gap-2",
                "py-2",
                "pr-2",
                "rounded",
                "cursor-pointer",
                "border",
                "hover:bg-gray-50",
                "hover:border-gray-300",
                if props.is_selected { "bg-blue-50 border-blue-300" } else { "bg-white border-gray-200" }
            )}
        >
            <div class="flex items-center justify-center flex-shrink-0">
                {render_shape_icon(&props.shape.shape_type)}
            </div>
            {
                if *editing {
                    html! {
                        <input
                            type="text"
                            class="text-sm flex-1 px-1 py-0 border border-blue-400 rounded outline-none focus:ring-1 focus:ring-blue-400"
                            value={(*edit_value).clone()}
                            {oninput}
                            {onkeydown}
                            {onblur}
                            autofocus=true
                        />
                    }
                } else {
                    html! {
                        <span class="text-sm flex-1 truncate" ondblclick={ondblclick}>
                            {&props.shape.name}
                        </span>
                    }
                }
            }
        </div>
    }
}

/// Group header component
#[derive(Properties, PartialEq)]
struct GroupHeaderProps {
    pub group_id: u64,
    pub name: String,
    pub expanded: bool,
    pub is_selected: bool,
    pub depth: usize,
    /// All shape IDs in this group (for selection)
    pub group_shape_ids: Vec<u64>,
    pub on_toggle: Callback<u64>,
    pub on_select: Callback<Vec<u64>>,
    pub on_rename: Option<Callback<(u64, String)>>,
}

#[function_component(GroupHeader)]
fn group_header(props: &GroupHeaderProps) -> Html {
    let editing = use_state(|| false);
    let edit_value = use_state(|| props.name.clone());

    {
        let edit_value = edit_value.clone();
        let name = props.name.clone();
        use_effect_with(name.clone(), move |name| {
            edit_value.set(name.clone());
            || ()
        });
    }

    let group_id = props.group_id;
    let group_shape_ids = props.group_shape_ids.clone();
    let on_toggle = props.on_toggle.clone();
    let on_select = props.on_select.clone();

    let onclick = {
        let editing = editing.clone();
        let group_shape_ids = group_shape_ids.clone();
        Callback::from(move |_| {
            if !*editing {
                // Select all shapes in this group
                on_select.emit(group_shape_ids.clone());
            }
        })
    };

    let on_chevron_click = {
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            on_toggle.emit(group_id);
        })
    };

    let ondblclick = {
        let editing = editing.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            editing.set(true);
        })
    };

    let oninput = {
        let edit_value = edit_value.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) {
                edit_value.set(input.value());
            }
        })
    };

    let on_rename = props.on_rename.clone();

    let finish_edit = {
        let editing = editing.clone();
        let edit_value = edit_value.clone();
        let on_rename = on_rename.clone();
        Callback::from(move |_| {
            if let Some(ref callback) = on_rename {
                callback.emit((group_id, (*edit_value).clone()));
            }
            editing.set(false);
        })
    };

    let cancel_edit = {
        let editing = editing.clone();
        let edit_value = edit_value.clone();
        let original_name = props.name.clone();
        Callback::from(move |_| {
            edit_value.set(original_name.clone());
            editing.set(false);
        })
    };

    let onkeydown = {
        let finish_edit = finish_edit.clone();
        let cancel_edit = cancel_edit.clone();
        Callback::from(move |e: KeyboardEvent| {
            match e.key().as_str() {
                "Enter" => finish_edit.emit(()),
                "Escape" => cancel_edit.emit(()),
                _ => {}
            }
        })
    };

    let onblur = {
        let finish_edit = finish_edit.clone();
        Callback::from(move |_: FocusEvent| {
            finish_edit.emit(());
        })
    };

    let indent_px = props.depth * 16;
    let box_style = format!("padding-left: {}px", indent_px + 12);

    // Chevron icon
    let chevron_icon = if props.expanded {
        html! {
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" class="text-gray-400">
                <path d="M3 4.5L6 7.5L9 4.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
        }
    } else {
        html! {
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" class="text-gray-400">
                <path d="M4.5 3L7.5 6L4.5 9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
        }
    };

    html! {
        <div
            key={format!("group-{}", group_id)}
            {onclick}
            style={box_style}
            class={classes!(
                "flex",
                "items-center",
                "gap-2",
                "py-2",
                "pr-2",
                "rounded",
                "cursor-pointer",
                "border",
                "hover:bg-gray-50",
                "hover:border-gray-300",
                if props.is_selected { "bg-blue-50 border-blue-300" } else { "bg-white border-gray-200" }
            )}
        >
            <span
                class="w-4 h-4 flex items-center justify-center cursor-pointer select-none"
                onclick={on_chevron_click}
            >
                {chevron_icon}
            </span>
            <div class="flex items-center justify-center flex-shrink-0">
                {render_group_icon()}
            </div>
            {
                if *editing {
                    html! {
                        <input
                            type="text"
                            class="text-sm flex-1 px-1 py-0 border border-blue-400 rounded outline-none focus:ring-1 focus:ring-blue-400 font-medium"
                            value={(*edit_value).clone()}
                            {oninput}
                            {onkeydown}
                            {onblur}
                            autofocus=true
                        />
                    }
                } else {
                    html! {
                        <span class="text-sm flex-1 font-medium truncate" ondblclick={ondblclick}>
                            {&props.name}
                        </span>
                    }
                }
            }
        </div>
    }
}

/// Render layer nodes recursively
/// parent_group_ids: shape IDs from parent group (for selection inheritance)
fn render_nodes(
    nodes: &[LayerNode],
    shapes: &std::collections::HashMap<u64, ShapeInfo>,
    selected_ids: &[u64],
    depth: usize,
    parent_group_ids: Option<Vec<u64>>,
    on_select: &Callback<Vec<u64>>,
    on_rename: &Option<Callback<(u64, String)>>,
    on_toggle_expand: &Option<Callback<u64>>,
) -> Html {
    nodes.iter().map(|node| {
        match node {
            LayerNode::Shape { shape_id } => {
                if let Some(shape) = shapes.get(shape_id) {
                    let is_selected = selected_ids.contains(shape_id);
                    // If this shape is inside a group, clicking it selects the whole group
                    let select_ids = parent_group_ids.clone().unwrap_or_else(|| vec![*shape_id]);
                    html! {
                        <LayerItem
                            shape_id={*shape_id}
                            shape={shape.clone()}
                            {is_selected}
                            {depth}
                            {select_ids}
                            on_select={on_select.clone()}
                            on_rename={on_rename.clone()}
                        />
                    }
                } else {
                    html! {}
                }
            }
            LayerNode::Group { id, name, children, expanded } => {
                let group_shape_ids = node.all_shape_ids();
                let is_selected = group_shape_ids.iter().any(|id| selected_ids.contains(id));

                let on_toggle = on_toggle_expand.clone().unwrap_or_else(|| {
                    Callback::from(|_: u64| {})
                });

                html! {
                    <>
                        <GroupHeader
                            group_id={*id}
                            name={name.clone()}
                            expanded={*expanded}
                            {is_selected}
                            {depth}
                            group_shape_ids={group_shape_ids.clone()}
                            on_toggle={on_toggle}
                            on_select={on_select.clone()}
                            on_rename={on_rename.clone()}
                        />
                        {
                            if *expanded {
                                render_nodes(
                                    children,
                                    shapes,
                                    selected_ids,
                                    depth + 1,
                                    Some(group_shape_ids),
                                    on_select,
                                    on_rename,
                                    on_toggle_expand,
                                )
                            } else {
                                html! {}
                            }
                        }
                    </>
                }
            }
        }
    }).collect::<Html>()
}

#[function_component(LayersPanel)]
pub fn layers_panel(props: &LayersPanelProps) -> Html {
    html! {
        <div class="w-64 flex-none bg-white border-r border-gray-300 p-4 overflow-y-auto flex flex-col">
            <div class="pb-3 mb-4 border-b border-gray-200">
                <h2 class="text-lg font-semibold">{"Layers"}</h2>
            </div>
            <div class="space-y-px flex-1 overflow-y-auto">
                {render_nodes(
                    &props.layer_tree.nodes,
                    &props.shapes,
                    &props.selected_ids,
                    0,
                    None,  // No parent group at top level
                    &props.on_select,
                    &props.on_rename,
                    &props.on_toggle_expand,
                )}
            </div>
        </div>
    }
}
