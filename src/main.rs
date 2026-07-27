#![forbid(unsafe_code)]

mod app;
mod audio;
mod game;
mod platform;
mod ui;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

use app::FerrofallApp;
#[cfg(not(target_arch = "wasm32"))]
use eframe::egui;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let initial_size = initial_window_size();
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size(initial_size)
        .with_min_inner_size([720.0, 560.0])
        .with_resizable(true)
        .with_icon(Arc::new(window_icon()));
    let options = eframe::NativeOptions {
        viewport,
        renderer: native_renderer(),
        centered: true,
        persist_window: false,
        ..Default::default()
    };

    eframe::run_native(
        "Ferrofall",
        options,
        Box::new(|context| Ok(Box::new(FerrofallApp::new(context)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    eframe::WebLogger::init(log::LevelFilter::Info).ok();

    let web_options = eframe::WebOptions {
        renderer: eframe::Renderer::Glow,
        webgl_context_option: eframe::WebGlContextOption::BestFirst,
        max_fps: Some(60),
        ..Default::default()
    };

    wasm_bindgen_futures::spawn_local(async move {
        let document = web_sys::window()
            .expect("browser window is unavailable")
            .document()
            .expect("browser document is unavailable");
        let canvas = document
            .get_element_by_id(platform::CANVAS_ID)
            .expect("Ferrofall canvas is missing")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("Ferrofall canvas element has the wrong type");

        let result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|context| Ok(Box::new(FerrofallApp::new(context)))),
            )
            .await;

        if let Some(loading) = document.get_element_by_id(platform::LOADING_ID) {
            match result {
                Ok(()) => loading.remove(),
                Err(error) => {
                    loading.set_inner_html(
                        "<h1>FERROFALL COULD NOT START</h1>\
                         <p>WebGL may be unavailable. Try a current desktop browser.</p>",
                    );
                    panic!("failed to start Ferrofall: {error:?}");
                }
            }
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn initial_window_size() -> [f32; 2] {
    #[cfg(feature = "qa-screenshot")]
    if std::env::var_os("FERROFALL_QA_MIN_SIZE").is_some() {
        return [720.0, 560.0];
    }

    [960.0, 720.0]
}

#[cfg(not(target_arch = "wasm32"))]
fn native_renderer() -> eframe::Renderer {
    #[cfg(feature = "qa-screenshot")]
    return eframe::Renderer::Glow;

    #[cfg(not(feature = "qa-screenshot"))]
    eframe::Renderer::Wgpu
}

#[cfg(not(target_arch = "wasm32"))]
fn window_icon() -> egui::IconData {
    const SIZE: usize = 32;
    let mut rgba = vec![0_u8; SIZE * SIZE * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[7, 16, 24, 255]);
    }

    let cells = [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (0, 2), (0, 3)];
    for (cell_x, cell_y) in cells {
        for y in 0..6 {
            for x in 0..6 {
                let px = 7 + cell_x * 6 + x;
                let py = 4 + cell_y * 6 + y;
                let index = (py * SIZE + px) * 4;
                let edge = x == 0 || y == 0;
                rgba[index..index + 4].copy_from_slice(if edge {
                    &[255, 199, 84, 255]
                } else {
                    &[225, 153, 42, 255]
                });
            }
        }
    }

    egui::IconData {
        rgba,
        width: SIZE as u32,
        height: SIZE as u32,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn generated_window_icon_has_valid_dimensions() {
        let icon = window_icon();
        assert_eq!(icon.width, 32);
        assert_eq!(icon.height, 32);
        assert_eq!(icon.rgba.len(), 32 * 32 * 4);
    }
}
