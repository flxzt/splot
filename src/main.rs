#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    pretty_env_logger::init();

    log::debug!("pretty_env_logger initialized.");

    let native_options = eframe::NativeOptions::default();

    eframe::run_native(
        "splot",
        native_options,
        Box::new(|cc| Box::new(splot::SplotApp::new(cc))),
    )
}

// when compiling to web using trunk.
#[cfg(target_arch = "wasm32")]
fn main() {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    if let Err(e) = console_log::init_with_level(log::Level::Debug) {
        eprintln!("could not initialize console log, Err `{e}`");
    }

    log::debug!("console_log initialized.");

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async move {
        eframe::start_web(
            "egui_canvas", // hardcode it
            web_options,
            Box::new(|cc| Box::new(splot::SplotApp::new(cc))),
        )
        .await
        .expect("failed to start eframe");
    });
}
