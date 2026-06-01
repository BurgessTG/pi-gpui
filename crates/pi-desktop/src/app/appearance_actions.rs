use super::*;

impl PiDesktop {
    pub(crate) fn set_appearance(
        &mut self,
        appearance: theme::AppearanceSettings,
        status: String,
        cx: &mut Context<Self>,
    ) {
        self.appearance = appearance;
        self.status = status.into();
        cx.notify();
    }
}
