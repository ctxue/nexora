use eframe::egui::{self, Color32};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsTheme {
    #[default]
    System,
    Light,
    Dark,
}
impl SettingsTheme {
    pub fn key(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
    pub fn parse(value: &str) -> Self {
        match value {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }
    pub fn dark(self, system_dark: bool) -> bool {
        match self {
            Self::System => system_dark,
            Self::Light => false,
            Self::Dark => true,
        }
    }
}

pub fn system_dark(ctx: &egui::Context) -> bool {
    let id = egui::Id::new("settings-system-theme");
    if let Some((at, dark)) = ctx.data(|d| d.get_temp::<(std::time::Instant, bool)>(id)) {
        if at.elapsed() < std::time::Duration::from_secs(1) {
            return dark;
        }
    }
    #[cfg(windows)]
    let dark = {
        use windows::{core::w, Win32::System::Registry::*};
        let mut value = 1u32;
        let mut size = 4u32;
        unsafe {
            let _ = RegGetValueW(
                HKEY_CURRENT_USER,
                w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
                w!("AppsUseLightTheme"),
                RRF_RT_REG_DWORD,
                None,
                Some((&mut value as *mut u32).cast()),
                Some(&mut size),
            );
        }
        value == 0
    };
    #[cfg(not(windows))]
    let dark = ctx.input(|i| i.raw.system_theme) == Some(egui::Theme::Dark);
    ctx.data_mut(|d| d.insert_temp(id, (std::time::Instant::now(), dark)));
    dark
}

pub fn color(ctx: &egui::Context, dark: Color32, light: Color32) -> Color32 {
    if ctx.style().visuals.dark_mode {
        dark
    } else {
        light
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub chrome: Color32,
    pub surface: Color32,
    pub surface_subtle: Color32,
    pub surface_hover: Color32,
    pub surface_selected: Color32,
    pub text: Color32,
    pub text_title: Color32,
    pub text_soft: Color32,
    pub text_medium: Color32,
    pub muted: Color32,
    pub subtle: Color32,
    pub line: Color32,
    pub line_strong: Color32,
    pub line_soft: Color32,
    pub line_muted: Color32,
    pub line_emphasis: Color32,
    pub control_bg: Color32,
    pub control_bg_hover: Color32,
    pub key_bg: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    pub toggle_off: Color32,
    pub knob_off: Color32,
    pub danger_text: Color32,
    pub danger_bg: Color32,
    pub danger_border: Color32,
}

pub fn palette(ctx: &egui::Context) -> Palette {
    palette_for(ctx.style().visuals.dark_mode)
}

pub fn palette_for(dark: bool) -> Palette {
    if dark {
        Palette {
            chrome: Color32::from_rgb(19, 19, 22),
            surface: Color32::from_rgb(27, 27, 31),
            surface_subtle: Color32::from_rgb(35, 35, 39),
            surface_hover: Color32::from_rgb(38, 38, 44),
            surface_selected: Color32::from_rgb(44, 44, 51),
            text: Color32::from_rgb(244, 244, 245),
            text_title: Color32::from_rgb(228, 228, 231),
            text_soft: Color32::from_rgb(212, 212, 216),
            text_medium: Color32::from_rgb(180, 180, 187),
            muted: Color32::from_rgb(161, 161, 170),
            subtle: Color32::from_rgb(113, 113, 122),
            line: Color32::from_rgb(51, 51, 58),
            line_strong: Color32::from_rgb(63, 63, 70),
            line_soft: Color32::from_rgb(43, 43, 49),
            line_muted: Color32::from_rgb(46, 46, 53),
            line_emphasis: Color32::from_rgb(74, 74, 83),
            control_bg: Color32::from_rgb(38, 38, 43),
            control_bg_hover: Color32::from_rgb(45, 45, 52),
            key_bg: Color32::from_rgb(42, 42, 48),
            accent: Color32::from_rgb(228, 228, 231),
            accent_text: Color32::from_rgb(24, 24, 27),
            toggle_off: Color32::from_rgb(63, 63, 70),
            knob_off: Color32::from_rgb(212, 212, 216),
            danger_text: Color32::from_rgb(252, 165, 165),
            danger_bg: Color32::from_rgb(42, 22, 24),
            danger_border: Color32::from_rgb(74, 34, 37),
        }
    } else {
        Palette {
            chrome: Color32::from_rgb(237, 237, 240),
            surface: Color32::WHITE,
            surface_subtle: Color32::from_rgb(250, 250, 250),
            surface_hover: Color32::from_rgb(247, 247, 248),
            surface_selected: Color32::from_rgb(244, 244, 245),
            text: Color32::from_rgb(24, 24, 27),
            text_title: Color32::from_rgb(39, 39, 42),
            text_soft: Color32::from_rgb(63, 63, 70),
            text_medium: Color32::from_rgb(82, 82, 91),
            muted: Color32::from_rgb(113, 113, 122),
            subtle: Color32::from_rgb(139, 139, 147),
            line: Color32::from_rgb(240, 240, 242),
            line_strong: Color32::from_rgb(228, 228, 231),
            line_soft: Color32::from_rgb(236, 236, 239),
            line_muted: Color32::from_rgb(233, 233, 236),
            line_emphasis: Color32::from_rgb(212, 212, 216),
            control_bg: Color32::WHITE,
            control_bg_hover: Color32::from_rgb(248, 248, 248),
            key_bg: Color32::from_rgb(250, 250, 250),
            accent: Color32::from_rgb(24, 24, 27),
            accent_text: Color32::from_rgb(250, 250, 250),
            toggle_off: Color32::from_rgb(228, 228, 231),
            knob_off: Color32::WHITE,
            danger_text: Color32::from_rgb(185, 28, 28),
            danger_bg: Color32::from_rgb(254, 242, 242),
            danger_border: Color32::from_rgb(254, 202, 202),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_theme_palette_keeps_chrome_and_surface_consistent() {
        let light = palette_for(false);
        let dark = palette_for(true);

        assert_eq!(light.chrome, Color32::from_rgb(237, 237, 240));
        assert_eq!(light.surface, Color32::WHITE);
        assert_eq!(dark.chrome, Color32::from_rgb(19, 19, 22));
        assert_eq!(dark.surface, Color32::from_rgb(27, 27, 31));
        assert_ne!(light.surface, dark.surface);
    }
}
