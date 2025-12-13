use yew::prelude::*;
use gloo::events::EventListener;
use web_sys::window;
use wasm_bindgen::JsCast;

use crate::resizable_canvas::ResizableCanvas;
use crate::layers_panel::LayersPanel;
use crate::properties_panel::PropertiesPanel;
use crate::chat_panel::ChatPanel;
use crate::types::{ActiveTab, Message, Polygon};

#[function_component(App)]
pub fn app() -> Html {
    let active_tab = use_state(|| ActiveTab::Design);

    // Keyboard shortcut for Cmd/Ctrl+K
    {
        let active_tab = active_tab.clone();
        use_effect_with((), move |_| {
            let window = window().expect("no global `window` exists");
            let document = window.document().expect("should have a document");

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

    html! {
        <ResizableCanvas />
    }
}
