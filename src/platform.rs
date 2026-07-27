use eframe::egui;

#[cfg(target_arch = "wasm32")]
pub(crate) const CANVAS_ID: &str = "ferrofall_canvas";
#[cfg(target_arch = "wasm32")]
pub(crate) const LOADING_ID: &str = "loading";

#[cfg(target_arch = "wasm32")]
const BEST_SCORE_KEY: &str = "ferrofall.best-score.v1";
#[cfg(target_arch = "wasm32")]
const MIN_PORTRAIT_WIDTH: f32 = 320.0;
#[cfg(target_arch = "wasm32")]
const MIN_PORTRAIT_HEIGHT: f32 = 500.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) enum BrowserSupportIssue {
    ViewportTooSmall,
}

pub(crate) fn browser_support_issue(size: egui::Vec2) -> Option<BrowserSupportIssue> {
    imp::browser_support_issue(size)
}

pub(crate) fn load_best_score() -> u64 {
    imp::load_best_score()
}

pub(crate) fn prefers_touch_controls() -> bool {
    imp::prefers_touch_controls()
}

pub(crate) fn fullscreen_available() -> bool {
    imp::fullscreen_available()
}

pub(crate) fn sync_canvas_resolution() -> bool {
    imp::sync_canvas_resolution()
}

pub(crate) fn save_best_score(score: u64) {
    imp::save_best_score(score);
}

pub(crate) fn set_accessible_status(screen: &str, message: &str) {
    imp::set_accessible_status(screen, message);
}

pub(crate) fn set_canvas_layout(layout: &str, touch_controls: bool) {
    imp::set_canvas_layout(layout, touch_controls);
}

pub(crate) fn set_canvas_touch_metadata(regions: &str, active: &str) {
    imp::set_canvas_touch_metadata(regions, active);
}

pub(crate) fn toggle_fullscreen(context: &egui::Context) {
    imp::toggle_fullscreen(context);
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::{
        BEST_SCORE_KEY, BrowserSupportIssue, CANVAS_ID, MIN_PORTRAIT_HEIGHT, MIN_PORTRAIT_WIDTH,
    };
    use eframe::egui;
    use wasm_bindgen::JsCast as _;

    pub(super) fn browser_support_issue(size: egui::Vec2) -> Option<BrowserSupportIssue> {
        let viewport_size = canvas_css_size().unwrap_or(size);
        let portrait = viewport_size.y >= viewport_size.x;
        let fits = if portrait {
            viewport_size.x >= MIN_PORTRAIT_WIDTH && viewport_size.y >= MIN_PORTRAIT_HEIGHT
        } else {
            viewport_size.x >= MIN_PORTRAIT_HEIGHT && viewport_size.y >= MIN_PORTRAIT_WIDTH
        };

        if !fits {
            Some(BrowserSupportIssue::ViewportTooSmall)
        } else {
            None
        }
    }

    pub(super) fn prefers_touch_controls() -> bool {
        web_sys::window()
            .and_then(|window| {
                window
                    .match_media("(hover: none) and (pointer: coarse)")
                    .ok()
                    .flatten()
            })
            .is_some_and(|query| query.matches())
    }

    pub(super) fn fullscreen_available() -> bool {
        web_sys::window()
            .and_then(|window| window.document())
            .is_some_and(|document| document.fullscreen_enabled())
    }

    pub(super) fn sync_canvas_resolution() -> bool {
        let Some(window) = web_sys::window() else {
            return false;
        };
        let Some(canvas) = window
            .document()
            .and_then(|document| document.get_element_by_id(CANVAS_ID))
            .and_then(|element| element.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        else {
            return false;
        };
        let css_width = canvas.client_width().max(0) as f64;
        let css_height = canvas.client_height().max(0) as f64;
        let scale = window.device_pixel_ratio();
        let desired_width = (css_width * scale).round() as u32;
        let desired_height = (css_height * scale).round() as u32;
        if desired_width == 0
            || desired_height == 0
            || (canvas.width() == desired_width && canvas.height() == desired_height)
        {
            return false;
        }
        canvas.set_width(desired_width);
        canvas.set_height(desired_height);
        true
    }

    pub(super) fn load_best_score() -> u64 {
        local_storage()
            .and_then(|storage| storage.get_item(BEST_SCORE_KEY).ok().flatten())
            .and_then(|score| score.parse().ok())
            .unwrap_or(0)
    }

    pub(super) fn save_best_score(score: u64) {
        if let Some(storage) = local_storage() {
            let _ = storage.set_item(BEST_SCORE_KEY, &score.to_string());
        }
    }

    pub(super) fn set_accessible_status(screen: &str, message: &str) {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        if let Some(canvas) = document.get_element_by_id(CANVAS_ID) {
            let _ = canvas.set_attribute("data-screen", screen);
        }
        if let Some(status) = document.get_element_by_id("app_status") {
            status.set_text_content(Some(message));
        }
    }

    pub(super) fn set_canvas_layout(layout: &str, touch_controls: bool) {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        if let Some(canvas) = document.get_element_by_id(CANVAS_ID) {
            let _ = canvas.set_attribute("data-layout", layout);
            let _ = canvas.set_attribute(
                "data-touch-controls",
                if touch_controls { "visible" } else { "hidden" },
            );
        }
    }

    pub(super) fn set_canvas_touch_metadata(regions: &str, active: &str) {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        if let Some(canvas) = document.get_element_by_id(CANVAS_ID) {
            let _ = canvas.set_attribute("data-touch-regions", regions);
            let _ = canvas.set_attribute("data-touch-active", active);
        }
    }

    pub(super) fn toggle_fullscreen(_context: &egui::Context) {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        if document.fullscreen_element().is_some() {
            document.exit_fullscreen();
        } else if let Some(canvas) = document.get_element_by_id(CANVAS_ID) {
            let _ = canvas.request_fullscreen();
        }
    }

    fn local_storage() -> Option<web_sys::Storage> {
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    }

    fn canvas_css_size() -> Option<egui::Vec2> {
        let canvas = web_sys::window()?
            .document()?
            .get_element_by_id(CANVAS_ID)?
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .ok()?;
        Some(egui::vec2(
            canvas.client_width() as f32,
            canvas.client_height() as f32,
        ))
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::BrowserSupportIssue;
    use eframe::egui;

    pub(super) fn browser_support_issue(_size: egui::Vec2) -> Option<BrowserSupportIssue> {
        None
    }

    pub(super) fn load_best_score() -> u64 {
        0
    }

    pub(super) fn prefers_touch_controls() -> bool {
        false
    }

    pub(super) fn fullscreen_available() -> bool {
        true
    }

    pub(super) fn sync_canvas_resolution() -> bool {
        false
    }

    pub(super) fn save_best_score(_score: u64) {}

    pub(super) fn set_accessible_status(_screen: &str, _message: &str) {}

    pub(super) fn set_canvas_layout(_layout: &str, _touch_controls: bool) {}

    pub(super) fn set_canvas_touch_metadata(_regions: &str, _active: &str) {}

    pub(super) fn toggle_fullscreen(context: &egui::Context) {
        let is_fullscreen = context.input(|input| input.viewport().fullscreen.unwrap_or(false));
        context.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fullscreen));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_build_has_no_browser_support_gate() {
        assert_eq!(browser_support_issue(egui::vec2(320.0, 240.0)), None);
    }
}
