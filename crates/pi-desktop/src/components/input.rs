use gpui::{Div, Entity, ParentElement as _, Styled as _, div};
use gpui_component::input::{Input, InputState};

use crate::design::theme;

pub fn pi_input(state: &Entity<InputState>) -> Div {
    div()
        .border_1()
        .border_color(theme::hairline())
        .bg(theme::surface())
        .child(Input::new(state).appearance(false))
}
