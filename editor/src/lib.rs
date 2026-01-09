#![warn(clippy::expect_used)]

pub use app::App;

mod app;
mod buffer;
mod color_scheme;
mod explorer;
mod platform;

#[cfg(target_arch = "wasm32")]
use {
    eframe::wasm_bindgen::JsCast as _, wasm_bindgen::prelude::*, wasm_bindgen_futures::spawn_local,
};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub async fn start() {
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let window = web_sys::window().expect("no window");

        let path = window.location().pathname().expect("no path");
        let path_end = path.strip_prefix("/editor/").expect("invalid path");
        let mut path_parts = path_end.split("/");

        let user = path_parts.next().expect("invalid path").to_string();
        let repo = path_parts.next().expect("invalid path").to_string();

        if path_parts.next().is_some() {
            panic!("invalid path");
        }

        let document = window.document().expect("no document");
        let canvas = document
            .get_element_by_id("canvas")
            .expect("no canvas")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("element with id canvas was not a HtmlCanvasElement");

        let new_app = app::App::new(user, repo).await;

        let result = eframe::WebRunner::new()
            .start(canvas, options, Box::new(move |_| Ok(Box::new(new_app))))
            .await;

        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match result {
                Ok(_) => loading_text.remove(),
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p>An error has occured. Check the developer console for details.</p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
