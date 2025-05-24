#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use firefox_session_ui_egui as lib;

// When compiling natively:
#[cfg(not(target_family = "wasm"))]
fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                // NOTE: Adding an icon is optional
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };

    let _cleanup_guard = {
        struct Cleanup;
        impl Drop for Cleanup {
            fn drop(&mut self) {
                lib::clipboard::cleanup_clipboard();
            }
        }
        Cleanup
    };

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _rt_guard = rt.enter();

    eframe::run_native(
        "FirefoxSessionDataUtilityEgui",
        native_options,
        Box::new(|cc| Ok(Box::new(lib::FirefoxSessionDataApp::new(cc)))),
    )
}

// When compiling to web using trunk:
#[cfg(target_family = "wasm")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    // Show dialog if app panics (otherwise just logs to console and silently stops working):
    #[cfg(not(debug_assertions))] // <- easier hot reloads in debug builds
    {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            // Log error to console:
            previous(info);
            // Then open an alert window so the user notices the issue:
            if let Some(win) = web_sys::window() {
                let _ = win.alert_with_message(&format!(
                    "App panicked and will now stop working:\n{info}"
                ));
            }
        }));
    }

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(lib::FirefoxSessionDataApp::new(cc)))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
