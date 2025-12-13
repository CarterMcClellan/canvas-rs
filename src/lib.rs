mod app;
mod resizable_canvas;
mod snap_logic;
mod types;
mod utils;
mod layers_panel;
mod properties_panel;
mod chat_panel;

use app::App;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn run_app() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
