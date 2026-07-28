use eframe::egui::{self, Color32};

pub(crate) const PREFERENCE_KEY: &str = "oxidefall.theme.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Palette {
    pub(crate) background: Color32,
    pub(crate) surface: Color32,
    pub(crate) well: Color32,
    pub(crate) well_border: Color32,
    pub(crate) grid: Color32,
    pub(crate) divider: Color32,
    pub(crate) border: Color32,
    pub(crate) text: Color32,
    pub(crate) muted: Color32,
    pub(crate) accent: Color32,
    pub(crate) accent_bright: Color32,
    pub(crate) accent_pressed: Color32,
    pub(crate) accent_text: Color32,
    pub(crate) accent_foreground: Color32,
    pub(crate) selected_fill: Color32,
    pub(crate) button_housing: Color32,
    pub(crate) button_housing_edge: Color32,
    pub(crate) button_face: Color32,
    pub(crate) button_face_hover: Color32,
    pub(crate) button_face_active: Color32,
    pub(crate) button_edge_hover: Color32,
    pub(crate) button_highlight: Color32,
    pub(crate) accent_highlight: Color32,
    pub(crate) shadow: Color32,
    pub(crate) background_grid: Color32,
    pub(crate) overlay_scrim: Color32,
}

impl Palette {
    pub(crate) const DARK: Self = Self {
        background: Color32::from_rgb(13, 26, 34),
        surface: Color32::from_rgb(20, 38, 48),
        well: Color32::from_rgb(11, 24, 32),
        well_border: Color32::from_rgb(55, 82, 95),
        grid: Color32::from_rgb(32, 56, 68),
        divider: Color32::from_rgb(38, 62, 73),
        border: Color32::from_rgb(82, 107, 119),
        text: Color32::from_rgb(232, 238, 240),
        muted: Color32::from_rgb(154, 174, 183),
        accent: Color32::from_rgb(232, 162, 58),
        accent_bright: Color32::from_rgb(243, 187, 93),
        accent_pressed: Color32::from_rgb(200, 130, 36),
        accent_text: Color32::from_rgb(239, 180, 81),
        accent_foreground: Color32::from_rgb(7, 16, 24),
        selected_fill: Color32::from_rgb(59, 51, 36),
        button_housing: Color32::from_rgb(8, 18, 24),
        button_housing_edge: Color32::from_rgb(43, 66, 77),
        button_face: Color32::from_rgb(22, 42, 53),
        button_face_hover: Color32::from_rgb(29, 53, 65),
        button_face_active: Color32::from_rgb(16, 35, 45),
        button_edge_hover: Color32::from_rgb(142, 166, 177),
        button_highlight: Color32::from_rgba_premultiplied(59, 65, 67, 72),
        accent_highlight: Color32::from_rgba_premultiplied(120, 109, 83, 120),
        shadow: Color32::from_rgba_premultiplied(1, 3, 5, 96),
        background_grid: Color32::from_rgba_premultiplied(8, 10, 11, 18),
        overlay_scrim: Color32::from_rgba_premultiplied(3, 9, 13, 178),
    };

    pub(crate) const LIGHT: Self = Self {
        background: Color32::from_rgb(241, 239, 232),
        surface: Color32::from_rgb(250, 248, 242),
        well: Color32::from_rgb(11, 24, 32),
        well_border: Color32::from_rgb(55, 82, 95),
        grid: Color32::from_rgb(32, 56, 68),
        divider: Color32::from_rgb(214, 209, 199),
        border: Color32::from_rgb(121, 137, 145),
        text: Color32::from_rgb(23, 42, 51),
        muted: Color32::from_rgb(82, 105, 114),
        accent: Color32::from_rgb(185, 104, 15),
        accent_bright: Color32::from_rgb(205, 127, 29),
        accent_pressed: Color32::from_rgb(183, 103, 15),
        accent_text: Color32::from_rgb(146, 80, 12),
        accent_foreground: Color32::from_rgb(7, 16, 24),
        selected_fill: Color32::from_rgb(245, 232, 210),
        button_housing: Color32::from_rgb(200, 194, 183),
        button_housing_edge: Color32::from_rgb(142, 138, 130),
        button_face: Color32::from_rgb(248, 246, 239),
        button_face_hover: Color32::from_rgb(255, 254, 249),
        button_face_active: Color32::from_rgb(230, 226, 216),
        button_edge_hover: Color32::from_rgb(83, 107, 117),
        button_highlight: Color32::from_rgba_premultiplied(170, 170, 168, 170),
        accent_highlight: Color32::from_rgba_premultiplied(122, 108, 81, 122),
        shadow: Color32::from_rgba_premultiplied(4, 9, 12, 70),
        background_grid: Color32::from_rgba_premultiplied(5, 6, 7, 18),
        overlay_scrim: Color32::from_rgba_premultiplied(5, 12, 16, 150),
    };

    pub(crate) const fn for_theme(theme: egui::Theme) -> Self {
        match theme {
            egui::Theme::Dark => Self::DARK,
            egui::Theme::Light => Self::LIGHT,
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) const fn browser_background_hex(theme: egui::Theme) -> &'static str {
    match theme {
        egui::Theme::Dark => "#0d1a22",
        egui::Theme::Light => "#f1efe8",
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
        assert_eq!(Palette::LIGHT.well_border, Palette::DARK.well_border);
        assert_eq!(Palette::LIGHT.grid, Palette::DARK.grid);
        assert_ne!(Palette::LIGHT.background, Palette::DARK.background);
    }

    #[test]
    fn functional_text_meets_wcag_aa_contrast() {
        for palette in [Palette::DARK, Palette::LIGHT] {
            assert!(contrast_ratio(palette.text, palette.background) >= 4.5);
            assert!(contrast_ratio(palette.muted, palette.background) >= 4.5);
            assert!(contrast_ratio(palette.text, palette.surface) >= 4.5);
            assert!(contrast_ratio(palette.accent_foreground, palette.accent) >= 4.5);
            assert!(contrast_ratio(palette.accent_text, palette.surface) >= 4.5);
        }
    }

    fn contrast_ratio(left: Color32, right: Color32) -> f32 {
        let left = relative_luminance(left);
        let right = relative_luminance(right);
        let (lighter, darker) = if left >= right {
            (left, right)
        } else {
            (right, left)
        };
        (lighter + 0.05) / (darker + 0.05)
    }

    fn relative_luminance(color: Color32) -> f32 {
        [color.r(), color.g(), color.b()]
            .into_iter()
            .zip([0.2126, 0.7152, 0.0722])
            .map(|(channel, weight)| {
                let channel = f32::from(channel) / 255.0;
                let linear = if channel <= 0.04045 {
                    channel / 12.92
                } else {
                    ((channel + 0.055) / 1.055).powf(2.4)
                };
                linear * weight
            })
            .sum()
    }
}
