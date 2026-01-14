mod app;
mod resizable_canvas;
mod types;
mod utils;
mod layers_panel;
mod properties_panel;
mod chat_panel;
mod version;
mod version_panel;
mod demo_paths;
mod snap_logic;

// GPU rendering modules (Phase 1+)
pub mod components;
pub mod gpu;
pub mod scene;

use app::App;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn run_app() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
