use eframe::egui;

#[cfg(target_arch = "wasm32")]
pub(crate) const CANVAS_ID: &str = "ferrofall_canvas";
#[cfg(target_arch = "wasm32")]
pub(crate) const LOADING_ID: &str = "loading";

#[cfg(target_arch = "wasm32")]
const BEST_SCORE_KEY: &str = "ferrofall.best-score.v1";
#[cfg(target_arch = "wasm32")]
const MIN_VIEWPORT_WIDTH: f32 = 720.0;
#[cfg(target_arch = "wasm32")]
const MIN_VIEWPORT_HEIGHT: f32 = 560.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) enum BrowserSupportIssue {
    TouchOnly,
    ViewportTooSmall,
}

pub(crate) fn browser_support_issue(size: egui::Vec2) -> Option<BrowserSupportIssue> {
    imp::browser_support_issue(size)
}

pub(crate) fn load_best_score() -> u64 {
    imp::load_best_score()
}

pub(crate) fn save_best_score(score: u64) {
    imp::save_best_score(score);
}

pub(crate) fn set_accessible_status(screen: &str, message: &str) {
    imp::set_accessible_status(screen, message);
}

pub(crate) fn toggle_fullscreen(context: &egui::Context) {
    imp::toggle_fullscreen(context);
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::{
        BEST_SCORE_KEY, BrowserSupportIssue, CANVAS_ID, MIN_VIEWPORT_HEIGHT, MIN_VIEWPORT_WIDTH,
    };
    use eframe::egui;

    pub(super) fn browser_support_issue(size: egui::Vec2) -> Option<BrowserSupportIssue> {
        let window = web_sys::window();
        let touch_only = window
            .as_ref()
            .and_then(|window| {
                window
                    .match_media("(hover: none) and (pointer: coarse)")
                    .ok()
                    .flatten()
            })
            .is_some_and(|query| query.matches());

        // eframe can adjust its logical points-per-pixel scale to keep the UI
        // usable in a compact canvas. The support boundary is a browser CSS
        // viewport requirement, so read the viewport directly rather than
        // deriving it from egui's scaled coordinate space.
        let viewport_size = window
            .and_then(|window| {
                Some(egui::vec2(
                    window.inner_width().ok()?.as_f64()? as f32,
                    window.inner_height().ok()?.as_f64()? as f32,
                ))
            })
            .unwrap_or(size);

        if touch_only {
            Some(BrowserSupportIssue::TouchOnly)
        } else if viewport_size.x < MIN_VIEWPORT_WIDTH || viewport_size.y < MIN_VIEWPORT_HEIGHT {
            Some(BrowserSupportIssue::ViewportTooSmall)
        } else {
            None
        }
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

    pub(super) fn save_best_score(_score: u64) {}

    pub(super) fn set_accessible_status(_screen: &str, _message: &str) {}

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
