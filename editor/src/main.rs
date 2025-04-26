#![warn(clippy::expect_used)]

mod app;
mod buffer;
mod explorer;
mod platform;

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
        Box::new(|_| Ok(Box::<app::App>::default())),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast;

    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let window = web_sys::window().expect("no window");

        let query_string = window.location().search().expect("no query string");
        let params = web_sys::UrlSearchParams::new_with_str(&query_string).expect("invalid query string");
        
        let project_id = params.get("project_id").expect("missing project id");

        let document = window.document().expect("no document");
        let canvas = document
            .get_element_by_id("canvas")
            .expect("no canvas")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("element with id canvas was not a HtmlCanvasElement");

        let result = eframe::WebRunner::new()
            .start(
                canvas,
                options,
                Box::new(move |_| Ok(Box::new(app::App::new(&project_id)))),
            )
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
