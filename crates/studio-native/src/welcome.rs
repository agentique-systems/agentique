//! The start screen and the first run's welcome (R-46).
use crate::studio::Studio;
use gpui::{Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div};

pub struct Welcome {
    #[allow(dead_code)]
    studio: Entity<Studio>,
}

impl Welcome {
    pub fn new(studio: Entity<Studio>, cx: &mut Context<Self>) -> Self {
        let _ = cx;
        Welcome { studio }
    }
}

impl Render for Welcome {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child("Welcome")
    }
}
