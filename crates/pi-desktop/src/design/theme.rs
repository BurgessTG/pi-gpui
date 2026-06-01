use std::sync::{OnceLock, RwLock};

use gpui::{App, Hsla, SharedString, Window, hsla, px, rgb};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ThemePreset {
    Pi,
    Dark,
    Light,
}

impl ThemePreset {
    pub(crate) const ALL: [Self; 3] = [Self::Pi, Self::Dark, Self::Light];

    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Pi => "pi",
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Pi => "Pi",
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }

    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::Pi => "Pi blue slate with warm canvas contrast.",
            Self::Dark => "Neutral low-glare dark surfaces.",
            Self::Light => "Warm paper surface for bright rooms.",
        }
    }

    pub(crate) fn palette(self) -> Palette {
        match self {
            Self::Pi => Palette {
                app_bg: rgb(0x161d27).into(),
                surface: rgb(0x212730).into(),
                surface_hover: rgb(0x252f3d).into(),
                surface_selected: rgb(0x252f3d).into(),
                text: rgb(0xebe7e4).into(),
                text_muted: rgb(0x9fa4ab).into(),
                hairline: rgb(0x495059).into(),
                accent: rgb(0x6a9fcc).into(),
                complement: rgb(0xcc976a).into(),
                danger: rgb(0xe8704f).into(),
                danger_soft: rgb(0x2b1a18).into(),
                success: rgb(0x5db87a).into(),
                grid_minor: hsla(218.0 / 360.0, 0.60, 0.80, 0.028),
                grid_major: hsla(218.0 / 360.0, 0.60, 0.80, 0.075),
            },
            Self::Dark => Palette {
                app_bg: rgb(0x101114).into(),
                surface: rgb(0x1a1d22).into(),
                surface_hover: rgb(0x232833).into(),
                surface_selected: rgb(0x2b303b).into(),
                text: rgb(0xf1f3f5).into(),
                text_muted: rgb(0xa4abb6).into(),
                hairline: rgb(0x3a404a).into(),
                accent: rgb(0x8aa4ff).into(),
                complement: rgb(0xd2a35f).into(),
                danger: rgb(0xff746b).into(),
                danger_soft: rgb(0x2a1517).into(),
                success: rgb(0x65c587).into(),
                grid_minor: hsla(226.0 / 360.0, 0.30, 0.86, 0.026),
                grid_major: hsla(226.0 / 360.0, 0.30, 0.86, 0.070),
            },
            Self::Light => Palette {
                app_bg: rgb(0xf5f1ea).into(),
                surface: rgb(0xfffbf5).into(),
                surface_hover: rgb(0xebe4d9).into(),
                surface_selected: rgb(0xe2d8cb).into(),
                text: rgb(0x1f252d).into(),
                text_muted: rgb(0x69707a).into(),
                hairline: rgb(0xc6bdb0).into(),
                accent: rgb(0x2f6f9f).into(),
                complement: rgb(0x9a6235).into(),
                danger: rgb(0xb64b36).into(),
                danger_soft: rgb(0xf4ddd6).into(),
                success: rgb(0x2e7d4f).into(),
                grid_minor: hsla(211.0 / 360.0, 0.42, 0.27, 0.035),
                grid_major: hsla(211.0 / 360.0, 0.42, 0.27, 0.090),
            },
        }
    }

    fn component_mode(self) -> gpui_component::ThemeMode {
        match self {
            Self::Pi | Self::Dark => gpui_component::ThemeMode::Dark,
            Self::Light => gpui_component::ThemeMode::Light,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppFont {
    System,
    Plantin,
    Mono,
}

impl AppFont {
    pub(crate) const ALL: [Self; 3] = [Self::System, Self::Plantin, Self::Mono];

    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Plantin => "plantin",
            Self::Mono => "mono",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Plantin => "Plantin",
            Self::Mono => "Mono",
        }
    }

    pub(crate) fn family(self) -> &'static str {
        match self {
            Self::System => ".SystemUIFont",
            Self::Plantin => "PlantinNowVariable-Upright",
            Self::Mono => "DejaVu Sans Mono",
        }
    }

    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::System => "Native platform UI font.",
            Self::Plantin => "Bundled Pi editorial serif.",
            Self::Mono => "Monospace for dense work.",
        }
    }

    pub(crate) fn preview(self) -> &'static str {
        match self {
            Self::System => "Ask Pi to reason through the next step.",
            Self::Plantin => "Ask Pi to reason through the next step.",
            Self::Mono => "fn ask_pi(next_step: Plan) -> Result<()>;",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AppearanceSettings {
    pub(crate) preset: ThemePreset,
    pub(crate) font: AppFont,
}

impl AppearanceSettings {
    pub(crate) fn with_preset(self, preset: ThemePreset) -> Self {
        Self { preset, ..self }
    }

    pub(crate) fn with_font(self, font: AppFont) -> Self {
        Self { font, ..self }
    }
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            preset: ThemePreset::Pi,
            font: AppFont::System,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Palette {
    pub(crate) app_bg: Hsla,
    pub(crate) surface: Hsla,
    pub(crate) surface_hover: Hsla,
    pub(crate) surface_selected: Hsla,
    pub(crate) text: Hsla,
    pub(crate) text_muted: Hsla,
    pub(crate) hairline: Hsla,
    pub(crate) accent: Hsla,
    pub(crate) complement: Hsla,
    pub(crate) danger: Hsla,
    pub(crate) danger_soft: Hsla,
    pub(crate) success: Hsla,
    pub(crate) grid_minor: Hsla,
    pub(crate) grid_major: Hsla,
}

static APPEARANCE: OnceLock<RwLock<AppearanceSettings>> = OnceLock::new();

pub(crate) fn current_appearance() -> AppearanceSettings {
    let lock = APPEARANCE.get_or_init(|| RwLock::new(AppearanceSettings::default()));
    match lock.read() {
        Ok(guard) => *guard,
        Err(error) => *error.into_inner(),
    }
}

pub(crate) fn set_appearance(settings: AppearanceSettings) {
    let lock = APPEARANCE.get_or_init(|| RwLock::new(AppearanceSettings::default()));
    match lock.write() {
        Ok(mut guard) => *guard = settings,
        Err(error) => *error.into_inner() = settings,
    }
}

pub(crate) fn apply_component_theme(cx: &mut App) {
    let settings = current_appearance();
    gpui_component::Theme::change(settings.preset.component_mode(), None, cx);

    let palette = settings.preset.palette();
    let theme = gpui_component::Theme::global_mut(cx);
    theme.radius = px(0.0);
    theme.radius_lg = px(0.0);
    theme.shadow = false;
    theme.tile_radius = px(0.0);
    theme.tile_shadow = false;

    theme.font_family = SharedString::from(settings.font.family());
    if settings.font == AppFont::Mono {
        theme.mono_font_family = SharedString::from(settings.font.family());
    }

    theme.background = palette.app_bg;
    theme.foreground = palette.text;
    theme.border = palette.hairline;
    theme.ring = palette.accent;

    theme.popover = palette.surface;
    theme.popover_foreground = palette.text;

    theme.list = palette.surface;
    theme.list_head = palette.surface;
    theme.list_hover = palette.surface_hover;
    theme.list_active = palette.surface_selected;
    theme.list_active_border = palette.hairline;
    theme.list_even = palette.surface;

    theme.muted = palette.surface_hover;
    theme.muted_foreground = palette.text_muted;
    theme.input = palette.hairline;
    theme.selection = palette.accent.opacity(0.28);

    theme.primary = palette.accent;
    theme.primary_hover = palette.text_muted;
    theme.primary_active = palette.accent;
    theme.primary_foreground = palette.app_bg;
    theme.secondary = palette.surface;
    theme.secondary_hover = palette.surface_hover;
    theme.secondary_active = palette.surface_selected;
    theme.secondary_foreground = palette.text;

    theme.danger = palette.danger;
    theme.danger_hover = palette.complement;
    theme.danger_active = palette.danger;
    theme.danger_foreground = palette.app_bg;

    theme.sidebar = palette.surface;
    theme.sidebar_foreground = palette.text;
    theme.sidebar_border = palette.hairline;
    theme.sidebar_accent = palette.surface_hover;
    theme.sidebar_accent_foreground = palette.text;
    theme.sidebar_primary = palette.accent;
    theme.sidebar_primary_foreground = palette.app_bg;

    theme.accent = palette.surface_hover;
    theme.accent_foreground = palette.text;

    theme.tab = palette.surface;
    theme.tab_bar = palette.surface;
    theme.tab_bar_segmented = palette.app_bg;
    theme.tab_active = palette.surface_selected;
    theme.tab_active_foreground = palette.text;
    theme.tab_foreground = palette.text_muted;

    theme.scrollbar = palette.surface;
    theme.scrollbar_thumb = palette.hairline;
    theme.scrollbar_thumb_hover = palette.text_muted;
}

pub(crate) fn apply_component_theme_for_window(window: &mut Window, cx: &mut App) {
    apply_component_theme(cx);
    window.refresh();
}

fn current_palette() -> Palette {
    current_appearance().preset.palette()
}

pub fn app_bg() -> Hsla {
    current_palette().app_bg
}

pub fn surface() -> Hsla {
    current_palette().surface
}

pub fn surface_hover() -> Hsla {
    current_palette().surface_hover
}

pub fn surface_selected() -> Hsla {
    current_palette().surface_selected
}

pub fn text() -> Hsla {
    current_palette().text
}

pub fn text_muted() -> Hsla {
    current_palette().text_muted
}

pub fn hairline() -> Hsla {
    current_palette().hairline
}

pub fn accent() -> Hsla {
    current_palette().accent
}

pub fn complement() -> Hsla {
    current_palette().complement
}

pub fn danger() -> Hsla {
    current_palette().danger
}

pub fn danger_soft() -> Hsla {
    current_palette().danger_soft
}

pub fn success() -> Hsla {
    current_palette().success
}

pub fn grid_minor() -> Hsla {
    current_palette().grid_minor
}

pub fn grid_major() -> Hsla {
    current_palette().grid_major
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_have_distinct_backgrounds() {
        assert_ne!(
            ThemePreset::Pi.palette().app_bg,
            ThemePreset::Dark.palette().app_bg
        );
        assert_ne!(
            ThemePreset::Dark.palette().app_bg,
            ThemePreset::Light.palette().app_bg
        );
    }

    #[test]
    fn font_options_have_families() {
        for font in AppFont::ALL {
            assert!(!font.family().is_empty());
            assert!(!font.label().is_empty());
        }
    }
}
