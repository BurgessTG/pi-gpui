use gpui::{Div, Styled as _, div};

use crate::design::theme;

pub fn quiet_panel() -> Div {
    div()
        .border_1()
        .border_color(theme::hairline())
        .bg(theme::app_bg())
}
