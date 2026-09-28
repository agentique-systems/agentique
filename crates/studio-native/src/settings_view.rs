//! The Settings view (§3.7).
use crate::studio::Studio;
use gpui::{Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div};

pub struct SettingsView {
    #[allow(dead_code)]
    studio: Entity<Studio>,
}

impl SettingsView {
    pub fn new(studio: Entity<Studio>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let _ = cx;
        SettingsView { studio }
    }
}

impl SettingsView {
    pub fn focus(&mut self, _: &mut Window, _: &mut Context<Self>) {}
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child("SettingsView")
    }
}
