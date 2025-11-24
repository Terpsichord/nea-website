#![cfg(not(target_arch = "wasm32"))]

use editor::App;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1080.0, 608.0]),
        ..Default::default()
    };
    eframe::run_native(
        "My IDE",
        options,
        Box::new(|_| Ok(Box::<App>::default())),
    )
}