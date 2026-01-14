use yew::prelude::*;
use crate::resizable_canvas::ResizableCanvas;

#[function_component(App)]
pub fn app() -> Html {
    html! {
        <ResizableCanvas />
    }
}
