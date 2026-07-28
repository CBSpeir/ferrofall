use eframe::egui::{self, Color32};

pub(crate) const PREFERENCE_KEY: &str = "oxidefall.theme.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Palette {
    pub(crate) background: Color32,
    pub(crate) surface: Color32,
    pub(crate) well: Color32,
    pub(crate) grid: Color32,
    pub(crate) divider: Color32,
    pub(crate) border: Color32,
    pub(crate) text: Color32,
    pub(crate) muted: Color32,
    pub(crate) accent: Color32,
    pub(crate) accent_bright: Color32,
    pub(crate) accent_text: Color32,
    pub(crate) accent_foreground: Color32,
    pub(crate) button_housing: Color32,
    pub(crate) button_housing_edge: Color32,
    pub(crate) button_face: Color32,
    pub(crate) button_face_hover: Color32,
    pub(crate) button_face_active: Color32,
    pub(crate) button_edge_hover: Color32,
    pub(crate) background_grid: Color32,
    pub(crate) overlay_scrim: Color32,
}

impl Palette {
    pub(crate) const DARK: Self = Self {
        background: Color32::from_rgb(7, 16, 24),
        surface: Color32::from_rgb(13, 26, 36),
        well: Color32::from_rgb(8, 18, 27),
        grid: Color32::from_rgb(30, 51, 64),
        divider: Color32::from_rgb(30, 51, 64),
        border: Color32::from_rgb(76, 99, 112),
        text: Color32::from_rgb(218, 226, 230),
        muted: Color32::from_rgb(126, 148, 160),
        accent: Color32::from_rgb(225, 153, 42),
        accent_bright: Color32::from_rgb(244, 177, 66),
        accent_text: Color32::from_rgb(225, 153, 42),
        accent_foreground: Color32::from_rgb(7, 16, 24),
        button_housing: Color32::from_rgb(3, 9, 15),
        button_housing_edge: Color32::from_rgb(22, 39, 49),
        button_face: Color32::from_rgb(10, 23, 33),
        button_face_hover: Color32::from_rgb(18, 36, 47),
        button_face_active: Color32::from_rgb(28, 48, 59),
        button_edge_hover: Color32::from_rgb(135, 158, 169),
        background_grid: Color32::from_rgba_premultiplied(8, 13, 17, 70),
        overlay_scrim: Color32::from_black_alpha(205),
    };

    pub(crate) const LIGHT: Self = Self {
        background: Color32::from_rgb(235, 232, 223),
        surface: Color32::from_rgb(248, 245, 237),
        well: Color32::from_rgb(8, 18, 27),
        grid: Color32::from_rgb(30, 51, 64),
        divider: Color32::from_rgb(179, 183, 181),
        border: Color32::from_rgb(111, 125, 132),
        text: Color32::from_rgb(24, 39, 48),
        muted: Color32::from_rgb(75, 94, 104),
        accent: Color32::from_rgb(183, 105, 13),
        accent_bright: Color32::from_rgb(207, 129, 24),
        accent_text: Color32::from_rgb(126, 70, 7),
        accent_foreground: Color32::from_rgb(7, 16, 24),
        button_housing: Color32::from_rgb(174, 168, 157),
        button_housing_edge: Color32::from_rgb(113, 107, 98),
        button_face: Color32::from_rgb(246, 242, 233),
        button_face_hover: Color32::from_rgb(255, 253, 247),
        button_face_active: Color32::from_rgb(219, 214, 203),
        button_edge_hover: Color32::from_rgb(73, 91, 101),
        background_grid: Color32::from_rgba_premultiplied(8, 10, 11, 32),
        overlay_scrim: Color32::from_rgba_premultiplied(5, 11, 16, 172),
    };

    pub(crate) const fn for_theme(theme: egui::Theme) -> Self {
        match theme {
            egui::Theme::Dark => Self::DARK,
            egui::Theme::Light => Self::LIGHT,
        }
    }
}

pub(crate) fn parse_preference(value: &str) -> Option<egui::ThemePreference> {
    match value {
        "system" => Some(egui::ThemePreference::System),
        "light" => Some(egui::ThemePreference::Light),
        "dark" => Some(egui::ThemePreference::Dark),
        _ => None,
    }
}

pub(crate) const fn preference_value(preference: egui::ThemePreference) -> &'static str {
    match preference {
        egui::ThemePreference::System => "system",
        egui::ThemePreference::Light => "light",
        egui::ThemePreference::Dark => "dark",
    }
}

pub(crate) const fn preference_label(preference: egui::ThemePreference) -> &'static str {
    match preference {
        egui::ThemePreference::System => "SYSTEM",
        egui::ThemePreference::Light => "LIGHT",
        egui::ThemePreference::Dark => "DARK",
    }
}

pub(crate) const fn theme_label(theme: egui::Theme) -> &'static str {
    match theme {
        egui::Theme::Dark => "DARK",
        egui::Theme::Light => "LIGHT",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_preferences_round_trip() {
        for preference in [
            egui::ThemePreference::System,
            egui::ThemePreference::Light,
            egui::ThemePreference::Dark,
        ] {
            assert_eq!(
                parse_preference(preference_value(preference)),
                Some(preference)
            );
        }
        assert_eq!(parse_preference("sepia"), None);
    }

    #[test]
    fn light_palette_keeps_playfield_dark() {
        assert_eq!(Palette::LIGHT.well, Palette::DARK.well);
        assert_eq!(Palette::LIGHT.grid, Palette::DARK.grid);
        assert_ne!(Palette::LIGHT.background, Palette::DARK.background);
    }
}
