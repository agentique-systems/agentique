//! The component gallery (`--fixture components`, R-26).
use gpui::{Context, IntoElement, ParentElement, Render, Styled, Window, div};

pub struct Gallery;

impl Gallery {
    pub fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
        Gallery
    }
}

impl Render for Gallery {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child("Gallery")
    }
}
