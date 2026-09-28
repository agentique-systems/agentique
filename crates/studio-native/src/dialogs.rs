//! The dialogs of the Studio.
use crate::studio::Studio;
use gpui::{Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div};

pub struct DialogsView {
    #[allow(dead_code)]
    studio: Entity<Studio>,
}

impl DialogsView {
    pub fn new(studio: Entity<Studio>, cx: &mut Context<Self>) -> Self {
        let _ = cx;
        DialogsView { studio }
    }
}

impl Render for DialogsView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child("DialogsView")
    }
}
