use gpui::{Hsla, rgb};

pub const SERIF_FONT: &str = "Plantin MT Pro";

pub fn app_bg() -> Hsla {
    rgb(0x161d27).into()
}

pub fn surface() -> Hsla {
    rgb(0x212730).into()
}

pub fn surface_hover() -> Hsla {
    rgb(0x252f3d).into()
}

pub fn surface_selected() -> Hsla {
    rgb(0x252f3d).into()
}

pub fn text() -> Hsla {
    rgb(0xebe7e4).into()
}

pub fn text_muted() -> Hsla {
    rgb(0x9fa4ab).into()
}

pub fn hairline() -> Hsla {
    rgb(0x495059).into()
}

pub fn accent() -> Hsla {
    rgb(0x6a9fcc).into()
}

pub fn danger() -> Hsla {
    rgb(0xe8704f).into()
}

pub fn danger_soft() -> Hsla {
    rgb(0x2b1a18).into()
}

pub fn success() -> Hsla {
    rgb(0x5db87a).into()
}

pub fn grid_minor() -> Hsla {
    gpui::hsla(218.0 / 360.0, 0.60, 0.80, 0.028)
}

pub fn grid_major() -> Hsla {
    gpui::hsla(218.0 / 360.0, 0.60, 0.80, 0.075)
}
